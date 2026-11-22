//! Signed messages that hand the host role back after failover.
//!
//! 1. The original host sees a snapshot from another host of its lobby and
//!    sends `Request`, signed with its history key.
//! 2. A stand-in whose continued chain contains that key replies `History`
//!    with its recorded signed history.
//! 3. The original host verifies and merges it, then sends `Completed`.
//! 4. The stand-in steps down to guest and sends `Handover` to its guests.
//!
//! Every message is signed by `author` and bound to the sending and
//! receiving transport peers. Authorization (which key may reclaim, which
//! key the original host trusts) is decided by the session loop.

use crate::domain::{MatchboxPeerId, PeerId, SignedHistoryBatch};
use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use uuid::Uuid;

const SIGNING_DOMAIN: &[u8] = b"konnekt-session/host-reclaim/v1\0";

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(crate) enum ReclaimBody {
    Request {
        lobby_id: Uuid,
    },
    History {
        lobby_id: Uuid,
        history: SignedHistoryBatch,
    },
    Completed {
        lobby_id: Uuid,
    },
    Handover {
        lobby_id: Uuid,
        host: Uuid,
    },
}

impl ReclaimBody {
    pub(crate) fn lobby_id(&self) -> Uuid {
        match self {
            Self::Request { lobby_id }
            | Self::History { lobby_id, .. }
            | Self::Completed { lobby_id }
            | Self::Handover { lobby_id, .. } => *lobby_id,
        }
    }

    pub(crate) fn handover(lobby_id: Uuid, host: PeerId) -> Self {
        Self::Handover {
            lobby_id,
            host: host.0.0,
        }
    }
}

pub(crate) fn peer_from_uuid(id: Uuid) -> PeerId {
    PeerId::new(MatchboxPeerId(id))
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct ReclaimMessage {
    host_reclaim_v1: String,
    author: [u8; 32],
    signature: Vec<u8>,
}

impl ReclaimMessage {
    pub(crate) fn sign(
        body: &ReclaimBody,
        key: &SigningKey,
        sender: PeerId,
        receiver: PeerId,
    ) -> serde_json::Result<Self> {
        let mut message = Self {
            host_reclaim_v1: serde_json::to_string(body)?,
            author: key.verifying_key().to_bytes(),
            signature: Vec::new(),
        };
        message.signature = key
            .sign(&message.signing_bytes(sender, receiver))
            .to_bytes()
            .to_vec();
        Ok(message)
    }

    /// Verify the signature for this sender and receiver and return the
    /// author and body.
    pub(crate) fn open(&self, sender: PeerId, receiver: PeerId) -> Option<([u8; 32], ReclaimBody)> {
        let signature = <[u8; 64]>::try_from(self.signature.as_slice()).ok()?;
        VerifyingKey::from_bytes(&self.author)
            .ok()?
            .verify_strict(
                &self.signing_bytes(sender, receiver),
                &Signature::from_bytes(&signature),
            )
            .ok()?;
        let body = serde_json::from_str(&self.host_reclaim_v1).ok()?;
        Some((self.author, body))
    }

    fn signing_bytes(&self, sender: PeerId, receiver: PeerId) -> Vec<u8> {
        let mut bytes = SIGNING_DOMAIN.to_vec();
        bytes.extend_from_slice(sender.0.0.as_bytes());
        bytes.extend_from_slice(receiver.0.0.as_bytes());
        bytes.extend_from_slice(&self.author);
        bytes.extend_from_slice(self.host_reclaim_v1.as_bytes());
        bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_bind_author_body_and_both_peers() {
        let key = SigningKey::from_bytes(&[3; 32]);
        let sender = peer_from_uuid(Uuid::new_v4());
        let receiver = peer_from_uuid(Uuid::new_v4());
        let body = ReclaimBody::Request {
            lobby_id: Uuid::new_v4(),
        };
        let message = ReclaimMessage::sign(&body, &key, sender, receiver)
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(
            message.open(sender, receiver),
            Some((key.verifying_key().to_bytes(), body))
        );
        assert_eq!(message.open(receiver, sender), None);
        let mut tampered = message.clone();
        tampered.host_reclaim_v1 = tampered.host_reclaim_v1.replace("request", "completed");
        assert_eq!(tampered.open(sender, receiver), None);
        let mut impostor = message;
        impostor.author = SigningKey::from_bytes(&[4; 32]).verifying_key().to_bytes();
        assert_eq!(impostor.open(sender, receiver), None);
    }
}
