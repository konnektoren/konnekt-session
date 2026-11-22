//! Local reconciliation of caller-supplied event logs.
//!
//! Callers must authenticate authority and independently verify history before
//! using these methods. V2 does not yet provide those guarantees. This
//! component never selects or applies a divergent history.

use crate::domain::{EventLog, LobbyEvent, PeerId, SignedHistoryBatch};
use crate::protocol::event_log_reconciliation::{
    DynamicEventLogReconciliation, EventLogReconciliationEvent as Event,
    EventLogReconciliationState as State,
};
use uuid::Uuid;

pub(crate) const MAX_HISTORY_EVENTS: usize = 100;
pub(crate) const MAX_HISTORY_BYTES: usize = 256 * 1024;

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "reconciliation_history_v1", content = "payload")]
pub(crate) enum HistoryMessage {
    Request {
        lobby_id: Uuid,
        request_id: Uuid,
    },
    Response {
        lobby_id: Uuid,
        request_id: Uuid,
        events: Vec<LobbyEvent>,
    },
    SignedRequest {
        lobby_id: Uuid,
        request_id: Uuid,
    },
    SignedResponse {
        lobby_id: Uuid,
        request_id: Uuid,
        history: SignedHistoryBatch,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct SignedHistory {
    pub reconciliation_history_v2: String,
    pub challenge: Uuid,
    pub signature: Vec<u8>,
}

impl SignedHistory {
    pub(crate) fn signing_bytes(&self, sender: PeerId, receiver: PeerId) -> Vec<u8> {
        let mut bytes = b"konnekt-session/reconciliation-history/v2\0".to_vec();
        bytes.extend_from_slice(sender.0.0.as_bytes());
        bytes.extend_from_slice(receiver.0.0.as_bytes());
        bytes.extend_from_slice(self.challenge.as_bytes());
        bytes.extend_from_slice(self.reconciliation_history_v2.as_bytes());
        bytes
    }

    pub(crate) fn verify(&self, sender: PeerId, receiver: PeerId, key: &[u8; 32]) -> bool {
        let Ok(signature) = <[u8; 64]>::try_from(self.signature.as_slice()) else {
            return false;
        };
        ed25519_dalek::VerifyingKey::from_bytes(key).is_ok_and(|key| {
            key.verify_strict(
                &self.signing_bytes(sender, receiver),
                &ed25519_dalek::Signature::from_bytes(&signature),
            )
            .is_ok()
        })
    }
}

pub(crate) fn valid_history(lobby_id: Uuid, events: &[LobbyEvent]) -> bool {
    events.len() <= MAX_HISTORY_EVENTS && events.iter().all(|event| event.lobby_id == lobby_id)
}

pub(crate) fn valid_signed_history(lobby_id: Uuid, history: &SignedHistoryBatch) -> bool {
    !history.events.is_empty()
        && history.events.len() <= MAX_HISTORY_EVENTS
        && history
            .anchor
            .is_none_or(|anchor| anchor.lobby_id == lobby_id)
        && history
            .events
            .iter()
            .all(|event| event.lobby_id == lobby_id)
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ReconciliationError {
    #[error("the reconciliation attempt is not in the required state")]
    InvalidState,
    #[error("history contains an event from another lobby")]
    WrongLobby,
    #[error("reconciliation is already in progress")]
    AlreadyInProgress,
    #[error("the reconciliation peer is not connected")]
    PeerDisconnected,
}

/// One reconciliation attempt. A new instance is required after a final state.
pub struct ReconciliationAttempt {
    lobby_id: Uuid,
    machine: DynamicEventLogReconciliation,
}

impl ReconciliationAttempt {
    pub fn new(lobby_id: Uuid) -> Self {
        Self {
            lobby_id,
            machine: DynamicEventLogReconciliation::new(()),
        }
    }

    pub fn state(&self) -> State {
        self.machine.current_state()
    }

    /// Report the result of the caller's authority verification. This method
    /// does not verify a key or a signature.
    pub fn authority_checked(&mut self, verified: bool) -> Result<(), ReconciliationError> {
        self.transition(if verified {
            Event::AuthorityVerified
        } else {
            Event::Rejected
        })
    }

    /// Compare complete, caller-verified histories, starting at sequence one.
    /// Truncated or non-contiguous histories are insufficient to establish a
    /// match. Different complete histories await verified ancestor discovery.
    pub fn compare(
        &mut self,
        local: &EventLog,
        remote: &EventLog,
    ) -> Result<State, ReconciliationError> {
        if self.state() != State::ComparingHistories {
            return Err(ReconciliationError::InvalidState);
        }
        let left = local.all_events();
        let right = remote.all_events();
        if left
            .iter()
            .chain(&right)
            .any(|event| event.lobby_id != self.lobby_id)
        {
            return Err(ReconciliationError::WrongLobby);
        }
        if !complete(local, &left) || !complete(remote, &right) {
            self.transition(Event::DivergenceDetected)?;
            self.transition(Event::HistoryUnavailable)?;
        } else if left == right {
            self.transition(Event::HistoriesMatch)?;
        } else {
            self.transition(Event::DivergenceDetected)?;
        }
        Ok(self.state())
    }

    pub fn cancel(&mut self) -> Result<(), ReconciliationError> {
        self.transition(Event::Cancelled)
    }

    fn transition(&mut self, event: Event) -> Result<(), ReconciliationError> {
        self.machine
            .handle(event)
            .map_err(|_| ReconciliationError::InvalidState)
    }
}

fn complete(log: &EventLog, events: &[LobbyEvent]) -> bool {
    events
        .last()
        .is_some_and(|last| last.sequence == log.highest_sequence())
        && events.iter().enumerate().all(|(index, event)| {
            u64::try_from(index)
                .ok()
                .and_then(|value| value.checked_add(1))
                == Some(event.sequence)
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{DomainEvent, MatchboxPeerId};
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn signed_messages_bind_payload_peers_and_connection_challenge() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let sender = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let recipient = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let mut message = SignedHistory {
            reconciliation_history_v2: "history".into(),
            challenge: Uuid::new_v4(),
            signature: Vec::new(),
        };
        message.signature = key
            .sign(&message.signing_bytes(sender, recipient))
            .to_bytes()
            .to_vec();
        assert!(message.verify(sender, recipient, &key.verifying_key().to_bytes()));
        assert!(!message.verify(recipient, sender, &key.verifying_key().to_bytes()));
        let challenge = message.challenge;
        message.challenge = Uuid::new_v4();
        assert!(!message.verify(sender, recipient, &key.verifying_key().to_bytes()));
        message.challenge = challenge;
        message.reconciliation_history_v2.push('!');
        assert!(!message.verify(sender, recipient, &key.verifying_key().to_bytes()));
    }

    #[test]
    fn complete_matching_and_divergent_histories_have_distinct_outcomes() {
        let lobby = Uuid::new_v4();
        let event = LobbyEvent::new(
            1,
            lobby,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        );
        let mut local = EventLog::new();
        local.add_event(event.clone());
        let mut remote = local.clone();
        let mut attempt = ReconciliationAttempt::new(lobby);
        assert_eq!(
            attempt.compare(&local, &remote),
            Err(ReconciliationError::InvalidState)
        );
        assert!(attempt.authority_checked(true).is_ok());
        assert_eq!(attempt.compare(&local, &remote), Ok(State::Reconciled));
        remote.add_event(LobbyEvent::new(2, lobby, event.event));
        let mut attempt = ReconciliationAttempt::new(lobby);
        assert!(attempt.authority_checked(true).is_ok());
        assert_eq!(attempt.compare(&local, &remote), Ok(State::FindingAncestor));
    }

    #[test]
    fn truncated_and_untrusted_histories_cannot_complete() {
        let lobby = Uuid::new_v4();
        let mut log = EventLog::new();
        log.add_event(LobbyEvent::new(
            2,
            lobby,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        ));
        let mut attempt = ReconciliationAttempt::new(lobby);
        assert!(attempt.authority_checked(true).is_ok());
        assert_eq!(attempt.compare(&log, &log), Ok(State::InsufficientHistory));
        let mut attempt = ReconciliationAttempt::new(lobby);
        assert!(attempt.authority_checked(false).is_ok());
        assert_eq!(
            attempt.compare(&log, &log),
            Err(ReconciliationError::InvalidState)
        );
    }
}
