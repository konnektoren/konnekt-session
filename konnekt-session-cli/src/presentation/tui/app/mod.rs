use crossterm::event::KeyCode;
use konnekt_session_core::{Lobby, domain::ActivityMetadata};
use std::collections::VecDeque;
use uuid::Uuid;

mod activities_tab;
mod events_tab;
mod help_tab;
mod lobby_tab;
mod participants_tab;
mod results_tab;
mod session_tab;

pub use activities_tab::ActivitiesTab;
pub use events_tab::EventsTab;
pub use help_tab::HelpTab;
pub use lobby_tab::LobbyTab;
pub use participants_tab::ParticipantsTab;
pub use results_tab::ResultsTab;
pub use session_tab::SessionTab;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Session,
    Lobby,
    Activities,
    Participants,
    Results, // 🆕 NEW
    Events,
    Help,
}

impl Tab {
    pub fn next(&self) -> Self {
        match self {
            Tab::Session => Tab::Lobby,
            Tab::Lobby => Tab::Activities,
            Tab::Activities => Tab::Participants,
            Tab::Participants => Tab::Results, // 🆕
            Tab::Results => Tab::Events,       // 🆕
            Tab::Events => Tab::Help,
            Tab::Help => Tab::Session,
        }
    }

    pub fn previous(&self) -> Self {
        match self {
            Tab::Session => Tab::Help,
            Tab::Lobby => Tab::Session,
            Tab::Activities => Tab::Lobby,
            Tab::Participants => Tab::Activities,
            Tab::Results => Tab::Participants, // 🆕
            Tab::Events => Tab::Results,       // 🆕
            Tab::Help => Tab::Events,
        }
    }

    pub fn title(&self) -> &str {
        match self {
            Tab::Session => "Session",
            Tab::Lobby => "Lobby",
            Tab::Activities => "Activities",
            Tab::Participants => "Participants",
            Tab::Results => "Results", // 🆕
            Tab::Events => "Events",
            Tab::Help => "Help",
        }
    }
}

/// User actions (pure presentation events)
#[derive(Debug, Clone)]
pub enum UserAction {
    // Session actions
    CopySessionId,
    CopyJoinCommand,

    // Participant actions
    ToggleParticipationMode,
    KickParticipant(Uuid),

    // Activity actions (🆕)
    PlanActivity(ActivityMetadata),
    StartActivity(Uuid),
    CancelActivity(Uuid),
    SubmitActivityResult { activity_id: Uuid, response: String },

    // General
    Quit,
}

/// Pure presentation state (no business logic)
pub struct App {
    // Current state
    pub session_id: String,
    pub current_tab: Tab,

    // Tab state
    pub session_tab: SessionTab,
    pub lobby_tab: LobbyTab,
    pub activities_tab: ActivitiesTab,
    pub results_tab: ResultsTab,
    pub participants_tab: ParticipantsTab,
    pub events_tab: EventsTab,
    pub help_tab: HelpTab,

    // Flags
    pub should_quit: bool,

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
            session_id: session_id.clone(),
            current_tab: Tab::Session,

            session_tab: SessionTab::new(session_id),
            lobby_tab: LobbyTab::new(),
            activities_tab: ActivitiesTab::new(),
            results_tab: ResultsTab::new(),
            participants_tab: ParticipantsTab::new(),
            events_tab: EventsTab::new(),
            help_tab: HelpTab::new(),

            should_quit: false,

            lobby_snapshot: None,
            local_peer_id: None,
            local_participant_id: None,
            peer_count: 0,
            is_host: false,
        }
    }

    /// Handle keyboard input → returns UserAction if applicable
    pub fn handle_key(&mut self, key: KeyCode) -> Option<UserAction> {
        // Global keys
        match key {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.should_quit = true;
                return Some(UserAction::Quit);
            }

            KeyCode::Tab | KeyCode::Right => {
                self.current_tab = self.current_tab.next();
                return None;
            }

            KeyCode::BackTab | KeyCode::Left => {
                self.current_tab = self.current_tab.previous();
                return None;
            }

            _ => {}
        }

        // Tab-specific keys
        match self.current_tab {
            Tab::Session => self.session_tab.handle_key(key),
            Tab::Lobby => self.lobby_tab.handle_key(key),
            Tab::Activities => self.activities_tab.handle_key(key, self.is_host),
            Tab::Participants => {
                self.participants_tab
                    .handle_key(key, self.is_host, &self.lobby_snapshot)
            }
            Tab::Results => self.results_tab.handle_key(key), // 🆕 NEW
            Tab::Events => self.events_tab.handle_key(key),
            Tab::Help => None,
        }
    }

    /// Update lobby snapshot from SessionLoop
    pub fn update_lobby(&mut self, lobby: Lobby) {
        // Find our participant ID by matching role
        if self.local_participant_id.is_none() {
            for participant in lobby.participants().values() {
                if self.is_host && participant.is_host() {
                    self.local_participant_id = Some(participant.id());
                    break;
                } else if !self.is_host && !participant.is_host() {
                    self.local_participant_id = Some(participant.id());
                    break;
                }
            }
        }

        // Update tab states
        self.lobby_tab.update_lobby(&lobby);
        self.activities_tab.update_lobby(&lobby);
        self.participants_tab.update_lobby(&lobby);
        self.results_tab.update_lobby(&lobby);
        self.lobby_snapshot = Some(lobby);
    }

    /// Update peer info from SessionLoop
    pub fn update_peer_info(&mut self, peer_id: String, peer_count: usize, is_host: bool) {
        self.local_peer_id = Some(peer_id.clone());
        self.peer_count = peer_count;
        self.is_host = is_host;

        self.session_tab.update_peer_info(peer_id, peer_count);
        self.activities_tab.update_is_host(is_host);
    }

    /// Get local participant ID
    pub fn get_local_participant_id(&self) -> Option<Uuid> {
        self.local_participant_id
    }

    /// Add event to log (for display only)
    pub fn add_event(&mut self, event: String) {
        self.events_tab.add_event(event);
    }

    /// Tick for UI animations
    pub fn tick(&mut self) {
        self.session_tab.tick();
    }

    /// Copy session ID to clipboard (presentation concern)
    pub fn copy_session_id(&mut self) -> Result<(), String> {
        self.session_tab.copy_session_id()
    }

    /// Copy join command to clipboard (presentation concern)
    pub fn copy_join_command(&mut self) -> Result<(), String> {
        self.session_tab.copy_join_command()
    }
}
