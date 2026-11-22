//! Peer-bound challenge exchange. It proves key possession, not authority.

use crate::application::IdentityChallengeVerifier;
use crate::domain::PeerId;
use crate::domain::{IdentityChallenge, VerifiedIdentity};
use crate::protocol::identity::IdentityVerificationState;
use ed25519_dalek::{Signer, SigningKey};
use instant::Instant;
use std::collections::{HashMap, HashSet};
use std::time::Duration;
use uuid::Uuid;

const VALIDITY: Duration = Duration::from_secs(30);
const RETRY: Duration = Duration::from_secs(2);

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "identity_v1", content = "payload")]
pub(crate) enum IdentityMessage {
    Challenge(IdentityChallenge),
    Proof {
        nonce: Uuid,
        public_key: [u8; 32],
        signature: Vec<u8>,
    },
}

struct Pending {
    verifier: IdentityChallengeVerifier,
    last_sent: Instant,
    identity: Option<VerifiedIdentity>,
}

pub(crate) struct IdentityExchange {
    signing_key: SigningKey,
    pending: HashMap<PeerId, Pending>,
    peer_challenges: HashMap<PeerId, Uuid>,
    history_requests: HashMap<PeerId, HashSet<Uuid>>,
}

impl IdentityExchange {
    pub(crate) fn new(mut secret: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&secret);
        secret.fill(0);
        Self {
            signing_key,
            pending: HashMap::new(),
            peer_challenges: HashMap::new(),
            history_requests: HashMap::new(),
        }
    }

    pub(crate) fn verified(&self, peer: PeerId) -> Option<&VerifiedIdentity> {
        let pending = self.pending.get(&peer)?;
        (pending.verifier.state() == IdentityVerificationState::Verified)
            .then_some(pending.identity.as_ref())
            .flatten()
    }

    pub(crate) fn disconnected(&mut self, peer: PeerId) {
        self.peer_challenges.remove(&peer);
        self.history_requests.remove(&peer);
        if let Some(mut pending) = self.pending.remove(&peer) {
            pending.verifier.disconnect();
        }
    }

    pub(crate) fn sign(&self, bytes: &[u8]) -> [u8; 64] {
        self.signing_key.sign(bytes).to_bytes()
    }

    pub(crate) fn peer_challenge(&self, peer: PeerId) -> Option<Uuid> {
        self.peer_challenges.get(&peer).copied()
    }

    pub(crate) fn issued_challenge(&self, peer: PeerId) -> Option<Uuid> {
        self.verified(peer)?;
        Some(self.pending.get(&peer)?.verifier.challenge().nonce)
    }

    pub(crate) fn accept_history_request(&mut self, peer: PeerId, id: Uuid) -> bool {
        let seen = self.history_requests.entry(peer).or_default();
        seen.len() < 1024 && seen.insert(id)
    }

    pub(crate) fn advance(
        &mut self,
        lobby: Uuid,
        peers: &[PeerId],
        now: Instant,
    ) -> Vec<(PeerId, IdentityMessage)> {
        let connected: HashSet<_> = peers.iter().copied().collect();
        self.peer_challenges
            .retain(|peer, _| connected.contains(peer));
        self.history_requests
            .retain(|peer, _| connected.contains(peer));
        self.pending.retain(|peer, pending| {
            if !connected.contains(peer) {
                pending.verifier.disconnect();
                return false;
            }
            true
        });
        let mut outgoing = Vec::new();
        for &peer in peers {
            let replace = self.pending.get_mut(&peer).is_none_or(|pending| {
                pending.verifier.expire_if_due(now);
                match pending.verifier.state() {
                    IdentityVerificationState::Expired | IdentityVerificationState::Cancelled => {
                        true
                    }
                    IdentityVerificationState::Rejected => pending.verifier.deadline_elapsed(now),
                    _ => false,
                }
            });
            if replace {
                let verifier = IdentityChallengeVerifier::issue(lobby, peer, now, VALIDITY);
                outgoing.push((
                    peer,
                    IdentityMessage::Challenge(verifier.challenge().clone()),
                ));
                self.pending.insert(
                    peer,
                    Pending {
                        verifier,
                        last_sent: now,
                        identity: None,
                    },
                );
            } else if let Some(pending) = self.pending.get_mut(&peer)
                && pending.verifier.state() == IdentityVerificationState::AwaitingProof
                && now
                    .checked_duration_since(pending.last_sent)
                    .is_some_and(|elapsed| elapsed >= RETRY)
            {
                pending.last_sent = now;
                outgoing.push((
                    peer,
                    IdentityMessage::Challenge(pending.verifier.challenge().clone()),
                ));
            }
        }
        outgoing
    }

    pub(crate) fn receive(
        &mut self,
        lobby: Uuid,
        local: PeerId,
        sender: PeerId,
        message: IdentityMessage,
        now: Instant,
    ) -> Option<IdentityMessage> {
        match message {
            IdentityMessage::Challenge(challenge)
                if challenge.lobby_id == lobby && challenge.peer == local =>
            {
                self.peer_challenges.insert(sender, challenge.nonce);
                Some(IdentityMessage::Proof {
                    nonce: challenge.nonce,
                    public_key: self.signing_key.verifying_key().to_bytes(),
                    signature: self
                        .signing_key
                        .sign(&challenge.signing_bytes())
                        .to_bytes()
                        .to_vec(),
                })
            }
            IdentityMessage::Proof {
                nonce,
                public_key,
                signature,
            } => {
                let signature: [u8; 64] = signature.try_into().ok()?;
                let pending = self.pending.get_mut(&sender)?;
                if pending.verifier.challenge().nonce != nonce {
                    return None;
                }
                if let Ok(identity) = pending.verifier.verify(sender, public_key, signature, now) {
                    pending.identity = Some(identity);
                }
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MatchboxPeerId;

    #[test]
    fn history_requests_cannot_be_replayed_during_a_connection() {
        let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let request = Uuid::new_v4();
        let mut exchange = IdentityExchange::new([1; 32]);
        assert!(exchange.accept_history_request(peer, request));
        assert!(!exchange.accept_history_request(peer, request));
        exchange.disconnected(peer);
        assert!(exchange.accept_history_request(peer, request));
    }

    #[test]
    fn rejected_challenge_is_not_retried_and_a_fresh_one_can_succeed() {
        let now = Instant::now();
        let host = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let guest = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let lobby = Uuid::new_v4();
        let mut a = IdentityExchange::new([1; 32]);
        let mut b = IdentityExchange::new([2; 32]);
        let mut initial = a.advance(lobby, &[guest], now);
        let challenge = initial.remove(0).1;
        let IdentityMessage::Challenge(ref value) = challenge else {
            panic!("expected challenge")
        };
        a.receive(
            lobby,
            host,
            guest,
            IdentityMessage::Proof {
                nonce: value.nonce,
                public_key: [0; 32],
                signature: vec![0; 64],
            },
            now,
        );
        assert_eq!(
            a.pending[&guest].verifier.state(),
            IdentityVerificationState::Rejected
        );
        assert!(a.advance(lobby, &[guest], now + RETRY).is_empty());
        let later = now + VALIDITY;
        let mut fresh = a.advance(lobby, &[guest], later);
        let proof = b
            .receive(lobby, guest, host, fresh.remove(0).1, later)
            .unwrap();
        a.receive(lobby, host, guest, proof, later);
        assert_eq!(
            a.pending[&guest].verifier.state(),
            IdentityVerificationState::Verified
        );
        assert!(a.verified(guest).is_some());
        assert!(a.advance(lobby, &[guest], later + VALIDITY).is_empty());
        a.disconnected(guest);
        assert!(a.verified(guest).is_none());
    }

    #[test]
    fn stale_proof_cannot_consume_a_new_challenge() {
        let now = Instant::now();
        let host = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let guest = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let lobby = Uuid::new_v4();
        let mut a = IdentityExchange::new([1; 32]);
        let mut b = IdentityExchange::new([2; 32]);
        let mut first = a.advance(lobby, &[guest], now);
        let old = b
            .receive(lobby, guest, host, first.remove(0).1, now)
            .unwrap();
        let later = now + VALIDITY;
        let mut second = a.advance(lobby, &[guest], later);
        a.receive(lobby, host, guest, old, later);
        let proof = b
            .receive(lobby, guest, host, second.remove(0).1, later)
            .unwrap();
        a.receive(lobby, host, guest, proof, later);
        assert!(a.verified(guest).is_some());
    }
}
