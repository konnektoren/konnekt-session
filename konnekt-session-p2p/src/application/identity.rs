//! Issue and verify peer-bound identity challenges.

use crate::domain::{IdentityChallenge, IdentityProofError, PeerId, VerifiedIdentity};
use crate::protocol::identity::{
    DynamicIdentityVerification, IdentityVerificationEvent, IdentityVerificationState,
};
use ed25519_dalek::{Signature, VerifyingKey};
use instant::Instant;
use std::time::Duration;
use uuid::Uuid;

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

    pub(crate) fn deadline_elapsed(&self, now: Instant) -> bool {
        now.checked_duration_since(self.issued_at)
            .is_some_and(|elapsed| elapsed >= self.validity)
    }

    pub(crate) fn expire_if_due(&mut self, now: Instant) {
        if self.state() == IdentityVerificationState::AwaitingProof && self.deadline_elapsed(now) {
            let _ = self.machine.handle(IdentityVerificationEvent::TimedOut);
        }
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
    use super::*;
    use crate::domain::{MatchboxPeerId, PeerId};
    use crate::protocol::identity::IdentityVerificationState as State;
    use ed25519_dalek::{Signer, SigningKey};
    use instant::Instant;
    use std::time::Duration;
    use uuid::Uuid;

    #[test]
    fn timeout_tick_expires_an_unanswered_challenge() {
        let now = Instant::now();
        let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let mut verifier =
            IdentityChallengeVerifier::issue(Uuid::new_v4(), peer, now, Duration::from_secs(1));
        verifier.expire_if_due(now);
        assert_eq!(verifier.state(), State::AwaitingProof);
        verifier.expire_if_due(now + Duration::from_secs(1));
        assert_eq!(verifier.state(), State::Expired);
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
