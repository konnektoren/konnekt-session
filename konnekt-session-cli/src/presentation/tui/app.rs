use crate::domain::SessionState;
use crossterm::event::KeyCode;
use konnekt_session_p2p::ConnectionEvent;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Session, // New: Show session ID prominently
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
    pub max_events: usize,
}

impl App {
    pub fn new(session_state: SessionState, session_id: String) -> Self {
        Self {
            session_state,
            session_id,
            local_peer_id: None,
            current_tab: Tab::Session, // Start on Session tab to show ID
            event_log: VecDeque::new(),
            scroll_offset: 0,
            should_quit: false,
            max_events: 100,
        }
    }

    pub fn set_local_peer_id(&mut self, peer_id: String) {
        self.local_peer_id = Some(peer_id);
    }

    pub fn handle_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
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
