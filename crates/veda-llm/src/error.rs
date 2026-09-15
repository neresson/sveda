#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum LlmError {
    #[error("rate limited")]
    RateLimited,
    #[error("provider overloaded")]
    Overloaded,
    #[error("insufficient credits")]
    InsufficientCredits,
    #[error("{0}")]
    Other(String),
}

impl LlmError {
    pub fn is_failoverable(&self) -> bool {
        matches!(
            self,
            Self::RateLimited | Self::Overloaded | Self::InsufficientCredits
        )
    }

    pub fn from_status(status: u16, body: &str) -> Self {
        let lowered = body.to_ascii_lowercase();
        if status == 429 || lowered.contains("rate_limit") || lowered.contains("rate limited") {
            return Self::RateLimited;
        }
        if status == 402
            || lowered.contains("insufficient")
            || lowered.contains("credit")
            || lowered.contains("quota")
        {
            return Self::InsufficientCredits;
        }
        if status == 503 || status == 529 || lowered.contains("overloaded") {
            return Self::Overloaded;
        }
        let message = if body.trim().is_empty() {
            format!("llm http {status}")
        } else {
            body.chars().take(500).collect()
        };
        Self::Other(message)
    }
}
