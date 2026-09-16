use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use veda_llm::{Catalog, ModelSpec, Protocol};

use crate::Config;

pub const MASK: &str = "••••••••";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SettingsDocument {
    pub default_model: String,
    pub model: String,
    pub failover: Vec<String>,
    pub deepseek: DeepseekSettings,
    pub models: Vec<ModelSettings>,
    pub max_steps: u32,
    pub compaction: CompactionSettings,
    pub cors: CorsSettings,
    pub welcome_message: String,
    pub system_prompt: String,
    #[serde(default = "default_mcp")]
    pub mcp: Value,
    #[serde(default = "default_appearance")]
    pub appearance: Value,
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
pub struct CompactionSettings {
    pub enabled: bool,
    pub min_messages: usize,
    pub keep_tail_messages: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct CorsSettings {
    #[serde(default)]
    pub allowed_origins: Vec<String>,
}

#[derive(Clone, Debug, Default, Deserialize)]
pub struct SettingsPatch {
    pub default_model: Option<String>,
    pub model: Option<String>,
    pub failover: Option<Vec<String>>,
    pub deepseek: Option<DeepseekSettings>,
    pub models: Option<Vec<ModelSettings>>,
    pub max_steps: Option<u32>,
    pub compaction: Option<CompactionPatch>,
    pub cors: Option<CorsPatch>,
    pub welcome_message: Option<String>,
    pub system_prompt: Option<String>,
    pub mcp: Option<Value>,
    pub appearance: Option<Value>,
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

impl SettingsDocument {
    pub fn from_runtime(config: &Config, catalog: &Catalog) -> Self {
        let default_model = catalog.default_id.clone();
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

fn default_mcp() -> Value {
    json!({ "mcpServers": {} })
}

fn default_appearance() -> Value {
    json!({})
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
    inner: std::sync::Arc<std::sync::Mutex<SettingsDocument>>,
}

impl SettingsStore {
    pub fn new(document: SettingsDocument) -> Self {
        Self {
            inner: std::sync::Arc::new(std::sync::Mutex::new(document)),
        }
    }

    pub fn document(&self) -> SettingsDocument {
        self.inner.lock().expect("settings").clone()
    }

    pub fn replace(&self, document: SettingsDocument) -> SettingsDocument {
        *self.inner.lock().expect("settings") = document.clone();
        document
    }
}
