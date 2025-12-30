use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Activity ID (unique within lobby)
pub type ActivityId = Uuid;

/// Activity lifecycle state
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum ActivityStatus {
    /// Queued for later
    Planned,
    /// Currently active
    InProgress,
    /// Finished successfully
    Completed,
    /// Stopped early
    Cancelled,
}

/// Activity metadata (transported over P2P)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActivityMetadata {
    /// Unique ID
    pub id: ActivityId,

    /// Activity type identifier (e.g., "trivia-quiz-v1")
    pub activity_type: String,

    /// Display name
    pub name: String,

    /// Current status
    pub status: ActivityStatus,

    /// Activity-specific configuration (opaque to core)
    #[serde(default)]
    pub config: serde_json::Value,
}

impl ActivityMetadata {
    /// Create a new planned activity
    pub fn new(activity_type: String, name: String, config: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            activity_type,
            name,
            status: ActivityStatus::Planned,
            config,
        }
    }

    /// Create with specific ID (for deserialization/sync)
    pub fn with_id(
        id: ActivityId,
        activity_type: String,
        name: String,
        status: ActivityStatus,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id,
            activity_type,
            name,
            status,
            config,
        }
    }
}

/// Activity result submitted by a participant
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ActivityResult {
    pub activity_id: ActivityId,
    pub participant_id: Uuid,

    /// Result data (opaque to core)
    #[serde(default)]
    pub data: serde_json::Value,

    /// Optional score (for leaderboard)
    pub score: Option<u32>,

    /// Time taken (milliseconds)
    pub time_taken_ms: Option<u64>,
}

impl ActivityResult {
    pub fn new(activity_id: ActivityId, participant_id: Uuid) -> Self {
        Self {
            activity_id,
            participant_id,
            data: serde_json::Value::Null,
            score: None,
            time_taken_ms: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = data;
        self
    }

    pub fn with_score(mut self, score: u32) -> Self {
        self.score = Some(score);
        self
    }

    pub fn with_time(mut self, time_ms: u64) -> Self {
        self.time_taken_ms = Some(time_ms);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_activity_metadata() {
        let config = serde_json::json!({
            "question_count": 10,
            "time_limit": 60
        });

        let metadata = ActivityMetadata::new(
            "trivia-quiz-v1".to_string(),
            "Friday Quiz".to_string(),
            config.clone(),
        );

        assert_eq!(metadata.activity_type, "trivia-quiz-v1");
        assert_eq!(metadata.name, "Friday Quiz");
        assert_eq!(metadata.status, ActivityStatus::Planned);
        assert_eq!(metadata.config, config);
    }

    #[test]
    fn test_activity_result_builder() {
        let activity_id = Uuid::new_v4();
        let participant_id = Uuid::new_v4();

        let result = ActivityResult::new(activity_id, participant_id)
            .with_score(42)
            .with_time(1500)
            .with_data(serde_json::json!({"answers": [1, 2, 3]}));

        assert_eq!(result.score, Some(42));
        assert_eq!(result.time_taken_ms, Some(1500));
        assert!(!result.data.is_null());
    }
}
