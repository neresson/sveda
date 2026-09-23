use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use axum::extract::{Path as AxumPath, Query, State};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use chrono::Utc;
use serde::Deserialize;
use serde_json::{json, Value};
use sveda_index::{IndexSnapshot, WorkspaceIndex};
use sveda_store::Postgres;

use crate::admin::require_admin;
use crate::AppState;

const SAMPLE_LIMIT: usize = 12;
const MAX_FILES: usize = 20_000;

#[derive(Clone)]
pub struct CodeIndex {
    sources: Arc<Mutex<Vec<Source>>>,
    postgres: Arc<Mutex<Option<Postgres>>>,
    jobs: Arc<Mutex<std::collections::HashMap<i64, Arc<AtomicBool>>>>,
}

impl CodeIndex {
    pub fn memory() -> Self {
        Self {
            sources: Arc::new(Mutex::new(Vec::new())),
            postgres: Arc::new(Mutex::new(None)),
            jobs: Arc::new(Mutex::new(std::collections::HashMap::new())),
        }
    }

    pub fn bind_store(&self, postgres: Postgres) {
        *self.postgres.lock().expect("code store") = Some(postgres);
    }

    pub async fn load_stored(&self) {
        let postgres = self.postgres.lock().expect("code store").clone();
        let Some(postgres) = postgres else {
            return;
        };
        let Ok(rows) = postgres.list_code_sources().await else {
            return;
        };
        let mut loaded = Vec::new();
        for (id, document) in rows {
            let Some(mut source) = Source::from_document(id, document) else {
                continue;
            };
            if matches!(source.status.as_str(), "pending" | "indexing") {
                source.status = "failed".into();
                source.error_message = Some("indexing_interrupted".into());
                source.indexing_phase = None;
            }
            if source.status == "ready" && source.index.is_none() {
                source.status = "failed".into();
                source.error_message = Some("index_snapshot_missing".into());
            }
            loaded.push(source);
        }
        *self.sources.lock().expect("code sources") = loaded.clone();
        for source in &loaded {
            if matches!(
                source.error_message.as_deref(),
                Some("indexing_interrupted") | Some("index_snapshot_missing")
            ) {
                self.persist(source).await;
            }
        }
    }

    pub fn ready_indexes(&self) -> Vec<Arc<WorkspaceIndex>> {
        self.sources
            .lock()
            .expect("code sources")
            .iter()
            .filter(|source| source.status == "ready")
            .filter_map(|source| source.index.clone())
            .collect()
    }

    fn list_json(&self) -> Vec<Value> {
        self.sources
            .lock()
            .expect("code sources")
            .iter()
            .map(Source::api)
            .collect()
    }

    fn payload(&self, local_enabled: bool) -> Value {
        json!({
            "sources": self.list_json(),
            "localIndexingEnabled": local_enabled,
        })
    }

    async fn insert(&self, source: Source) -> Value {
        {
            self.sources
                .lock()
                .expect("code sources")
                .push(source.clone());
        }
        self.persist(&source).await;
        json!({
            "source": source.api(),
            "sources": self.list_json(),
        })
    }

    async fn persist(&self, source: &Source) {
        let postgres = self.postgres.lock().expect("code store").clone();
        let Some(postgres) = postgres else {
            return;
        };
        let _ = postgres
            .upsert_code_source(source.id, &source.document())
            .await;
    }

    async fn remove(&self, id: i64) -> bool {
        let removed = {
            let mut sources = self.sources.lock().expect("code sources");
            let before = sources.len();
            sources.retain(|source| source.id != id);
            before != sources.len()
        };
        if removed {
            self.cancel_flag(id).store(true, Ordering::Relaxed);
            let postgres = self.postgres.lock().expect("code store").clone();
            if let Some(postgres) = postgres {
                let _ = postgres.delete_code_source(id).await;
            }
        }
        removed
    }

    fn next_id(&self) -> i64 {
        self.sources
            .lock()
            .expect("code sources")
            .iter()
            .map(|source| source.id)
            .max()
            .unwrap_or(0)
            + 1
    }

    fn cancel_flag(&self, id: i64) -> Arc<AtomicBool> {
        self.jobs
            .lock()
            .expect("code jobs")
            .get(&id)
            .cloned()
            .unwrap_or_else(|| Arc::new(AtomicBool::new(false)))
    }

    fn cancelled(&self, id: i64) -> bool {
        self.cancel_flag(id).load(Ordering::Relaxed)
    }

    fn clear_job(&self, id: i64) {
        self.jobs.lock().expect("code jobs").remove(&id);
    }

    async fn queue(&self, id: i64) -> Result<(), &'static str> {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return Err("not_found");
            };
            if matches!(source.status.as_str(), "pending" | "indexing") {
                return Err("reindex_skipped_already_running");
            }
            if !source.workspace_ready {
                return Err("workspace_not_ready");
            }
            if source
                .local_path
                .as_ref()
                .map(|path| path.trim().is_empty())
                .unwrap_or(true)
            {
                return Err("workspace_not_resolved");
            }
            source.status = "pending".into();
            source.error_message = None;
            source.indexing_phase = Some("starting".into());
            source.indexing_progress = 4;
            source.chunks_embedded = None;
            source.index = None;
            source.updated_at = now();
            let stored = source.clone();
            self.jobs
                .lock()
                .expect("code jobs")
                .insert(id, Arc::new(AtomicBool::new(false)));
            stored
        };
        self.persist(&stored).await;
        Ok(())
    }

    fn workspace(&self, id: i64) -> Option<(PathBuf, Vec<String>)> {
        let sources = self.sources.lock().expect("code sources");
        let source = sources.iter().find(|source| source.id == id)?;
        let path = source.local_path.clone()?;
        Some((PathBuf::from(path), source.exclude_paths.clone()))
    }

    async fn mutate(&self, id: i64, update: impl FnOnce(&mut Source)) -> bool {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return false;
            };
            if source.status == "cancelled" {
                return false;
            }
            update(source);
            source.updated_at = now();
            source.clone()
        };
        self.persist(&stored).await;
        true
    }

    async fn mark_cancelled(&self, id: i64) {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return;
            };
            if !matches!(
                source.status.as_str(),
                "pending" | "indexing" | "configuring"
            ) {
                return;
            }
            source.status = "cancelled".into();
            source.error_message = Some("indexing_cancelled".into());
            source.indexing_phase = None;
            source.index = None;
            source.updated_at = now();
            source.clone()
        };
        self.persist(&stored).await;
        self.clear_job(id);
    }

    async fn fail(&self, id: i64, message: String) {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return;
            };
            if source.status == "cancelled" {
                return;
            }
            source.status = "failed".into();
            source.error_message = Some(message);
            source.indexing_phase = None;
            source.index = None;
            source.updated_at = now();
            source.clone()
        };
        self.persist(&stored).await;
        self.clear_job(id);
    }

    async fn finish_ready(
        &self,
        id: i64,
        index: Arc<WorkspaceIndex>,
        model: &str,
        files: u64,
        chunks: u64,
    ) {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return;
            };
            if source.status == "cancelled" {
                return;
            }
            source.status = "ready".into();
            source.error_message = None;
            source.indexing_phase = Some("up_to_date".into());
            source.indexing_progress = 100;
            source.files_indexed = Some(files);
            source.chunks_total = Some(chunks);
            source.chunks_embedded = Some(chunks);
            source.embedding_model = Some(model.to_string());
            source.last_indexed_at = Some(now());
            source.updated_at = now();
            source.index = Some(index);
            source.clone()
        };
        self.persist(&stored).await;
        self.clear_job(id);
    }

    fn find_api(&self, id: i64) -> Option<Value> {
        self.sources
            .lock()
            .expect("code sources")
            .iter()
            .find(|source| source.id == id)
            .map(Source::api)
    }

    fn scope(&self, id: i64, parent: &str) -> Result<Value, &'static str> {
        let sources = self.sources.lock().expect("code sources");
        let Some(source) = sources.iter().find(|source| source.id == id) else {
            return Err("not_found");
        };
        if !source.workspace_ready {
            return Ok(json!({
                "ready": false,
                "parent": parent,
                "entries": [],
                "exclude_paths": source.exclude_paths,
            }));
        }
        let Some(root) = source.local_path.as_deref() else {
            return Err("workspace_not_resolved");
        };
        let entries = sveda_index::list_scope(Path::new(root), parent)
            .map_err(|_| "workspace_not_resolved")?;
        Ok(json!({
            "ready": true,
            "parent": parent,
            "entries": entries.into_iter().map(|entry| json!({
                "kind": entry.kind,
                "name": entry.name,
                "path": entry.path,
            })).collect::<Vec<_>>(),
            "exclude_paths": source.exclude_paths,
        }))
    }

    fn estimate(
        &self,
        id: i64,
        excludes: &[String],
        usd_per_million: f64,
    ) -> Result<Value, &'static str> {
        let sources = self.sources.lock().expect("code sources");
        let Some(source) = sources.iter().find(|source| source.id == id) else {
            return Err("not_found");
        };
        if !source.workspace_ready {
            return Err("workspace_not_ready");
        }
        let Some(root) = source.local_path.as_deref() else {
            return Err("workspace_not_resolved");
        };
        Ok(footprint(Path::new(root), excludes, usd_per_million))
    }

    async fn apply_edit(
        &self,
        id: i64,
        name: String,
        description: String,
        exclude_paths: Vec<String>,
    ) -> Result<(), &'static str> {
        let stored = {
            let mut sources = self.sources.lock().expect("code sources");
            let Some(source) = sources.iter_mut().find(|source| source.id == id) else {
                return Err("not_found");
            };
            if !name.trim().is_empty() {
                source.name = name.trim().to_string();
            }
            source.description = description;
            source.exclude_paths = clean_excludes(&exclude_paths);
            source.updated_at = now();
            source.clone()
        };
        self.persist(&stored).await;
        Ok(())
    }
}

#[derive(Clone)]
struct Source {
    id: i64,
    name: String,
    description: String,
    provider: String,
    status: String,
    local_path: Option<String>,
    git_url: Option<String>,
    git_branch: Option<String>,
    exclude_paths: Vec<String>,
    workspace_ready: bool,
    error_message: Option<String>,
    last_indexed_at: Option<String>,
    updated_at: String,
    indexing_progress: u32,
    indexing_phase: Option<String>,
    chunks_total: Option<u64>,
    chunks_embedded: Option<u64>,
    files_indexed: Option<u64>,
    embedding_model: Option<String>,
    index: Option<Arc<WorkspaceIndex>>,
}

impl Source {
    fn api(&self) -> Value {
        json!({
            "id": self.id,
            "name": self.name,
            "description": self.description,
            "provider": self.provider,
            "status": self.status,
            "last_indexed_at": self.last_indexed_at,
            "metadata": self.metadata(),
            "error_message": self.error_message,
            "local_absolute_path": self.local_path,
            "git_remote_url": self.git_url,
            "git_branch": self.git_branch,
            "indexing_progress": self.indexing_progress,
            "indexing_phase": self.indexing_phase,
            "updated_at": self.updated_at,
        })
    }

    fn metadata(&self) -> Value {
        let mut metadata = json!({
            "exclude_paths": self.exclude_paths,
            "workspace_ready": self.workspace_ready,
        });
        if let Some(value) = self.chunks_total {
            metadata["chunks_total"] = json!(value);
        }
        if let Some(value) = self.chunks_embedded {
            metadata["chunks_embedded"] = json!(value);
        }
        if let Some(value) = self.files_indexed {
            metadata["files_indexed"] = json!(value);
        }
        if let Some(value) = &self.embedding_model {
            metadata["embedding_model"] = json!(value);
        }
        metadata
    }

    fn document(&self) -> Value {
        let mut document = self.api();
        if let Some(index) = &self.index {
            document["snapshot"] = serde_json::to_value(index.snapshot()).unwrap_or(Value::Null);
        }
        document
    }

    fn from_document(id: i64, document: Value) -> Option<Self> {
        let metadata = document
            .get("metadata")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let index = document
            .get("snapshot")
            .cloned()
            .and_then(|value| serde_json::from_value::<IndexSnapshot>(value).ok())
            .map(|snapshot| Arc::new(WorkspaceIndex::from_snapshot(snapshot)));
        Some(Self {
            id,
            name: document.get("name")?.as_str()?.to_string(),
            description: document
                .get("description")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            provider: document
                .get("provider")
                .and_then(Value::as_str)
                .unwrap_or("local")
                .to_string(),
            status: document
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("configuring")
                .to_string(),
            local_path: document
                .get("local_absolute_path")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            git_url: document
                .get("git_remote_url")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            git_branch: document
                .get("git_branch")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            exclude_paths: metadata
                .get("exclude_paths")
                .and_then(Value::as_array)
                .map(|items| {
                    items
                        .iter()
                        .filter_map(Value::as_str)
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default(),
            workspace_ready: metadata
                .get("workspace_ready")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            error_message: document
                .get("error_message")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            last_indexed_at: document
                .get("last_indexed_at")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            updated_at: document
                .get("updated_at")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string(),
            indexing_progress: document
                .get("indexing_progress")
                .and_then(Value::as_u64)
                .unwrap_or(0) as u32,
            indexing_phase: document
                .get("indexing_phase")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            chunks_total: metadata.get("chunks_total").and_then(Value::as_u64),
            chunks_embedded: metadata.get("chunks_embedded").and_then(Value::as_u64),
            files_indexed: metadata.get("files_indexed").and_then(Value::as_u64),
            embedding_model: metadata
                .get("embedding_model")
                .and_then(Value::as_str)
                .map(ToOwned::to_owned),
            index,
        })
    }
}

#[derive(Deserialize)]
pub(crate) struct CreateBody {
    name: String,
    description: Option<String>,
    provider: String,
    local_absolute_path: Option<String>,
    git_remote_url: Option<String>,
    git_branch: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct UpdateBody {
    name: Option<String>,
    description: Option<String>,
    exclude_paths: Option<Vec<String>>,
    start_indexing: Option<bool>,
}

#[derive(Deserialize)]
pub(crate) struct PathBody {
    path: String,
}

#[derive(Deserialize)]
pub(crate) struct BrowseQuery {
    path: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct ParentQuery {
    parent: Option<String>,
}

#[derive(Deserialize)]
pub(crate) struct EstimateBody {
    exclude_paths: Option<Vec<String>>,
}

pub async fn list_sources(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    Json(
        state
            .code_index
            .payload(state.config.code_index_allow_local),
    )
    .into_response()
}

pub async fn progress(State(state): State<AppState>, headers: HeaderMap) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    Json(json!({ "sources": state.code_index.list_json() })).into_response()
}

pub async fn create_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<CreateBody>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    let name = body.name.trim();
    if name.is_empty() {
        return message(StatusCode::UNPROCESSABLE_ENTITY, "name is required");
    }
    let provider = body.provider.trim();
    if provider == "github" {
        return message(
            StatusCode::UNPROCESSABLE_ENTITY,
            "GitHub sources are not available. Add a local folder.",
        );
    }
    if provider != "local" {
        return message(StatusCode::UNPROCESSABLE_ENTITY, "local_path_not_found");
    }
    let Ok(path) = resolve_dir(
        body.local_absolute_path.as_deref().unwrap_or(""),
        state.config.code_index_allow_local,
    ) else {
        let code = if state.config.code_index_allow_local {
            "local_path_not_found"
        } else {
            "local_paths_disabled_set_allow_local_paths_true"
        };
        return message(StatusCode::UNPROCESSABLE_ENTITY, code);
    };
    let source = Source {
        id: state.code_index.next_id(),
        name: name.to_string(),
        description: body.description.unwrap_or_default(),
        provider: "local".into(),
        status: "configuring".into(),
        local_path: Some(path.to_string_lossy().to_string()),
        git_url: body.git_remote_url,
        git_branch: body.git_branch,
        exclude_paths: Vec::new(),
        workspace_ready: true,
        error_message: None,
        last_indexed_at: None,
        updated_at: now(),
        indexing_progress: 0,
        indexing_phase: None,
        chunks_total: None,
        chunks_embedded: None,
        files_indexed: None,
        embedding_model: None,
        index: None,
    };
    (
        StatusCode::CREATED,
        Json(state.code_index.insert(source).await),
    )
        .into_response()
}

pub async fn update_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
    Json(body): Json<UpdateBody>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    let current = state.code_index.find_api(id);
    let Some(current) = current else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let name = body
        .name
        .unwrap_or_else(|| current["name"].as_str().unwrap_or("").to_string());
    let description = body
        .description
        .unwrap_or_else(|| current["description"].as_str().unwrap_or("").to_string());
    let exclude_paths = body.exclude_paths.unwrap_or_else(|| {
        current["metadata"]["exclude_paths"]
            .as_array()
            .map(|items| {
                items
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ToOwned::to_owned)
                    .collect()
            })
            .unwrap_or_default()
    });
    if let Err(error) = state
        .code_index
        .apply_edit(id, name, description, exclude_paths)
        .await
    {
        return message(StatusCode::NOT_FOUND, error);
    }
    if body.start_indexing.unwrap_or(false) {
        if let Err(error) = state.code_index.queue(id).await {
            let status = if error == "not_found" {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::UNPROCESSABLE_ENTITY
            };
            return message(status, error);
        }
        spawn_index(state.clone(), id);
    }
    Json(json!({ "sources": state.code_index.list_json() })).into_response()
}

pub async fn delete_source(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    if !state.code_index.remove(id).await {
        return StatusCode::NOT_FOUND.into_response();
    }
    Json(json!({ "sources": state.code_index.list_json() })).into_response()
}

pub async fn cancel_indexing(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    if state.code_index.find_api(id).is_none() {
        return StatusCode::NOT_FOUND.into_response();
    }
    state
        .code_index
        .cancel_flag(id)
        .store(true, Ordering::Relaxed);
    state.code_index.mark_cancelled(id).await;
    Json(json!({ "sources": state.code_index.list_json() })).into_response()
}

pub async fn reindex(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    if let Err(error) = state.code_index.queue(id).await {
        let status = if error == "not_found" {
            StatusCode::NOT_FOUND
        } else {
            StatusCode::UNPROCESSABLE_ENTITY
        };
        return message(status, error);
    }
    spawn_index(state.clone(), id);
    Json(json!({ "sources": state.code_index.list_json() })).into_response()
}

pub async fn scope(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
    Query(query): Query<ParentQuery>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    match state
        .code_index
        .scope(id, query.parent.as_deref().unwrap_or(""))
    {
        Ok(body) => Json(body).into_response(),
        Err("not_found") => StatusCode::NOT_FOUND.into_response(),
        Err(error) => message(StatusCode::UNPROCESSABLE_ENTITY, error),
    }
}

pub async fn estimate_footprint(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<i64>,
    Json(body): Json<EstimateBody>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    let rate = embedding_rate(&state);
    match state
        .code_index
        .estimate(id, body.exclude_paths.as_deref().unwrap_or(&[]), rate)
    {
        Ok(body) => Json(body).into_response(),
        Err("not_found") => StatusCode::NOT_FOUND.into_response(),
        Err(error) => message(StatusCode::UNPROCESSABLE_ENTITY, error),
    }
}

pub async fn local_browse(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<BrowseQuery>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    if !state.config.code_index_allow_local {
        return message(
            StatusCode::UNPROCESSABLE_ENTITY,
            "local_paths_disabled_set_allow_local_paths_true",
        );
    }
    let Some(path) = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return Json(json!({
            "current_path": null,
            "parent_path": null,
            "entries": [],
            "needs_anchor": true,
        }))
        .into_response();
    };
    let Ok(dir) = resolve_dir(path, true) else {
        return message(StatusCode::UNPROCESSABLE_ENTITY, "local_path_not_found");
    };
    let mut entries = Vec::new();
    if let Ok(read) = std::fs::read_dir(&dir) {
        for entry in read.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            entries.push(json!({
                "name": name,
                "path": entry.path().to_string_lossy(),
            }));
        }
    }
    entries.sort_by(|left, right| {
        left["name"]
            .as_str()
            .unwrap_or("")
            .to_ascii_lowercase()
            .cmp(&right["name"].as_str().unwrap_or("").to_ascii_lowercase())
    });
    let parent = dir
        .parent()
        .filter(|parent| *parent != dir)
        .map(|parent| parent.to_string_lossy().to_string());
    Json(json!({
        "current_path": dir.to_string_lossy(),
        "parent_path": parent,
        "entries": entries,
        "needs_anchor": false,
    }))
    .into_response()
}

pub async fn local_preview(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<PathBody>,
) -> Response {
    if let Err(status) = require_admin(&state, &headers) {
        return status.into_response();
    }
    let Ok(dir) = resolve_dir(&body.path, state.config.code_index_allow_local) else {
        let code = if state.config.code_index_allow_local {
            "local_path_not_found"
        } else {
            "local_paths_disabled_set_allow_local_paths_true"
        };
        return message(StatusCode::UNPROCESSABLE_ENTITY, code);
    };
    let collected = sveda_index::collect(&dir);
    let total = collected.file_count();
    let sample = collected.sample_paths(SAMPLE_LIMIT);
    Json(json!({
        "path": dir.to_string_lossy(),
        "total_files": total,
        "sample_paths": sample,
        "truncated_sample": total > SAMPLE_LIMIT,
        "hit_index_cap": total >= MAX_FILES,
        "max_files": MAX_FILES,
    }))
    .into_response()
}

fn spawn_index(state: AppState, id: i64) {
    tokio::spawn(async move {
        run_index(state, id).await;
    });
}

async fn run_index(state: AppState, id: i64) {
    let Some((root, excludes)) = state.code_index.workspace(id) else {
        return;
    };
    if state.code_index.cancelled(id) {
        state.code_index.mark_cancelled(id).await;
        return;
    }
    if !state
        .code_index
        .mutate(id, |source| {
            source.status = "indexing".into();
            source.indexing_phase = Some("scan_files".into());
            source.indexing_progress = 20;
        })
        .await
    {
        return;
    }
    let collected = sveda_index::collect(&root).exclude(&excludes);
    let files = collected.file_count() as u64;
    let chunks = collected.chunk_count() as u64;
    if state.code_index.cancelled(id) {
        state.code_index.mark_cancelled(id).await;
        return;
    }
    if !state
        .code_index
        .mutate(id, |source| {
            source.indexing_phase = Some("build_chunks".into());
            source.indexing_progress = 45;
            source.files_indexed = Some(files);
            source.chunks_total = Some(chunks);
            source.chunks_embedded = Some(0);
        })
        .await
    {
        return;
    }
    let spec = state
        .settings
        .document()
        .active_embedding()
        .filter(|spec| !spec.key.trim().is_empty());
    let dims = spec
        .as_ref()
        .map(|spec| spec.dimensions as usize)
        .unwrap_or(sveda_index::EMBED_DIMS)
        .max(8);
    let model = spec
        .as_ref()
        .map(|spec| spec.api_model.clone())
        .unwrap_or_else(|| "local-hash".into());
    let texts = collected.embed_sources();
    let mut vectors = Vec::with_capacity(texts.len());
    if !texts.is_empty()
        && !state
            .code_index
            .mutate(id, |source| {
                source.indexing_phase = Some("embedding".into());
                source.indexing_progress = 74;
            })
            .await
    {
        return;
    }
    for batch in texts.chunks(sveda_llm::EMBED_BATCH_SIZE) {
        if state.code_index.cancelled(id) {
            state.code_index.mark_cancelled(id).await;
            return;
        }
        let part = if let Some(spec) = &spec {
            match state.embed_http.embed(spec, batch).await {
                Ok(part) => part,
                Err(error) => {
                    state.code_index.fail(id, error.to_string()).await;
                    return;
                }
            }
        } else {
            batch
                .iter()
                .map(|text| sveda_index::embed_text(text, dims))
                .collect()
        };
        vectors.extend(part);
        let embedded = vectors.len() as u64;
        if !state
            .code_index
            .mutate(id, |source| {
                source.chunks_embedded = Some(embedded);
            })
            .await
        {
            return;
        }
    }
    let embedding_id = spec.map(|spec| spec.id);
    match collected.finish(vectors, embedding_id) {
        Ok(index) => {
            state
                .code_index
                .finish_ready(id, Arc::new(index), &model, files, chunks)
                .await;
        }
        Err(error) => state.code_index.fail(id, error).await,
    }
}

fn footprint(root: &Path, excludes: &[String], usd_per_million: f64) -> Value {
    let collected = sveda_index::collect(root).exclude(excludes);
    let tokens = collected.embed_char_count() as f64 / 3.5;
    let cost = tokens / 1_000_000.0 * usd_per_million;
    json!({
        "files_indexable": collected.file_count(),
        "chunks_total": collected.chunk_count(),
        "estimated_cost_usd": (cost * 10_000.0).round() / 10_000.0,
        "rate_usd_per_million_tokens": usd_per_million,
    })
}

fn embedding_rate(state: &AppState) -> f64 {
    state
        .settings
        .document()
        .active_embedding()
        .map(|spec| spec.usd_per_million)
        .filter(|rate| *rate > 0.0)
        .unwrap_or(0.02)
}

fn resolve_dir(path: &str, allow_local: bool) -> Result<PathBuf, ()> {
    if !allow_local {
        return Err(());
    }
    let path = path.trim();
    let path = Path::new(path);
    if !path.is_absolute() {
        return Err(());
    }
    let canon = std::fs::canonicalize(path).map_err(|_| ())?;
    if !canon.is_dir() {
        return Err(());
    }
    Ok(canon)
}

fn clean_excludes(paths: &[String]) -> Vec<String> {
    let mut cleaned = Vec::new();
    for path in paths {
        let path = path.trim().replace('\\', "/");
        let path = path.trim_matches('/');
        if path.is_empty()
            || path
                .split('/')
                .any(|part| part.is_empty() || part == "." || part == "..")
        {
            continue;
        }
        if !cleaned.iter().any(|existing: &String| existing == path) {
            cleaned.push(path.to_string());
        }
    }
    cleaned
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn message(status: StatusCode, message: &str) -> Response {
    (status, Json(json!({ "message": message }))).into_response()
}
