use crossterm::event::KeyCode;
use konnekt_session_core::Lobby;
use std::collections::VecDeque;
use uuid::Uuid;

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

/// User actions (pure presentation events)
#[derive(Debug, Clone)]
pub enum UserAction {
    ToggleParticipationMode,
    KickParticipant(Uuid),
    CopySessionId,
    CopyJoinCommand,
    Quit,
}

/// Pure presentation state (no business logic)
pub struct App {
    // Display state
    pub session_id: String,
    pub current_tab: Tab,
    pub event_log: VecDeque<String>,
    pub scroll_offset: usize,
    pub selected_participant: usize,

    // UI feedback
    pub clipboard_message: Option<String>,
    pub clipboard_message_timer: usize,

    // Flags
    pub should_quit: bool,
    pub max_events: usize,

    // Cached state from SessionLoop (read-only snapshots)
    pub lobby_snapshot: Option<Lobby>,
    pub local_peer_id: Option<String>,
    pub local_participant_id: Option<Uuid>,
    pub peer_count: usize,
    pub is_host: bool,
}

impl App {
    pub fn new(session_id: String) -> Self {
        Self {
            session_id,
            current_tab: Tab::Session,
            event_log: VecDeque::new(),
            scroll_offset: 0,
            selected_participant: 0,
            clipboard_message: None,
            clipboard_message_timer: 0,
            should_quit: false,
            max_events: 100,
            lobby_snapshot: None,
            local_peer_id: None,
            local_participant_id: None,
            peer_count: 0,
            is_host: false,
        }
    }

    /// Handle keyboard input → returns UserAction if applicable
    pub fn handle_key(&mut self, key: KeyCode) -> Option<UserAction> {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                Some(UserAction::Quit)
            }

            KeyCode::Char('y') if self.current_tab == Tab::Session => {
                Some(UserAction::CopySessionId)
            }

            KeyCode::Char('c') if self.current_tab == Tab::Session => {
                Some(UserAction::CopyJoinCommand)
            }

            KeyCode::Char('t') => Some(UserAction::ToggleParticipationMode),

            KeyCode::Char('x') if self.current_tab == Tab::Participants && self.is_host => {
                if let Some(lobby) = &self.lobby_snapshot {
                    let participants: Vec<_> = lobby.participants().values().collect();
                    if self.selected_participant < participants.len() {
                        let selected = participants[self.selected_participant];
                        if !selected.is_host() {
                            return Some(UserAction::KickParticipant(selected.id()));
                        }
                    }
                }
                None
            }

            KeyCode::Tab | KeyCode::Right => {
                self.current_tab = self.current_tab.next();
                self.scroll_offset = 0;
                self.selected_participant = 0;
                None
            }

            KeyCode::BackTab | KeyCode::Left => {
                self.current_tab = self.current_tab.previous();
                self.scroll_offset = 0;
                self.selected_participant = 0;
                None
            }

            KeyCode::Char('j') | KeyCode::Down => {
                if self.current_tab == Tab::Participants {
                    if let Some(lobby) = &self.lobby_snapshot {
                        let max = lobby.participants().len().saturating_sub(1);
                        self.selected_participant = (self.selected_participant + 1).min(max);
                    }
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_add(1);
                }
                None
            }

            KeyCode::Char('k') | KeyCode::Up => {
                if self.current_tab == Tab::Participants {
                    self.selected_participant = self.selected_participant.saturating_sub(1);
                } else {
                    self.scroll_offset = self.scroll_offset.saturating_sub(1);
                }
                None
            }

            _ => None,
        }
    }

    /// Update lobby snapshot from SessionLoop
    pub fn update_lobby(&mut self, lobby: Lobby) {
        // Find our participant ID by matching peer ID or role
        if self.local_participant_id.is_none() {
            // Find ourselves in the lobby
            for participant in lobby.participants().values() {
                if self.is_host && participant.is_host() {
                    self.local_participant_id = Some(participant.id());
                    break;
                } else if !self.is_host && !participant.is_host() {
                    // For guests, we'll track the first one we see
                    // (This is simplified - in production, use peer-participant mapping)
                    self.local_participant_id = Some(participant.id());
                    break;
                }
            }
        }

        self.lobby_snapshot = Some(lobby);
    }

    /// Update peer info from SessionLoop
    pub fn update_peer_info(&mut self, peer_id: String, peer_count: usize, is_host: bool) {
        self.local_peer_id = Some(peer_id);
        self.peer_count = peer_count;
        self.is_host = is_host;
    }

    /// Get local participant ID
    pub fn get_local_participant_id(&self) -> Option<Uuid> {
        self.local_participant_id
    }

    /// Add event to log (for display only)
    pub fn add_event(&mut self, event: String) {
        self.event_log.push_front(event);
        if self.event_log.len() > self.max_events {
            self.event_log.pop_back();
        }
    }

    /// Tick for UI animations
    pub fn tick(&mut self) {
        if self.clipboard_message_timer > 0 {
            self.clipboard_message_timer -= 1;
            if self.clipboard_message_timer == 0 {
                self.clipboard_message = None;
            }
        }
    }

    /// Show clipboard feedback
    pub fn show_clipboard_message(&mut self, message: String) {
        self.clipboard_message = Some(message);
        self.clipboard_message_timer = 30; // 3 seconds at 100ms ticks
    }

    /// Copy session ID to clipboard (presentation concern)
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

    /// Copy join command to clipboard (presentation concern)
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
}
