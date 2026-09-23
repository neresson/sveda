#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0}")]
    Message(String),
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error("{0}")]
    Redis(String),
}

impl StoreError {
    pub fn message(value: impl Into<String>) -> Self {
        Self::Message(value.into())
    }
}
