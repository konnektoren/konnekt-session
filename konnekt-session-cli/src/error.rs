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
}

impl CliError {
    pub fn schema_not_found(path: PathBuf) -> Self {
        CliError::SchemaFileNotFound { path }
    }

    pub fn invalid_directory(path: PathBuf) -> Self {
        CliError::InvalidSchemaDirectory { path }
    }
}
