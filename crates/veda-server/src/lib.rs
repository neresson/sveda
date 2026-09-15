use std::collections::HashMap;
use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{DefaultBodyLimit, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::response::sse::Sse;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;
use veda_llm::{Catalog, HttpClient, LlmClient, ScriptedClient};
use veda_protocol::{
    EmbedTokenRequest, MessageResponse, StreamEvent, StreamRequest, HEADER_ACCEL_BUFFERING,
    HEADER_ADMIN_KEY, HEADER_EMBED_TOKEN, HEADER_HOST_KEY, HEADER_PROTOCOL_VERSION,
    PROTOCOL_VERSION, TOKEN_PREFIX,
};
use veda_store::{
    parse_laravel_throttle, MemoryHistoryStore, Occupancy, OccupancyError, RateLimiter,
};

use settings::SettingsStore;

mod admin;
mod documents;
mod histories;
mod settings;
mod token;
mod turn;
mod ui;

#[derive(Clone, Debug)]
pub struct Config {
    pub embed_enabled: bool,
    pub host_api_key: Option<String>,
    pub token_ttl_seconds: u64,
    pub hmac_key: Vec<u8>,
    pub cors_origins: Vec<String>,
    pub max_steps: u32,
    pub context_max_tokens: u64,
    pub stream_timeout: u64,
    pub system_prompt: String,
    pub mcp_timeout: u64,
    pub defer_enabled: bool,
    pub defer_min_pool: usize,
    pub title_generation_enabled: bool,
    pub compaction_enabled: bool,
    pub compaction_min_messages: usize,
    pub compaction_keep_tail: usize,
    pub document_max_files: usize,
    pub document_max_file_kb: usize,
    pub document_max_total_chars: usize,
    pub admin_api_key: Option<String>,
    pub occupancy_global: usize,
    pub occupancy_per_visitor: usize,
    pub embed_throttle_max: u32,
    pub embed_throttle_window_secs: u64,
    pub redis_url: Option<String>,
    pub index_root: Option<PathBuf>,
}

impl Config {
    pub fn test() -> Self {
        Self {
            embed_enabled: true,
            host_api_key: None,
            token_ttl_seconds: 3600,
            hmac_key: b"veda-test-app-key".to_vec(),
            cors_origins: Vec::new(),
            max_steps: 30,
            context_max_tokens: 128000,
            stream_timeout: 1800,
            system_prompt: String::new(),
            mcp_timeout: 30,
            defer_enabled: true,
            defer_min_pool: 14,
            title_generation_enabled: true,
            compaction_enabled: true,
            compaction_min_messages: 40,
            compaction_keep_tail: 20,
            document_max_files: 5,
            document_max_file_kb: 40960,
            document_max_total_chars: 150000,
            admin_api_key: None,
            occupancy_global: 0,
            occupancy_per_visitor: 0,
            embed_throttle_max: 0,
            embed_throttle_window_secs: 60,
            redis_url: None,
            index_root: None,
        }
    }

    pub fn from_env() -> Self {
        let host_api_key = std::env::var("VEDA_EMBED_HOST_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let cors_origins = std::env::var("VEDA_CORS_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        let hmac_key = std::env::var("VEDA_APP_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| decode_app_key(&value))
            .unwrap_or_else(|| b"veda-dev-key".to_vec());

        Self {
            embed_enabled: env_flag("VEDA_EMBED_ENABLED", false),
            host_api_key,
            token_ttl_seconds: std::env::var("VEDA_EMBED_TOKEN_TTL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(3600),
            hmac_key,
            cors_origins,
            max_steps: std::env::var("VEDA_AGENT_MAX_STEPS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30)
                .max(1),
            context_max_tokens: std::env::var("VEDA_CONTEXT_MAX_TOKENS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(128000)
                .max(1),
            stream_timeout: std::env::var("VEDA_STREAM_TIMEOUT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1800)
                .max(1),
            system_prompt: std::env::var("VEDA_SYSTEM_PROMPT").unwrap_or_default(),
            mcp_timeout: std::env::var("VEDA_HOST_MCP_TIMEOUT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30)
                .max(1),
            defer_enabled: env_flag("VEDA_TOOL_DEFER_ENABLED", true),
            defer_min_pool: std::env::var("VEDA_TOOL_DEFER_MIN_POOL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(14)
                .max(1),
            title_generation_enabled: env_flag("VEDA_TITLE_GENERATION_ENABLED", true),
            compaction_enabled: env_flag("VEDA_COMPACTION_ENABLED", true),
            compaction_min_messages: std::env::var("VEDA_COMPACTION_MIN_MESSAGES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(40)
                .max(1),
            compaction_keep_tail: std::env::var("VEDA_COMPACTION_KEEP_TAIL_MESSAGES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(20)
                .max(1),
            document_max_files: std::env::var("VEDA_DOCUMENT_MAX_FILES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5)
                .max(1),
            document_max_file_kb: std::env::var("VEDA_DOCUMENT_MAX_FILE_KB")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(40960)
                .max(1),
            document_max_total_chars: std::env::var("VEDA_DOCUMENT_MAX_TOTAL_CHARS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(150000)
                .max(1),
            admin_api_key: std::env::var("VEDA_ADMIN_API_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            occupancy_global: std::env::var("VEDA_OCCUPANCY_GLOBAL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            occupancy_per_visitor: std::env::var("VEDA_OCCUPANCY_PER_VISITOR")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            embed_throttle_max: {
                let (max, _) = parse_laravel_throttle(
                    &std::env::var("VEDA_EMBED_TOKEN_THROTTLE").unwrap_or_else(|_| "30,1".into()),
                );
                max
            },
            embed_throttle_window_secs: {
                let (_, window) = parse_laravel_throttle(
                    &std::env::var("VEDA_EMBED_TOKEN_THROTTLE").unwrap_or_else(|_| "30,1".into()),
                );
                window
            },
            redis_url: std::env::var("VEDA_REDIS_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            index_root: std::env::var("VEDA_INDEX_ROOT")
                .ok()
                .map(|value| PathBuf::from(value.trim()))
                .filter(|path| !path.as_os_str().is_empty()),
        }
    }
}

fn env_flag(name: &str, default: bool) -> bool {
    match std::env::var(name) {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => default,
    }
}

fn decode_app_key(value: &str) -> Vec<u8> {
    value
        .strip_prefix("base64:")
        .and_then(|encoded| {
            use base64::Engine;
            base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .ok()
        })
        .unwrap_or_else(|| value.as_bytes().to_vec())
}

#[derive(Clone, Debug)]
pub struct HostMcpCreds {
    pub url: String,
    pub token: String,
}

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Config,
    pub(crate) catalog: Arc<Mutex<Catalog>>,
    pub(crate) llm: Arc<dyn LlmClient>,
    mcp: Arc<Mutex<HashMap<String, HostMcpCreds>>>,
    pub(crate) store: MemoryHistoryStore,
    pub(crate) settings: SettingsStore,
    pub(crate) cors_origins: Arc<Mutex<Vec<String>>>,
    pub occupancy: Occupancy,
    pub(crate) throttle: RateLimiter,
    pub(crate) index: Option<Arc<veda_index::WorkspaceIndex>>,
    pub(crate) admin_key: Arc<Mutex<Option<String>>>,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self::with_llm(config, Arc::new(ScriptedClient::hello_world()))
    }

    pub fn live(config: Config) -> Self {
        let timeout = config.stream_timeout;
        let catalog = Catalog::from_env();
        Self::compose(config, catalog, Arc::new(HttpClient::from_timeout(timeout)))
    }

    pub fn with_llm(config: Config, llm: Arc<dyn LlmClient>) -> Self {
        let catalog = Catalog::builtin("");
        Self::compose(config, catalog, llm)
    }

    fn compose(config: Config, catalog: Catalog, llm: Arc<dyn LlmClient>) -> Self {
        let settings =
            SettingsStore::new(settings::SettingsDocument::from_runtime(&config, &catalog));
        let cors_origins = Arc::new(Mutex::new(config.cors_origins.clone()));
        let occupancy = Occupancy::from_config(
            config.redis_url.as_deref(),
            config.occupancy_global,
            config.occupancy_per_visitor,
        );
        let throttle = RateLimiter::from_config(
            config.redis_url.as_deref(),
            config.embed_throttle_max,
            Duration::from_secs(config.embed_throttle_window_secs.max(1)),
        );
        let index = config
            .index_root
            .as_ref()
            .filter(|path| path.is_dir())
            .map(veda_index::prepare)
            .map(Arc::new);
        let admin_key = Arc::new(Mutex::new(config.admin_api_key.clone()));
        Self {
            catalog: Arc::new(Mutex::new(catalog)),
            llm,
            config,
            mcp: Arc::new(Mutex::new(HashMap::new())),
            store: MemoryHistoryStore::new(),
            settings,
            cors_origins,
            occupancy,
            throttle,
            index,
            admin_key,
        }
    }

    pub fn store(&self) -> &MemoryHistoryStore {
        &self.store
    }

    pub(crate) fn catalog(&self) -> Catalog {
        self.catalog.lock().expect("catalog").clone()
    }

    pub(crate) fn runtime_config(&self) -> Config {
        admin::overlay(&self.config, &self.settings)
    }

    pub fn host_mcp(&self, visitor_id: &str) -> Option<HostMcpCreds> {
        self.mcp.lock().expect("mcp lock").get(visitor_id).cloned()
    }
}

pub fn app(state: AppState) -> Router {
    let cors_origins = state.cors_origins.clone();
    let extract_limit = state
        .config
        .document_max_file_kb
        .saturating_mul(1024)
        .saturating_mul(state.config.document_max_files.max(1))
        .saturating_add(1024 * 1024)
        .max(2 * 1024 * 1024);
    let mut router = Router::new()
        .route("/veda/health", get(ui::health))
        .route("/veda/embed/token", post(issue_embed_token))
        .route("/veda/embed/config", get(admin::embed_config))
        .route("/veda/stream", post(stream_chat))
        .route("/veda/message", post(json_message))
        .route("/veda/chat-histories", get(histories::list_histories))
        .route(
            "/veda/chat-histories/{chat_id}",
            get(histories::show_history)
                .patch(histories::update_history)
                .delete(histories::delete_history),
        )
        .route(
            "/veda/documents/extract",
            post(documents::extract_documents),
        )
        .route(
            "/veda/admin/settings",
            get(admin::show_settings)
                .put(admin::update_settings)
                .post(admin::update_settings),
        )
        .route("/veda/admin/login", post(ui::login))
        .route("/veda/admin/setup", post(ui::setup))
        .route("/veda/admin/logout", post(ui::logout))
        .route("/veda/admin", get(ui::show))
        .route("/veda/admin/{page}", get(ui::section));
    let dist = ui::admin_dist();
    if dist.is_dir() {
        router = router.nest_service("/build", ServeDir::new(dist));
    }
    router
        .layer(DefaultBodyLimit::max(extract_limit))
        .layer(cors_layer(cors_origins))
        .with_state(state)
}

fn cors_layer(origins: Arc<Mutex<Vec<String>>>) -> CorsLayer {
    let allow_origin = AllowOrigin::predicate(
        move |origin: &HeaderValue, _request: &axum::http::request::Parts| {
            let list = origins.lock().expect("cors");
            if list.iter().any(|item| item == "*") {
                return true;
            }
            origin
                .to_str()
                .ok()
                .map(|value| list.iter().any(|item| item == value))
                .unwrap_or(false)
        },
    );

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods(AllowMethods::list([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
            Method::OPTIONS,
        ]))
        .allow_headers(AllowHeaders::list([
            header::ACCEPT,
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            HeaderName::from_static("x-requested-with"),
            HeaderName::from_static(HEADER_EMBED_TOKEN),
            HeaderName::from_static(HEADER_HOST_KEY),
            HeaderName::from_static(HEADER_ADMIN_KEY),
            HeaderName::from_static("x-veda-protocol"),
        ]))
}

async fn issue_embed_token(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<EmbedTokenRequest>,
) -> Response {
    if !state.config.embed_enabled {
        return StatusCode::NOT_FOUND.into_response();
    }
    if let Err(retry_after) = state.throttle.hit("embed-token") {
        return throttle_rejected(retry_after);
    }
    if !host_key_allowed(&state.config, &headers) {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let mcp_url = request
        .host_mcp_url
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    let mcp_token = request
        .host_mcp_token
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if mcp_url.is_some() != mcp_token.is_some() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "host_mcp_url and host_mcp_token are required together" })),
        )
            .into_response();
    }
    if let Some(url) = mcp_url {
        if !is_http_url(url) {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "message": "host_mcp_url must be a valid URL" })),
            )
                .into_response();
        }
    }

    let mut visitor_id = request.visitor_id.unwrap_or_default().trim().to_string();
    if visitor_id.is_empty() {
        visitor_id = Uuid::new_v4().to_string();
    }
    if visitor_id.len() > 64 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "visitor_id is too long" })),
        )
            .into_response();
    }

    let ttl = state.config.token_ttl_seconds.max(60);
    let token = token::issue(&state.config.hmac_key, &visitor_id, ttl);

    if let (Some(url), Some(mcp_token), true) =
        (mcp_url, mcp_token, state.config.host_api_key.is_some())
    {
        state.mcp.lock().expect("mcp lock").insert(
            visitor_id.clone(),
            HostMcpCreds {
                url: url.to_string(),
                token: mcp_token.to_string(),
            },
        );
    }

    Json(veda_protocol::EmbedTokenResponse {
        token,
        visitor_id,
        expires_in: ttl,
    })
    .into_response()
}

async fn stream_chat(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<StreamRequest>,
) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    let lease = match state.occupancy.acquire(&visitor_id) {
        Ok(lease) => lease,
        Err(error) => return occupancy_rejected(error),
    };
    let events = match turn::run_turn(state.clone(), visitor_id, request).await {
        Ok(events) => events,
        Err(response) => return *response,
    };
    let sse = async_stream::stream! {
        let _lease = lease;
        let mut events = events;
        while let Some(event) = events.next().await {
            yield turn::sse_event(&event);
        }
        yield Ok::<_, Infallible>(turn::sse_done());
    };

    let mut response = Sse::new(sse).into_response();
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-cache"));
    headers.insert(
        HeaderName::from_static(HEADER_ACCEL_BUFFERING),
        HeaderValue::from_static("no"),
    );
    headers.insert(
        HeaderName::from_static(HEADER_PROTOCOL_VERSION),
        HeaderValue::from_static(PROTOCOL_VERSION),
    );
    response
}

async fn json_message(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(request): Json<StreamRequest>,
) -> Response {
    let visitor_id = match visitor_from(&state, &headers) {
        Ok(visitor_id) => visitor_id,
        Err(status) => return status.into_response(),
    };
    let _lease = match state.occupancy.acquire(&visitor_id) {
        Ok(lease) => lease,
        Err(error) => return occupancy_rejected(error),
    };
    let fallback_chat_id = request
        .chat_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let events = match turn::run_turn(state, visitor_id, request).await {
        Ok(events) => events,
        Err(response) => return *response,
    };
    let mut explanation = String::new();
    let mut tokens_used = 0u64;
    let mut chat_id = fallback_chat_id.unwrap_or_default();
    let mut errored: Option<String> = None;
    let mut events = events;
    while let Some(event) = events.next().await {
        match event {
            StreamEvent::TextDelta { delta, .. } => explanation.push_str(&delta),
            StreamEvent::MessageEnd {
                usage,
                chat_id: event_chat,
                ..
            } => {
                tokens_used = usage.and_then(|usage| usage.total_tokens).unwrap_or(0);
                if let Some(id) = event_chat.filter(|value| !value.is_empty()) {
                    chat_id = id;
                }
            }
            StreamEvent::Error { message, .. } => errored = Some(message),
            _ => {}
        }
    }
    if explanation.is_empty() {
        if let Some(message) = errored {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({ "message": message })),
            )
                .into_response();
        }
    }
    Json(MessageResponse {
        explanation,
        tokens_used,
        chat_id,
    })
    .into_response()
}

fn occupancy_rejected(error: OccupancyError) -> Response {
    (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({ "message": error.message() })),
    )
        .into_response()
}

fn throttle_rejected(retry_after: u64) -> Response {
    let mut response = (
        StatusCode::TOO_MANY_REQUESTS,
        Json(serde_json::json!({ "message": "Too Many Attempts." })),
    )
        .into_response();
    if let Ok(value) = HeaderValue::from_str(&retry_after.max(1).to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

pub(crate) fn visitor_from(state: &AppState, headers: &HeaderMap) -> Result<String, StatusCode> {
    let Some(embed_token) = extract_embed_token(headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    token::validate(&state.config.hmac_key, &embed_token).ok_or(StatusCode::UNAUTHORIZED)
}

fn extract_embed_token(headers: &HeaderMap) -> Option<String> {
    if let Some(value) = header_str(headers, HEADER_EMBED_TOKEN) {
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return Some(trimmed.to_string());
        }
    }

    header_str(headers, header::AUTHORIZATION.as_str()).and_then(|value| {
        value
            .strip_prefix("Bearer ")
            .map(str::trim)
            .filter(|token| token.starts_with(TOKEN_PREFIX))
            .map(ToOwned::to_owned)
    })
}

fn host_key_allowed(config: &Config, headers: &HeaderMap) -> bool {
    let Some(expected) = config
        .host_api_key
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return true;
    };

    let provided = header_str(headers, HEADER_HOST_KEY)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| {
            header_str(headers, header::AUTHORIZATION.as_str()).and_then(|value| {
                value
                    .strip_prefix("Bearer ")
                    .map(str::trim)
                    .filter(|token| !token.is_empty() && !token.starts_with(TOKEN_PREFIX))
                    .map(ToOwned::to_owned)
            })
        });

    match provided {
        Some(value) => keys_match(expected, &value),
        None => false,
    }
}

fn header_str<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

pub(crate) fn keys_match(expected: &str, provided: &str) -> bool {
    if expected.len() != provided.len() {
        return false;
    }
    expected
        .bytes()
        .zip(provided.bytes())
        .fold(0u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

fn is_http_url(value: &str) -> bool {
    (value.starts_with("http://") || value.starts_with("https://"))
        && value.len() <= 2048
        && !value.chars().any(char::is_whitespace)
}

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_secs()
}
