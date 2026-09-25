use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sveda_llm::{
    default_dimensions, default_usd_per_million, Catalog, EmbeddingProtocol, EmbeddingSpec,
    ModelSpec, Protocol, DEFAULT_EMBEDDING_MODEL, MAX_EMBEDDING_DIMS, MIN_EMBEDDING_DIMS,
    OPENAI_EMBEDDINGS_URL,
};
use sveda_store::{DocumentStore, KvStore, StoreError};

use std::collections::BTreeMap;

use crate::policy::CapabilityPolicy;
use crate::Config;

pub const MASK: &str = "••••••••";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsDocument {
    pub default_model: String,
    pub model: String,
    pub failover: Vec<String>,
    pub deepseek: DeepseekSettings,
    pub models: Vec<ModelSettings>,
    #[serde(default)]
    pub default_embedding: String,
    #[serde(default)]
    pub embeddings: Vec<EmbeddingSettings>,
    pub max_steps: u32,
    pub compaction: CompactionSettings,
    pub cors: CorsSettings,
    pub welcome_message: String,
    pub system_prompt: String,
    #[serde(default = "default_mcp")]
    pub mcp: Value,
    #[serde(default = "default_appearance")]
    pub appearance: Value,
    #[serde(default = "default_web")]
    pub web: WebSettings,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub security: Option<SecuritySettings>,
    #[serde(default)]
    pub policies: BTreeMap<String, CapabilityPolicy>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct DeepseekSettings {
    #[serde(default)]
    pub key: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ModelSettings {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub api_model: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub thinking: bool,
    #[serde(default)]
    pub vision: bool,
    #[serde(default)]
    pub aliases: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EmbeddingSettings {
    pub id: String,
    #[serde(default)]
    pub label: String,
    #[serde(default)]
    pub protocol: String,
    #[serde(default)]
    pub api_model: String,
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub dimensions: u32,
    #[serde(default)]
    pub usd_per_million: f64,
}

impl EmbeddingSettings {
    pub fn to_spec(&self) -> EmbeddingSpec {
        let api_model = if self.api_model.trim().is_empty() {
            DEFAULT_EMBEDDING_MODEL.to_string()
        } else {
            self.api_model.clone()
        };
        let dimensions = if self.dimensions == 0 {
            default_dimensions(&api_model)
        } else {
            (self.dimensions as usize).clamp(MIN_EMBEDDING_DIMS, MAX_EMBEDDING_DIMS)
        };
        let usd_per_million = if self.usd_per_million <= 0.0 {
            default_usd_per_million(&api_model)
        } else {
            self.usd_per_million
        };
        EmbeddingSpec {
            id: self.id.clone(),
            label: if self.label.trim().is_empty() {
                self.id.clone()
            } else {
                self.label.clone()
            },
            protocol: EmbeddingProtocol::OpenAi,
            url: if self.url.trim().is_empty() {
                OPENAI_EMBEDDINGS_URL.to_string()
            } else {
                self.url.trim_end_matches('/').to_string()
            },
            key: self.key.clone(),
            api_model,
            dimensions,
            usd_per_million,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CompactionSettings {
    pub enabled: bool,
    pub min_messages: usize,
    pub keep_tail_messages: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebSettings {
    #[serde(default = "default_web_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub searxng_url: String,
}

fn default_web() -> WebSettings {
    WebSettings {
        enabled: true,
        searxng_url: String::new(),
    }
}

fn default_web_enabled() -> bool {
    true
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorsSettings {
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SecuritySettings {
    #[serde(default)]
    pub embed_token_throttle_max: u32,
    #[serde(default = "default_throttle_window_secs")]
    pub embed_token_throttle_window_secs: u64,
    #[serde(default)]
    pub stream_throttle_max: u32,
    #[serde(default = "default_throttle_window_secs")]
    pub stream_throttle_window_secs: u64,
    #[serde(default)]
    pub occupancy_global: usize,
    #[serde(default)]
    pub occupancy_per_visitor: usize,
    #[serde(default)]
    pub client_ip_header: String,
    #[serde(default)]
    pub ip_throttle_max: u32,
    #[serde(default = "default_throttle_window_secs")]
    pub ip_throttle_window_secs: u64,
}

fn default_throttle_window_secs() -> u64 {
    60
}

impl SecuritySettings {
    pub fn from_config(config: &Config) -> Self {
        Self {
            embed_token_throttle_max: config.embed_throttle_max,
            embed_token_throttle_window_secs: config.embed_throttle_window_secs.max(1),
            stream_throttle_max: config.stream_throttle_max,
            stream_throttle_window_secs: config.stream_throttle_window_secs.max(1),
            occupancy_global: config.occupancy_global,
            occupancy_per_visitor: config.occupancy_per_visitor,
            client_ip_header: sanitize_ip_header(&config.client_ip_header),
            ip_throttle_max: config.ip_throttle_max,
            ip_throttle_window_secs: config.ip_throttle_window_secs.max(1),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub default_model: Option<String>,
    pub model: Option<String>,
    pub failover: Option<Vec<String>>,
    pub deepseek: Option<DeepseekSettings>,
    pub models: Option<Vec<ModelSettings>>,
    pub default_embedding: Option<String>,
    pub embeddings: Option<Vec<EmbeddingSettings>>,
    pub max_steps: Option<u32>,
    pub compaction: Option<CompactionPatch>,
    pub cors: Option<CorsPatch>,
    pub welcome_message: Option<String>,
    pub system_prompt: Option<String>,
    pub mcp: Option<Value>,
    pub appearance: Option<Value>,
    pub web: Option<WebPatch>,
    pub security: Option<SecurityPatch>,
    pub policies: Option<BTreeMap<String, CapabilityPolicy>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CompactionPatch {
    pub enabled: Option<bool>,
    pub min_messages: Option<usize>,
    pub keep_tail_messages: Option<usize>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct CorsPatch {
    pub allowed_origins: Option<Vec<String>>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct WebPatch {
    pub enabled: Option<bool>,
    pub searxng_url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SecurityPatch {
    pub embed_token_throttle_max: Option<u32>,
    pub embed_token_throttle_window_secs: Option<u64>,
    pub stream_throttle_max: Option<u32>,
    pub stream_throttle_window_secs: Option<u64>,
    pub occupancy_global: Option<usize>,
    pub occupancy_per_visitor: Option<usize>,
    pub client_ip_header: Option<String>,
    pub ip_throttle_max: Option<u32>,
    pub ip_throttle_window_secs: Option<u64>,
}

impl SettingsDocument {
    pub fn from_runtime(config: &Config, catalog: &Catalog) -> Self {
        let default_model = catalog.default_id.clone();
        let (default_embedding, embeddings) = embeddings_from_env();
        Self {
            default_model: default_model.clone(),
            model: default_model,
            failover: catalog.failover_ids.clone(),
            deepseek: DeepseekSettings {
                key: catalog
                    .models
                    .first()
                    .map(|model| model.key.clone())
                    .unwrap_or_default(),
            },
            models: catalog.models.iter().map(model_from_spec).collect(),
            default_embedding: default_embedding.clone(),
            embeddings,
            max_steps: config.max_steps,
            compaction: CompactionSettings {
                enabled: config.compaction_enabled,
                min_messages: config.compaction_min_messages,
                keep_tail_messages: config.compaction_keep_tail,
            },
            cors: CorsSettings {
                allowed_origins: config.cors_origins.clone(),
            },
            welcome_message: String::new(),
            system_prompt: config.system_prompt.clone(),
            mcp: default_mcp(),
            appearance: default_appearance(),
            web: WebSettings {
                enabled: config.web_enabled,
                searxng_url: config.searxng_url.clone(),
            },
            security: Some(SecuritySettings::from_config(config)),
            policies: BTreeMap::new(),
        }
    }

    pub fn merge(&self, patch: SettingsPatch) -> Self {
        let mut next = self.clone();
        if let Some(value) = patch.default_model {
            next.default_model = value;
        }
        if let Some(value) = patch.model {
            next.model = value;
        }
        if !next.default_model.trim().is_empty() {
            next.model = next.default_model.clone();
        } else if !next.model.trim().is_empty() {
            next.default_model = next.model.clone();
        }
        if let Some(failover) = patch.failover {
            next.failover = string_list(failover);
        }
        if let Some(deepseek) = patch.deepseek {
            next.deepseek.key = deepseek.key;
        }
        if let Some(models) = patch.models {
            next.models = normalize_models(models, &self.models);
        }
        if let Some(embeddings) = patch.embeddings {
            next.embeddings = normalize_embeddings(embeddings, &self.embeddings);
        }
        if let Some(default_embedding) = patch.default_embedding {
            next.default_embedding = default_embedding;
        }
        next.default_embedding =
            resolve_default_embedding(&next.default_embedding, &next.embeddings);
        if let Some(max_steps) = patch.max_steps {
            next.max_steps = max_steps.max(1);
        }
        if let Some(compaction) = patch.compaction {
            if let Some(enabled) = compaction.enabled {
                next.compaction.enabled = enabled;
            }
            if let Some(min_messages) = compaction.min_messages {
                next.compaction.min_messages = min_messages.max(1);
            }
            if let Some(keep_tail) = compaction.keep_tail_messages {
                next.compaction.keep_tail_messages = keep_tail.max(1);
            }
        }
        if let Some(cors) = patch.cors {
            if let Some(origins) = cors.allowed_origins {
                next.cors.allowed_origins = string_list(origins);
            }
        }
        if let Some(welcome_message) = patch.welcome_message {
            next.welcome_message = welcome_message;
        }
        if let Some(system_prompt) = patch.system_prompt {
            next.system_prompt = system_prompt;
        }
        if let Some(mcp) = patch.mcp {
            next.mcp = mcp;
        }
        if let Some(appearance) = patch.appearance {
            next.appearance = appearance;
        }
        if let Some(web) = patch.web {
            if let Some(enabled) = web.enabled {
                next.web.enabled = enabled;
            }
            if let Some(searxng_url) = web.searxng_url {
                next.web.searxng_url = searxng_url.trim().to_string();
            }
        }
        if let Some(security) = patch.security {
            let mut current = next.security.take().unwrap_or_default();
            if current.embed_token_throttle_window_secs == 0 {
                current.embed_token_throttle_window_secs = default_throttle_window_secs();
            }
            if current.stream_throttle_window_secs == 0 {
                current.stream_throttle_window_secs = default_throttle_window_secs();
            }
            if current.ip_throttle_window_secs == 0 {
                current.ip_throttle_window_secs = default_throttle_window_secs();
            }
            if let Some(value) = security.embed_token_throttle_max {
                current.embed_token_throttle_max = value;
            }
            if let Some(value) = security.embed_token_throttle_window_secs {
                current.embed_token_throttle_window_secs = value.max(1);
            }
            if let Some(value) = security.stream_throttle_max {
                current.stream_throttle_max = value;
            }
            if let Some(value) = security.stream_throttle_window_secs {
                current.stream_throttle_window_secs = value.max(1);
            }
            if let Some(value) = security.occupancy_global {
                current.occupancy_global = value;
            }
            if let Some(value) = security.occupancy_per_visitor {
                current.occupancy_per_visitor = value;
            }
            if let Some(value) = security.client_ip_header {
                current.client_ip_header = sanitize_ip_header(&value);
            }
            if let Some(value) = security.ip_throttle_max {
                current.ip_throttle_max = value;
            }
            if let Some(value) = security.ip_throttle_window_secs {
                current.ip_throttle_window_secs = value.max(1);
            }
            next.security = Some(current);
        }
        if let Some(policies) = patch.policies {
            next.policies = policies;
        }
        next
    }

    pub fn preserve_secrets(&self, mut next: Self) -> Self {
        if next.deepseek.key == MASK || next.deepseek.key.is_empty() {
            next.deepseek.key = self.deepseek.key.clone();
        }
        let current: std::collections::HashMap<_, _> = self
            .models
            .iter()
            .map(|model| (model.id.clone(), model.clone()))
            .collect();
        for model in &mut next.models {
            if model.key == MASK || model.key.is_empty() {
                model.key = current
                    .get(&model.id)
                    .map(|item| item.key.clone())
                    .unwrap_or_default();
            }
        }
        let current_embeddings: std::collections::HashMap<_, _> = self
            .embeddings
            .iter()
            .map(|item| (item.id.clone(), item.clone()))
            .collect();
        for embedding in &mut next.embeddings {
            if embedding.key == MASK || embedding.key.is_empty() {
                embedding.key = current_embeddings
                    .get(&embedding.id)
                    .map(|item| item.key.clone())
                    .unwrap_or_default();
            }
        }
        next
    }

    pub fn masked(&self) -> Self {
        let mut masked = self.clone();
        masked.deepseek.key = if self.deepseek.key.is_empty() {
            String::new()
        } else {
            MASK.to_string()
        };
        for model in &mut masked.models {
            model.key = if model.key.is_empty() {
                String::new()
            } else {
                MASK.to_string()
            };
        }
        for embedding in &mut masked.embeddings {
            embedding.key = if embedding.key.is_empty() {
                String::new()
            } else {
                MASK.to_string()
            };
        }
        masked
    }

    pub fn public(&self) -> Value {
        json!({
            "default_model": self.default_model,
            "model": self.model,
            "models": self.models.iter().map(|model| json!({
                "id": model.id,
                "label": model.label,
                "protocol": model.protocol,
                "supports_thinking": model.thinking,
                "supportsThinking": model.thinking,
            })).collect::<Vec<_>>(),
            "welcome_message": self.welcome_message,
            "appearance": self.appearance.clone(),
        })
    }

    pub fn to_catalog(&self) -> Catalog {
        let fallback = self.deepseek.key.clone();
        Catalog {
            default_id: self.default_model.clone(),
            failover_ids: self.failover.clone(),
            models: self
                .models
                .iter()
                .map(|model| {
                    let key = if model.key.is_empty() {
                        fallback.clone()
                    } else {
                        model.key.clone()
                    };
                    ModelSpec {
                        id: model.id.clone(),
                        label: model.label.clone(),
                        protocol: Protocol::parse(&model.protocol).unwrap_or(Protocol::Responses),
                        api_model: if model.api_model.is_empty() {
                            model.id.clone()
                        } else {
                            model.api_model.clone()
                        },
                        url: model.url.trim_end_matches('/').to_string(),
                        key,
                        aliases: model.aliases.clone(),
                    }
                })
                .collect(),
        }
    }

    pub fn active_embedding(&self) -> Option<EmbeddingSpec> {
        let needle = self.default_embedding.trim();
        self.embeddings
            .iter()
            .find(|item| item.id == needle)
            .or_else(|| self.embeddings.first())
            .map(EmbeddingSettings::to_spec)
    }

    pub fn embeddings_fingerprint(&self) -> String {
        match self.active_embedding() {
            Some(spec) => format!(
                "{}|{}|{}|{}|{}",
                spec.id,
                spec.api_model,
                spec.url,
                spec.dimensions,
                spec.key.len()
            ),
            None => "hashed".into(),
        }
    }
}

fn model_from_spec(spec: &ModelSpec) -> ModelSettings {
    ModelSettings {
        id: spec.id.clone(),
        label: spec.label.clone(),
        protocol: match spec.protocol {
            Protocol::Responses => "responses".into(),
            Protocol::Anthropic => "anthropic".into(),
        },
        api_model: spec.api_model.clone(),
        url: spec.url.clone(),
        key: spec.key.clone(),
        thinking: true,
        vision: false,
        aliases: spec.aliases.clone(),
        preset: None,
    }
}

fn normalize_models(models: Vec<ModelSettings>, existing: &[ModelSettings]) -> Vec<ModelSettings> {
    let existing: std::collections::HashMap<_, _> = existing
        .iter()
        .map(|model| (model.id.clone(), model.clone()))
        .collect();
    models
        .into_iter()
        .filter(|model| !model.id.trim().is_empty())
        .map(|mut model| {
            if let Some(base) = existing.get(&model.id) {
                if model.label.trim().is_empty() {
                    model.label = base.label.clone();
                }
                if model.api_model.trim().is_empty() {
                    model.api_model = base.api_model.clone();
                }
                if model.url.trim().is_empty() {
                    model.url = base.url.clone();
                }
                if model.protocol.trim().is_empty() {
                    model.protocol = base.protocol.clone();
                }
                if model.preset.is_none() {
                    model.preset = base.preset.clone();
                }
            }
            let protocol = model.protocol.trim().to_ascii_lowercase();
            model.protocol = if protocol == "anthropic" {
                "anthropic".into()
            } else {
                "responses".into()
            };
            if model.label.trim().is_empty() {
                model.label = model.id.clone();
            }
            if model.api_model.trim().is_empty() {
                model.api_model = model.id.clone();
            }
            model.url = model.url.trim_end_matches('/').to_string();
            model.aliases = string_list(model.aliases);
            model
        })
        .collect()
}

fn normalize_embeddings(
    embeddings: Vec<EmbeddingSettings>,
    existing: &[EmbeddingSettings],
) -> Vec<EmbeddingSettings> {
    let existing: std::collections::HashMap<_, _> = existing
        .iter()
        .map(|item| (item.id.clone(), item.clone()))
        .collect();
    embeddings
        .into_iter()
        .filter(|item| !item.id.trim().is_empty())
        .map(|mut item| {
            if let Some(base) = existing.get(&item.id) {
                if item.label.trim().is_empty() {
                    item.label = base.label.clone();
                }
                if item.api_model.trim().is_empty() {
                    item.api_model = base.api_model.clone();
                }
                if item.url.trim().is_empty() {
                    item.url = base.url.clone();
                }
                if item.dimensions == 0 {
                    item.dimensions = base.dimensions;
                }
                if item.usd_per_million <= 0.0 {
                    item.usd_per_million = base.usd_per_million;
                }
            }
            item.protocol = EmbeddingProtocol::OpenAi.as_str().to_string();
            if item.label.trim().is_empty() {
                item.label = item.id.clone();
            }
            if item.api_model.trim().is_empty() {
                item.api_model = DEFAULT_EMBEDDING_MODEL.to_string();
            }
            if item.url.trim().is_empty() {
                item.url = OPENAI_EMBEDDINGS_URL.to_string();
            }
            item.url = item.url.trim_end_matches('/').to_string();
            if item.dimensions == 0 {
                item.dimensions = default_dimensions(&item.api_model) as u32;
            }
            item.dimensions =
                (item.dimensions as usize).clamp(MIN_EMBEDDING_DIMS, MAX_EMBEDDING_DIMS) as u32;
            if item.usd_per_million <= 0.0 {
                item.usd_per_million = default_usd_per_million(&item.api_model);
            }
            item
        })
        .collect()
}

fn resolve_default_embedding(current: &str, embeddings: &[EmbeddingSettings]) -> String {
    let current = current.trim();
    if embeddings.iter().any(|item| item.id == current) {
        return current.to_string();
    }
    embeddings
        .first()
        .map(|item| item.id.clone())
        .unwrap_or_default()
}

fn embeddings_from_env() -> (String, Vec<EmbeddingSettings>) {
    let key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("SVEDA_OPENAI_API_KEY"))
        .unwrap_or_default()
        .trim()
        .to_string();
    if key.is_empty() {
        return (String::new(), Vec::new());
    }
    let api_model = std::env::var("SVEDA_EMBEDDING_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_EMBEDDING_MODEL.to_string());
    let url = std::env::var("SVEDA_EMBEDDING_URL")
        .ok()
        .map(|value| value.trim().trim_end_matches('/').to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| OPENAI_EMBEDDINGS_URL.to_string());
    let dimensions = std::env::var("SVEDA_EMBEDDING_DIMENSIONS")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|value| *value > 0)
        .unwrap_or_else(|| default_dimensions(&api_model) as u32);
    let usd_per_million = default_usd_per_million(&api_model);
    let id = format!("openai-{api_model}")
        .replace('/', "-")
        .replace(' ', "-");
    (
        id.clone(),
        vec![EmbeddingSettings {
            label: format!("OpenAI {api_model}"),
            protocol: EmbeddingProtocol::OpenAi.as_str().to_string(),
            api_model,
            url,
            key,
            dimensions,
            usd_per_million,
            id,
        }],
    )
}

fn default_mcp() -> Value {
    json!({ "mcpServers": {} })
}

fn default_appearance() -> Value {
    json!({})
}

pub fn sanitize_ip_header(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if trimmed
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
    {
        trimmed.to_string()
    } else {
        String::new()
    }
}

fn string_list(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect()
}

#[derive(Clone)]
pub struct SettingsStore {
    inner: std::sync::Arc<std::sync::Mutex<CachedSettings>>,
    documents: DocumentStore,
    kv: KvStore,
}

#[derive(Clone)]
struct CachedSettings {
    document: SettingsDocument,
    rev: i64,
}

pub fn config_path() -> Option<std::path::PathBuf> {
    if let Ok(value) = std::env::var("SVEDA_CONFIG") {
        let path = std::path::PathBuf::from(value.trim());
        if !path.as_os_str().is_empty() {
            return Some(path);
        }
    }
    let local = std::path::PathBuf::from("sveda.yaml");
    if local.is_file() {
        Some(local)
    } else {
        None
    }
}

pub fn read_config_patch(path: &std::path::Path) -> Result<SettingsPatch, String> {
    let raw = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    serde_yaml::from_str(&raw).map_err(|error| error.to_string())
}

impl SettingsStore {
    #[allow(dead_code)]
    pub fn new(document: SettingsDocument) -> Self {
        Self::with_backends(document, DocumentStore::memory(), KvStore::memory())
    }

    pub fn with_backends(
        document: SettingsDocument,
        documents: DocumentStore,
        kv: KvStore,
    ) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(CachedSettings { document, rev: 0 })),
            documents,
            kv,
        }
    }

    pub fn is_shared(&self) -> bool {
        self.documents.is_shared()
    }

    pub fn document(&self) -> SettingsDocument {
        self.inner.lock().expect("settings").document.clone()
    }

    pub async fn persist(
        &self,
        document: SettingsDocument,
    ) -> Result<SettingsDocument, StoreError> {
        let value = serde_json::to_value(&document)
            .map_err(|error| StoreError::message(error.to_string()))?;
        self.documents.save(&value).await?;
        let rev = self.kv.bump_settings_rev().unwrap_or(0);
        let mut guard = self.inner.lock().expect("settings");
        guard.document = document.clone();
        guard.rev = rev;
        Ok(document)
    }

    pub async fn refresh(&self) -> bool {
        if !self.documents.is_shared() {
            return false;
        }
        let remote = self.kv.settings_rev().unwrap_or(0);
        let local = self.inner.lock().expect("settings").rev;
        if remote > 0 && remote <= local {
            return false;
        }
        let Ok(Some(value)) = self.documents.load().await else {
            return false;
        };
        let Ok(document) = serde_json::from_value::<SettingsDocument>(value) else {
            return false;
        };
        let mut guard = self.inner.lock().expect("settings");
        if remote > 0 && remote <= guard.rev {
            return false;
        }
        guard.document = document;
        if remote > 0 {
            guard.rev = remote;
        }
        true
    }

    pub async fn load_or_seed(&self, seed: SettingsDocument) -> SettingsDocument {
        if let Ok(Some(value)) = self.documents.load().await {
            if let Ok(mut document) = serde_json::from_value::<SettingsDocument>(value) {
                if document.security.is_none() {
                    document.security = seed.security.clone();
                    let _ = self.persist(document.clone()).await;
                    return document;
                }
                let mut guard = self.inner.lock().expect("settings");
                guard.document = document.clone();
                return document;
            }
        }
        let _ = self.persist(seed.clone()).await;
        seed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn yaml_patch_overlays_prompt_and_cors() {
        let raw = r#"
system_prompt: hello from yaml
cors:
  allowed_origins:
    - https://app.example
"#;
        let patch: SettingsPatch = serde_yaml::from_str(raw).unwrap();
        let base =
            SettingsDocument::from_runtime(&crate::Config::test(), &sveda_llm::Catalog::from_env());
        let merged = base.merge(patch);
        assert_eq!(merged.system_prompt, "hello from yaml");
        assert_eq!(
            merged.cors.allowed_origins,
            vec!["https://app.example".to_string()]
        );
    }

    #[test]
    fn deploy_example_parses() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../deploy/sveda.yaml");
        let raw = std::fs::read_to_string(path).unwrap();
        let patch: SettingsPatch = serde_yaml::from_str(&raw).unwrap();
        assert_eq!(patch.max_steps, Some(8));
        assert!(patch.mcp.is_none());
        assert!(patch.policies.is_none());
    }

    #[test]
    fn missing_security_deserializes_as_none() {
        let value = json!({
            "default_model": "x",
            "model": "x",
            "failover": [],
            "deepseek": {},
            "models": [],
            "max_steps": 1,
            "compaction": { "enabled": true, "min_messages": 1, "keep_tail_messages": 1 },
            "cors": {},
            "welcome_message": "",
            "system_prompt": ""
        });
        let document: SettingsDocument = serde_json::from_value(value).unwrap();
        assert!(document.security.is_none());
        assert!(document.web.enabled);
        assert!(document.web.searxng_url.is_empty());
    }

    #[test]
    fn sanitize_ip_header_rejects_junk() {
        assert_eq!(sanitize_ip_header("CF-Connecting-IP"), "CF-Connecting-IP");
        assert_eq!(sanitize_ip_header(" X-Real-IP "), "X-Real-IP");
        assert_eq!(sanitize_ip_header("X-Forwarded-For"), "X-Forwarded-For");
        assert_eq!(sanitize_ip_header("bad header"), "");
        assert_eq!(sanitize_ip_header(""), "");
    }
}
