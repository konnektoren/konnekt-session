use crate::domain::SessionState;
use crossterm::event::KeyCode;
use konnekt_session_p2p::ConnectionEvent;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Session,
    Lobby,
    Events,
    Participants,
    Help,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Session => Tab::Lobby,
            Tab::Lobby => Tab::Events,
            Tab::Events => Tab::Participants,
            Tab::Participants => Tab::Help,
            Tab::Help => Tab::Session,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Session => Tab::Help,
            Tab::Lobby => Tab::Session,
            Tab::Events => Tab::Lobby,
            Tab::Participants => Tab::Events,
            Tab::Help => Tab::Participants,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Session => "Session",
            Tab::Lobby => "Lobby",
            Tab::Events => "Events",
            Tab::Participants => "Participants",
            Tab::Help => "Help",
        }
    }
}

pub struct App {
    pub session_state: SessionState,
    pub session_id: String,
    pub local_peer_id: Option<String>,
    pub current_tab: Tab,
    pub event_log: VecDeque<String>,
    pub scroll_offset: usize,
    pub should_quit: bool,
    pub clipboard_message: Option<String>,
    pub clipboard_message_timer: usize,
    pub max_events: usize,
    pub toggle_spectator_requested: bool,
}

impl App {
    pub fn new(session_state: SessionState, session_id: String) -> Self {
        Self {
            session_state,
            session_id,
            local_peer_id: None,
            current_tab: Tab::Session,
            event_log: VecDeque::new(),
            scroll_offset: 0,
            should_quit: false,
            clipboard_message: None,
            clipboard_message_timer: 0,
            max_events: 100,
            toggle_spectator_requested: false,
        }
    }

    pub fn set_local_peer_id(&mut self, peer_id: String) {
        self.local_peer_id = Some(peer_id);
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
        self.clipboard_message_timer = 30; // Show for 3 seconds (30 * 100ms ticks)
    }

    pub fn copy_session_id(&mut self) -> Result<(), String> {
        #[cfg(feature = "tui")]
        {
            use arboard::Clipboard;

            match Clipboard::new() {
                Ok(mut clipboard) => match clipboard.set_text(&self.session_id) {
                    Ok(_) => {
                        self.show_clipboard_message(
                            "✓ Session ID copied to clipboard!".to_string(),
                        );
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("✗ Failed to copy: {}", e);
                        self.show_clipboard_message(msg.clone());
                        Err(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("✗ Clipboard not available: {}", e);
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
                        self.show_clipboard_message(
                            "✓ Join command copied to clipboard!".to_string(),
                        );
                        Ok(())
                    }
                    Err(e) => {
                        let msg = format!("✗ Failed to copy: {}", e);
                        self.show_clipboard_message(msg.clone());
                        Err(msg)
                    }
                },
                Err(e) => {
                    let msg = format!("✗ Clipboard not available: {}", e);
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

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
            }
            KeyCode::Char('y') if self.current_tab == Tab::Session => {
                // Copy session ID
                let _ = self.copy_session_id();
            }
            KeyCode::Char('c') if self.current_tab == Tab::Session => {
                // Copy join command
                let _ = self.copy_join_command();
            }
            KeyCode::Char('t') => {
                // Toggle participation mode (works on any tab)
                self.toggle_spectator_requested = true;
            }
            KeyCode::Tab | KeyCode::Right => {
                self.current_tab = self.current_tab.next();
                self.scroll_offset = 0;
            }
            KeyCode::BackTab | KeyCode::Left => {
                self.current_tab = self.current_tab.previous();
                self.scroll_offset = 0;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn add_event(&mut self, event: String) {
        self.event_log.push_front(event);
        if self.event_log.len() > self.max_events {
            self.event_log.pop_back();
        }
    }

    pub fn handle_connection_event(&mut self, event: &ConnectionEvent) {
        match event {
            ConnectionEvent::PeerConnected(peer_id) => {
                self.add_event(format!("🟢 Peer connected: {}", peer_id));
            }
            ConnectionEvent::PeerDisconnected(peer_id) => {
                self.add_event(format!("🔴 Peer disconnected: {}", peer_id));
            }
            ConnectionEvent::PeerTimedOut {
                peer_id, was_host, ..
            } => {
                self.add_event(format!(
                    "⏰ Peer timed out: {} (was_host: {})",
                    peer_id, was_host
                ));
            }
            ConnectionEvent::MessageReceived { from, data } => {
                self.add_event(format!("📥 Message from {}: {} bytes", from, data.len()));
            }
        }
    }
}
