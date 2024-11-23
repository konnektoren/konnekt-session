use thiserror::Error;

#[derive(Error, Debug)]
pub enum RepositoryError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Limit reached: {0}")]
    LimitReached(usize),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("Unknown error")]
    Unknown,
}
