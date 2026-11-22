use crate::application::{DomainCommand, DomainEvent};
use crate::domain::{ActivityRun, ActivityRunId, Lobby, Participant, ParticipationMode};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct DomainEventLoop {
    lobbies: HashMap<Uuid, Lobby>,
    runs: HashMap<ActivityRunId, ActivityRun>,
    /// Run IDs in the order the runs started (oldest first).
    run_order: Vec<ActivityRunId>,
}

impl DomainEventLoop {
    pub fn new() -> Self {
        Self {
            lobbies: HashMap::new(),
            runs: HashMap::new(),
            run_order: Vec::new(),
        }
    }

    pub fn handle_command(&mut self, command: DomainCommand) -> DomainEvent {
        match command {
            DomainCommand::CreateLobby {
                lobby_id,
                lobby_name,
                host_name,
            } => self.handle_create_lobby(lobby_id, lobby_name, host_name),

            DomainCommand::CreateLobbyWithHost {
                lobby_id,
                lobby_name,
                host,
            } => self.handle_create_lobby_with_host(lobby_id, lobby_name, host),

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
            } => self.handle_toggle_participation_mode(lobby_id, participant_id, requester_id),

            DomainCommand::DelegateHost {
                lobby_id,
                current_host_id,
                new_host_id,
            } => self.handle_delegate_host(lobby_id, current_host_id, new_host_id),

            DomainCommand::AddParticipant {
                lobby_id,
                participant,
            } => self.handle_add_participant(lobby_id, participant),

            DomainCommand::UpdateParticipantMode {
                lobby_id,
                participant_id,
                new_mode,
            } => self.handle_update_participant_mode(lobby_id, participant_id, new_mode),

            DomainCommand::QueueActivity { lobby_id, config } => {
                self.handle_queue_activity(lobby_id, config)
            }

            DomainCommand::StartNextRun { lobby_id } => self.handle_start_next_run(lobby_id),

            DomainCommand::StartNextRunIf {
                lobby_id,
                activity_id,
            } => {
                if self
                    .lobbies
                    .get(&lobby_id)
                    .and_then(|lobby| lobby.activity_queue().first())
                    .is_some_and(|activity| activity.id == activity_id)
                {
                    self.handle_start_next_run(lobby_id)
                } else {
                    DomainEvent::CommandFailed {
                        command: "StartNextRunIf".into(),
                        reason: "The requested activity is not next in the queue".into(),
                    }
                }
            }

            DomainCommand::SubmitResult {
                lobby_id,
                run_id,
                result,
            } => self.handle_submit_result(lobby_id, run_id, result),

            DomainCommand::CancelRun { lobby_id, run_id } => {
                self.handle_cancel_run(lobby_id, run_id)
            }

            DomainCommand::RemoveSubmitter {
                lobby_id,
                run_id,
                participant_id,
            } => self.handle_remove_submitter(lobby_id, run_id, participant_id),

            DomainCommand::SyncRunStarted {
                lobby_id,
                run_id,
                config,
                required_submitters,
            } => self.handle_sync_run_started(lobby_id, run_id, config, required_submitters),
        }
    }

    // ── Lobby handlers ────────────────────────────────────────────────────────

    fn handle_create_lobby(
        &mut self,
        lobby_id: Option<Uuid>,
        lobby_name: String,
        host_name: String,
    ) -> DomainEvent {
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

    fn handle_create_lobby_with_host(
        &mut self,
        lobby_id: Uuid,
        lobby_name: String,
        host: Participant,
    ) -> DomainEvent {
        match Lobby::with_id(lobby_id, lobby_name, host) {
            Ok(lobby) => {
                self.lobbies.insert(lobby.id(), lobby.clone());
                DomainEvent::LobbyCreated { lobby }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "CreateLobbyWithHost".to_string(),
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
        match lobby.toggle_participation_mode(participant_id, requester_id) {
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

    fn handle_add_participant(&mut self, lobby_id: Uuid, participant: Participant) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "AddParticipant".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };
        match lobby.add_guest(participant.clone()) {
            Ok(_) => DomainEvent::GuestJoined {
                lobby_id,
                participant,
            },
            Err(e) => DomainEvent::CommandFailed {
                command: "AddParticipant".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_update_participant_mode(
        &mut self,
        lobby_id: Uuid,
        participant_id: Uuid,
        new_mode: ParticipationMode,
    ) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "UpdateParticipantMode".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };
        match lobby.participants_mut().get_mut(&participant_id) {
            Some(p) => {
                p.force_participation_mode(new_mode);
                DomainEvent::ParticipationModeChanged {
                    lobby_id,
                    participant_id,
                    new_mode,
                }
            }
            None => DomainEvent::CommandFailed {
                command: "UpdateParticipantMode".to_string(),
                reason: format!("Participant {} not found", participant_id),
            },
        }
    }

    fn handle_queue_activity(
        &mut self,
        lobby_id: Uuid,
        config: crate::domain::ActivityConfig,
    ) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "QueueActivity".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };
        match lobby.queue_activity(config.clone()) {
            Ok(_) => DomainEvent::ActivityQueued { lobby_id, config },
            Err(e) => DomainEvent::CommandFailed {
                command: "QueueActivity".to_string(),
                reason: e.to_string(),
            },
        }
    }

    // ── Run handlers ──────────────────────────────────────────────────────────

    fn handle_start_next_run(&mut self, lobby_id: Uuid) -> DomainEvent {
        let lobby = match self.lobbies.get_mut(&lobby_id) {
            Some(l) => l,
            None => {
                return DomainEvent::CommandFailed {
                    command: "StartNextRun".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };

        // Snapshot active participants before dequeuing
        let snapshot = lobby.active_participant_ids();

        let config = match lobby.dequeue_next_activity() {
            Ok(c) => c,
            Err(e) => {
                return DomainEvent::CommandFailed {
                    command: "StartNextRun".to_string(),
                    reason: e.to_string(),
                };
            }
        };

        let run_id = Uuid::new_v4();
        let run = ActivityRun::new(run_id, lobby_id, config.clone(), snapshot);

        if let Err(e) = lobby.set_active_run(run_id) {
            return DomainEvent::CommandFailed {
                command: "StartNextRun".to_string(),
                reason: e.to_string(),
            };
        }

        self.insert_run(run);
        DomainEvent::RunStarted {
            lobby_id,
            run_id,
            config,
        }
    }

    fn handle_submit_result(
        &mut self,
        lobby_id: Uuid,
        run_id: ActivityRunId,
        result: crate::domain::ActivityResult,
    ) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => {
                return DomainEvent::CommandFailed {
                    command: "SubmitResult".to_string(),
                    reason: format!("Run {} not found", run_id),
                };
            }
        };

        match run.submit_result(result.clone()) {
            Ok(completed) => {
                if completed {
                    let results: Vec<_> = run.results().values().cloned().collect();
                    let status = run.status();
                    if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                        lobby.clear_active_run();
                    }
                    DomainEvent::RunEnded {
                        lobby_id,
                        run_id,
                        status,
                        results,
                    }
                } else {
                    DomainEvent::ResultSubmitted {
                        lobby_id,
                        run_id,
                        result,
                    }
                }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "SubmitResult".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_cancel_run(&mut self, lobby_id: Uuid, run_id: ActivityRunId) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => {
                return DomainEvent::CommandFailed {
                    command: "CancelRun".to_string(),
                    reason: format!("Run {} not found", run_id),
                };
            }
        };
        match run.cancel() {
            Ok(_) => {
                let results: Vec<_> = run.results().values().cloned().collect();
                let status = run.status();
                if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                    lobby.clear_active_run();
                }
                DomainEvent::RunEnded {
                    lobby_id,
                    run_id,
                    status,
                    results,
                }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "CancelRun".to_string(),
                reason: e.to_string(),
            },
        }
    }

    fn handle_remove_submitter(
        &mut self,
        lobby_id: Uuid,
        run_id: ActivityRunId,
        participant_id: Uuid,
    ) -> DomainEvent {
        let run = match self.runs.get_mut(&run_id) {
            Some(r) => r,
            None => {
                return DomainEvent::CommandFailed {
                    command: "RemoveSubmitter".to_string(),
                    reason: format!("Run {} not found", run_id),
                };
            }
        };
        match run.remove_submitter(participant_id) {
            Ok(ended) => {
                if ended {
                    let results: Vec<_> = run.results().values().cloned().collect();
                    let status = run.status();
                    if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                        lobby.clear_active_run();
                    }
                    DomainEvent::RunEnded {
                        lobby_id,
                        run_id,
                        status,
                        results,
                    }
                } else {
                    DomainEvent::SubmitterRemoved {
                        lobby_id,
                        run_id,
                        participant_id,
                    }
                }
            }
            Err(e) => DomainEvent::CommandFailed {
                command: "RemoveSubmitter".to_string(),
                reason: e.to_string(),
            },
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
            None => {
                return DomainEvent::CommandFailed {
                    command: "SyncRunStarted".to_string(),
                    reason: format!("Lobby {} not found", lobby_id),
                };
            }
        };
        let snapshot: std::collections::HashSet<Uuid> = required_submitters.into_iter().collect();
        let run = ActivityRun::new(run_id, lobby_id, config.clone(), snapshot);
        if let Err(e) = lobby.set_active_run(run_id) {
            return DomainEvent::CommandFailed {
                command: "SyncRunStarted".to_string(),
                reason: e.to_string(),
            };
        }
        // The host dequeued this activity when it started the run; mirror that
        // so a replayed or synchronized start does not leave it queued.
        let _ = lobby.remove_queued_activity(config.id);
        self.insert_run(run);
        DomainEvent::RunStarted {
            lobby_id,
            run_id,
            config,
        }
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

    fn insert_run(&mut self, run: ActivityRun) {
        let id = run.id();
        if self.runs.insert(id, run).is_none() {
            self.run_order.push(id);
        }
    }

    /// All runs of one lobby in start order (oldest first), including
    /// finished ones with their results.
    pub fn runs_of(&self, lobby_id: &Uuid) -> Vec<&ActivityRun> {
        self.run_order
            .iter()
            .filter_map(|id| self.runs.get(id))
            .filter(|run| run.lobby_id() == *lobby_id)
            .collect()
    }

    /// The `limit` most recently started runs of a lobby, oldest first. The
    /// active run is always included, even beyond the limit.
    pub fn recent_runs_of(&self, lobby_id: &Uuid, limit: usize) -> Vec<&ActivityRun> {
        let runs = self.runs_of(lobby_id);
        let mut recent = runs[runs.len().saturating_sub(limit)..].to_vec();
        let active = self
            .lobbies
            .get(lobby_id)
            .and_then(Lobby::active_run_id)
            .and_then(|id| self.runs.get(&id));
        if let Some(active) = active
            && !recent.iter().any(|run| run.id() == active.id())
        {
            recent.push(active);
        }
        recent
    }

    /// Restore a lobby from a complete state, e.g. a host snapshot that may
    /// carry only the most recent runs. The lobby and the given runs replace
    /// the current ones; finished runs not in `runs` are kept (they cannot
    /// change), runs still in progress are dropped. Everything is validated
    /// first; on error nothing changes.
    pub fn restore_lobby(
        &mut self,
        lobby: Lobby,
        runs: Vec<ActivityRun>,
    ) -> Result<(), RestoreError> {
        lobby.validate().map_err(RestoreError::Lobby)?;
        let lobby_id = lobby.id();
        let mut ids = std::collections::HashSet::new();
        for run in &runs {
            if run.lobby_id() != lobby_id || !ids.insert(run.id()) || !run.validate() {
                return Err(RestoreError::InvalidRun(run.id()));
            }
        }
        let in_progress: Vec<_> = runs
            .iter()
            .filter(|run| run.status() == crate::domain::RunStatus::InProgress)
            .map(ActivityRun::id)
            .collect();
        if in_progress.as_slice() != lobby.active_run_id().as_slice() {
            return Err(RestoreError::ActiveRunMismatch);
        }
        self.runs.retain(|id, run| {
            run.lobby_id() != lobby_id
                || (run.status() != crate::domain::RunStatus::InProgress && !ids.contains(id))
        });
        let runs_map = &self.runs;
        self.run_order.retain(|id| runs_map.contains_key(id));
        for run in runs {
            self.insert_run(run);
        }
        self.lobbies.insert(lobby_id, lobby);
        Ok(())
    }

    pub fn lobby_count(&self) -> usize {
        self.lobbies.len()
    }
}

/// Why a complete lobby state could not be restored.
#[derive(Debug, PartialEq, thiserror::Error)]
pub enum RestoreError {
    #[error("lobby is inconsistent: {0}")]
    Lobby(crate::domain::LobbyError),
    #[error("run {0} is invalid, duplicated or belongs to another lobby")]
    InvalidRun(Uuid),
    #[error("the active run does not match the runs in progress")]
    ActiveRunMismatch,
}

impl Default for DomainEventLoop {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conditional_start_does_not_start_a_different_queued_activity() {
        let mut event_loop = DomainEventLoop::new();
        let (lobby_id, _) = create_lobby(&mut event_loop, "Room", "Host");
        let first = ActivityConfig::new("test".into(), "First".into(), serde_json::Value::Null);
        let second = ActivityConfig::new("test".into(), "Second".into(), serde_json::Value::Null);
        let first_id = first.id;
        let second_id = second.id;
        event_loop.handle_command(DomainCommand::QueueActivity {
            lobby_id,
            config: first,
        });
        event_loop.handle_command(DomainCommand::QueueActivity {
            lobby_id,
            config: second,
        });
        assert!(matches!(
            event_loop.handle_command(DomainCommand::StartNextRunIf {
                lobby_id,
                activity_id: second_id
            }),
            DomainEvent::CommandFailed { .. }
        ));
        assert!(
            matches!(event_loop.handle_command(DomainCommand::StartNextRunIf { lobby_id, activity_id: first_id }), DomainEvent::RunStarted { config, .. } if config.id == first_id)
        );
    }
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
        match el.handle_command(DomainCommand::JoinLobby {
            lobby_id,
            guest_name: name.to_string(),
        }) {
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

        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
        el.handle_command(DomainCommand::QueueActivity { lobby_id, config });

        let run_id = match el.handle_command(DomainCommand::StartNextRun { lobby_id }) {
            DomainEvent::RunStarted { run_id, .. } => run_id,
            e => panic!("Expected RunStarted, got {:?}", e),
        };

        assert!(el.get_lobby(&lobby_id).unwrap().has_active_run());

        let result = ActivityResult::new(run_id, host_id);
        let event = el.handle_command(DomainCommand::SubmitResult {
            lobby_id,
            run_id,
            result,
        });

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

        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
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
        let event = el.handle_command(DomainCommand::RemoveSubmitter {
            lobby_id,
            run_id,
            participant_id: guest_id,
        });
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

        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
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

        let config =
            ActivityConfig::new("quiz".to_string(), "Q1".to_string(), serde_json::json!({}));
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

#[cfg(test)]
mod restore_tests {
    use super::*;
    use crate::domain::{ActivityConfig, ActivityResult, Participant, RunStatus};
    use std::collections::HashSet;

    fn lobby_with_guest() -> (Lobby, Uuid) {
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let mut lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        let guest = Participant::new_guest("Guest".into()).unwrap_or_else(|e| panic!("{e}"));
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap_or_else(|e| panic!("{e}"));
        (lobby, guest_id)
    }

    fn run(lobby: &Lobby, submitters: &[Uuid]) -> ActivityRun {
        ActivityRun::new(
            Uuid::new_v4(),
            lobby.id(),
            ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null),
            submitters.iter().copied().collect::<HashSet<_>>(),
        )
    }

    /// Edit a value through its serialized form, like a peer could.
    fn tamper<T: serde::Serialize + serde::de::DeserializeOwned>(
        value: &T,
        edit: impl FnOnce(&mut serde_json::Value),
    ) -> T {
        let mut json = serde_json::to_value(value).unwrap_or_else(|e| panic!("{e}"));
        edit(&mut json);
        serde_json::from_value(json).unwrap_or_else(|e| panic!("{e}"))
    }

    #[test]
    fn a_complete_state_replaces_the_lobby_and_its_runs() {
        let (mut lobby, guest) = lobby_with_guest();
        let mut events = DomainEventLoop::new();
        let stale = run(&lobby, &[guest]);
        events.add_lobby(lobby.clone());
        events.insert_run(stale.clone());

        let mut finished = run(&lobby, &[guest]);
        let result = ActivityResult::new(finished.id(), guest);
        assert_eq!(finished.submit_result(result), Ok(true));
        let active = run(&lobby, &[guest]);
        lobby
            .set_active_run(active.id())
            .unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            events.restore_lobby(lobby.clone(), vec![finished.clone(), active.clone()]),
            Ok(())
        );
        assert!(events.get_run(&stale.id()).is_none());
        assert_eq!(
            events.get_run(&finished.id()).map(ActivityRun::status),
            Some(RunStatus::Completed)
        );
        assert_eq!(events.get_lobby(&lobby.id()), Some(&lobby));
        assert_eq!(events.runs_of(&lobby.id()).len(), 2);
    }

    #[test]
    fn an_inconsistent_state_is_rejected_without_changes() {
        let (lobby, guest) = lobby_with_guest();
        let mut events = DomainEventLoop::new();
        events.add_lobby(lobby.clone());
        let active = run(&lobby, &[guest]);

        // Run in progress but the lobby has no active run.
        assert_eq!(
            events.restore_lobby(lobby.clone(), vec![active.clone()]),
            Err(RestoreError::ActiveRunMismatch)
        );
        // Completed without all results.
        let completed = tamper(&run(&lobby, &[guest]), |json| {
            json["status"] = "Completed".into();
        });
        assert_eq!(
            events.restore_lobby(lobby.clone(), vec![completed.clone()]),
            Err(RestoreError::InvalidRun(completed.id()))
        );
        // Two hosts.
        let two_hosts = tamper(&lobby, |json| {
            json["participants"][guest.to_string()]["lobby_role"] = "Host".into();
        });
        assert!(matches!(
            events.restore_lobby(two_hosts, vec![]),
            Err(RestoreError::Lobby(_))
        ));
        // A run of another lobby.
        let (other, other_guest) = lobby_with_guest();
        let foreign = run(&other, &[other_guest]);
        assert_eq!(
            events.restore_lobby(lobby.clone(), vec![foreign.clone()]),
            Err(RestoreError::InvalidRun(foreign.id()))
        );
        assert_eq!(events.get_lobby(&lobby.id()), Some(&lobby));
    }

    #[test]
    fn snapshots_can_carry_only_recent_runs_without_losing_older_results() {
        let (mut lobby, guest) = lobby_with_guest();
        let mut events = DomainEventLoop::new();
        let mut finished = Vec::new();
        for _ in 0..4 {
            let mut done = run(&lobby, &[guest]);
            let result = ActivityResult::new(done.id(), guest);
            assert_eq!(done.submit_result(result), Ok(true));
            finished.push(done);
        }
        let active = run(&lobby, &[guest]);
        lobby
            .set_active_run(active.id())
            .unwrap_or_else(|e| panic!("{e}"));
        let mut all = finished.clone();
        all.push(active.clone());
        assert_eq!(events.restore_lobby(lobby.clone(), all), Ok(()));

        let ids = |runs: Vec<&ActivityRun>| runs.iter().map(|run| run.id()).collect::<Vec<_>>();
        assert_eq!(
            ids(events.recent_runs_of(&lobby.id(), 2)),
            vec![finished[3].id(), active.id()]
        );
        assert_eq!(
            ids(events.recent_runs_of(&lobby.id(), 0)),
            vec![active.id()]
        );

        // A guest restoring a bounded snapshot keeps its older finished runs.
        let mut guest_loop = DomainEventLoop::new();
        assert_eq!(
            guest_loop.restore_lobby(lobby.clone(), finished[..2].to_vec()),
            Err(RestoreError::ActiveRunMismatch)
        );
        let mut idle = lobby.clone();
        idle.clear_active_run();
        assert_eq!(
            guest_loop.restore_lobby(idle, finished[..3].to_vec()),
            Ok(())
        );
        let recent: Vec<_> = events
            .recent_runs_of(&lobby.id(), 2)
            .into_iter()
            .cloned()
            .collect();
        assert_eq!(guest_loop.restore_lobby(lobby.clone(), recent), Ok(()));
        assert_eq!(
            ids(guest_loop.runs_of(&lobby.id())),
            ids(events.runs_of(&lobby.id()))[..3]
                .iter()
                .copied()
                .chain([finished[3].id(), active.id()])
                .collect::<Vec<_>>()
        );
    }
}
