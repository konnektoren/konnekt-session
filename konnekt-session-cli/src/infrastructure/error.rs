use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum CliError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Schema generation failed: {0}")]
    SchemaGeneration(String),

    #[error("Schema file not found: {path}")]
    SchemaFileNotFound { path: PathBuf },

    #[error("Invalid schema directory: {path}")]
    InvalidSchemaDirectory { path: PathBuf },

    #[error("Failed to create participant: {0}")]
    ParticipantCreation(String),

    #[error("P2P connection failed: {0}")]
    P2PConnection(String),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Failed to send message: {0}")]
    MessageSend(String),

    #[error("Serialization failed: {0}")]
    Serialization(String),

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}

impl CliError {
    pub fn schema_not_found(path: PathBuf) -> Self {
        CliError::SchemaFileNotFound { path }
    }

    pub fn invalid_directory(path: PathBuf) -> Self {
        CliError::InvalidSchemaDirectory { path }
    }
}

pub type Result<T> = std::result::Result<T, CliError>;
