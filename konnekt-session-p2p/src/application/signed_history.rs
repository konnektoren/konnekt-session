//! Version 1 signed event envelopes and local chain verification.
//!
//! Trust in author keys and checkpoints is supplied by the caller. A valid
//! chain alone does not establish host authority or distributed agreement.

use crate::domain::{EventHash, HistoryAnchor, LobbyEvent, SignedEvent};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use sha2::{Digest, Sha256};
use std::collections::VecDeque;
use uuid::Uuid;

const VERSION: u8 = 1;
const MAX_PAYLOAD: usize = 256 * 1024;
const SIGNING_DOMAIN: &[u8] = b"konnekt-session/signed-event/v1\0";
const HASH_DOMAIN: &[u8] = b"konnekt-session/event-hash/v1\0";

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum SignedHistoryError {
    #[error("history is empty")]
    Empty,
    #[error("event version is unsupported")]
    UnsupportedVersion,
    #[error("event or anchor belongs to another lobby")]
    WrongLobby,
    #[error("history does not start at genesis and has no trusted anchor")]
    MissingAncestry,
    #[error("event sequence or previous hash does not continue the chain")]
    BrokenChain,
    #[error("event author is not trusted")]
    UnknownAuthor,
    #[error("event signature is invalid")]
    InvalidSignature,
    #[error("event payload is invalid, oversized, or inconsistent with the envelope")]
    InvalidPayload,
    #[error("history capacity must be nonzero")]
    InvalidCapacity,
    #[error("history recording previously failed")]
    RecordingStopped,
    #[error("signed recording must begin with this lobby's creation")]
    MissingGenesis,
    #[error("signed recording must be enabled once on a host before the lobby exists")]
    RecordingUnavailable,
}

/// A bounded locally signed history. An evicted tip is retained as a
/// checkpoint; the caller must persist and trust it independently.
pub struct SignedHistoryRecorder {
    lobby_id: Uuid,
    key: SigningKey,
    capacity: usize,
    events: VecDeque<SignedEvent>,
    anchor: Option<HistoryAnchor>,
    tip: Option<HistoryAnchor>,
    failed: bool,
}

impl SignedHistoryRecorder {
    pub fn new(
        lobby_id: Uuid,
        key: SigningKey,
        capacity: usize,
    ) -> Result<Self, SignedHistoryError> {
        if capacity == 0 {
            return Err(SignedHistoryError::InvalidCapacity);
        }
        Ok(Self {
            lobby_id,
            key,
            capacity,
            events: VecDeque::new(),
            anchor: None,
            tip: None,
            failed: false,
        })
    }

    /// Record one domain event; after an error, further recording is disabled.
    pub fn append(&mut self, event: crate::domain::DomainEvent) -> Result<(), SignedHistoryError> {
        if self.failed {
            return Err(SignedHistoryError::RecordingStopped);
        }
        if self.tip.is_none()
            && !matches!(&event, crate::domain::DomainEvent::LobbyCreated { lobby_id, .. } if *lobby_id == self.lobby_id)
        {
            self.failed = true;
            return Err(SignedHistoryError::MissingGenesis);
        }
        let next = self.tip.map_or(Some(1), |tip| tip.sequence.checked_add(1));
        let result = next
            .ok_or(SignedHistoryError::BrokenChain)
            .and_then(|sequence| {
                let event = LobbyEvent::new(sequence, self.lobby_id, event);
                sign_event(&event, self.tip.map(|tip| tip.hash), &self.key)
            });
        let signed = match result {
            Ok(signed) => signed,
            Err(error) => {
                self.failed = true;
                return Err(error);
            }
        };
        self.tip = Some(HistoryAnchor {
            lobby_id: self.lobby_id,
            sequence: signed.sequence,
            hash: event_hash(&signed),
        });
        self.events.push_back(signed);
        if self.events.len() > self.capacity
            && let Some(evicted) = self.events.pop_front()
        {
            self.anchor = Some(HistoryAnchor {
                lobby_id: self.lobby_id,
                sequence: evicted.sequence,
                hash: event_hash(&evicted),
            });
        }
        Ok(())
    }

    pub fn events(&self) -> &VecDeque<SignedEvent> {
        &self.events
    }
    pub fn anchor(&self) -> Option<HistoryAnchor> {
        self.anchor
    }
    pub fn tip(&self) -> Option<HistoryAnchor> {
        self.tip
    }
    pub fn failed(&self) -> bool {
        self.failed
    }
    pub(crate) fn stop(&mut self) {
        self.failed = true;
    }
}

/// Sign a versioned envelope around an assigned event.
pub fn sign_event(
    event: &LobbyEvent,
    previous_hash: Option<EventHash>,
    key: &SigningKey,
) -> Result<SignedEvent, SignedHistoryError> {
    if event.sequence == 0 || (event.sequence == 1) != previous_hash.is_none() {
        return Err(SignedHistoryError::BrokenChain);
    }
    let payload = serde_json::to_vec(event).map_err(|_| SignedHistoryError::InvalidPayload)?;
    if payload.len() > MAX_PAYLOAD {
        return Err(SignedHistoryError::InvalidPayload);
    }
    let mut envelope = SignedEvent {
        version: VERSION,
        lobby_id: event.lobby_id,
        sequence: event.sequence,
        previous_hash,
        author: key.verifying_key().to_bytes(),
        payload,
        signature: Vec::new(),
    };
    envelope.signature = key.sign(&signing_bytes(&envelope)).to_bytes().to_vec();
    Ok(envelope)
}

/// Verify every signature and link against caller-trusted author keys and an
/// optional previously verified anchor. The returned anchor describes the tip.
pub fn verify_history(
    lobby_id: Uuid,
    events: &[SignedEvent],
    trusted_authors: &[[u8; 32]],
    anchor: Option<HistoryAnchor>,
) -> Result<HistoryAnchor, SignedHistoryError> {
    if events.is_empty() {
        return Err(SignedHistoryError::Empty);
    }
    if anchor.is_some_and(|anchor| anchor.lobby_id != lobby_id) {
        return Err(SignedHistoryError::WrongLobby);
    }
    if anchor.is_none() && (events[0].sequence != 1 || events[0].previous_hash.is_some()) {
        return Err(SignedHistoryError::MissingAncestry);
    }
    let mut sequence = anchor.map_or(0, |anchor| anchor.sequence);
    let mut previous = anchor.map(|anchor| anchor.hash);
    for event in events {
        if event.version != VERSION {
            return Err(SignedHistoryError::UnsupportedVersion);
        }
        if event.lobby_id != lobby_id {
            return Err(SignedHistoryError::WrongLobby);
        }
        if sequence.checked_add(1) != Some(event.sequence) || event.previous_hash != previous {
            return Err(SignedHistoryError::BrokenChain);
        }
        if !trusted_authors.contains(&event.author) {
            return Err(SignedHistoryError::UnknownAuthor);
        }
        if event.payload.len() > MAX_PAYLOAD {
            return Err(SignedHistoryError::InvalidPayload);
        }
        let signature = <[u8; 64]>::try_from(event.signature.as_slice())
            .map_err(|_| SignedHistoryError::InvalidSignature)?;
        let key = VerifyingKey::from_bytes(&event.author)
            .map_err(|_| SignedHistoryError::InvalidSignature)?;
        key.verify_strict(&signing_bytes(event), &Signature::from_bytes(&signature))
            .map_err(|_| SignedHistoryError::InvalidSignature)?;
        let payload: LobbyEvent = serde_json::from_slice(&event.payload)
            .map_err(|_| SignedHistoryError::InvalidPayload)?;
        if payload.lobby_id != event.lobby_id || payload.sequence != event.sequence {
            return Err(SignedHistoryError::InvalidPayload);
        }
        sequence = event.sequence;
        previous = Some(event_hash(event));
    }
    Ok(HistoryAnchor {
        lobby_id,
        sequence,
        hash: previous.ok_or(SignedHistoryError::Empty)?,
    })
}

fn signing_bytes(event: &SignedEvent) -> Vec<u8> {
    let mut bytes = SIGNING_DOMAIN.to_vec();
    bytes.push(event.version);
    bytes.extend_from_slice(event.lobby_id.as_bytes());
    bytes.extend_from_slice(&event.sequence.to_be_bytes());
    match event.previous_hash {
        Some(hash) => {
            bytes.push(1);
            bytes.extend_from_slice(&hash);
        }
        None => bytes.push(0),
    }
    bytes.extend_from_slice(&event.author);
    bytes.extend_from_slice(&(event.payload.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&event.payload);
    bytes
}

/// Calculate the digest of an envelope; verify it before trusting this digest.
pub fn event_hash(event: &SignedEvent) -> EventHash {
    let mut hash = Sha256::new();
    hash.update(HASH_DOMAIN);
    hash.update(signing_bytes(event));
    hash.update((event.signature.len() as u64).to_be_bytes());
    hash.update(&event.signature);
    hash.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainEvent;

    #[test]
    fn recording_stops_after_an_oversized_event() {
        let lobby = Uuid::new_v4();
        let key = SigningKey::from_bytes(&[4; 32]);
        let mut log =
            SignedHistoryRecorder::new(lobby, key, 2).unwrap_or_else(|error| panic!("{error}"));
        assert!(
            log.append(DomainEvent::LobbyCreated {
                lobby_id: lobby,
                host_id: Uuid::new_v4(),
                name: "Room".into()
            })
            .is_ok()
        );
        let config = konnekt_session_core::domain::ActivityConfig::new(
            "test".into(),
            "x".repeat(MAX_PAYLOAD + 1),
            serde_json::Value::Null,
        );
        assert_eq!(
            log.append(DomainEvent::ActivityQueued { config }),
            Err(SignedHistoryError::InvalidPayload)
        );
        assert!(log.failed());
        assert_eq!(
            log.append(DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4()
            }),
            Err(SignedHistoryError::RecordingStopped)
        );
        assert_eq!(log.tip().map(|tip| tip.sequence), Some(1));
    }

    #[test]
    fn verifies_chain_and_rejects_tampering_unknown_author_and_missing_ancestry() {
        let lobby = Uuid::new_v4();
        let key = SigningKey::from_bytes(&[7; 32]);
        let first = LobbyEvent::new(
            1,
            lobby,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        );
        let first = sign_event(&first, None, &key).unwrap_or_else(|error| panic!("{error}"));
        let second = LobbyEvent::new(
            2,
            lobby,
            DomainEvent::GuestLeft {
                participant_id: Uuid::new_v4(),
            },
        );
        let second = sign_event(&second, Some(event_hash(&first)), &key)
            .unwrap_or_else(|error| panic!("{error}"));
        let trusted = [key.verifying_key().to_bytes()];
        let anchor = verify_history(lobby, std::slice::from_ref(&first), &trusted, None)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(
            verify_history(lobby, std::slice::from_ref(&second), &trusted, Some(anchor)).is_ok()
        );
        assert_eq!(
            verify_history(lobby, std::slice::from_ref(&second), &trusted, None),
            Err(SignedHistoryError::MissingAncestry)
        );
        assert_eq!(
            verify_history(lobby, std::slice::from_ref(&first), &[], None),
            Err(SignedHistoryError::UnknownAuthor)
        );
        let mut tampered = first;
        tampered.payload.push(b' ');
        assert_eq!(
            verify_history(lobby, &[tampered], &trusted, None),
            Err(SignedHistoryError::InvalidSignature)
        );
    }
}
