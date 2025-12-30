/// Trait that consuming apps implement for their activities
pub trait Activity: Send + Sync {
    /// Unique type identifier (e.g., "trivia-quiz-v1")
    /// MUST be stable across versions for backwards compatibility
    fn activity_type(&self) -> &str;

    /// Human-readable name
    fn name(&self) -> &str;

    /// Serialize activity config to JSON
    fn serialize_config(&self) -> Result<serde_json::Value, ActivityError>;

    /// Deserialize activity config from JSON
    fn deserialize_config(config: serde_json::Value) -> Result<Self, ActivityError>
    where
        Self: Sized;

    /// Validate a result submission (optional, default = always valid)
    fn validate_result(&self, result: &serde_json::Value) -> Result<(), ActivityError> {
        let _ = result;
        Ok(())
    }

    /// Calculate score from result data (optional, default = None)
    fn calculate_score(&self, result: &serde_json::Value) -> Option<u32> {
        let _ = result;
        None
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ActivityError {
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("Invalid result: {0}")]
    InvalidResult(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
