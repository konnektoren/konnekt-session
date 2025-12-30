use crossterm::event::KeyCode;

use crate::presentation::tui::app::UserAction;

/// Session tab state (presentation only)
pub struct SessionTab {
    session_id: String,
    clipboard_message: Option<String>,
    clipboard_message_timer: usize,
    local_peer_id: Option<String>,
    peer_count: usize,
}

impl SessionTab {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            clipboard_message: None,
            clipboard_message_timer: 0,
            local_peer_id: None,
            peer_count: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyCode) -> Option<UserAction> {
        match key {
            KeyCode::Char('y') => Some(UserAction::CopySessionId),
            KeyCode::Char('c') => Some(UserAction::CopyJoinCommand),
            _ => None,
        }
    }

    pub fn update_peer_info(&mut self, peer_id: String, peer_count: usize) {
        self.local_peer_id = Some(peer_id);
        self.peer_count = peer_count;
    }

    pub fn tick(&mut self) {
        if self.clipboard_message_timer > 0 {
            self.clipboard_message_timer -= 1;
            if self.clipboard_message_timer == 0 {
                self.clipboard_message = None;
            }
        }
    }

    pub fn show_clipboard_message(&mut self, message: String) {
        self.clipboard_message = Some(message);
        self.clipboard_message_timer = 30; // 3 seconds at 100ms ticks
    }

    pub fn copy_session_id(&mut self) -> Result<(), String> {
        #[cfg(feature = "tui")]
        {
            use arboard::Clipboard;
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(&self.session_id) {
                    Ok(_) => {
                        self.show_clipboard_message("✓ Session ID copied!".to_string());
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("✗ Failed: {}", e);
                        self.show_clipboard_message(msg.clone());
                        Err(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("✗ Clipboard unavailable: {}", e);
                    self.show_clipboard_message(msg.clone());
                    Err(msg)
                }
            }
        }
        #[cfg(not(feature = "tui"))]
        {
            Err("Clipboard not available".to_string())
        }
    }

    pub fn copy_join_command(&mut self) -> Result<(), String> {
        #[cfg(feature = "tui")]
        {
            use arboard::Clipboard;
            let command = format!("konnekt-tui join --session-id {}", self.session_id);
            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(&command) {
                    Ok(_) => {
                        self.show_clipboard_message("✓ Join command copied!".to_string());
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("✗ Failed: {}", e);
                        self.show_clipboard_message(msg.clone());
                        Err(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("✗ Clipboard unavailable: {}", e);
                    self.show_clipboard_message(msg.clone());
                    Err(msg)
                }
            }
        }
        #[cfg(not(feature = "tui"))]
        {
            Err("Clipboard not available".to_string())
        }
    }

    // Getters for rendering
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    pub fn clipboard_message(&self) -> Option<&str> {
        self.clipboard_message.as_deref()
    }

    pub fn local_peer_id(&self) -> Option<&str> {
        self.local_peer_id.as_deref()
    }

    pub fn peer_count(&self) -> usize {
        self.peer_count
    }
}
