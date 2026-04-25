use crossterm::event::KeyCode;
use konnekt_session_core::{
    Lobby,
    domain::ActivityId,
};
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
        // Current lobby snapshot does not include completed run history/results.
        // Keep this tab empty until a history resource is introduced.
        let _ = lobby;
        self.completed_activities.clear();

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
