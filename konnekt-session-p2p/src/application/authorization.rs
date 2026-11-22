//! Which guest commands a host accepts from an authenticated peer.
//!
//! In authenticated sessions every connected peer proves possession of a key,
//! and its participant ID is derived from that key
//! ([`participant_id`](crate::domain::participant_id)). A guest may act only
//! as that participant; commands that only the host may issue are refused.
//! The domain still enforces its own rules (e.g. whether a requester may
//! toggle another participant's mode).

use konnekt_session_core::{DomainCommand, Lobby};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum CommandRejection {
    #[error("command targets another lobby")]
    WrongLobby,
    #[error("command acts for a participant other than the sender")]
    Impersonation,
    #[error("sender is not a participant of this lobby")]
    NotParticipant,
    #[error("only the host may issue this command")]
    HostOnly,
    #[error("joins must carry the key-derived participant")]
    UnboundJoin,
}

/// Decide whether `actor` (the sender's key-derived participant ID) may send
/// `command` to the host of `lobby_id`. `lobby` is the host's current state.
pub fn authorize_guest_command(
    command: &DomainCommand,
    actor: Uuid,
    lobby_id: Uuid,
    lobby: Option<&Lobby>,
) -> Result<(), CommandRejection> {
    use DomainCommand as C;
    let is_participant = || lobby.is_some_and(|lobby| lobby.participants().contains_key(&actor));
    let acting_as = |id: Uuid| {
        if id == actor {
            Ok(())
        } else {
            Err(CommandRejection::Impersonation)
        }
    };
    let target = match command {
        C::CreateLobby { .. }
        | C::CreateLobbyWithHost { .. }
        | C::CancelRun { .. }
        | C::RemoveSubmitter { .. }
        | C::SyncRunStarted { .. }
        | C::DelegateHost { .. } => return Err(CommandRejection::HostOnly),
        C::JoinLobby { .. } => return Err(CommandRejection::UnboundJoin),
        C::AddParticipant {
            lobby_id,
            participant,
        } => {
            if participant.is_host() {
                return Err(CommandRejection::HostOnly);
            }
            acting_as(participant.id())?;
            *lobby_id
        }
        C::LeaveLobby {
            lobby_id,
            participant_id,
        }
        | C::UpdateParticipantMode {
            lobby_id,
            participant_id,
            ..
        } => {
            acting_as(*participant_id)?;
            *lobby_id
        }
        C::ToggleParticipationMode {
            lobby_id,
            requester_id,
            ..
        } => {
            acting_as(*requester_id)?;
            *lobby_id
        }
        C::KickGuest {
            lobby_id, host_id, ..
        } => {
            acting_as(*host_id)?;
            *lobby_id
        }
        C::SubmitResult {
            lobby_id, result, ..
        } => {
            acting_as(result.participant_id)?;
            *lobby_id
        }
        C::QueueActivity { lobby_id, .. }
        | C::StartNextRun { lobby_id }
        | C::StartNextRunIf { lobby_id, .. } => *lobby_id,
    };
    if target != lobby_id {
        return Err(CommandRejection::WrongLobby);
    }
    // Joining is the only command a non-participant may send.
    if !matches!(command, C::AddParticipant { .. }) && !is_participant() {
        return Err(CommandRejection::NotParticipant);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::{ActivityConfig, Participant, domain::ActivityResult};

    fn lobby_with(guest: Uuid) -> Lobby {
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let mut lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        lobby
            .add_guest(
                Participant::guest_with_id(guest, "Guest".into()).unwrap_or_else(|e| panic!("{e}")),
            )
            .unwrap_or_else(|e| panic!("{e}"));
        lobby
    }

    #[test]
    fn guests_act_only_as_themselves_and_never_as_host() {
        let me = Uuid::new_v4();
        let other = Uuid::new_v4();
        let lobby = lobby_with(me);
        let id = lobby.id();
        let check =
            |command: DomainCommand| authorize_guest_command(&command, me, id, Some(&lobby));

        let own = Participant::guest_with_id(me, "Me".into()).unwrap_or_else(|e| panic!("{e}"));
        let fake = Participant::guest_with_id(other, "Me".into()).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(
            check(DomainCommand::AddParticipant {
                lobby_id: id,
                participant: own
            }),
            Ok(())
        );
        assert_eq!(
            check(DomainCommand::AddParticipant {
                lobby_id: id,
                participant: fake
            }),
            Err(CommandRejection::Impersonation)
        );
        assert_eq!(
            check(DomainCommand::JoinLobby {
                lobby_id: id,
                guest_name: "Me".into()
            }),
            Err(CommandRejection::UnboundJoin)
        );
        assert_eq!(
            check(DomainCommand::LeaveLobby {
                lobby_id: id,
                participant_id: other
            }),
            Err(CommandRejection::Impersonation)
        );
        assert_eq!(
            check(DomainCommand::SubmitResult {
                lobby_id: id,
                run_id: Uuid::new_v4(),
                result: ActivityResult::new(Uuid::new_v4(), other),
            }),
            Err(CommandRejection::Impersonation)
        );
        assert_eq!(
            check(DomainCommand::DelegateHost {
                lobby_id: id,
                current_host_id: me,
                new_host_id: me,
            }),
            Err(CommandRejection::HostOnly)
        );
        let queue = DomainCommand::QueueActivity {
            lobby_id: id,
            config: ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null),
        };
        assert_eq!(check(queue.clone()), Ok(()));
        assert_eq!(
            authorize_guest_command(&queue, other, id, Some(&lobby)),
            Err(CommandRejection::NotParticipant)
        );
        assert_eq!(
            authorize_guest_command(&queue, me, Uuid::new_v4(), Some(&lobby)),
            Err(CommandRejection::WrongLobby)
        );
    }
}
