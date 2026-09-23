#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Protocol {
    Responses,
    Anthropic,
}

impl Protocol {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "responses" => Some(Self::Responses),
            "anthropic" => Some(Self::Anthropic),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ModelSpec {
    pub id: String,
    pub label: String,
    pub protocol: Protocol,
    pub api_model: String,
    pub url: String,
    pub key: String,
    pub aliases: Vec<String>,
}

impl ModelSpec {
    pub fn matches(&self, needle: &str) -> bool {
        let needle = needle.trim().to_ascii_lowercase();
        if needle.is_empty() {
            return false;
        }
        if self.id.eq_ignore_ascii_case(&needle) {
            return true;
        }
        if self.api_model.eq_ignore_ascii_case(&needle) {
            return true;
        }
        self.aliases
            .iter()
            .any(|alias| alias.eq_ignore_ascii_case(&needle))
    }

    pub fn uses_plaintext_reasoning(&self) -> bool {
        let haystack = format!("{} {} {}", self.url, self.api_model, self.id).to_ascii_lowercase();
        haystack.contains("deepseek")
    }
}

#[derive(Debug, Clone)]
pub struct Catalog {
    pub models: Vec<ModelSpec>,
    pub default_id: String,
    pub failover_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownModelError {
    pub id: String,
}

impl std::fmt::Display for UnknownModelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.id.is_empty() {
            write!(f, "Unknown Sveda model.")
        } else {
            write!(f, "Unknown Sveda model [{}].", self.id)
        }
    }
}

impl Catalog {
    pub fn builtin(deepseek_key: impl Into<String>) -> Self {
        let key = deepseek_key.into();
        Self {
            default_id: "deepseek-v4-flash-responses".into(),
            failover_ids: vec!["deepseek-v4-flash-anthropic".into()],
            models: vec![
                ModelSpec {
                    id: "deepseek-v4-flash-responses".into(),
                    label: "DeepSeek V4 Flash (Responses)".into(),
                    protocol: Protocol::Responses,
                    api_model: "deepseek-v4-flash".into(),
                    url: "https://api.deepseek.com".into(),
                    key: key.clone(),
                    aliases: vec!["deepseek-v4-flash".into()],
                },
                ModelSpec {
                    id: "deepseek-v4-flash-anthropic".into(),
                    label: "DeepSeek V4 Flash (Anthropic)".into(),
                    protocol: Protocol::Anthropic,
                    api_model: "deepseek-v4-flash".into(),
                    url: "https://api.deepseek.com/anthropic/v1".into(),
                    key: key.clone(),
                    aliases: Vec::new(),
                },
                ModelSpec {
                    id: "deepseek-v4-pro".into(),
                    label: "DeepSeek V4 Pro".into(),
                    protocol: Protocol::Responses,
                    api_model: "deepseek-v4-pro".into(),
                    url: "https://api.deepseek.com".into(),
                    key,
                    aliases: Vec::new(),
                },
            ],
        }
    }

    pub fn from_env() -> Self {
        let key = std::env::var("DEEPSEEK_API_KEY")
            .or_else(|_| std::env::var("SVEDA_DEEPSEEK_API_KEY"))
            .unwrap_or_default();
        let mut catalog = Self::builtin(key);
        if let Ok(default_id) =
            std::env::var("SVEDA_DEFAULT_MODEL").or_else(|_| std::env::var("SVEDA_MODEL"))
        {
            let trimmed = default_id.trim();
            if !trimmed.is_empty() {
                catalog.default_id = trimmed.to_string();
            }
        }
        if let Ok(failover) = std::env::var("SVEDA_FAILOVER") {
            catalog.failover_ids = failover
                .split(',')
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty())
                .collect();
        }
        catalog
    }

    pub fn find(&self, id: &str) -> Option<&ModelSpec> {
        let needle = id.trim();
        self.models
            .iter()
            .find(|model| model.matches(needle))
            .or_else(|| {
                self.models
                    .iter()
                    .find(|model| model.api_model.eq_ignore_ascii_case(needle))
            })
    }

    pub fn resolve(&self, id: Option<&str>) -> Result<&ModelSpec, UnknownModelError> {
        let needle = id.unwrap_or("").trim();
        if needle.is_empty() {
            return self.default();
        }
        self.find(needle).ok_or_else(|| UnknownModelError {
            id: needle.to_string(),
        })
    }

    pub fn default(&self) -> Result<&ModelSpec, UnknownModelError> {
        if let Some(found) = self.find(&self.default_id) {
            return Ok(found);
        }
        self.models
            .first()
            .ok_or_else(|| UnknownModelError { id: String::new() })
    }

    pub fn stream_chain(&self, primary: &ModelSpec) -> Vec<ModelSpec> {
        let mut chain = vec![primary.clone()];
        for id in &self.failover_ids {
            let Some(candidate) = self.find(id) else {
                continue;
            };
            if chain.iter().any(|existing| existing.id == candidate.id) {
                continue;
            }
            chain.push(candidate.clone());
        }
        chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_deepseek_models_use_plaintext_reasoning() {
        let catalog = Catalog::builtin("k");
        for model in &catalog.models {
            assert!(model.uses_plaintext_reasoning());
        }
    }

    #[test]
    fn non_deepseek_providers_keep_native_reasoning() {
        let openai = ModelSpec {
            id: "gpt-4o".into(),
            label: "GPT".into(),
            protocol: Protocol::Responses,
            api_model: "gpt-4o".into(),
            url: "https://api.openai.com/v1".into(),
            key: "k".into(),
            aliases: Vec::new(),
        };
        let claude = ModelSpec {
            id: "claude".into(),
            label: "Claude".into(),
            protocol: Protocol::Anthropic,
            api_model: "claude-sonnet-4".into(),
            url: "https://api.anthropic.com".into(),
            key: "k".into(),
            aliases: Vec::new(),
        };
        assert!(!openai.uses_plaintext_reasoning());
        assert!(!claude.uses_plaintext_reasoning());
    }
}
