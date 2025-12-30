use crossterm::event::KeyCode;
use konnekt_session_core::{
    EchoResult, Lobby,
    domain::{ActivityId, ActivityResult, ActivityStatus},
};
use std::collections::HashMap;
use uuid::Uuid;

/// Activity result with participant name (for display)
#[derive(Debug, Clone)]
pub struct DisplayResult {
    pub participant_name: String,
    pub participant_id: Uuid,
    pub score: Option<u32>,
    pub response: Option<String>,
    pub time_ms: Option<u64>,
}

/// Results for a completed activity
#[derive(Debug, Clone)]
pub struct ActivityResults {
    pub activity_id: ActivityId,
    pub activity_name: String,
    pub results: Vec<DisplayResult>,
}

/// Results tab state (presentation only)
pub struct ResultsTab {
    /// All completed activities with results
    completed_activities: Vec<ActivityResults>,

    /// Selected activity index
    selected_activity: usize,

    /// Selected result index (for detail view)
    selected_result: usize,
}

impl ResultsTab {
    pub fn new() -> Self {
        Self {
            completed_activities: Vec::new(),
            selected_activity: 0,
            selected_result: 0,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
    ) -> Option<crate::presentation::tui::app::UserAction> {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                if !self.completed_activities.is_empty() {
                    let max = self.completed_activities.len().saturating_sub(1);
                    self.selected_activity = (self.selected_activity + 1).min(max);
                    self.selected_result = 0; // Reset result selection
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected_activity = self.selected_activity.saturating_sub(1);
                self.selected_result = 0; // Reset result selection
                None
            }
            _ => None,
        }
    }

    pub fn update_lobby(&mut self, lobby: &Lobby) {
        // Rebuild completed activities from lobby
        self.completed_activities.clear();

        for activity in lobby.activities() {
            if activity.status == ActivityStatus::Completed {
                let results: Vec<DisplayResult> = lobby
                    .get_results(activity.id)
                    .into_iter()
                    .filter_map(|result| {
                        // Look up participant name
                        let participant = lobby.participants().get(&result.participant_id)?;

                        // Parse response from result data
                        let response =
                            if let Ok(echo_result) = EchoResult::from_json(result.data.clone()) {
                                Some(echo_result.response)
                            } else {
                                None
                            };

                        Some(DisplayResult {
                            participant_name: participant.name().to_string(),
                            participant_id: result.participant_id,
                            score: result.score,
                            response,
                            time_ms: result.time_taken_ms,
                        })
                    })
                    .collect();

                self.completed_activities.push(ActivityResults {
                    activity_id: activity.id,
                    activity_name: activity.name.clone(),
                    results,
                });
            }
        }

        // Clamp selections
        if !self.completed_activities.is_empty() {
            let max_activity = self.completed_activities.len().saturating_sub(1);
            self.selected_activity = self.selected_activity.min(max_activity);
        }
    }

    // Getters for rendering
    pub fn completed_activities(&self) -> &[ActivityResults] {
        &self.completed_activities
    }

    pub fn selected_activity(&self) -> usize {
        self.selected_activity
    }

    pub fn selected_result(&self) -> usize {
        self.selected_result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::{EchoChallenge, Participant, domain::ActivityMetadata};

    fn create_test_lobby_with_results() -> Lobby {
        let host = Participant::new_host("Host".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host.clone()).unwrap();

        let guest = Participant::new_guest("Alice".to_string()).unwrap();
        lobby.add_guest(guest.clone()).unwrap();

        // Create and start activity
        let challenge = EchoChallenge::new("Hello Rust".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Echo Test".to_string(),
            challenge.to_config(),
        );
        let activity_id = metadata.id;

        lobby.plan_activity(metadata).unwrap();
        lobby.start_activity(activity_id).unwrap();

        // Submit results
        let echo_result = EchoResult::new("Hello Rust".to_string(), 1500);
        let result1 = ActivityResult::new(activity_id, host.id())
            .with_data(echo_result.to_json())
            .with_score(100)
            .with_time(1500);

        let echo_result2 = EchoResult::new("Hello Rust".to_string(), 2000);
        let result2 = ActivityResult::new(activity_id, guest.id())
            .with_data(echo_result2.to_json())
            .with_score(100)
            .with_time(2000);

        lobby.submit_result(result1).unwrap();
        lobby.submit_result(result2).unwrap();

        lobby
    }

    #[test]
    fn test_update_lobby_parses_results() {
        let lobby = create_test_lobby_with_results();
        let mut tab = ResultsTab::new();

        tab.update_lobby(&lobby);

        assert_eq!(tab.completed_activities.len(), 1);
        assert_eq!(tab.completed_activities[0].activity_name, "Echo Test");
        assert_eq!(tab.completed_activities[0].results.len(), 2);

        let first_result = &tab.completed_activities[0].results[0];
        assert_eq!(first_result.participant_name, "Host");
        assert_eq!(first_result.score, Some(100));
        assert_eq!(first_result.response, Some("Hello Rust".to_string()));
        assert_eq!(first_result.time_ms, Some(1500));
    }

    #[test]
    fn test_navigation() {
        let mut tab = ResultsTab::new();

        // Add fake data
        tab.completed_activities = vec![
            ActivityResults {
                activity_id: Uuid::new_v4(),
                activity_name: "Activity 1".to_string(),
                results: vec![],
            },
            ActivityResults {
                activity_id: Uuid::new_v4(),
                activity_name: "Activity 2".to_string(),
                results: vec![],
            },
        ];

        assert_eq!(tab.selected_activity, 0);

        tab.handle_key(KeyCode::Down);
        assert_eq!(tab.selected_activity, 1);

        tab.handle_key(KeyCode::Down);
        assert_eq!(tab.selected_activity, 1); // Clamped

        tab.handle_key(KeyCode::Up);
        assert_eq!(tab.selected_activity, 0);
    }
}
