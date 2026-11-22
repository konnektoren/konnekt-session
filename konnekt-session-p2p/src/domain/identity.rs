use crate::domain::PeerId;
use uuid::Uuid;

/// Namespace for participant IDs derived from public keys.
const PARTICIPANT_NAMESPACE: Uuid = Uuid::from_bytes([
    0x6b, 0x6f, 0x6e, 0x6e, 0x65, 0x6b, 0x74, 0x2d, 0x70, 0x61, 0x72, 0x74, 0x69, 0x63, 0x69, 0x70,
]);

/// Stable participant ID for an Ed25519 public key (UUID v5). The same key
/// yields the same participant in every lobby and after reloads.
pub fn participant_id(public_key: &[u8; 32]) -> Uuid {
    Uuid::new_v5(&PARTICIPANT_NAMESPACE, public_key)
}

/// Public, versioned challenge sent to a peer for signing.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IdentityChallenge {
    pub lobby_id: Uuid,
    pub peer: PeerId,
    pub nonce: Uuid,
}

impl IdentityChallenge {
    /// Fixed signing representation; do not substitute serialized JSON.
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut bytes = b"konnekt-session/identity-challenge/v1\0".to_vec();
        bytes.extend_from_slice(self.lobby_id.as_bytes());
        bytes.extend_from_slice(self.peer.0.0.as_bytes());
        bytes.extend_from_slice(self.nonce.as_bytes());
        bytes
    }
}

/// Key possession verified for a peer and lobby, for this challenge only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedIdentity {
    pub(crate) lobby_id: Uuid,
    pub(crate) peer: PeerId,
    pub(crate) public_key: [u8; 32],
}

impl VerifiedIdentity {
    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }
    pub fn peer(&self) -> PeerId {
        self.peer
    }
    pub fn public_key(&self) -> &[u8; 32] {
        &self.public_key
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdentityProofError {
    #[error("challenge is no longer awaiting proof")]
    ChallengeClosed,
    #[error("proof came from a different peer")]
    WrongPeer,
    #[error("verification time precedes challenge issuance")]
    InvalidTime,
    #[error("challenge expired")]
    Expired,
    #[error("invalid Ed25519 public key or signature")]
    InvalidProof,
}
