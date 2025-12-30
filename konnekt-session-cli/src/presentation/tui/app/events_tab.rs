use crossterm::event::KeyCode;
use std::collections::VecDeque;

/// Events tab state (presentation only)
pub struct EventsTab {
    event_log: VecDeque<String>,
    scroll_offset: usize,
    max_events: usize,
}

impl EventsTab {
    pub fn new() -> Self {
        Self {
            event_log: VecDeque::new(),
            scroll_offset: 0,
            max_events: 100,
        }
    }

    pub fn handle_key(
        &mut self,
        key: KeyCode,
    ) -> Option<crate::presentation::tui::app::UserAction> {
        match key {
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                None
            }
            _ => None,
        }
    }

    pub fn add_event(&mut self, event: String) {
        self.event_log.push_front(event);
        if self.event_log.len() > self.max_events {
            self.event_log.pop_back();
        }
    }

    pub fn event_log(&self) -> &VecDeque<String> {
        &self.event_log
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }
}
