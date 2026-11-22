//! Local reconciliation of caller-supplied event logs.
//!
//! Callers must authenticate authority before using these methods. Legacy
//! logs are compared as supplied. Signed histories are verified against
//! caller-trusted author keys and checkpoints, compared by hash, and a fork is
//! chosen by proof of authority. This component never applies a merge.

use crate::application::merge_plan::{MergePlan, plan_merge};
use crate::application::signed_history::{SignedHistoryError, event_hash, verify_history};
use crate::domain::{
    EventHash, EventLog, HistoryAnchor, LobbyEvent, PeerId, SignedEvent, SignedHistoryBatch,
};
use crate::protocol::event_log_reconciliation::{
    DynamicEventLogReconciliation, EventLogReconciliationEvent as Event,
    EventLogReconciliationState as State,
};
use std::collections::HashMap;
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
    #[error("no complete local signed history is available")]
    NoLocalHistory,
    #[error("supplied events do not match the verified fork choice")]
    HistoryMismatch,
    #[error("only the authority's host applies this merge; this client must resynchronize")]
    NotMergeAuthority,
}

/// One side of a signed comparison and the trust the caller places in it.
///
/// `checkpoint` must be independently trusted (for example persisted by this
/// client); a sender-provided `SignedHistoryBatch::anchor` is not trusted.
/// A segment that does not start at genesis requires a checkpoint
/// immediately preceding its first event.
#[derive(Debug, Clone, Copy)]
pub struct SignedHistoryView<'a> {
    pub events: &'a [SignedEvent],
    pub checkpoint: Option<HistoryAnchor>,
    pub trusted_authors: &'a [[u8; 32]],
}

/// Which history a reconciliation selected as the main chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistorySide {
    Local,
    Remote,
}

/// Why a signed reconciliation was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectionReason {
    /// The caller reported that authority could not be verified.
    UntrustedAuthority,
    /// A history failed signature, chain, author or payload verification.
    InvalidHistory {
        side: HistorySide,
        error: SignedHistoryError,
    },
    /// Both histories start at genesis but share no event: they are different
    /// sessions that reuse one lobby ID.
    UnrelatedHistories,
    /// Both histories diverged and neither fork is signed solely by the
    /// authority key.
    NoAuthoritativeFork,
    /// The caller could not validate a merge plan for the fork choice.
    InvalidMergePlan,
}

/// The result of a successful fork choice. `ancestor` is the last event both
/// histories share; events after it on the other side must be rebased or
/// dropped by the caller's merge plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForkChoice {
    pub ancestor: HistoryAnchor,
    pub selected: HistorySide,
    pub local_tip: HistoryAnchor,
    pub remote_tip: HistoryAnchor,
    /// Key whose fork wins a conflict; only its holder applies the merge.
    pub authority: [u8; 32],
}

impl ForkChoice {
    /// True when the other side has no events after the ancestor, so the
    /// selected history extends it without a conflict.
    pub fn is_fast_forward(&self) -> bool {
        match self.selected {
            HistorySide::Local => self.remote_tip == self.ancestor,
            HistorySide::Remote => self.local_tip == self.ancestor,
        }
    }

    /// True when the merge consists of appending events to the local history:
    /// the local fork won, or the local side has no fork and is fast-forwarded.
    pub fn applies_locally(&self) -> bool {
        self.selected == HistorySide::Local || self.local_tip == self.ancestor
    }
}

/// One reconciliation attempt. A new instance is required after a final state.
pub struct ReconciliationAttempt {
    lobby_id: Uuid,
    machine: DynamicEventLogReconciliation,
    rejection: Option<RejectionReason>,
    fork_choice: Option<ForkChoice>,
    merge_plan: Option<MergePlan>,
}

impl ReconciliationAttempt {
    pub fn new(lobby_id: Uuid) -> Self {
        Self {
            lobby_id,
            machine: DynamicEventLogReconciliation::new(()),
            rejection: None,
            fork_choice: None,
            merge_plan: None,
        }
    }

    pub fn state(&self) -> State {
        self.machine.current_state()
    }

    /// Why the attempt reached `Rejected`, when known.
    pub fn rejection(&self) -> Option<&RejectionReason> {
        self.rejection.as_ref()
    }

    /// The selected history after a signed comparison found a fork.
    pub fn fork_choice(&self) -> Option<ForkChoice> {
        self.fork_choice
    }

    /// The plan validated by `prepare_merge`.
    pub fn merge_plan(&self) -> Option<&MergePlan> {
        self.merge_plan.as_ref()
    }

    /// Report the result of the caller's authority verification. This method
    /// does not verify a key or a signature.
    pub fn authority_checked(&mut self, verified: bool) -> Result<(), ReconciliationError> {
        if verified {
            self.transition(Event::AuthorityVerified)
        } else {
            self.reject(RejectionReason::UntrustedAuthority)
        }
    }

    /// Verify and compare two signed histories, then choose a fork.
    ///
    /// Both sides are verified against their trusted authors and checkpoint.
    /// Matching tips reconcile immediately. Otherwise the last shared event
    /// hash is the common ancestor; a side with no events after it is fast
    /// forwarded, and when both sides diverged the fork signed solely by
    /// `authority` (the previous host's key) is selected. The attempt then
    /// awaits a caller-validated merge plan in `PreparingMerge`.
    ///
    /// Verification failures, unrelated geneses and forks without authority
    /// reject; missing ancestry or non-overlapping segments are insufficient.
    pub fn compare_signed(
        &mut self,
        local: SignedHistoryView<'_>,
        remote: SignedHistoryView<'_>,
        authority: [u8; 32],
    ) -> Result<State, ReconciliationError> {
        if self.state() != State::ComparingHistories {
            return Err(ReconciliationError::InvalidState);
        }
        let local_tip = match self.verify_side(HistorySide::Local, local)? {
            Some(tip) => tip,
            None => return Ok(self.state()),
        };
        let remote_tip = match self.verify_side(HistorySide::Remote, remote)? {
            Some(tip) => tip,
            None => return Ok(self.state()),
        };
        if local_tip == remote_tip {
            self.transition(Event::HistoriesMatch)?;
            return Ok(self.state());
        }
        self.transition(Event::DivergenceDetected)?;
        let local_hashes = known_hashes(local);
        let remote_hashes = known_hashes(remote);
        let Some(ancestor) = common_ancestor(self.lobby_id, &local_hashes, &remote_hashes) else {
            let genesis_differs = matches!(
                (local_hashes.get(&1), remote_hashes.get(&1)),
                (Some(left), Some(right)) if left != right
            );
            if genesis_differs {
                self.reject(RejectionReason::UnrelatedHistories)?;
            } else {
                self.transition(Event::HistoryUnavailable)?;
            }
            return Ok(self.state());
        };
        self.transition(Event::AncestorFound)?;
        let local_fork = fork_after(local.events, ancestor.sequence);
        let remote_fork = fork_after(remote.events, ancestor.sequence);
        let selected = match (local_fork.is_empty(), remote_fork.is_empty()) {
            (true, _) => Some(HistorySide::Remote),
            (_, true) => Some(HistorySide::Local),
            _ => match (
                signed_only_by(local_fork, &authority),
                signed_only_by(remote_fork, &authority),
            ) {
                (true, false) => Some(HistorySide::Local),
                (false, true) => Some(HistorySide::Remote),
                // Neither, or an equivocating authority that signed both forks.
                _ => None,
            },
        };
        let Some(selected) = selected else {
            self.reject(RejectionReason::NoAuthoritativeFork)?;
            return Ok(self.state());
        };
        self.fork_choice = Some(ForkChoice {
            ancestor,
            selected,
            local_tip,
            remote_tip,
            authority,
        });
        self.transition(Event::HistorySelected)?;
        Ok(self.state())
    }

    /// Build the merge plan for the fork choice from the same histories that
    /// were compared. Events after the ancestor must match the verified tips.
    ///
    /// When [`ForkChoice::applies_locally`] holds, `rebased` lists the events
    /// to append to the local history: the remote fork's surviving events, or
    /// on a fast-forward every remote event except authority changes.
    /// Otherwise it lists local events the remote side must rebase.
    ///
    /// A valid plan moves the attempt to `ApplyingMerge`; a fork containing a
    /// lobby creation rejects it with `InvalidMergePlan`.
    pub fn prepare_merge(
        &mut self,
        local: &[SignedEvent],
        remote: &[SignedEvent],
    ) -> Result<MergePlan, ReconciliationError> {
        let choice = match (self.state(), self.fork_choice) {
            (State::PreparingMerge, Some(choice)) => choice,
            _ => return Err(ReconciliationError::InvalidState),
        };
        let local = decode_fork(local, choice.ancestor, choice.local_tip)?;
        let remote = decode_fork(remote, choice.ancestor, choice.remote_tip)?;
        let (main, other) = if choice.applies_locally() {
            (local, remote)
        } else {
            (remote, local)
        };
        match plan_merge(&main, &other) {
            Ok(plan) => {
                self.transition(Event::MergePlanValidated)?;
                self.merge_plan = Some(plan.clone());
                Ok(plan)
            }
            Err(_) => {
                self.reject(RejectionReason::InvalidMergePlan)?;
                Err(ReconciliationError::InvalidState)
            }
        }
    }

    /// Report that the caller atomically applied the validated merge.
    pub fn merge_applied(&mut self) -> Result<(), ReconciliationError> {
        self.transition(Event::MergeApplied)
    }

    /// Verify one side. `Ok(None)` means the attempt reached a final state.
    fn verify_side(
        &mut self,
        side: HistorySide,
        view: SignedHistoryView<'_>,
    ) -> Result<Option<HistoryAnchor>, ReconciliationError> {
        if view.events.is_empty() {
            return match view.checkpoint {
                Some(checkpoint) if checkpoint.lobby_id == self.lobby_id => Ok(Some(checkpoint)),
                Some(_) => Err(ReconciliationError::WrongLobby),
                None => {
                    self.transition(Event::HistoryUnavailable)?;
                    Ok(None)
                }
            };
        }
        match verify_history(
            self.lobby_id,
            view.events,
            view.trusted_authors,
            view.checkpoint,
        ) {
            Ok(tip) => Ok(Some(tip)),
            Err(SignedHistoryError::MissingAncestry) => {
                self.transition(Event::HistoryUnavailable)?;
                Ok(None)
            }
            Err(SignedHistoryError::WrongLobby) => Err(ReconciliationError::WrongLobby),
            Err(error) => {
                self.reject(RejectionReason::InvalidHistory { side, error })?;
                Ok(None)
            }
        }
    }

    fn reject(&mut self, reason: RejectionReason) -> Result<(), ReconciliationError> {
        self.transition(Event::Rejected)?;
        self.rejection = Some(reason);
        Ok(())
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

/// Decode the events after `ancestor` and check that they are exactly the
/// verified chain from `ancestor` to `tip`.
fn decode_fork(
    events: &[SignedEvent],
    ancestor: HistoryAnchor,
    tip: HistoryAnchor,
) -> Result<Vec<LobbyEvent>, ReconciliationError> {
    let fork = fork_after(events, ancestor.sequence);
    let mut previous = ancestor;
    let mut decoded = Vec::with_capacity(fork.len());
    for event in fork {
        if event.lobby_id != ancestor.lobby_id
            || previous.sequence.checked_add(1) != Some(event.sequence)
            || event.previous_hash != Some(previous.hash)
        {
            return Err(ReconciliationError::HistoryMismatch);
        }
        decoded.push(
            serde_json::from_slice::<LobbyEvent>(&event.payload)
                .map_err(|_| ReconciliationError::HistoryMismatch)?,
        );
        previous = HistoryAnchor {
            lobby_id: ancestor.lobby_id,
            sequence: event.sequence,
            hash: event_hash(event),
        };
    }
    if previous != tip {
        return Err(ReconciliationError::HistoryMismatch);
    }
    Ok(decoded)
}

/// Sequence-to-hash map of every verified position, including the checkpoint.
fn known_hashes(view: SignedHistoryView<'_>) -> HashMap<u64, EventHash> {
    view.checkpoint
        .map(|anchor| (anchor.sequence, anchor.hash))
        .into_iter()
        .chain(
            view.events
                .iter()
                .map(|event| (event.sequence, event_hash(event))),
        )
        .collect()
}

/// The highest position both verified chains agree on. Because each hash
/// commits to its predecessor, agreement there implies agreement before it.
fn common_ancestor(
    lobby_id: Uuid,
    local: &HashMap<u64, EventHash>,
    remote: &HashMap<u64, EventHash>,
) -> Option<HistoryAnchor> {
    local
        .iter()
        .filter(|(sequence, hash)| remote.get(sequence) == Some(hash))
        .max_by_key(|(sequence, _)| **sequence)
        .map(|(sequence, hash)| HistoryAnchor {
            lobby_id,
            sequence: *sequence,
            hash: *hash,
        })
}

fn fork_after(events: &[SignedEvent], sequence: u64) -> &[SignedEvent] {
    let start = events.partition_point(|event| event.sequence <= sequence);
    &events[start..]
}

fn signed_only_by(fork: &[SignedEvent], authority: &[u8; 32]) -> bool {
    fork.iter().all(|event| &event.author == authority)
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

    mod signed {
        use super::super::*;
        use crate::application::signed_history::sign_event;
        use crate::domain::DomainEvent;
        use ed25519_dalek::SigningKey;

        fn chain(lobby: Uuid, base: &[SignedEvent], keys: &[&SigningKey]) -> Vec<SignedEvent> {
            let mut events = base.to_vec();
            for key in keys {
                let sequence = events.last().map_or(1, |event| event.sequence + 1);
                let event = if sequence == 1 {
                    DomainEvent::LobbyCreated {
                        lobby_id: lobby,
                        host_id: Uuid::new_v4(),
                        name: "Room".into(),
                    }
                } else {
                    DomainEvent::GuestLeft {
                        participant_id: Uuid::new_v4(),
                    }
                };
                let previous = events.last().map(event_hash);
                events.push(
                    sign_event(&LobbyEvent::new(sequence, lobby, event), previous, key)
                        .unwrap_or_else(|error| panic!("{error}")),
                );
            }
            events
        }

        struct Keys {
            host: SigningKey,
            stand_in: SigningKey,
            trusted: Vec<[u8; 32]>,
        }

        fn keys() -> Keys {
            let host = SigningKey::from_bytes(&[1; 32]);
            let stand_in = SigningKey::from_bytes(&[2; 32]);
            let trusted = vec![
                host.verifying_key().to_bytes(),
                stand_in.verifying_key().to_bytes(),
            ];
            Keys {
                host,
                stand_in,
                trusted,
            }
        }

        fn view<'a>(events: &'a [SignedEvent], trusted: &'a [[u8; 32]]) -> SignedHistoryView<'a> {
            SignedHistoryView {
                events,
                checkpoint: None,
                trusted_authors: trusted,
            }
        }

        fn attempt(lobby: Uuid) -> ReconciliationAttempt {
            let mut attempt = ReconciliationAttempt::new(lobby);
            assert!(attempt.authority_checked(true).is_ok());
            attempt
        }

        #[test]
        fn identical_signed_histories_reconcile_without_a_merge() {
            let lobby = Uuid::new_v4();
            let k = keys();
            let history = chain(lobby, &[], &[&k.host, &k.host]);
            let mut attempt = attempt(lobby);
            let authority = k.host.verifying_key().to_bytes();
            assert_eq!(
                attempt.compare_signed(
                    view(&history, &k.trusted),
                    view(&history, &k.trusted),
                    authority
                ),
                Ok(State::Reconciled)
            );
            assert_eq!(attempt.fork_choice(), None);
        }

        #[test]
        fn a_returning_host_fast_forwards_to_the_stand_in_history() {
            let lobby = Uuid::new_v4();
            let k = keys();
            let local = chain(lobby, &[], &[&k.host, &k.host]);
            let remote = chain(lobby, &local, &[&k.stand_in]);
            let mut attempt = attempt(lobby);
            let authority = k.host.verifying_key().to_bytes();
            assert_eq!(
                attempt.compare_signed(
                    view(&local, &k.trusted),
                    view(&remote, &k.trusted),
                    authority
                ),
                Ok(State::PreparingMerge)
            );
            let choice = attempt
                .fork_choice()
                .unwrap_or_else(|| panic!("no fork choice"));
            assert_eq!(choice.selected, HistorySide::Remote);
            assert_eq!(choice.ancestor.sequence, 2);
            assert!(choice.is_fast_forward());
            let plan = attempt
                .prepare_merge(&local, &remote)
                .unwrap_or_else(|error| panic!("{error}"));
            // The returning host appends the stand-in's event to its history.
            assert!(choice.applies_locally());
            assert_eq!(plan.rebased.len(), 1);
            assert_eq!(attempt.merge_plan(), Some(&plan));
            assert_eq!(attempt.state(), State::ApplyingMerge);
            assert!(attempt.merge_applied().is_ok());
            assert_eq!(attempt.state(), State::Reconciled);
        }

        #[test]
        fn a_split_session_selects_the_fork_signed_by_the_previous_host() {
            let lobby = Uuid::new_v4();
            let k = keys();
            let shared = chain(lobby, &[], &[&k.host]);
            let host_fork = chain(lobby, &shared, &[&k.host, &k.host]);
            let stand_in_fork = chain(lobby, &shared, &[&k.stand_in]);
            let authority = k.host.verifying_key().to_bytes();

            let mut split = attempt(lobby);
            assert_eq!(
                split.compare_signed(
                    view(&stand_in_fork, &k.trusted),
                    view(&host_fork, &k.trusted),
                    authority
                ),
                Ok(State::PreparingMerge)
            );
            let choice = split
                .fork_choice()
                .unwrap_or_else(|| panic!("no fork choice"));
            assert_eq!(choice.selected, HistorySide::Remote);
            assert_eq!(choice.ancestor.sequence, 1);
            assert!(!choice.is_fast_forward());
            assert_eq!(
                split.prepare_merge(&host_fork, &stand_in_fork),
                Err(ReconciliationError::HistoryMismatch)
            );
            assert_eq!(split.state(), State::PreparingMerge);
            let plan = split
                .prepare_merge(&stand_in_fork, &host_fork)
                .unwrap_or_else(|error| panic!("{error}"));
            // The stand-in's guest departure is conflict-free and survives.
            assert_eq!(plan.rebased.len(), 1);
            assert!(plan.dropped.is_empty());
            assert_eq!(split.state(), State::ApplyingMerge);

            let mut rejected = attempt(lobby);
            let unknown = SigningKey::from_bytes(&[9; 32]).verifying_key().to_bytes();
            assert_eq!(
                rejected.compare_signed(
                    view(&stand_in_fork, &k.trusted),
                    view(&host_fork, &k.trusted),
                    unknown
                ),
                Ok(State::Rejected)
            );
            assert_eq!(
                rejected.rejection(),
                Some(&RejectionReason::NoAuthoritativeFork)
            );
        }

        #[test]
        fn unrelated_tampered_and_untrusted_histories_are_rejected() {
            let lobby = Uuid::new_v4();
            let k = keys();
            let authority = k.host.verifying_key().to_bytes();
            let ours = chain(lobby, &[], &[&k.host]);
            let theirs = chain(lobby, &[], &[&k.stand_in]);

            let mut unrelated = attempt(lobby);
            assert_eq!(
                unrelated.compare_signed(
                    view(&ours, &k.trusted),
                    view(&theirs, &k.trusted),
                    authority
                ),
                Ok(State::Rejected)
            );
            assert_eq!(
                unrelated.rejection(),
                Some(&RejectionReason::UnrelatedHistories)
            );

            let mut tampered_events = ours.clone();
            tampered_events[0].payload.push(b' ');
            let mut tampered = attempt(lobby);
            assert_eq!(
                tampered.compare_signed(
                    view(&ours, &k.trusted),
                    view(&tampered_events, &k.trusted),
                    authority
                ),
                Ok(State::Rejected)
            );
            assert_eq!(
                tampered.rejection(),
                Some(&RejectionReason::InvalidHistory {
                    side: HistorySide::Remote,
                    error: SignedHistoryError::InvalidSignature,
                })
            );

            let only_host = [authority];
            let mut untrusted = attempt(lobby);
            assert_eq!(
                untrusted.compare_signed(
                    view(&ours, &k.trusted),
                    view(&theirs, &only_host),
                    authority
                ),
                Ok(State::Rejected)
            );
            assert!(matches!(
                untrusted.rejection(),
                Some(RejectionReason::InvalidHistory {
                    error: SignedHistoryError::UnknownAuthor,
                    ..
                })
            ));
        }

        #[test]
        fn bounded_segments_need_a_trusted_checkpoint_to_find_an_ancestor() {
            let lobby = Uuid::new_v4();
            let k = keys();
            let authority = k.host.verifying_key().to_bytes();
            let local = chain(lobby, &[], &[&k.host, &k.host, &k.host]);
            let remote = chain(lobby, &local[..2], &[&k.stand_in, &k.stand_in]);
            let segment = &remote[2..];

            let mut missing = attempt(lobby);
            assert_eq!(
                missing.compare_signed(
                    view(&local, &k.trusted),
                    view(segment, &k.trusted),
                    authority
                ),
                Ok(State::InsufficientHistory)
            );

            let checkpoint = HistoryAnchor {
                lobby_id: lobby,
                sequence: 2,
                hash: event_hash(&remote[1]),
            };
            let mut anchored = attempt(lobby);
            assert_eq!(
                anchored.compare_signed(
                    view(&local, &k.trusted),
                    SignedHistoryView {
                        events: segment,
                        checkpoint: Some(checkpoint),
                        trusted_authors: &k.trusted,
                    },
                    authority
                ),
                Ok(State::PreparingMerge)
            );
            let choice = anchored
                .fork_choice()
                .unwrap_or_else(|| panic!("no fork choice"));
            assert_eq!(choice.ancestor, checkpoint);
            assert_eq!(choice.selected, HistorySide::Local);
        }
    }
}
