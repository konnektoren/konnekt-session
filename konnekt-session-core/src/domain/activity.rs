use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub type ActivityId = Uuid;

/// Value object sitting in the Lobby's activity queue.
/// Promoted to ActivityRun when the host starts it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityConfig {
    pub id: ActivityId,
    pub activity_type: String,
    pub name: String,
    /// Game-specific config — opaque to the library.
    #[serde(default)]
    pub config: serde_json::Value,
}

impl ActivityConfig {
    pub fn new(activity_type: String, name: String, config: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            activity_type,
            name,
            config,
        }
    }

    pub fn with_id(
        id: ActivityId,
        activity_type: String,
        name: String,
        config: serde_json::Value,
    ) -> Self {
        Self {
            id,
            activity_type,
            name,
            config,
        }
    }
}

/// Result submitted by a participant for a run.
/// `data` is opaque — the consuming app owns the concrete type.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ActivityResult {
    pub run_id: Uuid,
    pub participant_id: Uuid,
    #[serde(default)]
    pub data: serde_json::Value,
    pub score: Option<u32>,
    pub time_taken_ms: Option<u64>,
}

impl ActivityResult {
    pub fn new(run_id: Uuid, participant_id: Uuid) -> Self {
        Self {
            run_id,
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
    fn test_create_activity_config() {
        let config = serde_json::json!({"question_count": 10});
        let ac = ActivityConfig::new("trivia-v1".to_string(), "Friday Quiz".to_string(), config.clone());

        assert_eq!(ac.activity_type, "trivia-v1");
        assert_eq!(ac.name, "Friday Quiz");
        assert_eq!(ac.config, config);
    }

    #[test]
    fn test_activity_result_builder() {
        let run_id = Uuid::new_v4();
        let participant_id = Uuid::new_v4();

        let result = ActivityResult::new(run_id, participant_id)
            .with_score(42)
            .with_time(1500)
            .with_data(serde_json::json!({"answers": [1, 2, 3]}));

        assert_eq!(result.score, Some(42));
        assert_eq!(result.time_taken_ms, Some(1500));
        assert!(!result.data.is_null());
    }
}
