use crossterm::event::KeyCode;
use konnekt_session_core::{
    EchoChallenge, Lobby,
    domain::{ActivityMetadata, ActivityStatus},
};
use uuid::Uuid;

use crate::presentation::tui::app::UserAction;

/// Available activity template
#[derive(Debug, Clone)]
pub struct ActivityTemplate {
    pub name: String,
    pub activity_type: String,
    pub description: String,
    pub config: serde_json::Value,
}

impl ActivityTemplate {
    /// Create metadata from this template
    pub fn to_metadata(&self) -> ActivityMetadata {
        ActivityMetadata::new(
            self.activity_type.clone(),
            self.name.clone(),
            self.config.clone(),
        )
    }
}

/// Activities tab state (presentation only)
pub struct ActivitiesTab {
    // Host: Available activity templates
    available_activities: Vec<ActivityTemplate>,
    selected_template: usize,

    // Shared: Planned/running activities
    planned_activities: Vec<ActivityMetadata>,
    current_activity: Option<ActivityMetadata>,

    // Host + Guest: Activity input
    activity_input: String,
    cursor_position: usize,

    // State
    is_host: bool,
}

impl ActivitiesTab {
    pub fn new() -> Self {
        Self {
            available_activities: Self::create_default_templates(),
            selected_template: 0,
            planned_activities: Vec::new(),
            current_activity: None,
            activity_input: String::new(),
            cursor_position: 0,
            is_host: false,
        }
    }

    /// Create default activity templates (5 Echo challenges)
    fn create_default_templates() -> Vec<ActivityTemplate> {
        vec![
            ActivityTemplate {
                name: "Echo: Hello Rust".to_string(),
                activity_type: "echo-challenge-v1".to_string(),
                description: "Echo back: Hello Rust".to_string(),
                config: EchoChallenge::new("Hello Rust".to_string()).to_config(),
            },
            ActivityTemplate {
                name: "Echo: WebAssembly".to_string(),
                activity_type: "echo-challenge-v1".to_string(),
                description: "Echo back: WebAssembly".to_string(),
                config: EchoChallenge::new("WebAssembly".to_string()).to_config(),
            },
            ActivityTemplate {
                name: "Echo: Konnekt".to_string(),
                activity_type: "echo-challenge-v1".to_string(),
                description: "Echo back: Konnekt".to_string(),
                config: EchoChallenge::new("Konnekt".to_string()).to_config(),
            },
            ActivityTemplate {
                name: "Echo: P2P Session".to_string(),
                activity_type: "echo-challenge-v1".to_string(),
                description: "Echo back: P2P Session".to_string(),
                config: EchoChallenge::new("P2P Session".to_string()).to_config(),
            },
            ActivityTemplate {
                name: "Echo: DDD + Hexagonal".to_string(),
                activity_type: "echo-challenge-v1".to_string(),
                description: "Echo back: DDD + Hexagonal".to_string(),
                config: EchoChallenge::new("DDD + Hexagonal".to_string()).to_config(),
            },
        ]
    }

    pub fn handle_key(&mut self, key: KeyCode, is_host: bool) -> Option<UserAction> {
        // If activity is running, both host and guest use input mode
        if self.current_activity.is_some() {
            return self.handle_activity_input(key);
        }

        // Otherwise, host gets planning/management controls
        if is_host {
            self.handle_host_planning_key(key)
        } else {
            None // Guest has no controls when no activity
        }
    }

    /// Handle activity input (both host and guest during active activity)
    fn handle_activity_input(&mut self, key: KeyCode) -> Option<UserAction> {
        match key {
            // 🔧 FIX: Check for 'x' BEFORE generic Char(c)
            KeyCode::Char('x') if self.is_host => {
                if let Some(activity) = &self.current_activity {
                    Some(UserAction::CancelActivity(activity.id))
                } else {
                    None
                }
            }

            KeyCode::Char(c) => {
                self.activity_input.insert(self.cursor_position, c);
                self.cursor_position += 1;
                None
            }

            KeyCode::Backspace => {
                if self.cursor_position > 0 {
                    self.cursor_position -= 1;
                    self.activity_input.remove(self.cursor_position);
                }
                None
            }

            KeyCode::Left => {
                self.cursor_position = self.cursor_position.saturating_sub(1);
                None
            }

            KeyCode::Right => {
                self.cursor_position = (self.cursor_position + 1).min(self.activity_input.len());
                None
            }

            KeyCode::Enter => {
                if let Some(activity) = &self.current_activity {
                    let response = self.activity_input.clone();
                    self.activity_input.clear();
                    self.cursor_position = 0;

                    Some(UserAction::SubmitActivityResult {
                        activity_id: activity.id,
                        response,
                    })
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    /// Host-specific planning/management keys (when NO activity is running)
    fn handle_host_planning_key(&mut self, key: KeyCode) -> Option<UserAction> {
        match key {
            // Navigate templates (only when no activity running)
            KeyCode::Up | KeyCode::Char('k') => {
                self.selected_template = self.selected_template.saturating_sub(1);
                None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let max = self.available_activities.len().saturating_sub(1);
                self.selected_template = (self.selected_template + 1).min(max);
                None
            }

            // Plan activity (only when no activity running)
            KeyCode::Char('p') => {
                if let Some(template) = self.available_activities.get(self.selected_template) {
                    let metadata = template.to_metadata();
                    Some(UserAction::PlanActivity(metadata))
                } else {
                    None
                }
            }

            // Start activity (only when activities are planned but none running)
            KeyCode::Char('s') if !self.planned_activities.is_empty() => {
                if let Some(activity) = self.planned_activities.first() {
                    Some(UserAction::StartActivity(activity.id))
                } else {
                    None
                }
            }

            _ => None,
        }
    }

    pub fn update_lobby(&mut self, lobby: &Lobby) {
        self.planned_activities = lobby
            .activities()
            .iter()
            .filter(|a| matches!(a.status, ActivityStatus::Planned))
            .cloned()
            .collect();

        self.current_activity = lobby.current_activity().cloned();

        // Clear input if activity completed
        if self.current_activity.is_none() {
            self.activity_input.clear();
            self.cursor_position = 0;
        }
    }

    pub fn update_is_host(&mut self, is_host: bool) {
        self.is_host = is_host;
    }

    // Getters for rendering
    pub fn available_activities(&self) -> &[ActivityTemplate] {
        &self.available_activities
    }

    pub fn selected_template(&self) -> usize {
        self.selected_template
    }

    pub fn planned_activities(&self) -> &[ActivityMetadata] {
        &self.planned_activities
    }

    pub fn current_activity(&self) -> Option<&ActivityMetadata> {
        self.current_activity.as_ref()
    }

    pub fn activity_input(&self) -> &str {
        &self.activity_input
    }

    pub fn cursor_position(&self) -> usize {
        self.cursor_position
    }

    pub fn is_host(&self) -> bool {
        self.is_host
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_host_can_input_during_activity() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(true);

        // Simulate activity starting
        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        tab.current_activity = Some(metadata);

        // Host should be able to type
        tab.handle_key(KeyCode::Char('H'), true);
        tab.handle_key(KeyCode::Char('i'), true);

        assert_eq!(tab.activity_input, "Hi");
    }

    #[test]
    fn test_host_can_cancel_during_activity() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(true);

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        let activity_id = metadata.id;
        tab.current_activity = Some(metadata);

        // Host presses 'x' to cancel
        let action = tab.handle_key(KeyCode::Char('x'), true);

        match action {
            Some(UserAction::CancelActivity(id)) => {
                assert_eq!(id, activity_id);
            }
            _ => panic!("Expected CancelActivity action, got: {:?}", action),
        }
    }

    #[test]
    fn test_host_cannot_navigate_during_activity() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(true);

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        tab.current_activity = Some(metadata);

        let initial_selection = tab.selected_template;

        // Try to navigate (should be ignored)
        tab.handle_key(KeyCode::Down, true);

        // Selection should not change
        assert_eq!(tab.selected_template, initial_selection);
    }

    #[test]
    fn test_host_can_navigate_when_no_activity() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(true);
        tab.current_activity = None;

        assert_eq!(tab.selected_template, 0);

        tab.handle_key(KeyCode::Down, true);
        assert_eq!(tab.selected_template, 1);

        tab.handle_key(KeyCode::Up, true);
        assert_eq!(tab.selected_template, 0);
    }

    #[test]
    fn test_guest_input_during_activity() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(false);

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        tab.current_activity = Some(metadata);

        tab.handle_key(KeyCode::Char('T'), false);
        tab.handle_key(KeyCode::Char('e'), false);
        tab.handle_key(KeyCode::Char('s'), false);
        tab.handle_key(KeyCode::Char('t'), false);

        assert_eq!(tab.activity_input, "Test");
    }

    #[test]
    fn test_guest_cannot_cancel() {
        let mut tab = ActivitiesTab::new();
        tab.update_is_host(false);

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        tab.current_activity = Some(metadata);

        // Guest presses 'x' (should be treated as text input, not cancel)
        let action = tab.handle_key(KeyCode::Char('x'), false);

        assert!(action.is_none());
        assert_eq!(tab.activity_input, "x");
    }

    #[test]
    fn test_submit_result_clears_input() {
        let mut tab = ActivitiesTab::new();

        let challenge = EchoChallenge::new("Test".to_string());
        let metadata = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Test Activity".to_string(),
            challenge.to_config(),
        );
        tab.current_activity = Some(metadata);

        tab.activity_input = "My Answer".to_string();
        tab.cursor_position = 9;

        tab.handle_key(KeyCode::Enter, false);

        assert_eq!(tab.activity_input, "");
        assert_eq!(tab.cursor_position, 0);
    }
}
