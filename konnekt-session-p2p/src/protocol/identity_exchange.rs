//! Peer-bound challenge exchange. It proves key possession, not authority.

use crate::domain::PeerId;
use crate::protocol::identity::{IdentityChallenge, IdentityChallengeVerifier, VerifiedIdentity};
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
    issued: Instant,
    last_sent: Instant,
}

pub(crate) struct IdentityExchange {
    signing_key: SigningKey,
    pending: HashMap<PeerId, Pending>,
    verified: HashMap<PeerId, VerifiedIdentity>,
}

impl IdentityExchange {
    pub(crate) fn new(mut secret: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&secret);
        secret.fill(0);
        Self {
            signing_key,
            pending: HashMap::new(),
            verified: HashMap::new(),
        }
    }

    pub(crate) fn verified(&self, peer: PeerId) -> Option<&VerifiedIdentity> {
        self.verified.get(&peer)
    }

    pub(crate) fn advance(
        &mut self,
        lobby: Uuid,
        peers: &[PeerId],
        now: Instant,
    ) -> Vec<(PeerId, IdentityMessage)> {
        let connected: HashSet<_> = peers.iter().copied().collect();
        self.pending.retain(|peer, _| connected.contains(peer));
        self.verified.retain(|peer, _| connected.contains(peer));
        let mut outgoing = Vec::new();
        for &peer in peers {
            if self.verified.contains_key(&peer) {
                continue;
            }
            let replace = self
                .pending
                .get(&peer)
                .is_none_or(|p| now.duration_since(p.issued) >= VALIDITY);
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
                        issued: now,
                        last_sent: now,
                    },
                );
            } else if let Some(pending) = self.pending.get_mut(&peer)
                && now.duration_since(pending.last_sent) >= RETRY
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
                    self.verified.insert(sender, identity);
                    self.pending.remove(&sender);
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
