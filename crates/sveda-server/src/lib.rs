use std::convert::Infallible;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::{DefaultBodyLimit, Path, Request, State};
use axum::http::{header, HeaderMap, HeaderName, HeaderValue, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::sse::Sse;
use axum::response::{IntoResponse, Redirect, Response};
use axum::routing::{get, patch, post};
use axum::{Json, Router};
use futures_util::StreamExt;
use sveda_llm::{Catalog, HttpClient, LlmChunk, LlmClient, ScriptedClient};
use sveda_protocol::{
    EmbedTokenRequest, MessageResponse, StreamEvent, StreamRequest, ADMIN_VISITOR_ID,
    HEADER_ACCEL_BUFFERING, HEADER_ADMIN_KEY, HEADER_EMBED_TOKEN, HEADER_HOST_KEY,
    HEADER_PROTOCOL_VERSION, PROTOCOL_VERSION, TOKEN_PREFIX,
};
use sveda_store::{
    mcp_key, parse_laravel_throttle, parse_optional_laravel_throttle, DocumentStore, HistoryStore,
    KvStore, Occupancy, OccupancyError, Postgres, RateLimiter, RedisClient, UsageStore,
};
use tower_http::cors::{AllowHeaders, AllowMethods, AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;
use uuid::Uuid;

use settings::SettingsStore;

mod admin;
#[cfg(test)]
mod admin_operator_tests;
#[cfg(test)]
mod tool_confirmation_tests;
mod admin_tools;
mod code_index;
mod documents;
mod dotenv;
mod histories;
pub mod policy;
mod settings;
mod token;
mod turn;
mod ui;
mod web;

pub use dotenv::load_runtime_env;

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
    pub web_enabled: bool,
    pub web_timeout: u64,
    pub searxng_url: String,
    pub admin_api_key: Option<String>,
    pub occupancy_global: usize,
    pub occupancy_per_visitor: usize,
    pub embed_throttle_max: u32,
    pub embed_throttle_window_secs: u64,
    pub stream_throttle_max: u32,
    pub stream_throttle_window_secs: u64,
    pub ip_throttle_max: u32,
    pub ip_throttle_window_secs: u64,
    pub client_ip_header: String,
    pub redis_url: Option<String>,
    pub database_url: Option<String>,
    pub index_root: Option<PathBuf>,
    pub code_index_allow_local: bool,
}

impl Config {
    pub fn test() -> Self {
        Self {
            embed_enabled: true,
            host_api_key: None,
            token_ttl_seconds: 3600,
            hmac_key: b"sveda-test-app-key".to_vec(),
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
            web_enabled: true,
            web_timeout: 20,
            searxng_url: String::new(),
            admin_api_key: None,
            occupancy_global: 0,
            occupancy_per_visitor: 0,
            embed_throttle_max: 0,
            embed_throttle_window_secs: 60,
            stream_throttle_max: 0,
            stream_throttle_window_secs: 60,
            ip_throttle_max: 0,
            ip_throttle_window_secs: 60,
            client_ip_header: String::new(),
            redis_url: None,
            database_url: None,
            index_root: None,
            code_index_allow_local: true,
        }
    }

    pub fn from_env() -> Self {
        let host_api_key = std::env::var("SVEDA_EMBED_HOST_API_KEY")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty());
        let cors_origins = std::env::var("SVEDA_CORS_ORIGINS")
            .unwrap_or_default()
            .split(',')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        let hmac_key = std::env::var("SVEDA_APP_KEY")
            .ok()
            .filter(|value| !value.is_empty())
            .map(|value| decode_app_key(&value))
            .unwrap_or_else(|| b"sveda-dev-key".to_vec());

        Self {
            embed_enabled: env_flag("SVEDA_EMBED_ENABLED", false),
            host_api_key,
            token_ttl_seconds: std::env::var("SVEDA_EMBED_TOKEN_TTL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(3600),
            hmac_key,
            cors_origins,
            max_steps: std::env::var("SVEDA_AGENT_MAX_STEPS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30)
                .max(1),
            context_max_tokens: std::env::var("SVEDA_CONTEXT_MAX_TOKENS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(128000)
                .max(1),
            stream_timeout: std::env::var("SVEDA_STREAM_TIMEOUT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(1800)
                .max(1),
            system_prompt: std::env::var("SVEDA_SYSTEM_PROMPT").unwrap_or_default(),
            mcp_timeout: std::env::var("SVEDA_HOST_MCP_TIMEOUT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(30)
                .max(1),
            defer_enabled: env_flag("SVEDA_TOOL_DEFER_ENABLED", true),
            defer_min_pool: std::env::var("SVEDA_TOOL_DEFER_MIN_POOL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(14)
                .max(1),
            title_generation_enabled: env_flag("SVEDA_TITLE_GENERATION_ENABLED", true),
            compaction_enabled: env_flag("SVEDA_COMPACTION_ENABLED", true),
            compaction_min_messages: std::env::var("SVEDA_COMPACTION_MIN_MESSAGES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(40)
                .max(1),
            compaction_keep_tail: std::env::var("SVEDA_COMPACTION_KEEP_TAIL_MESSAGES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(20)
                .max(1),
            document_max_files: std::env::var("SVEDA_DOCUMENT_MAX_FILES")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(5)
                .max(1),
            document_max_file_kb: std::env::var("SVEDA_DOCUMENT_MAX_FILE_KB")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(40960)
                .max(1),
            document_max_total_chars: std::env::var("SVEDA_DOCUMENT_MAX_TOTAL_CHARS")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(150000)
                .max(1),
            web_enabled: env_flag("SVEDA_WEB_ENABLED", true),
            web_timeout: std::env::var("SVEDA_WEB_TIMEOUT")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(20)
                .max(1),
            searxng_url: std::env::var("SVEDA_SEARXNG_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .unwrap_or_default(),
            admin_api_key: std::env::var("SVEDA_ADMIN_API_KEY")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            occupancy_global: std::env::var("SVEDA_OCCUPANCY_GLOBAL")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            occupancy_per_visitor: std::env::var("SVEDA_OCCUPANCY_PER_VISITOR")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            embed_throttle_max: {
                let (max, _) = parse_laravel_throttle(
                    &std::env::var("SVEDA_EMBED_TOKEN_THROTTLE").unwrap_or_else(|_| "30,1".into()),
                );
                max
            },
            embed_throttle_window_secs: {
                let (_, window) = parse_laravel_throttle(
                    &std::env::var("SVEDA_EMBED_TOKEN_THROTTLE").unwrap_or_else(|_| "30,1".into()),
                );
                window
            },
            stream_throttle_max: {
                let (max, _) = parse_optional_laravel_throttle(
                    &std::env::var("SVEDA_STREAM_THROTTLE").unwrap_or_default(),
                );
                max
            },
            stream_throttle_window_secs: {
                let (_, window) = parse_optional_laravel_throttle(
                    &std::env::var("SVEDA_STREAM_THROTTLE").unwrap_or_default(),
                );
                window
            },
            ip_throttle_max: {
                let (max, _) = parse_optional_laravel_throttle(
                    &std::env::var("SVEDA_IP_THROTTLE").unwrap_or_default(),
                );
                max
            },
            ip_throttle_window_secs: {
                let (_, window) = parse_optional_laravel_throttle(
                    &std::env::var("SVEDA_IP_THROTTLE").unwrap_or_default(),
                );
                window
            },
            client_ip_header: settings::sanitize_ip_header(
                &std::env::var("SVEDA_CLIENT_IP_HEADER").unwrap_or_default(),
            ),
            redis_url: std::env::var("SVEDA_REDIS_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            database_url: std::env::var("SVEDA_DATABASE_URL")
                .ok()
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            index_root: std::env::var("SVEDA_INDEX_ROOT")
                .ok()
                .map(|value| PathBuf::from(value.trim()))
                .filter(|path| !path.as_os_str().is_empty()),
            code_index_allow_local: env_flag("SVEDA_CODE_INDEX_ALLOW_LOCAL_PATHS", true),
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
    mcp: KvStore,
    pub(crate) store: HistoryStore,
    pub(crate) usage: UsageStore,
    pub(crate) settings: SettingsStore,
    pub(crate) cors_origins: Arc<Mutex<Vec<String>>>,
    pub occupancy: Occupancy,
    pub(crate) throttle: RateLimiter,
    pub(crate) stream_throttle: RateLimiter,
    pub(crate) ip_throttle: RateLimiter,
    pub(crate) client_ip_header: Arc<Mutex<String>>,
    pub(crate) index: Arc<Mutex<Option<Arc<sveda_index::WorkspaceIndex>>>>,
    pub(crate) index_embed_fp: Arc<Mutex<String>>,
    pub(crate) embed_http: HttpClient,
    pub(crate) admin_key: Arc<Mutex<Option<String>>>,
    pub(crate) code_index: code_index::CodeIndex,
}

impl AppState {
    pub fn new(config: Config) -> Self {
        Self::with_llm(config, Arc::new(ScriptedClient::hello_world()))
    }

    pub fn live(config: Config) -> impl std::future::Future<Output = Self> + Send {
        async move { Self::live_inner(config).await }
    }

    pub fn with_llm(config: Config, llm: Arc<dyn LlmClient>) -> Self {
        let catalog = Catalog::builtin("");
        Self::compose(config, catalog, llm, None, None)
    }

    async fn live_inner(config: Config) -> Self {
        if config.database_url.is_some() && config.admin_api_key.is_none() {
            panic!("SVEDA_ADMIN_API_KEY is required when SVEDA_DATABASE_URL is set");
        }
        let redis = match config.redis_url.as_deref() {
            Some(url) => Some(
                RedisClient::connect(url)
                    .unwrap_or_else(|error| panic!("SVEDA_REDIS_URL: {error}")),
            ),
            None => None,
        };
        let postgres = match config.database_url.as_deref() {
            Some(url) => {
                let postgres = Postgres::connect(url)
                    .await
                    .unwrap_or_else(|error| panic!("SVEDA_DATABASE_URL: {error}"));
                postgres
                    .migrate()
                    .await
                    .unwrap_or_else(|error| panic!("migrate: {error}"));
                Some(postgres)
            }
            None => None,
        };
        let timeout = config.stream_timeout;
        let catalog = Catalog::from_env();
        let sources_db = postgres.clone();
        let llm: Arc<dyn LlmClient> = match std::env::var("SVEDA_LLM_CLIENT")
            .ok()
            .map(|value| value.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("scripted") => Arc::new(compat_scripted_llm()),
            _ => Arc::new(HttpClient::from_timeout(timeout)),
        };
        let state = Self::compose(config, catalog, llm, postgres, redis);
        if let Some(postgres) = sources_db {
            state.code_index.bind_store(postgres);
            state.code_index.load_stored().await;
        }
        let seed = state.settings.document();
        let loaded = state.settings.load_or_seed(seed).await;
        admin::apply_runtime(&state, &loaded);
        admin::refresh_workspace_index(&state).await;
        if state.settings.is_shared() {
            let cloned = state.clone();
            tokio::spawn(async move {
                let mut interval = tokio::time::interval(Duration::from_secs(2));
                interval.tick().await;
                loop {
                    interval.tick().await;
                    cloned.refresh_settings().await;
                }
            });
        }
        state
    }

    fn compose(
        config: Config,
        catalog: Catalog,
        llm: Arc<dyn LlmClient>,
        postgres: Option<Postgres>,
        redis: Option<RedisClient>,
    ) -> Self {
        let documents = match postgres.clone() {
            Some(postgres) => DocumentStore::postgres(postgres),
            None => DocumentStore::memory(),
        };
        let kv = KvStore::connect(redis.clone());
        let seed = settings::SettingsDocument::from_runtime(&config, &catalog);
        let settings = SettingsStore::with_backends(seed, documents, kv.clone());
        let cors_origins = Arc::new(Mutex::new(config.cors_origins.clone()));
        let occupancy = Occupancy::connect(
            redis.as_ref(),
            config.occupancy_global,
            config.occupancy_per_visitor,
        );
        let throttle = RateLimiter::connect(
            redis.as_ref(),
            config.embed_throttle_max,
            Duration::from_secs(config.embed_throttle_window_secs.max(1)),
        );
        let stream_throttle = RateLimiter::connect(
            redis.as_ref(),
            config.stream_throttle_max,
            Duration::from_secs(config.stream_throttle_window_secs.max(1)),
        );
        let ip_throttle = RateLimiter::connect(
            redis.as_ref(),
            config.ip_throttle_max,
            Duration::from_secs(config.ip_throttle_window_secs.max(1)),
        );
        let client_ip_header = Arc::new(Mutex::new(config.client_ip_header.clone()));
        let embed_http = HttpClient::from_timeout(config.stream_timeout.max(30));
        let index = Arc::new(Mutex::new(
            config
                .index_root
                .as_ref()
                .filter(|path| path.is_dir())
                .map(sveda_index::prepare)
                .map(Arc::new),
        ));
        let admin_key = Arc::new(Mutex::new(config.admin_api_key.clone()));
        let store = match postgres.clone() {
            Some(postgres) => HistoryStore::postgres(postgres),
            None => HistoryStore::memory(),
        };
        let usage = match postgres {
            Some(postgres) => UsageStore::postgres(postgres),
            None => UsageStore::memory(),
        };
        let state = Self {
            catalog: Arc::new(Mutex::new(catalog)),
            llm,
            config,
            mcp: kv,
            store,
            usage,
            settings,
            cors_origins,
            occupancy,
            throttle,
            stream_throttle,
            ip_throttle,
            client_ip_header,
            index,
            index_embed_fp: Arc::new(Mutex::new(String::new())),
            embed_http,
            admin_key,
            code_index: code_index::CodeIndex::memory(),
        };
        admin::apply_runtime(&state, &state.settings.document());
        state
    }

    pub fn store(&self) -> &HistoryStore {
        &self.store
    }

    pub fn usage(&self) -> &UsageStore {
        &self.usage
    }

    pub(crate) fn catalog(&self) -> Catalog {
        self.catalog.lock().expect("catalog").clone()
    }

    pub(crate) fn runtime_config(&self) -> Config {
        admin::overlay(&self.config, &self.settings)
    }

    pub async fn refresh_settings(&self) {
        if self.settings.refresh().await {
            admin::apply_runtime(self, &self.settings.document());
            admin::refresh_workspace_index(self).await;
        }
    }

    pub async fn readiness(&self) -> Result<(), String> {
        self.store.ping().await.map_err(|error| error.to_string())?;
        self.mcp.ping().map_err(|error| error.to_string())?;
        Ok(())
    }

    pub fn host_mcp(&self, visitor_id: &str) -> Option<HostMcpCreds> {
        let value = self.mcp.get_json(&mcp_key(visitor_id)).ok().flatten()?;
        Some(HostMcpCreds {
            url: value.get("url")?.as_str()?.to_string(),
            token: value.get("token")?.as_str()?.to_string(),
        })
    }

    #[cfg(test)]
    pub(crate) fn inject_host_mcp(&self, visitor_id: &str, url: &str, token: &str) {
        self.mcp
            .set_json(
                &mcp_key(visitor_id),
                &serde_json::json!({ "url": url, "token": token }),
                None,
            )
            .expect("inject host mcp");
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
        .route("/", get(ui::home))
        .route("/admin", get(ui::show))
        .route("/admin/{page}", get(ui::section))
        .route(
            "/admin/settings",
            get(admin::show_settings)
                .put(admin::update_settings)
                .post(admin::update_settings),
        )
        .route("/admin/login", post(ui::login))
        .route("/admin/session", post(ui::admin_session))
        .route("/admin/setup", post(ui::setup))
        .route("/admin/logout", post(ui::logout))
        .route("/admin/code-index/sources", get(code_index::list_sources))
        .route("/admin/code-index/progress", get(code_index::progress))
        .route("/admin/code-index/store", post(code_index::create_source))
        .route(
            "/admin/code-index/local-browse",
            get(code_index::local_browse),
        )
        .route(
            "/admin/code-index/local-preview",
            post(code_index::local_preview),
        )
        .route(
            "/admin/code-index/sources/{id}",
            patch(code_index::update_source).delete(code_index::delete_source),
        )
        .route(
            "/admin/code-index/sources/{id}/cancel-indexing",
            post(code_index::cancel_indexing),
        )
        .route(
            "/admin/code-index/sources/{id}/reindex",
            post(code_index::reindex),
        )
        .route(
            "/admin/code-index/sources/{id}/scope",
            get(code_index::scope),
        )
        .route(
            "/admin/code-index/sources/{id}/estimate-footprint",
            post(code_index::estimate_footprint),
        )
        .route(
            "/sveda/admin",
            get(|| async { Redirect::permanent(ui::ADMIN_BASE) }),
        )
        .route(
            "/sveda/admin/{page}",
            get(|Path(page): Path<String>| async move {
                let location = format!("{}/{page}", ui::ADMIN_BASE);
                let value = HeaderValue::from_str(&location).expect("redirect location");
                (StatusCode::MOVED_PERMANENTLY, [(header::LOCATION, value)]).into_response()
            }),
        )
        .route("/sveda/health", get(ui::health))
        .route("/sveda/ready", get(ui::ready))
        .route("/sveda/embed", get(ui::embed_page))
        .route("/sveda/embed/token", post(issue_embed_token))
        .route("/sveda/embed/config", get(admin::embed_config))
        .route("/sveda/stream", post(stream_chat))
        .route("/sveda/message", post(json_message))
        .route("/sveda/chat-histories", get(histories::list_histories))
        .route(
            "/sveda/chat-histories/{chat_id}",
            get(histories::show_history)
                .patch(histories::update_history)
                .delete(histories::delete_history),
        )
        .route(
            "/sveda/documents/extract",
            post(documents::extract_documents),
        );
    let dist = ui::admin_dist();
    if dist.is_dir() {
        router = router.nest_service("/build", ServeDir::new(dist));
    }
    router
        .fallback(ui::not_found)
        .layer(DefaultBodyLimit::max(extract_limit))
        .layer(cors_layer(cors_origins))
        .layer(middleware::from_fn(expose_embed_assets))
        .with_state(state)
}

async fn expose_embed_assets(request: Request, next: Next) -> Response {
    let is_embed_asset = request.uri().path().starts_with("/build/sveda/");
    let mut response = next.run(request).await;
    if !is_embed_asset {
        return response;
    }
    if response.status().is_success() {
        // Unhashed sveda-chat.js/css. Revalidate on every HIT so a rolled
        // runtime is visible on host pages without a Cloudflare purge.
        response.headers_mut().insert(
            header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=0, must-revalidate"),
        );
        response.headers_mut().insert(
            HeaderName::from_static("cloudflare-cdn-cache-control"),
            HeaderValue::from_static("no-cache"),
        );
    }
    if !response
        .headers()
        .contains_key(header::ACCESS_CONTROL_ALLOW_ORIGIN)
    {
        response.headers_mut().insert(
            header::ACCESS_CONTROL_ALLOW_ORIGIN,
            HeaderValue::from_static("*"),
        );
    }
    response
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
            HeaderName::from_static("x-sveda-protocol"),
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
        .unwrap_or("");
    if !mcp_token.is_empty() && mcp_url.is_none() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "host_mcp_token requires host_mcp_url" })),
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
    if visitor_id.eq_ignore_ascii_case(ADMIN_VISITOR_ID) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "visitor_id is reserved" })),
        )
            .into_response();
    }
    if visitor_id.len() > 64 {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(serde_json::json!({ "message": "visitor_id is too long" })),
        )
            .into_response();
    }

    let policy_name = request
        .policy
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty());
    if let Some(name) = policy_name {
        if !policy::policy_name_valid(name) {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "message": "policy name is invalid" })),
            )
                .into_response();
        }
        if !state.settings.document().policies.contains_key(name) {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(serde_json::json!({ "message": "unknown policy" })),
            )
                .into_response();
        }
    }

    let grants = request
        .grants
        .as_ref()
        .and_then(|value| serde_json::from_value::<policy::CapabilityPolicy>(value.clone()).ok());

    let ttl = state.config.token_ttl_seconds.max(60);
    let token = token::issue_with_options(
        &state.config.hmac_key,
        &visitor_id,
        ttl,
        token::IssueOptions {
            policy: policy_name.map(ToOwned::to_owned),
            grants,
            admin: false,
        },
    );

    if let (Some(url), true) = (mcp_url, state.config.host_api_key.is_some()) {
        if state
            .mcp
            .set_json(
                &mcp_key(&visitor_id),
                &serde_json::json!({ "url": url, "token": mcp_token }),
                Some(Duration::from_secs(ttl)),
            )
            .is_err()
        {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({ "message": "runtime store is unavailable" })),
            )
                .into_response();
        }
    }

    Json(sveda_protocol::EmbedTokenResponse {
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
    let claims = match visitor_claims(&state, &headers) {
        Ok(claims) => claims,
        Err(status) => return status.into_response(),
    };
    let lease = match admit_turn(&state, &headers, &claims.visitor_id) {
        Ok(lease) => lease,
        Err(response) => return response,
    };
    let events = match turn::run_turn(state.clone(), claims, request).await {
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
    let claims = match visitor_claims(&state, &headers) {
        Ok(claims) => claims,
        Err(status) => return status.into_response(),
    };
    let _lease = match admit_turn(&state, &headers, &claims.visitor_id) {
        Ok(lease) => lease,
        Err(response) => return response,
    };
    let fallback_chat_id = request
        .chat_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let events = match turn::run_turn(state, claims, request).await {
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

fn admit_turn(
    state: &AppState,
    headers: &HeaderMap,
    visitor_id: &str,
) -> Result<sveda_store::OccupancyLease, Response> {
    if let Err(retry_after) = state.stream_throttle.hit(&format!("stream:{visitor_id}")) {
        return Err(throttle_rejected(retry_after));
    }
    let header_name = state.client_ip_header.lock().expect("ip header").clone();
    if !header_name.is_empty() {
        if let Some(ip) = client_ip(headers, &header_name) {
            if let Err(retry_after) = state.ip_throttle.hit(&format!("ip:{ip}")) {
                return Err(throttle_rejected(retry_after));
            }
        }
    }
    match state.occupancy.acquire(visitor_id) {
        Ok(lease) => Ok(lease),
        Err(error) => Err(occupancy_rejected(error)),
    }
}

fn client_ip(headers: &HeaderMap, name: &str) -> Option<String> {
    let header = HeaderName::from_bytes(name.as_bytes()).ok()?;
    headers
        .get(&header)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
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
    visitor_claims(state, headers).map(|claims| claims.visitor_id)
}

pub(crate) fn visitor_claims(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<token::TokenClaims, StatusCode> {
    let Some(embed_token) = extract_embed_token(headers) else {
        return Err(StatusCode::UNAUTHORIZED);
    };
    token::parse(&state.config.hmac_key, &embed_token).ok_or(StatusCode::UNAUTHORIZED)
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

fn compat_scripted_llm() -> ScriptedClient {
    let step = vec![
        Ok(LlmChunk::TextDelta("Hello from Sveda".into())),
        Ok(LlmChunk::Usage {
            prompt_tokens: 0,
            completion_tokens: 3,
        }),
        Ok(LlmChunk::End {
            finish_reason: "stop".into(),
        }),
    ];
    ScriptedClient::queue(vec![
        step.clone(),
        step.clone(),
        step.clone(),
        step.clone(),
        step.clone(),
        step.clone(),
    ])
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
