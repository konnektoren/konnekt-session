use crate::application::{DomainCommand, DomainEvent};
use crate::domain::{
    ActivityRun, ActivityRunId, Lobby, Participant, ParticipationMode,
};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DomainEventLoop {
    lobbies: HashMap<Uuid, Lobby>,
    runs: HashMap<ActivityRunId, ActivityRun>,
}

impl DomainEventLoop {
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
            runs: HashMap::new(),
        }
    }

    pub fn handle_command(&mut self, command: DomainCommand) -> DomainEvent {
        match command {
            DomainCommand::CreateLobby { lobby_id, lobby_name, host_name } =>
                self.handle_create_lobby(lobby_id, lobby_name, host_name),

            DomainCommand::CreateLobbyWithHost { lobby_id, lobby_name, host } =>
                self.handle_create_lobby_with_host(lobby_id, lobby_name, host),

            DomainCommand::JoinLobby { lobby_id, guest_name } =>
                self.handle_join_lobby(lobby_id, guest_name),

            DomainCommand::LeaveLobby { lobby_id, participant_id } =>
                self.handle_leave_lobby(lobby_id, participant_id),

            DomainCommand::KickGuest { lobby_id, host_id, guest_id } =>
                self.handle_kick_guest(lobby_id, host_id, guest_id),

            DomainCommand::ToggleParticipationMode { lobby_id, participant_id, requester_id } =>
                self.handle_toggle_participation_mode(lobby_id, participant_id, requester_id),

            DomainCommand::DelegateHost { lobby_id, current_host_id, new_host_id } =>
                self.handle_delegate_host(lobby_id, current_host_id, new_host_id),

            DomainCommand::AddParticipant { lobby_id, participant } =>
                self.handle_add_participant(lobby_id, participant),

            DomainCommand::UpdateParticipantMode { lobby_id, participant_id, new_mode } =>
                self.handle_update_participant_mode(lobby_id, participant_id, new_mode),

            DomainCommand::QueueActivity { lobby_id, config } =>
                self.handle_queue_activity(lobby_id, config),

            DomainCommand::StartNextRun { lobby_id } =>
                self.handle_start_next_run(lobby_id),

            DomainCommand::SubmitResult { lobby_id, run_id, result } =>
                self.handle_submit_result(lobby_id, run_id, result),

            DomainCommand::CancelRun { lobby_id, run_id } =>
                self.handle_cancel_run(lobby_id, run_id),

            DomainCommand::RemoveSubmitter { lobby_id, run_id, participant_id } =>
                self.handle_remove_submitter(lobby_id, run_id, participant_id),

            DomainCommand::SyncRunStarted { lobby_id, run_id, config, required_submitters } =>
                self.handle_sync_run_started(lobby_id, run_id, config, required_submitters),
        }
    }

    // ── Lobby handlers ────────────────────────────────────────────────────────

    fn handle_create_lobby(&mut self, lobby_id: Option<Uuid>, lobby_name: String, host_name: String) -> DomainEvent {
        match Participant::new_host(host_name) {
            Ok(host) => {
                let result = if let Some(id) = lobby_id {
                    Lobby::with_id(id, lobby_name, host)
                } else {
                    Lobby::new(lobby_name, host)
                };
                match result {
                    Ok(lobby) => {
                        let id = lobby.id();
                        self.lobbies.insert(id, lobby.clone());
                        DomainEvent::LobbyCreated { lobby }
                    }
                    Err(e) => DomainEvent::CommandFailed { command: "CreateLobby".to_string(), reason: e.to_string() },
                }
            }
            Err(e) => DomainEvent::CommandFailed { command: "CreateLobby".to_string(), reason: e.to_string() },
        }
    }

    fn handle_create_lobby_with_host(&mut self, lobby_id: Uuid, lobby_name: String, host: Participant) -> DomainEvent {
        match Lobby::with_id(lobby_id, lobby_name, host) {
            Ok(lobby) => {
                self.lobbies.insert(lobby.id(), lobby.clone());
                DomainEvent::LobbyCreated { lobby }
            }
            Err(e) => DomainEvent::CommandFailed { command: "CreateLobbyWithHost".to_string(), reason: e.to_string() },
        }
    }

    fn handle_join_lobby(&mut self, lobby_id: Uuid, guest_name: String) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "JoinLobby".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match Participant::new_guest(guest_name) {
            Ok(guest) => match lobby.add_guest(guest.clone()) {
                Ok(_) => DomainEvent::GuestJoined { lobby_id, participant: guest },
                Err(e) => DomainEvent::CommandFailed { command: "JoinLobby".to_string(), reason: e.to_string() },
            },
            Err(e) => DomainEvent::CommandFailed { command: "JoinLobby".to_string(), reason: e.to_string() },
        }
    }

    fn handle_leave_lobby(&mut self, lobby_id: Uuid, participant_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "LeaveLobby".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.remove_participant(participant_id) {
            Ok(_) => DomainEvent::GuestLeft { lobby_id, participant_id },
            Err(e) => DomainEvent::CommandFailed { command: "LeaveLobby".to_string(), reason: e.to_string() },
        }
    }

    fn handle_kick_guest(&mut self, lobby_id: Uuid, host_id: Uuid, guest_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "KickGuest".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.kick_guest(guest_id, host_id) {
            Ok(_) => DomainEvent::GuestKicked { lobby_id, participant_id: guest_id, kicked_by: host_id },
            Err(e) => DomainEvent::CommandFailed { command: "KickGuest".to_string(), reason: e.to_string() },
        }
    }

    fn handle_toggle_participation_mode(&mut self, lobby_id: Uuid, participant_id: Uuid, requester_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "ToggleParticipationMode".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.toggle_participation_mode(participant_id, requester_id) {
            Ok(new_mode) => DomainEvent::ParticipationModeChanged { lobby_id, participant_id, new_mode },
            Err(e) => DomainEvent::CommandFailed { command: "ToggleParticipationMode".to_string(), reason: e.to_string() },
        }
    }

    fn handle_delegate_host(&mut self, lobby_id: Uuid, _current_host_id: Uuid, new_host_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "DelegateHost".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        let old_host_id = lobby.host_id();
        match lobby.delegate_host(new_host_id) {
            Ok(_) => DomainEvent::HostDelegated { lobby_id, from: old_host_id, to: new_host_id },
            Err(e) => DomainEvent::CommandFailed { command: "DelegateHost".to_string(), reason: e.to_string() },
        }
    }

    fn handle_add_participant(&mut self, lobby_id: Uuid, participant: Participant) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "AddParticipant".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.add_guest(participant.clone()) {
            Ok(_) => DomainEvent::GuestJoined { lobby_id, participant },
            Err(e) => DomainEvent::CommandFailed { command: "AddParticipant".to_string(), reason: e.to_string() },
        }
    }

    fn handle_update_participant_mode(&mut self, lobby_id: Uuid, participant_id: Uuid, new_mode: ParticipationMode) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "UpdateParticipantMode".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.participants_mut().get_mut(&participant_id) {
            Some(p) => {
                p.force_participation_mode(new_mode);
                DomainEvent::ParticipationModeChanged { lobby_id, participant_id, new_mode }
            }
            None => DomainEvent::CommandFailed { command: "UpdateParticipantMode".to_string(), reason: format!("Participant {} not found", participant_id) },
        }
    }

    fn handle_queue_activity(&mut self, lobby_id: Uuid, config: crate::domain::ActivityConfig) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "QueueActivity".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        match lobby.queue_activity(config.clone()) {
            Ok(_) => DomainEvent::ActivityQueued { lobby_id, config },
            Err(e) => DomainEvent::CommandFailed { command: "QueueActivity".to_string(), reason: e.to_string() },
        }
    }

    // ── Run handlers ──────────────────────────────────────────────────────────

    fn handle_start_next_run(&mut self, lobby_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "StartNextRun".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };

        // Snapshot active participants before dequeuing
        let snapshot = lobby.active_participant_ids();

        let config = match lobby.dequeue_next_activity() {
            Ok(c) => c,
            Err(e) => return DomainEvent::CommandFailed { command: "StartNextRun".to_string(), reason: e.to_string() },
        };

        let run_id = Uuid::new_v4();
        let run = ActivityRun::new(run_id, lobby_id, config.clone(), snapshot);

        if let Err(e) = lobby.set_active_run(run_id) {
            return DomainEvent::CommandFailed { command: "StartNextRun".to_string(), reason: e.to_string() };
        }

        self.runs.insert(run_id, run);
        DomainEvent::RunStarted { lobby_id, run_id, config }
    }

    fn handle_submit_result(&mut self, lobby_id: Uuid, run_id: ActivityRunId, result: crate::domain::ActivityResult) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => return DomainEvent::CommandFailed { command: "SubmitResult".to_string(), reason: format!("Run {} not found", run_id) },
        };

        match run.submit_result(result.clone()) {
            Ok(completed) => {
                if completed {
                    let results: Vec<_> = run.results().values().cloned().collect();
                    let status = run.status();
                    if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                        lobby.clear_active_run();
                    }
                    DomainEvent::RunEnded { lobby_id, run_id, status, results }
                } else {
                    DomainEvent::ResultSubmitted { lobby_id, run_id, result }
                }
            }
            Err(e) => DomainEvent::CommandFailed { command: "SubmitResult".to_string(), reason: e.to_string() },
        }
    }

    fn handle_cancel_run(&mut self, lobby_id: Uuid, run_id: ActivityRunId) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => return DomainEvent::CommandFailed { command: "CancelRun".to_string(), reason: format!("Run {} not found", run_id) },
        };
        match run.cancel() {
            Ok(_) => {
                let results: Vec<_> = run.results().values().cloned().collect();
                let status = run.status();
                if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                    lobby.clear_active_run();
                }
                DomainEvent::RunEnded { lobby_id, run_id, status, results }
            }
            Err(e) => DomainEvent::CommandFailed { command: "CancelRun".to_string(), reason: e.to_string() },
        }
    }

    fn handle_remove_submitter(&mut self, lobby_id: Uuid, run_id: ActivityRunId, participant_id: Uuid) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => return DomainEvent::CommandFailed { command: "RemoveSubmitter".to_string(), reason: format!("Run {} not found", run_id) },
        };
        match run.remove_submitter(participant_id) {
            Ok(ended) => {
                if ended {
                    let results: Vec<_> = run.results().values().cloned().collect();
                    let status = run.status();
                    if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                        lobby.clear_active_run();
                    }
                    DomainEvent::RunEnded { lobby_id, run_id, status, results }
                } else {
                    DomainEvent::SubmitterRemoved { lobby_id, run_id, participant_id }
                }
            }
            Err(e) => DomainEvent::CommandFailed { command: "RemoveSubmitter".to_string(), reason: e.to_string() },
        }
    }

    fn handle_sync_run_started(
        &mut self,
        lobby_id: Uuid,
        run_id: crate::domain::ActivityRunId,
        config: crate::domain::ActivityConfig,
        required_submitters: Vec<Uuid>,
    ) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => return DomainEvent::CommandFailed { command: "SyncRunStarted".to_string(), reason: format!("Lobby {} not found", lobby_id) },
        };
        let snapshot: std::collections::HashSet<Uuid> = required_submitters.into_iter().collect();
        let run = ActivityRun::new(run_id, lobby_id, config.clone(), snapshot);
        if let Err(e) = lobby.set_active_run(run_id) {
            return DomainEvent::CommandFailed { command: "SyncRunStarted".to_string(), reason: e.to_string() };
        }
        self.runs.insert(run_id, run);
        DomainEvent::RunStarted { lobby_id, run_id, config }
    }

    // ── Inspection ────────────────────────────────────────────────────────────

    pub fn add_lobby(&mut self, lobby: Lobby) {
        self.lobbies.insert(lobby.id(), lobby);
    }

    pub fn get_lobby(&self, lobby_id: &Uuid) -> Option<&Lobby> {
        self.lobbies.get(lobby_id)
    }

    pub fn get_run(&self, run_id: &ActivityRunId) -> Option<&ActivityRun> {
        self.runs.get(run_id)
    }

    pub fn lobby_count(&self) -> usize {
        self.lobbies.len()
    }
}

impl Default for DomainEventLoop {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::application::DomainCommand;
    use crate::domain::{ActivityConfig, ActivityResult, RunStatus};

    fn create_lobby(el: &mut DomainEventLoop, name: &str, host: &str) -> (Uuid, Uuid) {
        match el.handle_command(DomainCommand::CreateLobby {
            lobby_name: name.to_string(),
            host_name: host.to_string(),
            lobby_id: None,
        }) {
            DomainEvent::LobbyCreated { lobby } => (lobby.id(), lobby.host_id()),
            e => panic!("Expected LobbyCreated, got {:?}", e),
        }
    }

    fn join_lobby(el: &mut DomainEventLoop, lobby_id: Uuid, name: &str) -> Uuid {
        match el.handle_command(DomainCommand::JoinLobby { lobby_id, guest_name: name.to_string() }) {
            DomainEvent::GuestJoined { participant, .. } => participant.id(),
            e => panic!("Expected GuestJoined, got {:?}", e),
        }
    }

    #[test]
    fn test_create_lobby() {
        let mut el = DomainEventLoop::new();
        let (lobby_id, _) = create_lobby(&mut el, "Test", "Alice");
        assert!(el.get_lobby(&lobby_id).is_some());
    }

    #[test]
    fn test_start_run_and_submit_result() {
        let mut el = DomainEventLoop::new();
        let (lobby_id, host_id) = create_lobby(&mut el, "Test", "Alice");

        let config = ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        el.handle_command(DomainCommand::QueueActivity { lobby_id, config });

        let run_id = match el.handle_command(DomainCommand::StartNextRun { lobby_id }) {
            DomainEvent::RunStarted { run_id, .. } => run_id,
            e => panic!("Expected RunStarted, got {:?}", e),
        };

        assert!(el.get_lobby(&lobby_id).unwrap().has_active_run());

        let result = ActivityResult::new(run_id, host_id);
        let event = el.handle_command(DomainCommand::SubmitResult { lobby_id, run_id, result });

        match event {
            DomainEvent::RunEnded { status, .. } => {
                assert_eq!(status, RunStatus::Completed);
                assert!(!el.get_lobby(&lobby_id).unwrap().has_active_run());
            }
            e => panic!("Expected RunEnded, got {:?}", e),
        }
    }

    #[test]
    fn test_remove_submitter_completes_run() {
        let mut el = DomainEventLoop::new();
        let (lobby_id, host_id) = create_lobby(&mut el, "Test", "Alice");
        let guest_id = join_lobby(&mut el, lobby_id, "Bob");

        let config = ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        el.handle_command(DomainCommand::QueueActivity { lobby_id, config });

        let run_id = match el.handle_command(DomainCommand::StartNextRun { lobby_id }) {
            DomainEvent::RunStarted { run_id, .. } => run_id,
            e => panic!("Expected RunStarted, got {:?}", e),
        };

        // Host submits
        el.handle_command(DomainCommand::SubmitResult {
            lobby_id,
            run_id,
            result: ActivityResult::new(run_id, host_id),
        });

        // Bob disconnects → run completes
        let event = el.handle_command(DomainCommand::RemoveSubmitter { lobby_id, run_id, participant_id: guest_id });
        match event {
            DomainEvent::RunEnded { status, .. } => assert_eq!(status, RunStatus::Completed),
            e => panic!("Expected RunEnded, got {:?}", e),
        }
    }

    #[test]
    fn test_toggle_participation_mode_blocked_during_run() {
        let mut el = DomainEventLoop::new();
        let (lobby_id, _host_id) = create_lobby(&mut el, "Test", "Alice");
        let guest_id = join_lobby(&mut el, lobby_id, "Bob");

        let config = ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        el.handle_command(DomainCommand::QueueActivity { lobby_id, config });
        el.handle_command(DomainCommand::StartNextRun { lobby_id });

        let event = el.handle_command(DomainCommand::ToggleParticipationMode {
            lobby_id,
            participant_id: guest_id,
            requester_id: guest_id,
        });

        match event {
            DomainEvent::CommandFailed { .. } => {}
            e => panic!("Expected CommandFailed, got {:?}", e),
        }
    }

    #[test]
    fn test_cancel_run() {
        let mut el = DomainEventLoop::new();
        let (lobby_id, _) = create_lobby(&mut el, "Test", "Alice");

        let config = ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        el.handle_command(DomainCommand::QueueActivity { lobby_id, config });

        let run_id = match el.handle_command(DomainCommand::StartNextRun { lobby_id }) {
            DomainEvent::RunStarted { run_id, .. } => run_id,
            e => panic!("Expected RunStarted, got {:?}", e),
        };

        let event = el.handle_command(DomainCommand::CancelRun { lobby_id, run_id });
        match event {
            DomainEvent::RunEnded { status, .. } => assert_eq!(status, RunStatus::Cancelled),
            e => panic!("Expected RunEnded, got {:?}", e),
        }
        assert!(!el.get_lobby(&lobby_id).unwrap().has_active_run());
    }
}
