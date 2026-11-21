//! Lifecycle for a single peer identity challenge.
//!
//! The caller issues a fresh challenge and verifies its response, including
//! its session and peer binding, before reporting the outcome. A machine is
//! scoped to one challenge and cannot be reused after reaching a final state.

use crate::domain::PeerId;
use ed25519_dalek::{Signature, VerifyingKey};
use instant::Instant;
use state_machines::state_machine;
use std::time::Duration;
use uuid::Uuid;

state_machine! {
    name: IdentityVerification,
    context: (),
    dynamic: true,
    initial: AwaitingChallenge,
    states: [AwaitingChallenge, AwaitingProof, Verified, Rejected, Expired, Cancelled],
    final_states: [Verified, Rejected, Expired, Cancelled],
    events {
        challenge_issued { transition: { from: AwaitingChallenge, to: AwaitingProof } }
        proof_verified { transition: { from: AwaitingProof, to: Verified } }
        proof_rejected { transition: { from: AwaitingProof, to: Rejected } }
        timed_out { transition: { from: AwaitingProof, to: Expired } }
        disconnected {
            transition: { from: AwaitingChallenge, to: Cancelled }
            transition: { from: AwaitingProof, to: Cancelled }
        }
    }
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
    lobby_id: Uuid,
    peer: PeerId,
    public_key: [u8; 32],
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

/// One locally issued challenge. Time values must come from the issuer's monotonic clock.
pub struct IdentityChallengeVerifier {
    machine: DynamicIdentityVerification,
    challenge: IdentityChallenge,
    issued_at: Instant,
    validity: Duration,
}

impl IdentityChallengeVerifier {
    pub fn issue(lobby_id: Uuid, peer: PeerId, issued_at: Instant, validity: Duration) -> Self {
        let mut machine = DynamicIdentityVerification::new(());
        let _ = machine.handle(IdentityVerificationEvent::ChallengeIssued);
        Self {
            machine,
            challenge: IdentityChallenge {
                lobby_id,
                peer,
                nonce: Uuid::new_v4(),
            },
            issued_at,
            validity,
        }
    }

    pub fn challenge(&self) -> &IdentityChallenge {
        &self.challenge
    }
    pub fn state(&self) -> IdentityVerificationState {
        self.machine.current_state()
    }

    /// Verify once; the proof is bound to the sender, lobby and fresh nonce.
    pub fn verify(
        &mut self,
        sender: PeerId,
        public_key: [u8; 32],
        signature: [u8; 64],
        now: Instant,
    ) -> Result<VerifiedIdentity, IdentityProofError> {
        if self.state() != IdentityVerificationState::AwaitingProof {
            return Err(IdentityProofError::ChallengeClosed);
        }
        if sender != self.challenge.peer {
            return Err(IdentityProofError::WrongPeer);
        }
        let elapsed = now
            .checked_duration_since(self.issued_at)
            .ok_or(IdentityProofError::InvalidTime)?;
        if elapsed >= self.validity {
            let _ = self.machine.handle(IdentityVerificationEvent::TimedOut);
            return Err(IdentityProofError::Expired);
        }
        let valid = VerifyingKey::from_bytes(&public_key).is_ok_and(|key| {
            key.verify_strict(
                &self.challenge.signing_bytes(),
                &Signature::from_bytes(&signature),
            )
            .is_ok()
        });
        if !valid {
            let _ = self
                .machine
                .handle(IdentityVerificationEvent::ProofRejected);
            return Err(IdentityProofError::InvalidProof);
        }
        let _ = self
            .machine
            .handle(IdentityVerificationEvent::ProofVerified);
        Ok(VerifiedIdentity {
            lobby_id: self.challenge.lobby_id,
            peer: sender,
            public_key,
        })
    }

    /// Cancel the challenge when its peer disconnects.
    pub fn disconnect(&mut self) {
        let _ = self.machine.handle(IdentityVerificationEvent::Disconnected);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicIdentityVerification, IdentityChallengeVerifier, IdentityProofError,
        IdentityVerificationEvent as Event, IdentityVerificationState as State,
    };
    use crate::domain::{MatchboxPeerId, PeerId};
    use ed25519_dalek::{Signer, SigningKey};
    use instant::Instant;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn verification_requires_an_issued_challenge() {
        let mut machine = DynamicIdentityVerification::new(());
        assert!(machine.handle(Event::ProofVerified).is_err());
        assert!(machine.handle(Event::ChallengeIssued).is_ok());
        assert!(machine.handle(Event::ProofVerified).is_ok());
        assert_eq!(machine.current_state(), State::Verified);
        assert!(machine.handle(Event::ProofVerified).is_err());
    }

    #[test]
    fn expired_challenges_cannot_be_verified() {
        let mut machine = DynamicIdentityVerification::new(());
        assert!(machine.handle(Event::ChallengeIssued).is_ok());
        assert!(machine.handle(Event::TimedOut).is_ok());
        assert!(machine.handle(Event::ProofVerified).is_err());
        assert_eq!(machine.current_state(), State::Expired);
    }

    #[test]
    fn valid_proof_is_bound_to_the_peer_and_cannot_be_replayed() {
        let now = Instant::now();
        let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let other = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let lobby = Uuid::new_v4();
        let key = SigningKey::from_bytes(&[7; 32]);
        let mut verifier =
            IdentityChallengeVerifier::issue(lobby, peer, now, Duration::from_secs(5));
        let signature = key.sign(&verifier.challenge().signing_bytes()).to_bytes();
        let public = key.verifying_key().to_bytes();
        assert_eq!(
            verifier.verify(other, public, signature, now),
            Err(IdentityProofError::WrongPeer)
        );
        let identity = verifier.verify(peer, public, signature, now).unwrap();
        assert_eq!(identity.lobby_id(), lobby);
        assert_eq!(identity.peer(), peer);
        assert_eq!(identity.public_key(), &public);
        assert_eq!(
            verifier.verify(peer, public, signature, now),
            Err(IdentityProofError::ChallengeClosed)
        );
        let mut next = IdentityChallengeVerifier::issue(lobby, peer, now, Duration::from_secs(5));
        assert_eq!(
            next.verify(peer, public, signature, now),
            Err(IdentityProofError::InvalidProof)
        );
    }

    #[test]
    fn wrong_lobby_expiry_and_disconnect_cannot_verify() {
        let now = Instant::now();
        let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let key = SigningKey::from_bytes(&[9; 32]);
        let mut verifier =
            IdentityChallengeVerifier::issue(Uuid::new_v4(), peer, now, Duration::from_secs(5));
        let mut wrong = verifier.challenge().clone();
        wrong.lobby_id = Uuid::new_v4();
        let public = key.verifying_key().to_bytes();
        let signature = key.sign(&wrong.signing_bytes()).to_bytes();
        assert_eq!(
            verifier.verify(peer, public, signature, now),
            Err(IdentityProofError::InvalidProof)
        );
        assert_eq!(verifier.state(), State::Rejected);
        let mut expired =
            IdentityChallengeVerifier::issue(Uuid::new_v4(), peer, now, Duration::from_secs(1));
        let proof = key.sign(&expired.challenge().signing_bytes()).to_bytes();
        assert_eq!(
            expired.verify(peer, public, proof, now + Duration::from_secs(1)),
            Err(IdentityProofError::Expired)
        );
        assert_eq!(expired.state(), State::Expired);
        let mut cancelled =
            IdentityChallengeVerifier::issue(Uuid::new_v4(), peer, now, Duration::from_secs(1));
        cancelled.disconnect();
        assert_eq!(
            cancelled.verify(peer, public, proof, now),
            Err(IdentityProofError::ChallengeClosed)
        );
    }
}
