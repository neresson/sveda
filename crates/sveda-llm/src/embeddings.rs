#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmbeddingProtocol {
    OpenAi,
}

impl EmbeddingProtocol {
    pub fn parse(_value: &str) -> Self {
        Self::OpenAi
    }

    pub fn as_str(self) -> &'static str {
        "openai"
    }
}

#[derive(Debug, Clone)]
pub struct EmbeddingSpec {
    pub id: String,
    pub label: String,
    pub protocol: EmbeddingProtocol,
    pub api_model: String,
    pub url: String,
    pub key: String,
    pub dimensions: usize,
    pub usd_per_million: f64,
}

pub const OPENAI_EMBEDDINGS_URL: &str = "https://api.openai.com/v1";
pub const DEFAULT_EMBEDDING_MODEL: &str = "text-embedding-3-small";
pub const DEFAULT_EMBEDDING_DIMS: usize = 1536;
pub const MIN_EMBEDDING_DIMS: usize = 8;
pub const MAX_EMBEDDING_DIMS: usize = 8192;
pub const MAX_EMBEDDING_CHARS: usize = 8000;
pub const EMBED_BATCH_SIZE: usize = 64;

impl EmbeddingSpec {
    pub fn openai(key: impl Into<String>) -> Self {
        let api_model = DEFAULT_EMBEDDING_MODEL.to_string();
        Self {
            id: "openai-text-embedding-3-small".into(),
            label: "OpenAI text-embedding-3-small".into(),
            protocol: EmbeddingProtocol::OpenAi,
            dimensions: default_dimensions(&api_model),
            usd_per_million: default_usd_per_million(&api_model),
            api_model,
            url: OPENAI_EMBEDDINGS_URL.into(),
            key: key.into(),
        }
    }

    pub fn clamp_dimensions(value: usize) -> usize {
        value.clamp(MIN_EMBEDDING_DIMS, MAX_EMBEDDING_DIMS)
    }
}

pub fn default_dimensions(model: &str) -> usize {
    let model = model.to_ascii_lowercase();
    if model.contains("3-large") {
        3072
    } else {
        DEFAULT_EMBEDDING_DIMS
    }
}

pub fn default_usd_per_million(model: &str) -> f64 {
    let model = model.to_ascii_lowercase();
    if model.contains("3-large") {
        0.13
    } else if model.contains("ada-002") {
        0.10
    } else {
        0.02
    }
}

pub fn supports_dimensions_param(model: &str) -> bool {
    model.to_ascii_lowercase().contains("text-embedding-3")
}

pub fn clamp_embedding_input(text: &str) -> String {
    let text = text.trim();
    if text.chars().count() <= MAX_EMBEDDING_CHARS {
        return text.to_string();
    }
    text.chars().take(MAX_EMBEDDING_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn large_model_defaults_to_3072_dims() {
        assert_eq!(default_dimensions("text-embedding-3-large"), 3072);
        assert_eq!(default_dimensions("text-embedding-3-small"), 1536);
        assert_eq!(default_dimensions("text-embedding-ada-002"), 1536);
    }

    #[test]
    fn only_v3_models_send_dimensions() {
        assert!(supports_dimensions_param("text-embedding-3-small"));
        assert!(supports_dimensions_param("text-embedding-3-large"));
        assert!(!supports_dimensions_param("text-embedding-ada-002"));
    }

    #[test]
    fn protocol_is_always_openai() {
        assert_eq!(
            EmbeddingProtocol::parse("anthropic"),
            EmbeddingProtocol::OpenAi
        );
        assert_eq!(
            EmbeddingProtocol::parse("openai"),
            EmbeddingProtocol::OpenAi
        );
        assert_eq!(EmbeddingProtocol::parse("").as_str(), "openai");
    }
}
