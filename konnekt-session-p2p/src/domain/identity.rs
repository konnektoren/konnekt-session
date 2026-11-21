use crate::domain::PeerId;
use uuid::Uuid;

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
