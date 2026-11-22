//! Rebase rules for reconciling two forks of one lobby history.
//!
//! The selected (main) fork is kept unchanged. Events from the other fork are
//! either rebased, meaning the current host re-signs them after the main tip in
//! their original order, or dropped with a reason the application can show.
//!
//! | Event | Rule |
//! |---|---|
//! | `LobbyCreated` | Invalid after a common ancestor; the plan is rejected. |
//! | `GuestJoined` | Rebased unless the participant joined on the main fork. |
//! | `GuestLeft`, `GuestKicked` | Rebased unless the participant already left on the main fork. |
//! | `ParticipationModeChanged` | Rebased unless the main fork changed that participant's mode. |
//! | `HostDelegated` | Dropped; authority follows the main fork and the failover protocol. |
//! | `ActivityQueued` | Rebased unless the main fork queued an activity. |
//! | Runs started on the other fork | Rebased as a unit with their results unless the main fork started a run. |
//! | Results for runs older than the fork | Rebased unless the main fork ended the run or holds that participant's result. |
//! | `SubmitterRemoved` for older runs | Rebased unless the main fork ended the run. |
//! | `RunEnded` for older runs | Dropped; the host decides completion again from the merged results. |
//!
//! A plan is data only; applying it atomically is the caller's job.

use crate::domain::{DomainEvent, LobbyEvent};
use konnekt_session_core::{DomainCommand, ParticipationMode, RunStatus};
use std::collections::HashSet;
use uuid::Uuid;

/// Why an event from the other fork is not part of the merged history.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// The main fork already contains an equivalent event.
    Duplicate,
    /// The main fork made a conflicting decision, e.g. queued its own activity.
    ReplacedByMainFork,
    /// Host authority is decided by the main fork, not by rebased events.
    AuthorityChange,
    /// The run it belongs to was dropped or already ended on the main fork.
    RunNotAdopted,
}

/// One event of the other fork that the merge discards.
#[derive(Debug, Clone, PartialEq)]
pub struct DroppedEvent {
    /// Sequence on the other fork.
    pub sequence: u64,
    pub event: DomainEvent,
    pub reason: DropReason,
}

/// A validated plan for one fork choice.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MergePlan {
    /// Events of the other fork to append after the main tip, in order.
    /// They must be re-sequenced and re-signed by the current host.
    pub rebased: Vec<DomainEvent>,
    /// Events of the other fork that are discarded; use them to notify players.
    pub dropped: Vec<DroppedEvent>,
}

impl MergePlan {
    /// True when the merged history equals the main fork.
    pub fn is_empty(&self) -> bool {
        self.rebased.is_empty() && self.dropped.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum MergePlanError {
    #[error("a fork contains a lobby creation after the common ancestor")]
    GenesisInFork,
}

/// Facts about the main fork that decide conflicts.
#[derive(Default)]
struct MainFork {
    joined: HashSet<Uuid>,
    removed: HashSet<Uuid>,
    mode_changed: HashSet<Uuid>,
    queued: bool,
    started_run: bool,
    ended_runs: HashSet<Uuid>,
    results: HashSet<(Uuid, Uuid)>,
}

impl MainFork {
    fn new(events: &[LobbyEvent]) -> Result<Self, MergePlanError> {
        let mut main = Self::default();
        for event in events {
            match &event.event {
                DomainEvent::LobbyCreated { .. } => return Err(MergePlanError::GenesisInFork),
                DomainEvent::GuestJoined { participant } => {
                    main.joined.insert(participant.id());
                }
                DomainEvent::GuestLeft { participant_id }
                | DomainEvent::GuestKicked { participant_id, .. } => {
                    main.removed.insert(*participant_id);
                }
                DomainEvent::ParticipationModeChanged { participant_id, .. } => {
                    main.mode_changed.insert(*participant_id);
                }
                DomainEvent::ActivityQueued { .. } => main.queued = true,
                DomainEvent::RunStarted { .. } => main.started_run = true,
                DomainEvent::RunEnded { run_id, .. } => {
                    main.ended_runs.insert(*run_id);
                }
                DomainEvent::ResultSubmitted { run_id, result } => {
                    main.results.insert((*run_id, result.participant_id));
                }
                DomainEvent::HostDelegated { .. } | DomainEvent::SubmitterRemoved { .. } => {}
            }
        }
        Ok(main)
    }
}

/// Build the plan for rebasing `other` (events after the common ancestor on
/// the non-selected fork) onto `main` (events after it on the selected fork).
pub fn plan_merge(main: &[LobbyEvent], other: &[LobbyEvent]) -> Result<MergePlan, MergePlanError> {
    let facts = MainFork::new(main)?;
    let mut plan = MergePlan::default();
    // Runs started on the other fork and adopted as a unit.
    let mut adopted_runs = HashSet::new();
    let mut dropped_runs = HashSet::new();
    for event in other {
        let decision = match &event.event {
            DomainEvent::LobbyCreated { .. } => return Err(MergePlanError::GenesisInFork),
            DomainEvent::GuestJoined { participant } => (!facts.joined.contains(&participant.id()))
                .then_some(())
                .ok_or(DropReason::Duplicate),
            DomainEvent::GuestLeft { participant_id }
            | DomainEvent::GuestKicked { participant_id, .. } => {
                (!facts.removed.contains(participant_id))
                    .then_some(())
                    .ok_or(DropReason::Duplicate)
            }
            DomainEvent::ParticipationModeChanged { participant_id, .. } => {
                (!facts.mode_changed.contains(participant_id))
                    .then_some(())
                    .ok_or(DropReason::ReplacedByMainFork)
            }
            DomainEvent::HostDelegated { .. } => Err(DropReason::AuthorityChange),
            DomainEvent::ActivityQueued { .. } => (!facts.queued)
                .then_some(())
                .ok_or(DropReason::ReplacedByMainFork),
            DomainEvent::RunStarted { run_id, .. } => {
                if facts.started_run {
                    dropped_runs.insert(*run_id);
                    Err(DropReason::ReplacedByMainFork)
                } else {
                    adopted_runs.insert(*run_id);
                    Ok(())
                }
            }
            DomainEvent::ResultSubmitted { run_id, result } => run_event(
                &facts,
                &adopted_runs,
                &dropped_runs,
                *run_id,
                facts.results.contains(&(*run_id, result.participant_id)),
            ),
            DomainEvent::SubmitterRemoved { run_id, .. } => {
                run_event(&facts, &adopted_runs, &dropped_runs, *run_id, false)
            }
            DomainEvent::RunEnded { run_id, .. } => {
                if adopted_runs.contains(run_id) {
                    Ok(())
                } else {
                    Err(DropReason::RunNotAdopted)
                }
            }
        };
        match decision {
            Ok(()) => plan.rebased.push(event.event.clone()),
            Err(reason) => plan.dropped.push(DroppedEvent {
                sequence: event.sequence,
                event: event.event.clone(),
                reason,
            }),
        }
    }
    Ok(plan)
}

/// The command that replays a rebased event on the host that applies the
/// merge. `Ok(None)` means the event follows from earlier commands (a
/// completed run ends once its results are replayed). Kicks are re-issued in
/// the name of `host_id`, because authority belongs to the main fork.
pub fn rebase_command(
    lobby_id: Uuid,
    host_id: Uuid,
    event: &DomainEvent,
) -> Result<Option<DomainCommand>, &'static str> {
    Ok(Some(match event {
        DomainEvent::GuestJoined { participant } => DomainCommand::AddParticipant {
            lobby_id,
            participant: participant.clone(),
        },
        DomainEvent::GuestLeft { participant_id } => DomainCommand::LeaveLobby {
            lobby_id,
            participant_id: *participant_id,
        },
        DomainEvent::GuestKicked { participant_id, .. } => DomainCommand::KickGuest {
            lobby_id,
            host_id,
            guest_id: *participant_id,
        },
        DomainEvent::ParticipationModeChanged {
            participant_id,
            new_mode,
        } => DomainCommand::UpdateParticipantMode {
            lobby_id,
            participant_id: *participant_id,
            new_mode: match new_mode.as_str() {
                "Active" => ParticipationMode::Active,
                "Spectating" => ParticipationMode::Spectating,
                _ => return Err("unknown participation mode"),
            },
        },
        DomainEvent::ActivityQueued { config } => DomainCommand::QueueActivity {
            lobby_id,
            config: config.clone(),
        },
        DomainEvent::RunStarted {
            run_id,
            config,
            required_submitters,
        } => DomainCommand::SyncRunStarted {
            lobby_id,
            run_id: *run_id,
            config: config.clone(),
            required_submitters: required_submitters.clone(),
        },
        DomainEvent::ResultSubmitted { run_id, result } => DomainCommand::SubmitResult {
            lobby_id,
            run_id: *run_id,
            result: result.clone(),
        },
        DomainEvent::SubmitterRemoved {
            run_id,
            participant_id,
        } => DomainCommand::RemoveSubmitter {
            lobby_id,
            run_id: *run_id,
            participant_id: *participant_id,
        },
        DomainEvent::RunEnded {
            run_id,
            status: RunStatus::Cancelled,
            ..
        } => DomainCommand::CancelRun {
            lobby_id,
            run_id: *run_id,
        },
        DomainEvent::RunEnded { .. } => return Ok(None),
        DomainEvent::LobbyCreated { .. } | DomainEvent::HostDelegated { .. } => {
            return Err("event is never rebased");
        }
    }))
}

/// Decide a result or submitter change for `run_id`.
fn run_event(
    facts: &MainFork,
    adopted: &HashSet<Uuid>,
    dropped: &HashSet<Uuid>,
    run_id: Uuid,
    duplicate: bool,
) -> Result<(), DropReason> {
    if adopted.contains(&run_id) {
        Ok(())
    } else if dropped.contains(&run_id) || facts.ended_runs.contains(&run_id) {
        Err(DropReason::RunNotAdopted)
    } else if duplicate {
        Err(DropReason::Duplicate)
    } else {
        // A run older than the fork that the main fork has not ended.
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::Participant;
    use konnekt_session_core::domain::{ActivityConfig, ActivityResult, RunStatus};

    fn events(list: Vec<DomainEvent>) -> Vec<LobbyEvent> {
        let lobby = Uuid::new_v4();
        list.into_iter()
            .enumerate()
            .map(|(index, event)| LobbyEvent::new(index as u64 + 2, lobby, event))
            .collect()
    }

    fn config() -> ActivityConfig {
        ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null)
    }

    fn guest(name: &str) -> Participant {
        Participant::new_guest(name.into()).unwrap_or_else(|error| panic!("{error}"))
    }

    fn reasons(plan: &MergePlan) -> Vec<DropReason> {
        plan.dropped.iter().map(|dropped| dropped.reason).collect()
    }

    #[test]
    fn joins_and_leaves_merge_without_duplicates() {
        let alice = guest("Alice");
        let bob = guest("Bob");
        let main = events(vec![DomainEvent::GuestJoined {
            participant: alice.clone(),
        }]);
        let other = events(vec![
            DomainEvent::GuestJoined { participant: alice },
            DomainEvent::GuestJoined {
                participant: bob.clone(),
            },
            DomainEvent::GuestLeft {
                participant_id: bob.id(),
            },
        ]);
        let plan = plan_merge(&main, &other).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(plan.rebased.len(), 2);
        assert_eq!(reasons(&plan), vec![DropReason::Duplicate]);
    }

    #[test]
    fn stand_in_selection_and_authority_yield_to_the_main_fork() {
        let main = events(vec![DomainEvent::ActivityQueued { config: config() }]);
        let other = events(vec![
            DomainEvent::HostDelegated {
                from: Uuid::new_v4(),
                to: Uuid::new_v4(),
                reason: crate::domain::DelegationReason::Timeout,
            },
            DomainEvent::ActivityQueued { config: config() },
        ]);
        let plan = plan_merge(&main, &other).unwrap_or_else(|error| panic!("{error}"));
        assert!(plan.rebased.is_empty());
        assert_eq!(
            reasons(&plan),
            vec![DropReason::AuthorityChange, DropReason::ReplacedByMainFork]
        );
        let plan = plan_merge(&[], &other[1..]).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(plan.rebased.len(), 1);
    }

    #[test]
    fn a_finished_stand_in_run_is_adopted_with_its_results() {
        let run_id = Uuid::new_v4();
        let player = Uuid::new_v4();
        let result = ActivityResult::new(run_id, player);
        let other = events(vec![
            DomainEvent::RunStarted {
                run_id,
                config: config(),
                required_submitters: vec![player],
            },
            DomainEvent::ResultSubmitted {
                run_id,
                result: result.clone(),
            },
            DomainEvent::RunEnded {
                run_id,
                status: RunStatus::Completed,
                results: vec![result],
            },
        ]);
        let plan = plan_merge(&[], &other).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(plan.rebased.len(), 3);

        let main = events(vec![DomainEvent::RunStarted {
            run_id: Uuid::new_v4(),
            config: config(),
            required_submitters: vec![],
        }]);
        let plan = plan_merge(&main, &other).unwrap_or_else(|error| panic!("{error}"));
        assert!(plan.rebased.is_empty());
        assert_eq!(
            reasons(&plan),
            vec![
                DropReason::ReplacedByMainFork,
                DropReason::RunNotAdopted,
                DropReason::RunNotAdopted
            ]
        );
    }

    #[test]
    fn results_for_an_older_run_survive_unless_the_main_fork_ended_it() {
        let run_id = Uuid::new_v4();
        let player = Uuid::new_v4();
        let other = events(vec![
            DomainEvent::ResultSubmitted {
                run_id,
                result: ActivityResult::new(run_id, player),
            },
            DomainEvent::RunEnded {
                run_id,
                status: RunStatus::Completed,
                results: vec![],
            },
        ]);
        let plan = plan_merge(&[], &other).unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(plan.rebased.len(), 1);
        assert_eq!(reasons(&plan), vec![DropReason::RunNotAdopted]);

        let main = events(vec![DomainEvent::RunEnded {
            run_id,
            status: RunStatus::Completed,
            results: vec![],
        }]);
        let plan = plan_merge(&main, &other).unwrap_or_else(|error| panic!("{error}"));
        assert!(plan.rebased.is_empty());
    }

    #[test]
    fn a_lobby_creation_inside_a_fork_is_invalid() {
        let created = events(vec![DomainEvent::LobbyCreated {
            lobby_id: Uuid::new_v4(),
            host_id: Uuid::new_v4(),
            name: "Room".into(),
        }]);
        assert_eq!(
            plan_merge(&[], &created),
            Err(MergePlanError::GenesisInFork)
        );
        assert_eq!(
            plan_merge(&created, &[]),
            Err(MergePlanError::GenesisInFork)
        );
    }
}
