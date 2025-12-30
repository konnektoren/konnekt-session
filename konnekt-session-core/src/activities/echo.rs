use serde::{Deserialize, Serialize};

/// Echo Challenge - Simplest possible activity for testing
///
/// Participants receive a prompt and must echo it back.
/// Score = 100 if exact match, 0 otherwise.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoChallenge {
    /// The word/phrase to echo
    pub prompt: String,

    /// Optional time limit in milliseconds
    pub time_limit_ms: Option<u64>,
}

impl EchoChallenge {
    /// Create a new echo challenge
    pub fn new(prompt: String) -> Self {
        Self {
            prompt,
            time_limit_ms: None,
        }
    }

    /// With time limit
    pub fn with_time_limit(mut self, ms: u64) -> Self {
        self.time_limit_ms = Some(ms);
        self
    }

    /// Activity type identifier
    pub fn activity_type() -> &'static str {
        "echo-challenge-v1"
    }

    /// Validate a response
    pub fn validate_response(&self, response: &str) -> bool {
        response == self.prompt
    }

    /// Calculate score
    pub fn calculate_score(&self, response: &str) -> u32 {
        if self.validate_response(response) {
            100
        } else {
            0
        }
    }

    /// Serialize to JSON for transport
    pub fn to_config(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    /// Deserialize from JSON
    pub fn from_config(config: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(config)
    }
}

/// Echo result data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EchoResult {
    /// The response given
    pub response: String,

    /// Time taken in milliseconds
    pub time_ms: u64,
}

impl EchoResult {
    pub fn new(response: String, time_ms: u64) -> Self {
        Self { response, time_ms }
    }

    pub fn to_json(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap()
    }

    pub fn from_json(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_echo_challenge_basic() {
        let challenge = EchoChallenge::new("Hello World".to_string());

        assert!(challenge.validate_response("Hello World"));
        assert!(!challenge.validate_response("hello world")); // Case sensitive
        assert!(!challenge.validate_response("Hello"));
    }

    #[test]
    fn test_echo_challenge_scoring() {
        let challenge = EchoChallenge::new("Rust".to_string());

        assert_eq!(challenge.calculate_score("Rust"), 100);
        assert_eq!(challenge.calculate_score("rust"), 0);
        assert_eq!(challenge.calculate_score("Python"), 0);
    }

    #[test]
    fn test_serialization() {
        let challenge = EchoChallenge::new("Test".to_string()).with_time_limit(5000);

        let config = challenge.to_config();
        let deserialized = EchoChallenge::from_config(config).unwrap();

        assert_eq!(deserialized.prompt, "Test");
        assert_eq!(deserialized.time_limit_ms, Some(5000));
    }

    #[test]
    fn test_result_serialization() {
        let result = EchoResult::new("Hello".to_string(), 1234);

        let json = result.to_json();
        let deserialized = EchoResult::from_json(json).unwrap();

        assert_eq!(deserialized.response, "Hello");
        assert_eq!(deserialized.time_ms, 1234);
    }
}
