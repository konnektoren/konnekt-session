use crate::application::{DomainCommand, DomainEvent};
use crate::domain::{
    ActivityId, ActivityMetadata, ActivityResult, ActivityStatus, Lobby, LobbyError, Participant,
};
use std::collections::HashMap;
use uuid::Uuid;

/// Domain event loop that processes commands and emits events
#[derive(Debug, Clone)]
pub struct DomainEventLoop {
    /// All lobbies indexed by ID
    lobbies: HashMap<Uuid, Lobby>,
}

impl DomainEventLoop {
    /// Create a new domain event loop
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
        }
    }

    /// Process a single command and return the resulting event
    pub fn handle_command(&mut self, command: DomainCommand) -> DomainEvent {
        match command {
            DomainCommand::CreateLobby {
                lobby_id,
                lobby_name,
                host_name,
            } => self.handle_create_lobby(lobby_id, lobby_name, host_name),

            DomainCommand::JoinLobby {
                lobby_id,
                guest_name,
            } => self.handle_join_lobby(lobby_id, guest_name),

            DomainCommand::LeaveLobby {
                lobby_id,
                participant_id,
            } => self.handle_leave_lobby(lobby_id, participant_id),

            DomainCommand::KickGuest {
                lobby_id,
                host_id,
                guest_id,
            } => self.handle_kick_guest(lobby_id, host_id, guest_id),

            DomainCommand::ToggleParticipationMode {
                lobby_id,
                participant_id,
                requester_id,
                activity_in_progress,
            } => self.handle_toggle_participation_mode(
                lobby_id,
                participant_id,
                requester_id,
                activity_in_progress,
            ),

            DomainCommand::DelegateHost {
                lobby_id,
                current_host_id,
                new_host_id,
            } => self.handle_delegate_host(lobby_id, current_host_id, new_host_id),

            DomainCommand::PlanActivity { lobby_id, metadata } => {
                self.handle_plan_activity(lobby_id, metadata)
            }

            DomainCommand::StartActivity {
                lobby_id,
                activity_id,
            } => self.handle_start_activity(lobby_id, activity_id),

            DomainCommand::SubmitResult { lobby_id, result } => {
                self.handle_submit_result(lobby_id, result)
            }

            DomainCommand::CancelActivity {
                lobby_id,
                activity_id,
            } => self.handle_cancel_activity(lobby_id, activity_id),
        }
    }

    fn handle_create_lobby(
        &mut self,
        lobby_id: Option<Uuid>, // 🆕 Accept optional ID
        lobby_name: String,
        host_name: String,
    ) -> DomainEvent {
        match Participant::new_host(host_name) {
            Ok(host) => {
                let result = if let Some(id) = lobby_id {
                    Lobby::with_id(id, lobby_name, host) // 🆕 Use specific ID
                } else {
                    Lobby::new(lobby_name, host) // Generate random ID
                };

                match result {
                    Ok(lobby) => {
                        let lobby_id = lobby.id();
                        self.lobbies.insert(lobby_id, lobby.clone());
                        DomainEvent::LobbyCreated { lobby }
                    }
                    Err(e) => DomainEvent::CommandFailed {
                        command: "CreateLobby".to_string(),
                        reason: e.to_string(),
                    },
                }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "CreateLobby".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_join_lobby(&mut self, lobby_id: Uuid, guest_name: String) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "JoinLobby".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match Participant::new_guest(guest_name) {
            Ok(guest) => match lobby.add_guest(guest.clone()) {
                Ok(_) => DomainEvent::GuestJoined {
                    lobby_id,
                    participant: guest,
                },
                Err(e) => DomainEvent::CommandFailed {
                    command: "JoinLobby".to_string(),
                    reason: e.to_string(),
                },
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "JoinLobby".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_leave_lobby(&mut self, lobby_id: Uuid, participant_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "LeaveLobby".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.remove_participant(participant_id) {
            Ok(_) => DomainEvent::GuestLeft {
                lobby_id,
                participant_id,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "LeaveLobby".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_kick_guest(&mut self, lobby_id: Uuid, host_id: Uuid, guest_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "KickGuest".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.kick_guest(guest_id, host_id) {
            Ok(_) => DomainEvent::GuestKicked {
                lobby_id,
                participant_id: guest_id,
                kicked_by: host_id,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "KickGuest".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_toggle_participation_mode(
        &mut self,
        lobby_id: Uuid,
        participant_id: Uuid,
        requester_id: Uuid,
        activity_in_progress: bool,
    ) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "ToggleParticipationMode".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.toggle_participation_mode(participant_id, requester_id, activity_in_progress) {
            Ok(new_mode) => DomainEvent::ParticipationModeChanged {
                lobby_id,
                participant_id,
                new_mode,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "ToggleParticipationMode".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_delegate_host(
        &mut self,
        lobby_id: Uuid,
        _current_host_id: Uuid,
        new_host_id: Uuid,
    ) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "DelegateHost".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        let old_host_id = lobby.host_id();

        match lobby.delegate_host(new_host_id) {
            Ok(_) => DomainEvent::HostDelegated {
                lobby_id,
                from: old_host_id,
                to: new_host_id,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "DelegateHost".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_plan_activity(&mut self, lobby_id: Uuid, metadata: ActivityMetadata) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "PlanActivity".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.plan_activity(metadata.clone()) {
            Ok(_) => DomainEvent::ActivityPlanned { lobby_id, metadata },
            Err(e) => DomainEvent::CommandFailed {
                command: "PlanActivity".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_start_activity(&mut self, lobby_id: Uuid, activity_id: ActivityId) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "StartActivity".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.start_activity(activity_id) {
            Ok(_) => DomainEvent::ActivityStarted {
                lobby_id,
                activity_id,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "StartActivity".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_submit_result(&mut self, lobby_id: Uuid, result: ActivityResult) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "SubmitResult".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.submit_result(result.clone()) {
            Ok(_) => {
                // Check if activity completed
                if lobby.get_activity(result.activity_id).unwrap().status
                    == ActivityStatus::Completed
                {
                    let results = lobby.get_results(result.activity_id);
                    DomainEvent::ActivityCompleted {
                        lobby_id,
                        activity_id: result.activity_id,
                        results: results.into_iter().cloned().collect(),
                    }
                } else {
                    DomainEvent::ResultSubmitted { lobby_id, result }
                }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "SubmitResult".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_cancel_activity(&mut self, lobby_id: Uuid, activity_id: ActivityId) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "CancelActivity".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        match lobby.cancel_activity(activity_id) {
            Ok(_) => DomainEvent::ActivityCancelled {
                lobby_id,
                activity_id,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "CancelActivity".to_string(),
                reason: e.to_string(),
            },
        }
    }

    /// Add a lobby directly (for P2P sync)
    pub fn add_lobby(&mut self, lobby: Lobby) {
        self.lobbies.insert(lobby.id(), lobby);
    }

    /// Get a lobby by ID (for testing/inspection)
    pub fn get_lobby(&self, lobby_id: &Uuid) -> Option<&Lobby> {
        self.lobbies.get(lobby_id)
    }

    /// Get lobby count (for testing)
    pub fn lobby_count(&self) -> usize {
        self.lobbies.len()
    }
}

impl Default for DomainEventLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ParticipationMode;

    #[test]
    fn test_create_lobby() {
        let mut event_loop = DomainEventLoop::new();

        let cmd = DomainCommand::CreateLobby {
            lobby_name: "Test Lobby".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };

        let event = event_loop.handle_command(cmd);

        match event {
            DomainEvent::LobbyCreated { lobby } => {
                assert_eq!(lobby.name(), "Test Lobby");
                assert_eq!(lobby.participants().len(), 1);
                assert!(event_loop.get_lobby(&lobby.id()).is_some());
            }
            _ => panic!("Expected LobbyCreated event"),
        }
    }

    #[test]
    fn test_join_lobby() {
        let mut event_loop = DomainEventLoop::new();

        // Create lobby
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test Lobby".to_string(),
            host_name: "Alice".to_string(),

            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let lobby_id = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        // Join lobby
        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let event = event_loop.handle_command(join_cmd);

        match event {
            DomainEvent::GuestJoined {
                lobby_id: lid,
                participant,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(participant.name(), "Bob");
                assert!(!participant.is_host());

                let lobby = event_loop.get_lobby(&lobby_id).unwrap();
                assert_eq!(lobby.participants().len(), 2);
            }
            _ => panic!("Expected GuestJoined event"),
        }
    }

    #[test]
    fn test_join_nonexistent_lobby() {
        let mut event_loop = DomainEventLoop::new();

        let cmd = DomainCommand::JoinLobby {
            lobby_id: Uuid::new_v4(),
            guest_name: "Bob".to_string(),
        };

        let event = event_loop.handle_command(cmd);

        match event {
            DomainEvent::CommandFailed { command, reason } => {
                assert_eq!(command, "JoinLobby");
                assert!(reason.contains("not found"));
            }
            _ => panic!("Expected CommandFailed"),
        }
    }

    #[test]
    fn test_leave_lobby() {
        let mut event_loop = DomainEventLoop::new();

        // Create and join
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let lobby_id = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let guest_id = match event_loop.handle_command(join_cmd) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            _ => panic!("Expected GuestJoined"),
        };

        // Leave
        let leave_cmd = DomainCommand::LeaveLobby {
            lobby_id,
            participant_id: guest_id,
        };
        let event = event_loop.handle_command(leave_cmd);

        match event {
            DomainEvent::GuestLeft {
                lobby_id: lid,
                participant_id: pid,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, guest_id);

                let lobby = event_loop.get_lobby(&lobby_id).unwrap();
                assert_eq!(lobby.participants().len(), 1); // Only host remains
            }
            _ => panic!("Expected GuestLeft event"),
        }
    }

    #[test]
    fn test_kick_guest() {
        let mut event_loop = DomainEventLoop::new();

        // Create lobby
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let (lobby_id, host_id) = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => (lobby.id(), lobby.host_id()),
            _ => panic!("Expected LobbyCreated"),
        };

        // Guest joins
        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let guest_id = match event_loop.handle_command(join_cmd) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            _ => panic!("Expected GuestJoined"),
        };

        // Host kicks guest
        let kick_cmd = DomainCommand::KickGuest {
            lobby_id,
            host_id,
            guest_id,
        };
        let event = event_loop.handle_command(kick_cmd);

        match event {
            DomainEvent::GuestKicked {
                lobby_id: lid,
                participant_id: pid,
                kicked_by,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, guest_id);
                assert_eq!(kicked_by, host_id);

                let lobby = event_loop.get_lobby(&lobby_id).unwrap();
                assert_eq!(lobby.participants().len(), 1); // Only host
            }
            _ => panic!("Expected GuestKicked event"),
        }
    }

    #[test]
    fn test_toggle_participation_mode() {
        let mut event_loop = DomainEventLoop::new();

        // Create and join
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let lobby_id = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let guest_id = match event_loop.handle_command(join_cmd) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            _ => panic!("Expected GuestJoined"),
        };

        // Toggle to spectating
        let toggle_cmd = DomainCommand::ToggleParticipationMode {
            lobby_id,
            participant_id: guest_id,
            requester_id: guest_id, // Self-toggle
            activity_in_progress: false,
        };
        let event = event_loop.handle_command(toggle_cmd);

        match event {
            DomainEvent::ParticipationModeChanged {
                lobby_id: lid,
                participant_id: pid,
                new_mode,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(pid, guest_id);
                assert_eq!(new_mode, ParticipationMode::Spectating);

                let lobby = event_loop.get_lobby(&lobby_id).unwrap();
                let participant = lobby.participants().get(&guest_id).unwrap();
                assert_eq!(
                    participant.participation_mode(),
                    ParticipationMode::Spectating
                );
            }
            _ => panic!("Expected ParticipationModeChanged event"),
        }
    }

    #[test]
    fn test_cannot_toggle_during_activity() {
        let mut event_loop = DomainEventLoop::new();

        // Setup
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let lobby_id = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let guest_id = match event_loop.handle_command(join_cmd) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            _ => panic!("Expected GuestJoined"),
        };

        // Try to toggle during activity
        let toggle_cmd = DomainCommand::ToggleParticipationMode {
            lobby_id,
            participant_id: guest_id,
            requester_id: guest_id,
            activity_in_progress: true, // ← Activity running
        };
        let event = event_loop.handle_command(toggle_cmd);

        match event {
            DomainEvent::CommandFailed { command, reason } => {
                assert_eq!(command, "ToggleParticipationMode");
                assert!(reason.contains("during"));
            }
            _ => panic!("Expected CommandFailed"),
        }
    }

    #[test]
    fn test_delegate_host() {
        let mut event_loop = DomainEventLoop::new();

        // Create lobby
        let create_cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        let (lobby_id, host_id) = match event_loop.handle_command(create_cmd) {
            DomainEvent::LobbyCreated { lobby } => (lobby.id(), lobby.host_id()),
            _ => panic!("Expected LobbyCreated"),
        };

        // Guest joins
        let join_cmd = DomainCommand::JoinLobby {
            lobby_id,
            guest_name: "Bob".to_string(),
        };
        let guest_id = match event_loop.handle_command(join_cmd) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            _ => panic!("Expected GuestJoined"),
        };

        // Delegate
        let delegate_cmd = DomainCommand::DelegateHost {
            lobby_id,
            current_host_id: host_id,
            new_host_id: guest_id,
        };
        let event = event_loop.handle_command(delegate_cmd);

        match event {
            DomainEvent::HostDelegated {
                lobby_id: lid,
                from,
                to,
            } => {
                assert_eq!(lid, lobby_id);
                assert_eq!(from, host_id);
                assert_eq!(to, guest_id);

                let lobby = event_loop.get_lobby(&lobby_id).unwrap();
                assert_eq!(lobby.host_id(), guest_id);
                assert!(lobby.participants().get(&guest_id).unwrap().is_host());
                assert!(!lobby.participants().get(&host_id).unwrap().is_host());
            }
            _ => panic!("Expected HostDelegated event"),
        }
    }

    #[test]
    fn test_multiple_lobbies() {
        let mut event_loop = DomainEventLoop::new();

        // Create two lobbies
        let cmd1 = DomainCommand::CreateLobby {
            lobby_name: "Lobby 1".to_string(),
            host_name: "Alice".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        event_loop.handle_command(cmd1);

        let cmd2 = DomainCommand::CreateLobby {
            lobby_name: "Lobby 2".to_string(),
            host_name: "Bob".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        };
        event_loop.handle_command(cmd2);

        assert_eq!(event_loop.lobby_count(), 2);
    }
}
