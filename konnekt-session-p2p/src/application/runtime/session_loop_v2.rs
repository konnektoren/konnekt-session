use crate::application::event_log_reconciliation::{
    ForkChoice, HistoryMessage, MAX_HISTORY_BYTES, ReconciliationAttempt, ReconciliationError,
    RejectionReason, SignedHistory, SignedHistoryView, valid_history, valid_signed_history,
};
use crate::application::host_failover::{
    DEFAULT_CANDIDATE_TURN, DEFAULT_GRACE, HostObservation, failover_candidates,
};
use crate::application::host_reclaim::{ReclaimBody, ReclaimMessage, peer_from_uuid};
use crate::application::identity_exchange::{IdentityExchange, IdentityMessage};
use crate::application::merge_plan::{MergePlan, rebase_command};
use crate::application::persistence::{
    PersistenceError, SESSION_STATE_VERSION, SessionState, SessionStore,
};
use crate::application::signed_history::{
    SignedHistoryError, SignedHistoryRecorder, SignedHistoryReplica,
};
use crate::domain::VerifiedIdentity;
use crate::infrastructure::error::Result;
use crate::infrastructure::transport::{NetworkConnection, P2PTransport, TransportEvent};
use crate::protocol::command_outcome::{
    CommandOutcomeEvent, CommandOutcomeState, DynamicCommandOutcome,
};
use crate::protocol::event_log_reconciliation::EventLogReconciliationState;
use crate::protocol::host_failover::{DynamicHostFailover, HostFailoverEvent, HostFailoverState};
use crate::protocol::join::{DynamicJoinSession, JoinSessionEvent, JoinSessionState};
use crate::protocol::synchronization::{
    DynamicStateSynchronization, StateSynchronizationEvent, StateSynchronizationState,
};
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use std::collections::HashMap;
use uuid::Uuid;

const RECOVERY_RETRY_POLLS: u8 = 50;
/// Runs a host includes in each snapshot by default (most recent first).
pub const DEFAULT_SNAPSHOT_RUNS: usize = 32;

/// Unified session loop (translation layer between domain and transport)
/// Generic over connection type to allow mocking in tests
pub struct SessionLoopV2<C: NetworkConnection> {
    /// Domain layer (business logic)
    domain: DomainLoop,

    /// Transport layer (networking)
    transport: P2PTransport<C>,

    /// Are we the host?
    is_host: bool,

    /// Lobby ID
    lobby_id: Uuid,
    join: Option<DynamicJoinSession>,
    join_participant: Option<konnekt_session_core::Participant>,
    join_connected: bool,
    join_retry_ticks: u8,
    synchronization: Option<DynamicStateSynchronization>,
    recovery_retry_ticks: u8,
    requested_gap: Option<(u64, u64)>,
    snapshot_retry_ticks: u8,
    command_outcomes: HashMap<Uuid, TrackedCommand>,
    pending_guest_commands: HashMap<Uuid, DomainCommand>,
    identity_exchange: Option<IdentityExchange>,
    host_observation: Option<HostObservation>,
    host_grace: std::time::Duration,
    reconciliation: Option<PeerReconciliation>,
    history_offer: Option<HistoryOffer>,
    signed_history_offer: Option<SignedHistoryOffer>,
    signed_history: Option<SignedHistoryRecorder>,
    signed_replica: Option<SignedHistoryReplica>,
    failover: Option<FailoverConfig>,
    candidate_turn: std::time::Duration,
    /// Original host: authors it trusts in a stand-in's history.
    reclaim_trust: Option<Vec<[u8; 32]>>,
    /// Original host: the reclaim in progress or last attempted.
    reclaim: Option<OwnReclaim>,
    /// Stand-in: authors of the chain it continued; any of them may reclaim.
    previous_authorities: Vec<[u8; 32]>,
    /// Stand-in: the accepted reclaim request (peer and author).
    handover: Option<(crate::domain::PeerId, [u8; 32])>,
    /// Key-derived participant ID when authentication is enforced.
    authenticated_as: Option<Uuid>,
    /// Latest snapshot per peer that has not proven its key yet.
    deferred_snapshots: HashMap<crate::domain::PeerId, (serde_json::Value, u64)>,
    /// Most recent runs included in snapshots.
    snapshot_run_limit: usize,
    /// Host: participant each connected guest peer joined or rejoined as.
    peer_participants: HashMap<crate::domain::PeerId, Uuid>,
    /// Guest: host peer this guest last announced its participant to.
    announced_to: Option<crate::domain::PeerId>,
    /// Guest: host participant of the last applied snapshot. Survives the
    /// replica being dropped on disconnect, so an authenticated guest still
    /// recognizes its host when it comes back on a new connection.
    followed_host: Option<Uuid>,
    /// Where the session is saved after changes.
    store: Option<Box<dyn SessionStore>>,
    /// Last saved state, to skip unchanged writes.
    saved: Option<String>,
    /// After a restore: when the active run stops waiting for players who
    /// have not reconnected.
    restore_deadline: Option<instant::Instant>,
}

struct OwnReclaim {
    peer: crate::domain::PeerId,
    machine: DynamicHostFailover,
}

/// Opt-in automatic takeover when this guest is elected after host loss.
struct FailoverConfig {
    /// Key that continues the signed chain; `None` disables recording.
    key: Option<ed25519_dalek::SigningKey>,
    /// Authors followed in the replica, restored after stepping down.
    trusted_authors: Vec<[u8; 32]>,
}

/// Replica size after a stand-in steps down; the chain restarts on the next event.
const MAX_REPLICA_EVENTS: usize = crate::application::event_log_reconciliation::MAX_HISTORY_EVENTS;

/// Control message carrying one recorded host event to guests.
#[derive(serde::Serialize, serde::Deserialize)]
struct SignedEventMessage {
    signed_event_v1: crate::domain::SignedEvent,
}

struct PeerReconciliation {
    peer: crate::domain::PeerId,
    attempt: ReconciliationAttempt,
    request_id: Option<Uuid>,
    expected_key: Option<[u8; 32]>,
    received_history: Option<Vec<crate::domain::LobbyEvent>>,
    requested_signed: bool,
    received_signed_history: Option<crate::domain::SignedHistoryBatch>,
}

struct HistoryOffer {
    peer: crate::domain::PeerId,
    expected_key: [u8; 32],
    events: Vec<crate::domain::LobbyEvent>,
}

struct SignedHistoryOffer {
    peer: crate::domain::PeerId,
    expected_key: [u8; 32],
    history: crate::domain::SignedHistoryBatch,
}

/// Result of applying a merge plan on the authority's host.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MergeOutcome {
    /// Rebased events the host replayed (or that followed from replayed ones).
    pub applied: Vec<crate::domain::DomainEvent>,
    /// Rebased events the domain rejected, with its reason.
    pub failed: Vec<(crate::domain::DomainEvent, String)>,
}

struct TrackedCommand {
    machine: DynamicCommandOutcome,
    rejection: Option<String>,
}

impl<C: NetworkConnection> SessionLoopV2<C> {
    /// Create new session loop
    pub fn new(
        domain: DomainLoop,
        mut transport: P2PTransport<C>,
        is_host: bool,
        lobby_id: Uuid,
    ) -> Self {
        if !is_host {
            transport.use_managed_recovery();
        }
        Self {
            domain,
            transport,
            is_host,
            lobby_id,
            join: (!is_host).then(|| DynamicJoinSession::new(())),
            join_participant: None,
            join_connected: false,
            join_retry_ticks: 0,
            synchronization: (!is_host).then(|| DynamicStateSynchronization::new(())),
            recovery_retry_ticks: 0,
            requested_gap: None,
            snapshot_retry_ticks: 0,
            command_outcomes: HashMap::new(),
            pending_guest_commands: HashMap::new(),
            identity_exchange: None,
            host_observation: None,
            host_grace: DEFAULT_GRACE,
            reconciliation: None,
            history_offer: None,
            signed_history_offer: None,
            signed_history: None,
            signed_replica: None,
            failover: None,
            candidate_turn: DEFAULT_CANDIDATE_TURN,
            reclaim_trust: None,
            reclaim: None,
            previous_authorities: Vec::new(),
            handover: None,
            authenticated_as: None,
            deferred_snapshots: HashMap::new(),
            snapshot_run_limit: DEFAULT_SNAPSHOT_RUNS,
            peer_participants: HashMap::new(),
            announced_to: None,
            followed_host: None,
            store: None,
            saved: None,
            restore_deadline: None,
        }
    }

    /// Request admission using an ID that remains stable for retries in this session.
    pub fn join(&mut self, name: String) -> Result<Uuid> {
        if self.is_host || self.join_participant.is_some() {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "A guest join is already configured or this session is the host".into(),
            ));
        }
        let participant = match self.authenticated_as {
            Some(id) => konnekt_session_core::Participant::guest_with_id(id, name),
            None => konnekt_session_core::Participant::new_guest(name),
        }
        .map_err(|error| crate::infrastructure::error::P2PError::SendFailed(error.to_string()))?;
        let id = participant.id();
        self.join_participant = Some(participant);
        Ok(id)
    }

    /// Guest admission and synchronization state. Hosts have no join state.
    pub fn join_state(&self) -> Option<JoinSessionState> {
        self.join.as_ref().map(DynamicJoinSession::current_state)
    }

    /// Guest synchronization state. Hosts do not synchronize from another peer.
    ///
    /// `Current` means a snapshot was applied and no sequence gap is known;
    /// it does not prove that all host messages have arrived or authenticate them.
    pub fn synchronization_state(&self) -> Option<StateSynchronizationState> {
        self.synchronization
            .as_ref()
            .map(DynamicStateSynchronization::current_state)
    }

    /// Observed availability of the peer that supplied the first applied snapshot.
    /// This peer is not authenticated, and this does not perform host election.
    /// Returns `None` on hosts and before a guest applies its first snapshot.
    pub fn host_failover_state(&self) -> Option<HostFailoverState> {
        self.host_observation.as_ref().map(HostObservation::state)
    }

    /// Enable bounded host event recording before creating this lobby. The
    /// caller owns the key and must persist retained events and checkpoints.
    /// Recorded history is not yet used to restore or synchronize a session.
    pub fn enable_signed_history(
        &mut self,
        key: ed25519_dalek::SigningKey,
        capacity: usize,
    ) -> std::result::Result<(), SignedHistoryError> {
        if !self.is_host || self.signed_history.is_some() || self.get_lobby().is_some() {
            return Err(SignedHistoryError::RecordingUnavailable);
        }
        self.signed_history = Some(SignedHistoryRecorder::new(self.lobby_id, key, capacity)?);
        Ok(())
    }

    /// The retained local chain and its checkpoint, if recording was enabled.
    pub fn signed_history(&self) -> Option<&SignedHistoryRecorder> {
        self.signed_history.as_ref()
    }

    fn record_signed_event(&mut self, event: &CoreDomainEvent) {
        if self.signed_history.is_none() || matches!(event, CoreDomainEvent::CommandFailed { .. }) {
            return;
        }
        let event_lobby = match event {
            CoreDomainEvent::LobbyCreated { lobby } => lobby.id(),
            CoreDomainEvent::GuestJoined { lobby_id, .. }
            | CoreDomainEvent::GuestLeft { lobby_id, .. }
            | CoreDomainEvent::GuestKicked { lobby_id, .. }
            | CoreDomainEvent::ParticipationModeChanged { lobby_id, .. }
            | CoreDomainEvent::HostDelegated { lobby_id, .. }
            | CoreDomainEvent::ActivityQueued { lobby_id, .. }
            | CoreDomainEvent::RunStarted { lobby_id, .. }
            | CoreDomainEvent::ResultSubmitted { lobby_id, .. }
            | CoreDomainEvent::SubmitterRemoved { lobby_id, .. }
            | CoreDomainEvent::RunEnded { lobby_id, .. } => *lobby_id,
            CoreDomainEvent::CommandFailed { .. } => return,
        };
        if event_lobby != self.lobby_id {
            return;
        }
        let mut translated =
            crate::application::EventTranslator::new(self.lobby_id).to_p2p_event(event.clone());
        if let CoreDomainEvent::RunStarted { run_id, .. } = event {
            if let (
                Some(crate::domain::DomainEvent::RunStarted {
                    required_submitters,
                    ..
                }),
                Some(run),
            ) = (&mut translated, self.domain.event_loop().get_run(run_id))
            {
                *required_submitters = run.required_submitters().iter().copied().collect();
                required_submitters.sort_unstable();
            } else {
                if let Some(recorder) = &mut self.signed_history {
                    recorder.stop();
                }
                return;
            }
        }
        let mut recorded = None;
        if let Some(recorder) = &mut self.signed_history {
            if let Some(event) = translated {
                match recorder.append(event) {
                    Ok(()) => recorded = recorder.events().back().cloned(),
                    Err(error) => tracing::warn!(%error, "Signed history recording stopped"),
                }
            } else {
                recorder.stop();
            }
        }
        if let Some(event) = recorded {
            self.share_signed_event(event);
        }
    }

    /// Send a recorded event to every connected peer for their replicas.
    fn share_signed_event(&mut self, event: crate::domain::SignedEvent) {
        let Ok(payload) = serde_json::to_value(SignedEventMessage {
            signed_event_v1: event,
        }) else {
            return;
        };
        for peer in self.transport.connected_peers() {
            if let Err(error) = self.transport.send_control_to(peer, payload.clone()) {
                tracing::debug!(%error, "Unable to share signed event");
            }
        }
    }

    /// Keep a verified copy of the host's signed chain (guest only). Only
    /// events signed by `trusted_authors` are kept; include every key that
    /// may host this lobby, so the chain can be followed across failover.
    pub fn follow_signed_history(
        &mut self,
        trusted_authors: Vec<[u8; 32]>,
        capacity: usize,
    ) -> std::result::Result<(), SignedHistoryError> {
        if self.is_host || self.signed_replica.is_some() {
            return Err(SignedHistoryError::RecordingUnavailable);
        }
        self.signed_replica = Some(SignedHistoryReplica::new(
            self.lobby_id,
            trusted_authors,
            capacity,
        )?);
        Ok(())
    }

    /// The guest's verified copy of the host's signed chain.
    pub fn signed_history_replica(&self) -> Option<&SignedHistoryReplica> {
        self.signed_replica.as_ref()
    }

    /// Let this guest take over as host when it is elected after host loss
    /// (see [`failover_candidates`](crate::application::host_failover::failover_candidates)).
    /// With `key`, the followed signed chain is resumed and continued; the
    /// key should be one the other peers trust. `candidate_turn` is the delay
    /// between successive candidates. Must be called before the first
    /// snapshot is applied.
    pub fn enable_failover(
        &mut self,
        key: Option<ed25519_dalek::SigningKey>,
        candidate_turn: std::time::Duration,
    ) -> bool {
        if self.is_host || self.host_observation.is_some() {
            return false;
        }
        self.failover = Some(FailoverConfig {
            key,
            trusted_authors: Vec::new(),
        });
        self.candidate_turn = candidate_turn;
        true
    }

    /// This guest's position in the failover order, if it may take over.
    fn failover_rank(&self) -> Option<usize> {
        self.failover.as_ref()?;
        let me = self.join_participant.as_ref()?.id();
        failover_candidates(self.get_lobby()?)
            .iter()
            .position(|id| *id == me)
    }

    /// Let this original host take its role back from a stand-in after
    /// failover. `trusted_authors` are the keys that may have continued its
    /// signed chain (e.g. every player's key). Requires signed recording.
    pub fn enable_reclaim(&mut self, trusted_authors: Vec<[u8; 32]>) -> bool {
        if !self.is_host || self.signed_history.is_none() {
            return false;
        }
        self.reclaim_trust = Some(trusted_authors);
        true
    }

    /// Progress of the latest reclaim by this original host:
    /// `ReclaimPending` while waiting for the stand-in's history,
    /// `HostAvailable` when the history was merged or the request rejected.
    pub fn reclaim_state(&self) -> Option<HostFailoverState> {
        self.reclaim
            .as_ref()
            .map(|reclaim| reclaim.machine.current_state())
    }

    /// Original host: another peer announced itself as host of this lobby.
    fn start_reclaim(&mut self, from: crate::domain::PeerId, snapshot: &serde_json::Value) {
        if self.reclaim_trust.is_none()
            || !self.previous_authorities.is_empty()
            || self.reclaim.as_ref().is_some_and(|reclaim| {
                reclaim.machine.current_state() != HostFailoverState::HostAvailable
            })
        {
            return;
        }
        let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot.clone()) else {
            return;
        };
        let own_host = self.get_lobby().map(Lobby::host_id);
        let other_host = snapshot.host();
        if snapshot.lobby_id != self.lobby_id || other_host.is_none() || other_host == own_host {
            return;
        }
        let mut machine = DynamicHostFailover::new(());
        let _ = machine.handle(HostFailoverEvent::ReclaimRequested);
        tracing::info!(%from, "👑 Reclaiming the host role");
        if self.send_reclaim(
            from,
            &ReclaimBody::Request {
                lobby_id: self.lobby_id,
            },
        ) {
            self.reclaim = Some(OwnReclaim {
                peer: from,
                machine,
            });
        }
    }

    fn send_reclaim(&mut self, peer: crate::domain::PeerId, body: &ReclaimBody) -> bool {
        let (Some(local), Some(recorder)) = (self.transport.local_peer_id(), &self.signed_history)
        else {
            return false;
        };
        let sent = ReclaimMessage::sign(body, recorder.signing_key(), local, peer)
            .and_then(serde_json::to_value)
            .map_err(crate::infrastructure::error::P2PError::Serialization)
            .and_then(|payload| self.transport.send_control_to(peer, payload));
        if let Err(error) = &sent {
            tracing::debug!(%error, "Unable to send reclaim message");
        }
        sent.is_ok()
    }

    fn receive_reclaim(
        &mut self,
        sender: crate::domain::PeerId,
        author: [u8; 32],
        body: ReclaimBody,
    ) {
        match body {
            // Stand-in: a previous host asks for its role back.
            ReclaimBody::Request { .. }
                if self.is_host && self.previous_authorities.contains(&author) =>
            {
                let Some(history) = self.signed_history.as_ref().map(|recorder| {
                    crate::domain::SignedHistoryBatch {
                        anchor: recorder.anchor(),
                        events: recorder.events().iter().cloned().collect(),
                    }
                }) else {
                    return;
                };
                let body = ReclaimBody::History {
                    lobby_id: self.lobby_id,
                    history,
                };
                if self.send_reclaim(sender, &body) {
                    self.handover = Some((sender, author));
                }
            }
            // Original host: verify and merge the stand-in's history.
            ReclaimBody::History { history, .. }
                if self.is_host
                    && self.reclaim.as_ref().is_some_and(|reclaim| {
                        reclaim.peer == sender
                            && reclaim.machine.current_state() == HostFailoverState::ReclaimPending
                    })
                    && self
                        .reclaim_trust
                        .as_ref()
                        .is_some_and(|trust| trust.contains(&author)) =>
            {
                self.complete_reclaim(sender, &history);
            }
            // Stand-in: the original host merged our history; step down.
            ReclaimBody::Completed { .. }
                if self.is_host && self.handover == Some((sender, author)) =>
            {
                self.step_down_to_guest(sender);
            }
            // Guest: our host handed its role to another peer.
            ReclaimBody::Handover { host, .. }
                if !self.is_host
                    && self
                        .host_observation
                        .as_ref()
                        .is_some_and(|observation| observation.peer() == sender) =>
            {
                let host = peer_from_uuid(host);
                if let Some(observation) = &mut self.host_observation {
                    observation.handed_over(host);
                }
                self.transport.follow_host(host);
                self.synchronization = Some(DynamicStateSynchronization::new(()));
                self.snapshot_retry_ticks = 0;
            }
            _ => tracing::debug!(%sender, "Ignoring unauthorized reclaim message"),
        }
    }

    /// Original host: merge the stand-in's verified history, then tell it to
    /// step down and announce this host to all peers.
    fn complete_reclaim(
        &mut self,
        stand_in: crate::domain::PeerId,
        history: &crate::domain::SignedHistoryBatch,
    ) {
        let (Some(own), Some(mut trust)) = (
            self.signed_history
                .as_ref()
                .map(SignedHistoryRecorder::author),
            self.reclaim_trust.clone(),
        ) else {
            return;
        };
        trust.push(own);
        let compared = self
            .begin_reconciliation(stand_in)
            .and_then(|()| self.reconciliation_authority_checked(true))
            .and_then(|()| self.compare_signed_reconciliation_history(history, None, &trust, own));
        let merged = match compared {
            Ok(EventLogReconciliationState::Reconciled) => Ok(()),
            Ok(EventLogReconciliationState::PreparingMerge) => self
                .prepare_reconciliation_merge(history)
                .and_then(|_| self.apply_reconciliation_merge())
                .map(|outcome| {
                    for (event, reason) in outcome.failed {
                        tracing::warn!(?event, %reason, "Stand-in event could not be replayed");
                    }
                }),
            Ok(_) => Err(ReconciliationError::InvalidState),
            Err(error) => Err(error),
        };
        let Some(reclaim) = &mut self.reclaim else {
            return;
        };
        if let Err(error) = merged {
            // The stand-in keeps the role; its history stays intact.
            tracing::warn!(%error, "Reclaim rejected: stand-in history cannot be merged");
            let _ = reclaim.machine.handle(HostFailoverEvent::ReclaimRejected);
            return;
        }
        let _ = reclaim.machine.handle(HostFailoverEvent::ReclaimAccepted);
        let _ = reclaim.machine.handle(HostFailoverEvent::Synchronized);
        self.send_reclaim(
            stand_in,
            &ReclaimBody::Completed {
                lobby_id: self.lobby_id,
            },
        );
        for peer in self.transport.connected_peers() {
            self.send_snapshot_to_peer(peer);
        }
    }

    /// Stand-in: hand the role to `host`, tell our guests, and follow it.
    fn step_down_to_guest(&mut self, host: crate::domain::PeerId) {
        tracing::info!(%host, "Handing the host role back");
        let handover = ReclaimBody::handover(self.lobby_id, host);
        for peer in self.transport.connected_peers() {
            if peer != host {
                self.send_reclaim(peer, &handover);
            }
        }
        let key = self
            .signed_history
            .take()
            .map(|recorder| recorder.signing_key().clone());
        let trusted = self
            .failover
            .as_ref()
            .map(|config| config.trusted_authors.clone())
            .unwrap_or_default();
        if let Some(config) = &mut self.failover {
            config.key = key;
        }
        if !trusted.is_empty() {
            self.signed_replica =
                SignedHistoryReplica::new(self.lobby_id, trusted, MAX_REPLICA_EVENTS).ok();
        }
        self.transport.demote_to_guest(host);
        self.is_host = false;
        self.previous_authorities.clear();
        self.handover = None;
        self.pending_guest_commands.clear();
        let mut join = DynamicJoinSession::new(());
        let _ = join.handle(JoinSessionEvent::Connected);
        self.join = Some(join);
        self.join_connected = true;
        self.synchronization = Some(DynamicStateSynchronization::new(()));
        self.snapshot_retry_ticks = 0;
        self.host_observation = Some(HostObservation::with_turn(
            host,
            self.host_grace,
            self.candidate_turn,
        ));
    }

    /// Take over as host after being elected: switch the transport and
    /// domain to host, resume the signed chain, record the delegation and
    /// announce the new host to connected peers with snapshots.
    fn promote_to_host(&mut self) {
        let (Some(me), Some(previous)) = (
            self.join_participant
                .as_ref()
                .map(|participant| participant.id()),
            self.get_lobby().map(Lobby::host_id),
        ) else {
            return;
        };
        tracing::info!("👑 Taking over as host after failover");
        self.transport.promote_to_host();
        self.is_host = true;
        self.join = None;
        self.synchronization = None;
        if let Some(replica) = &self.signed_replica {
            self.previous_authorities = replica.events().iter().map(|event| event.author).collect();
            self.previous_authorities.sort_unstable();
            self.previous_authorities.dedup();
            if let Some(config) = &mut self.failover {
                config.trusted_authors = replica.trusted_authors().to_vec();
            }
        }
        if let (Some(key), Some(replica)) = (
            self.failover.as_mut().and_then(|config| config.key.take()),
            self.signed_replica.take(),
        ) {
            match SignedHistoryRecorder::resume(
                self.lobby_id,
                key,
                replica.capacity(),
                replica.anchor(),
                replica.events().iter().cloned().collect(),
            ) {
                Ok(recorder) => self.signed_history = Some(recorder),
                Err(error) => tracing::warn!(%error, "Cannot continue the signed chain"),
            }
        }
        if let Err(error) = self.domain.submit(DomainCommand::DelegateHost {
            lobby_id: self.lobby_id,
            current_host_id: previous,
            new_host_id: me,
        }) {
            tracing::warn!(%error, "Unable to record host delegation");
        }
        // The previous host is gone; the run does not wait for it.
        self.stop_waiting_for(previous);
        while self.domain.poll() > 0 {
            self.publish_host_events();
        }
        for peer in self.transport.connected_peers() {
            self.send_snapshot_to_peer(peer);
        }
        if let Some(observation) = &mut self.host_observation {
            observation.promoted();
        }
        self.host_observation = None;
    }

    /// Start a local reconciliation attempt for one currently connected peer.
    /// This does not exchange histories or authenticate the peer.
    pub fn begin_reconciliation(
        &mut self,
        peer: crate::domain::PeerId,
    ) -> std::result::Result<(), ReconciliationError> {
        if self.reconciliation.as_ref().is_some_and(|entry| {
            !matches!(
                entry.attempt.state(),
                EventLogReconciliationState::Reconciled
                    | EventLogReconciliationState::Rejected
                    | EventLogReconciliationState::InsufficientHistory
                    | EventLogReconciliationState::Cancelled
            )
        }) {
            return Err(ReconciliationError::AlreadyInProgress);
        }
        if !self.transport.connected_peers().contains(&peer) {
            return Err(ReconciliationError::PeerDisconnected);
        }
        self.reconciliation = Some(PeerReconciliation {
            peer,
            attempt: ReconciliationAttempt::new(self.lobby_id),
            request_id: None,
            expected_key: None,
            received_history: None,
            requested_signed: false,
            received_signed_history: None,
        });
        Ok(())
    }

    /// Local progress for the latest attempt; it is not distributed agreement.
    pub fn reconciliation_state(&self) -> Option<EventLogReconciliationState> {
        self.reconciliation
            .as_ref()
            .map(|entry| entry.attempt.state())
    }

    /// Supply the independently verified authority result for the selected peer.
    pub fn reconciliation_authority_checked(
        &mut self,
        verified: bool,
    ) -> std::result::Result<(), ReconciliationError> {
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .authority_checked(verified)
    }

    /// Compare independently verified, complete logs for this lobby.
    /// V2 does not collect, verify or apply these histories.
    pub fn compare_reconciliation_histories(
        &mut self,
        local: &crate::domain::EventLog,
        remote: &crate::domain::EventLog,
    ) -> std::result::Result<EventLogReconciliationState, ReconciliationError> {
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .compare(local, remote)
    }

    /// Verify the received signed segment against `remote_authors` and the
    /// independently trusted `remote_checkpoint`, compare it with this
    /// client's recorded history, and choose a fork by `authority`.
    /// Without `remote_checkpoint`, the sender's `anchor` is trusted only if
    /// it is a position of this client's own recorded chain.
    pub fn compare_signed_reconciliation_history(
        &mut self,
        remote: &crate::domain::SignedHistoryBatch,
        remote_checkpoint: Option<crate::domain::HistoryAnchor>,
        remote_authors: &[[u8; 32]],
        authority: [u8; 32],
    ) -> std::result::Result<EventLogReconciliationState, ReconciliationError> {
        let recorder = self
            .signed_history
            .as_ref()
            .filter(|recorder| !recorder.failed())
            .ok_or(ReconciliationError::NoLocalHistory)?;
        let local_events: Vec<_> = recorder.events().iter().cloned().collect();
        let local_authors = [recorder.author()];
        let local = SignedHistoryView {
            events: &local_events,
            checkpoint: recorder.anchor(),
            trusted_authors: &local_authors,
        };
        let remote_checkpoint = remote_checkpoint.or_else(|| {
            remote.anchor.filter(|anchor| {
                recorder.anchor() == Some(*anchor)
                    || local_events.iter().any(|event| {
                        event.sequence == anchor.sequence
                            && crate::application::signed_history::event_hash(event) == anchor.hash
                    })
            })
        });
        let remote = SignedHistoryView {
            events: &remote.events,
            checkpoint: remote_checkpoint,
            trusted_authors: remote_authors,
        };
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .compare_signed(local, remote, authority)
    }

    /// The fork selected by the latest signed comparison, if any.
    pub fn reconciliation_fork_choice(&self) -> Option<ForkChoice> {
        self.reconciliation
            .as_ref()
            .and_then(|entry| entry.attempt.fork_choice())
    }

    /// Why the latest attempt was rejected, if it was.
    pub fn reconciliation_rejection(&self) -> Option<&RejectionReason> {
        self.reconciliation
            .as_ref()
            .and_then(|entry| entry.attempt.rejection())
    }

    /// Build the merge plan for the latest fork choice from this client's
    /// recorded history and the same remote segment that was compared.
    pub fn prepare_reconciliation_merge(
        &mut self,
        remote: &crate::domain::SignedHistoryBatch,
    ) -> std::result::Result<MergePlan, ReconciliationError> {
        let local: Vec<_> = self
            .signed_history
            .as_ref()
            .filter(|recorder| !recorder.failed())
            .ok_or(ReconciliationError::NoLocalHistory)?
            .events()
            .iter()
            .cloned()
            .collect();
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .prepare_merge(&local, &remote.events)
    }

    /// Apply the validated merge plan on this host, then complete the attempt.
    ///
    /// Only the host holding the fork choice's authority key applies a plan,
    /// and only when the merge appends to its history (see
    /// [`ForkChoice::applies_locally`]). Its history must not have changed
    /// since the comparison. Rebased events are replayed as domain commands in
    /// one batch before any other queued or incoming command, so they are
    /// re-signed, recorded and broadcast like other host events.
    ///
    /// Commands the domain rejects are reported; the domain cannot roll back
    /// earlier commands, so the attempt still completes as `Reconciled`.
    pub fn apply_reconciliation_merge(
        &mut self,
    ) -> std::result::Result<MergeOutcome, ReconciliationError> {
        let entry = self
            .reconciliation
            .as_ref()
            .ok_or(ReconciliationError::InvalidState)?;
        let (Some(choice), Some(plan)) = (entry.attempt.fork_choice(), entry.attempt.merge_plan())
        else {
            return Err(ReconciliationError::InvalidState);
        };
        if entry.attempt.state() != EventLogReconciliationState::ApplyingMerge {
            return Err(ReconciliationError::InvalidState);
        }
        let recorder = self
            .signed_history
            .as_ref()
            .filter(|recorder| !recorder.failed())
            .ok_or(ReconciliationError::NoLocalHistory)?;
        if !self.is_host || recorder.author() != choice.authority || !choice.applies_locally() {
            return Err(ReconciliationError::NotMergeAuthority);
        }
        let host_id = self
            .get_lobby()
            .map(|lobby| lobby.host_id())
            .ok_or(ReconciliationError::InvalidState)?;
        let events = plan.rebased.clone();

        // Settle already queued commands so the batch is not interleaved; the
        // plan is stale if they changed the history after the comparison.
        while self.domain.poll() > 0 {
            self.publish_host_events();
        }
        if self
            .signed_history
            .as_ref()
            .and_then(|recorder| recorder.tip())
            != Some(choice.local_tip)
        {
            return Err(ReconciliationError::HistoryMismatch);
        }
        let mut outcome = MergeOutcome::default();
        let mut tracked = Vec::new();
        for event in events {
            match rebase_command(self.lobby_id, host_id, &event) {
                Ok(None) => outcome.applied.push(event),
                Ok(Some(command)) => match self.submit_tracked_command(command) {
                    Ok(id) => tracked.push((id, event)),
                    Err(error) => outcome.failed.push((event, error.to_string())),
                },
                Err(reason) => outcome.failed.push((event, reason.to_string())),
            }
            // Keep the bounded command queue from overflowing.
            while self.domain.poll() > 0 {
                self.publish_host_events();
            }
        }
        for (id, event) in tracked {
            let rejection = self
                .command_outcome(id)
                .and_then(|(_, reason)| reason.map(str::to_owned));
            self.forget_command_outcome(id);
            match rejection {
                Some(reason) => outcome.failed.push((event, reason)),
                None => outcome.applied.push(event),
            }
        }
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .merge_applied()?;
        Ok(outcome)
    }

    /// Report that the caller atomically applied the validated merge.
    pub fn reconciliation_merge_applied(&mut self) -> std::result::Result<(), ReconciliationError> {
        self.reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?
            .attempt
            .merge_applied()
    }

    pub fn cancel_reconciliation(&mut self) -> std::result::Result<(), ReconciliationError> {
        let entry = self
            .reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?;
        entry.attempt.cancel()?;
        entry.request_id = None;
        entry.received_history = None;
        entry.received_signed_history = None;
        Ok(())
    }

    /// Offer a bounded history to one connected peer. This replaces any prior
    /// offer; the expected key must match the peer's key-possession proof.
    /// The caller is responsible for choosing a trusted key.
    pub fn offer_reconciliation_history(
        &mut self,
        peer: crate::domain::PeerId,
        expected_key: [u8; 32],
        events: Vec<crate::domain::LobbyEvent>,
    ) -> Result<()> {
        use crate::infrastructure::error::P2PError;
        if !self.transport.connected_peers().contains(&peer) {
            return Err(P2PError::SendFailed("History peer is not connected".into()));
        }
        if !self
            .verified_peer_identity(peer)
            .is_some_and(|identity| identity.public_key() == &expected_key)
        {
            return Err(P2PError::SendFailed(
                "History peer key is not verified or does not match".into(),
            ));
        }
        let sample = HistoryMessage::Response {
            lobby_id: self.lobby_id,
            request_id: Uuid::nil(),
            events: events.clone(),
        };
        if !valid_history(self.lobby_id, &events)
            || serde_json::to_vec(&sample)
                .map_err(P2PError::Serialization)?
                .len()
                > MAX_HISTORY_BYTES
        {
            return Err(P2PError::SendFailed(
                "History has invalid lobby or exceeds transfer limits".into(),
            ));
        }
        self.history_offer = Some(HistoryOffer {
            peer,
            expected_key,
            events,
        });
        Ok(())
    }

    /// Remove the current history offer.
    pub fn clear_reconciliation_history_offer(&mut self) {
        self.history_offer = None;
        self.signed_history_offer = None;
    }

    /// Offer the current locally recorded signed segment to a pinned peer.
    /// Its checkpoint must be trusted independently by the recipient.
    pub fn offer_recorded_signed_history(
        &mut self,
        peer: crate::domain::PeerId,
        expected_key: [u8; 32],
    ) -> Result<()> {
        use crate::infrastructure::error::P2PError;
        if !self.transport.connected_peers().contains(&peer)
            || !self
                .verified_peer_identity(peer)
                .is_some_and(|identity| identity.public_key() == &expected_key)
        {
            return Err(P2PError::SendFailed(
                "History peer is disconnected or its key is not verified".into(),
            ));
        }
        let recorder = self
            .signed_history
            .as_ref()
            .filter(|history| !history.failed())
            .ok_or_else(|| {
                P2PError::SendFailed("No complete signed recording is available".into())
            })?;
        let history = crate::domain::SignedHistoryBatch {
            anchor: recorder.anchor(),
            events: recorder.events().iter().cloned().collect(),
        };
        let sample = HistoryMessage::SignedResponse {
            lobby_id: self.lobby_id,
            request_id: Uuid::nil(),
            history: history.clone(),
        };
        if !valid_signed_history(self.lobby_id, &history)
            || serde_json::to_vec(&sample)
                .map_err(P2PError::Serialization)?
                .len()
                > MAX_HISTORY_BYTES
        {
            return Err(P2PError::SendFailed(
                "Signed history exceeds transfer limits".into(),
            ));
        }
        self.signed_history_offer = Some(SignedHistoryOffer {
            peer,
            expected_key,
            history,
        });
        Ok(())
    }

    /// Request the selected peer's offered history after the caller reports a
    /// successful authority check and key-possession proof matching the pinned
    /// key. The received history still needs validation before comparison. A new request supersedes any
    /// outstanding request or previously received response.
    pub fn request_reconciliation_history(&mut self, expected_key: [u8; 32]) -> Result<Uuid> {
        use crate::infrastructure::error::P2PError;
        let Some(entry) = &self.reconciliation else {
            return Err(P2PError::SendFailed("No reconciliation attempt".into()));
        };
        if entry.attempt.state() != EventLogReconciliationState::ComparingHistories {
            return Err(P2PError::SendFailed(
                "Reconciliation is not awaiting history".into(),
            ));
        }
        let peer = entry.peer;
        if !self
            .verified_peer_identity(peer)
            .is_some_and(|identity| identity.public_key() == &expected_key)
        {
            return Err(P2PError::SendFailed(
                "History peer key is not verified or does not match".into(),
            ));
        }
        let request_id = Uuid::new_v4();
        let message = HistoryMessage::Request {
            lobby_id: self.lobby_id,
            request_id,
        };
        self.send_signed_history(peer, message)?;
        let Some(entry) = &mut self.reconciliation else {
            return Err(P2PError::SendFailed("No reconciliation attempt".into()));
        };
        entry.request_id = Some(request_id);
        entry.expected_key = Some(expected_key);
        entry.received_history = None;
        entry.requested_signed = false;
        entry.received_signed_history = None;
        Ok(request_id)
    }

    /// Request a recorded signed segment. This supersedes any previous request.
    pub fn request_signed_reconciliation_history(
        &mut self,
        expected_key: [u8; 32],
    ) -> Result<Uuid> {
        use crate::infrastructure::error::P2PError;
        let entry = self
            .reconciliation
            .as_ref()
            .ok_or_else(|| P2PError::SendFailed("No reconciliation attempt".into()))?;
        if entry.attempt.state() != EventLogReconciliationState::ComparingHistories {
            return Err(P2PError::SendFailed(
                "Reconciliation is not awaiting history".into(),
            ));
        }
        let peer = entry.peer;
        if !self
            .verified_peer_identity(peer)
            .is_some_and(|identity| identity.public_key() == &expected_key)
        {
            return Err(P2PError::SendFailed(
                "History peer key is not verified or does not match".into(),
            ));
        }
        let request_id = Uuid::new_v4();
        self.send_signed_history(
            peer,
            HistoryMessage::SignedRequest {
                lobby_id: self.lobby_id,
                request_id,
            },
        )?;
        if let Some(entry) = &mut self.reconciliation {
            entry.request_id = Some(request_id);
            entry.expected_key = Some(expected_key);
            entry.requested_signed = true;
            entry.received_history = None;
            entry.received_signed_history = None;
        }
        Ok(request_id)
    }

    /// Take the authenticated message's signed events. Verify their signatures
    /// and chain against independently trusted authors and checkpoint.
    pub fn take_signed_reconciliation_history(
        &mut self,
    ) -> Option<crate::domain::SignedHistoryBatch> {
        let entry = self.reconciliation.as_mut()?;
        (entry.attempt.state() == EventLogReconciliationState::ComparingHistories)
            .then(|| entry.received_signed_history.take())?
    }

    fn send_signed_history(
        &mut self,
        peer: crate::domain::PeerId,
        message: HistoryMessage,
    ) -> Result<()> {
        use crate::infrastructure::error::P2PError;
        let exchange = self
            .identity_exchange
            .as_ref()
            .ok_or_else(|| P2PError::SendFailed("Identity exchange is disabled".into()))?;
        let challenge = exchange
            .peer_challenge(peer)
            .ok_or_else(|| P2PError::SendFailed("Peer challenge not received".into()))?;
        let local = self
            .transport
            .local_peer_id()
            .ok_or_else(|| P2PError::SendFailed("Local peer ID is unavailable".into()))?;
        let body = serde_json::to_string(&message).map_err(P2PError::Serialization)?;
        if body.len() > MAX_HISTORY_BYTES {
            return Err(P2PError::SendFailed(
                "History message exceeds transfer limits".into(),
            ));
        }
        let mut signed = SignedHistory {
            reconciliation_history_v2: body,
            challenge,
            signature: Vec::new(),
        };
        signed.signature = exchange.sign(&signed.signing_bytes(local, peer)).to_vec();
        let payload = serde_json::to_value(signed).map_err(P2PError::Serialization)?;
        self.transport.send_control_to(peer, payload)
    }

    /// Take the unverified response to the most recent matching request.
    pub fn take_reconciliation_history(&mut self) -> Option<Vec<crate::domain::LobbyEvent>> {
        let entry = self.reconciliation.as_mut()?;
        (entry.attempt.state() == EventLogReconciliationState::ComparingHistories)
            .then(|| entry.received_history.take())?
    }

    fn receive_history_message(&mut self, sender: crate::domain::PeerId, message: HistoryMessage) {
        match message {
            HistoryMessage::SignedRequest {
                lobby_id,
                request_id,
            } if lobby_id == self.lobby_id => {
                let Some(offer) = self.signed_history_offer.as_ref().filter(|offer| {
                    offer.peer == sender
                        && self
                            .verified_peer_identity(sender)
                            .is_some_and(|identity| identity.public_key() == &offer.expected_key)
                }) else {
                    return;
                };
                if !self
                    .identity_exchange
                    .as_mut()
                    .is_some_and(|exchange| exchange.accept_history_request(sender, request_id))
                {
                    return;
                }
                let response = HistoryMessage::SignedResponse {
                    lobby_id,
                    request_id,
                    history: offer.history.clone(),
                };
                if let Err(error) = self.send_signed_history(sender, response) {
                    tracing::debug!(%error, "Unable to send signed reconciliation history");
                }
            }
            HistoryMessage::SignedResponse {
                lobby_id,
                request_id,
                history,
            } if lobby_id == self.lobby_id && valid_signed_history(lobby_id, &history) => {
                if let Some(entry) = &mut self.reconciliation
                    && entry.peer == sender
                    && entry.requested_signed
                    && entry.request_id == Some(request_id)
                    && self
                        .identity_exchange
                        .as_ref()
                        .and_then(|exchange| exchange.verified(sender))
                        .is_some_and(|identity| Some(*identity.public_key()) == entry.expected_key)
                    && entry.attempt.state() == EventLogReconciliationState::ComparingHistories
                {
                    entry.request_id = None;
                    entry.received_signed_history = Some(history);
                }
            }
            HistoryMessage::Request {
                lobby_id,
                request_id,
            } if lobby_id == self.lobby_id => {
                let Some(offer) = self.history_offer.as_ref().filter(|offer| {
                    offer.peer == sender
                        && self
                            .verified_peer_identity(sender)
                            .is_some_and(|identity| identity.public_key() == &offer.expected_key)
                }) else {
                    return;
                };
                if !self
                    .identity_exchange
                    .as_mut()
                    .is_some_and(|exchange| exchange.accept_history_request(sender, request_id))
                {
                    return;
                }
                let response = HistoryMessage::Response {
                    lobby_id,
                    request_id,
                    events: offer.events.clone(),
                };
                if let Err(error) = self.send_signed_history(sender, response) {
                    tracing::debug!(%error, "Unable to send reconciliation history");
                }
            }
            HistoryMessage::Response {
                lobby_id,
                request_id,
                events,
            } if lobby_id == self.lobby_id && valid_history(lobby_id, &events) => {
                if let Some(entry) = &mut self.reconciliation
                    && entry.peer == sender
                    && !entry.requested_signed
                    && entry.request_id == Some(request_id)
                    && self
                        .identity_exchange
                        .as_ref()
                        .and_then(|exchange| exchange.verified(sender))
                        .is_some_and(|identity| Some(*identity.public_key()) == entry.expected_key)
                    && entry.attempt.state() == EventLogReconciliationState::ComparingHistories
                {
                    entry.request_id = None;
                    entry.received_history = Some(events);
                }
            }
            _ => {}
        }
    }

    /// Save the session to `store` whenever it changes, so a reload can
    /// reopen it with [`restore_state`](Self::restore_state).
    pub fn enable_persistence(&mut self, store: Box<dyn SessionStore>) {
        self.store = Some(store);
        self.saved = None;
        self.persist();
    }

    /// The stored session of this lobby, if the store has one.
    pub fn stored_state(&self) -> Option<std::result::Result<SessionState, PersistenceError>> {
        let json = self.store.as_ref()?.load(self.lobby_id)?;
        Some(SessionState::from_json(&json))
    }

    /// Delete the stored session, e.g. when the player leaves the lobby.
    pub fn forget_stored_state(&mut self) {
        if let Some(store) = &mut self.store {
            store.remove(self.lobby_id);
        }
        self.saved = None;
    }

    /// The current state as it would be stored: the lobby, the most recent
    /// runs (see [`set_snapshot_run_limit`](Self::set_snapshot_run_limit))
    /// and the retained signed history.
    pub fn session_state(&self) -> Option<SessionState> {
        let lobby = self.get_lobby()?.clone();
        let runs = self
            .domain
            .event_loop()
            .recent_runs_of(&self.lobby_id, self.snapshot_run_limit)
            .into_iter()
            .cloned()
            .collect();
        let signed_history = self
            .signed_history
            .as_ref()
            .filter(|recorder| !recorder.failed())
            .map(|recorder| crate::domain::SignedHistoryBatch {
                anchor: recorder.anchor(),
                events: recorder.events().iter().cloned().collect(),
            })
            .or_else(|| {
                self.signed_replica
                    .as_ref()
                    .map(|replica| crate::domain::SignedHistoryBatch {
                        anchor: replica.anchor(),
                        events: replica.events().iter().cloned().collect(),
                    })
            });
        Some(SessionState {
            version: SESSION_STATE_VERSION,
            lobby,
            runs,
            signed_history,
        })
    }

    fn persist(&mut self) {
        if self.store.is_none() {
            return;
        }
        let Some(json) = self.session_state().and_then(|state| state.to_json().ok()) else {
            return;
        };
        if self.saved.as_ref() == Some(&json) {
            return;
        }
        if let Some(store) = &mut self.store {
            store.save(self.lobby_id, &json);
        }
        self.saved = Some(json);
    }

    /// Reopen a stored lobby as its host, before the lobby exists.
    ///
    /// - A reloaded host continues where it stopped; with signed recording
    ///   enabled, its chain is resumed from the stored history.
    /// - An authenticated participant that is not the stored host (e.g. a
    ///   guest reopening a lobby nobody answers in) takes over as host.
    /// - A run in progress keeps its submitters until the host grace period
    ///   passes; players that have not reconnected by then are not waited for.
    pub fn restore_state(
        &mut self,
        state: SessionState,
    ) -> std::result::Result<(), PersistenceError> {
        if !self.is_host {
            return Err(PersistenceError::NotHost);
        }
        if self.get_lobby().is_some() {
            return Err(PersistenceError::LobbyExists);
        }
        if state.version != SESSION_STATE_VERSION {
            return Err(PersistenceError::UnsupportedVersion(state.version));
        }
        if state.lobby.id() != self.lobby_id {
            return Err(PersistenceError::WrongLobby);
        }
        let previous_host = state.lobby.host_id();
        let take_over = match self.authenticated_as {
            Some(me) if me != previous_host => {
                if !state.lobby.participants().contains_key(&me) {
                    return Err(PersistenceError::NotAParticipant);
                }
                Some(me)
            }
            _ => None,
        };
        let resumed = match (&state.signed_history, &self.signed_history) {
            (Some(history), Some(recorder)) if !history.events.is_empty() => {
                let mut authors: Vec<_> = history.events.iter().map(|event| event.author).collect();
                authors.sort_unstable();
                authors.dedup();
                crate::application::signed_history::verify_history(
                    self.lobby_id,
                    &history.events,
                    &authors,
                    history.anchor,
                )
                .map_err(PersistenceError::InvalidHistory)?;
                Some(
                    SignedHistoryRecorder::resume(
                        self.lobby_id,
                        recorder.signing_key().clone(),
                        recorder.capacity(),
                        history.anchor,
                        history.events.clone(),
                    )
                    .map_err(PersistenceError::InvalidHistory)?,
                )
            }
            _ => None,
        };
        let has_active_run = state.lobby.active_run_id().is_some();
        self.domain
            .event_loop_mut()
            .restore_lobby(state.lobby, state.runs)
            .map_err(|error| PersistenceError::Inconsistent(error.to_string()))?;
        if let Some(recorder) = resumed {
            self.signed_history = Some(recorder);
        }
        if let Some(me) = take_over {
            let _ = self.domain.submit(DomainCommand::DelegateHost {
                lobby_id: self.lobby_id,
                current_host_id: previous_host,
                new_host_id: me,
            });
            while self.domain.poll() > 0 {
                self.publish_host_events();
            }
        }
        if has_active_run {
            self.restore_deadline = Some(instant::Instant::now() + self.host_grace);
        }
        self.persist();
        Ok(())
    }

    /// After a restore's grace period: stop waiting for run submitters that
    /// have not reconnected.
    fn expire_restore_deadline(&mut self, now: instant::Instant) {
        if self.restore_deadline.is_none_or(|deadline| now < deadline) {
            return;
        }
        self.restore_deadline = None;
        let host = self.get_lobby().map(Lobby::host_id);
        let connected = self.connected_participants();
        let absent: Vec<_> = self
            .get_active_run()
            .map(|run| {
                run.required_submitters()
                    .iter()
                    .copied()
                    .filter(|id| Some(*id) != host && !connected.contains(id))
                    .collect()
            })
            .unwrap_or_default();
        for participant in absent {
            self.stop_waiting_for(participant);
        }
    }

    /// Participants whose guest connection is currently known to the host.
    /// Disconnected participants stay in the lobby so they can rejoin.
    pub fn connected_participants(&self) -> Vec<Uuid> {
        let mut ids: Vec<_> = self.peer_participants.values().copied().collect();
        ids.sort_unstable();
        ids.dedup();
        ids
    }

    /// Host: `peer` joined or rejoined as `participant`. In unauthenticated
    /// sessions a participant already linked to another connected peer is not
    /// taken over.
    fn link_participant(&mut self, peer: crate::domain::PeerId, participant: Uuid) {
        let connected = self.transport.connected_peers();
        let taken = self
            .peer_participants
            .iter()
            .any(|(other, id)| *id == participant && *other != peer && connected.contains(other));
        if taken && self.authenticated_as.is_none() {
            return;
        }
        self.peer_participants
            .retain(|other, id| *id != participant || *other == peer);
        self.peer_participants.insert(peer, participant);
    }

    /// Host: a guest connection closed. The participant stays in the lobby
    /// and is linked again when it rejoins, but runs no longer wait for it.
    fn participant_left(&mut self, peer: crate::domain::PeerId) {
        let Some(participant) = self.peer_participants.remove(&peer) else {
            return;
        };
        if !self.peer_participants.values().any(|id| *id == participant) {
            tracing::info!(%participant, "Participant disconnected; not waiting for it");
            self.stop_waiting_for(participant);
        }
    }

    /// Host: remove `participant` from the active run's pending submitters.
    fn stop_waiting_for(&mut self, participant: Uuid) {
        let Some(run) = self.get_active_run() else {
            return;
        };
        if !run.required_submitters().contains(&participant)
            || run.results().contains_key(&participant)
        {
            return;
        }
        let command = DomainCommand::RemoveSubmitter {
            lobby_id: self.lobby_id,
            run_id: run.id(),
            participant_id: participant,
        };
        if let Err(error) = self.domain.submit(command) {
            tracing::warn!(%error, "Unable to remove a departed submitter");
        }
    }

    /// Guest: tell a host this guest (re)connected to which participant it
    /// is, so the host links the connection without adding a new participant.
    fn announce_to(&mut self, host: crate::domain::PeerId) {
        let Some(participant) = self.join_participant.clone() else {
            return;
        };
        if self.announced_to == Some(host)
            || !self
                .get_lobby()
                .is_some_and(|lobby| lobby.participants().contains_key(&participant.id()))
        {
            return;
        }
        let command = DomainCommand::AddParticipant {
            lobby_id: self.lobby_id,
            participant,
        };
        if self.submit_command(command).is_ok() {
            self.announced_to = Some(host);
        }
    }

    /// Limit snapshots to the `limit` most recently started runs (default
    /// [`DEFAULT_SNAPSHOT_RUNS`]); the active run is always included. Guests
    /// keep finished runs they already have, so only peers that join later
    /// miss older results.
    pub fn set_snapshot_run_limit(&mut self, limit: usize) {
        self.snapshot_run_limit = limit;
    }

    /// Configure the grace period before host-loss observation begins.
    /// Returns false once the first host snapshot has been applied.
    pub fn set_host_grace_period(&mut self, grace: std::time::Duration) -> bool {
        if self.host_observation.is_some() {
            return false;
        }
        self.host_grace = grace;
        true
    }

    fn update_join_connection(&mut self) {
        let connected = !self.transport.connected_peers().is_empty();
        if connected == self.join_connected {
            return;
        }
        self.join_connected = connected;
        self.join_retry_ticks = 0;
        self.recovery_retry_ticks = 0;
        self.requested_gap = None;
        self.snapshot_retry_ticks = 0;
        let Some(join) = &mut self.join else { return };
        if connected {
            if let Some(sync) = &mut self.synchronization
                && sync.current_state() == StateSynchronizationState::Disconnected
            {
                let _ = sync.handle(StateSynchronizationEvent::Reconnected);
            }
            if join.current_state() == JoinSessionState::Disconnected {
                let _ = join.handle(JoinSessionEvent::Retry);
            }
            let _ = join.handle(JoinSessionEvent::Connected);
        } else {
            if let Some(sync) = &mut self.synchronization {
                let _ = sync.handle(StateSynchronizationEvent::Disconnected);
            }
            let _ = join.handle(JoinSessionEvent::Disconnected);
            // A failover candidate keeps its replica so it can take over even
            // when no other peer is left.
            if self.join_participant.is_some() && self.failover.is_none() {
                self.domain = DomainLoop::new(10, 100);
            }
        }
    }

    fn update_synchronization_gap(&mut self) {
        let Some(sync) = &mut self.synchronization else {
            return;
        };
        let event = match (sync.current_state(), self.transport.has_sequence_gap()) {
            (StateSynchronizationState::Current, true) => {
                Some(StateSynchronizationEvent::GapDetected)
            }
            (StateSynchronizationState::Recovering, false) => {
                Some(StateSynchronizationEvent::GapRecovered)
            }
            _ => None,
        };
        if let Some(event) = event {
            let _ = sync.handle(event);
        }
        if sync.current_state() == StateSynchronizationState::Recovering {
            let missing = self.transport.missing_range();
            if let Some((from, to)) = missing
                && (self.recovery_retry_ticks == 0 || missing != self.requested_gap)
            {
                self.transport.request_missing_messages(from, to);
                self.requested_gap = missing;
                self.recovery_retry_ticks = RECOVERY_RETRY_POLLS;
            } else if self.recovery_retry_ticks > 0 {
                self.recovery_retry_ticks -= 1;
            }
        } else {
            self.recovery_retry_ticks = 0;
            self.requested_gap = None;
        }
        if sync.current_state() == StateSynchronizationState::AwaitingSnapshot
            && self.join_connected
        {
            if self.snapshot_retry_ticks == 0 {
                if let Err(error) = self.transport.request_snapshot() {
                    tracing::debug!(%error, "Unable to request synchronization snapshot");
                }
                self.snapshot_retry_ticks = RECOVERY_RETRY_POLLS;
            } else {
                self.snapshot_retry_ticks -= 1;
            }
        } else {
            self.snapshot_retry_ticks = 0;
        }
    }

    fn update_join_admission(&mut self) {
        let Some(participant) = &self.join_participant else {
            return;
        };
        let id = participant.id();
        let admitted = self
            .get_lobby()
            .is_some_and(|lobby| lobby.participants().contains_key(&id));
        let Some(join) = &mut self.join else { return };
        if admitted {
            let event = match join.current_state() {
                JoinSessionState::AwaitingAdmission => Some(JoinSessionEvent::AdmissionConfirmed),
                _ => None,
            };
            if let Some(event) = event {
                let _ = join.handle(event);
            }
        }
    }

    fn send_join_request(&mut self) {
        if self.join_state() != Some(JoinSessionState::AwaitingAdmission) {
            return;
        }
        if self.join_retry_ticks > 0 {
            self.join_retry_ticks -= 1;
            return;
        }
        let Some(participant) = self.join_participant.clone() else {
            return;
        };
        let command = DomainCommand::AddParticipant {
            lobby_id: self.lobby_id,
            participant,
        };
        if let Err(error) = self.submit_command(command) {
            tracing::debug!(%error, "Unable to send join request");
        }
        self.announced_to = self.host_observation.as_ref().map(HostObservation::peer);
        self.join_retry_ticks = 49;
    }

    /// Submit a domain command
    pub fn submit_command(&mut self, cmd: DomainCommand) -> Result<()> {
        // An authenticated host is the participant derived from its key.
        let cmd = match (cmd, self.authenticated_as) {
            (
                DomainCommand::CreateLobby {
                    lobby_id,
                    lobby_name,
                    host_name,
                },
                Some(host_id),
            ) if self.is_host => DomainCommand::CreateLobbyWithHost {
                lobby_id: lobby_id.unwrap_or(self.lobby_id),
                lobby_name,
                host: konnekt_session_core::Participant::host_with_id(host_id, host_name).map_err(
                    |e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()),
                )?,
            },
            (cmd, _) => cmd,
        };
        if self.is_host {
            // Host: execute locally
            self.domain
                .submit(cmd)
                .map_err(|e| crate::infrastructure::error::P2PError::SendFailed(e.to_string()))?;
        } else {
            // Guest: send to host
            let payload = serde_json::to_value(&cmd)
                .map_err(crate::infrastructure::error::P2PError::Serialization)?;
            self.transport.send_to_host(payload)?;
        }
        Ok(())
    }

    /// Submit a host-local command and track its actual domain outcome.
    ///
    /// Guest requests are not yet supported: they require authenticated,
    /// correlated replies and host-side deduplication.
    pub fn submit_tracked_command(&mut self, cmd: DomainCommand) -> Result<Uuid> {
        if !self.is_host {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "Tracked guest commands are not supported yet".into(),
            ));
        }
        let id = Uuid::new_v4();
        self.domain.submit_tracked(id, cmd).map_err(|error| {
            crate::infrastructure::error::P2PError::SendFailed(error.to_string())
        })?;
        let mut machine = DynamicCommandOutcome::new(());
        let _ = machine.handle(CommandOutcomeEvent::Sent);
        self.command_outcomes.insert(
            id,
            TrackedCommand {
                machine,
                rejection: None,
            },
        );
        Ok(id)
    }

    /// Read a tracked command's state and, for rejection, its domain reason.
    /// Acceptance means the host applied the command, not that peers received it.
    pub fn command_outcome(&self, id: Uuid) -> Option<(CommandOutcomeState, Option<&str>)> {
        self.command_outcomes.get(&id).map(|tracked| {
            (
                tracked.machine.current_state(),
                tracked.rejection.as_deref(),
            )
        })
    }

    /// Stop retaining a tracked outcome. This does not cancel the command.
    pub fn forget_command_outcome(&mut self, id: Uuid) -> bool {
        self.command_outcomes.remove(&id).is_some()
    }

    /// Enable peer key-possession exchange with an application-owned Ed25519 secret.
    ///
    /// This does not authenticate a host, authorize commands, or gate admission.
    /// Callers must retain their own key if they need identity across reconnects.
    pub fn enable_identity_exchange(&mut self, secret: [u8; 32]) {
        self.history_offer = None;
        self.signed_history_offer = None;
        if let Some(entry) = &mut self.reconciliation {
            let _ = entry.attempt.cancel();
            entry.request_id = None;
            entry.expected_key = None;
            entry.received_history = None;
            entry.received_signed_history = None;
        }
        self.identity_exchange = Some(IdentityExchange::new(secret));
    }

    /// Enforce authentication with an application-owned Ed25519 secret.
    ///
    /// Enables the key exchange and derives this client's participant ID
    /// from the public key ([`participant_id`](crate::domain::participant_id)),
    /// which is returned. Call before creating or joining the lobby.
    ///
    /// - Hosts accept guest commands only from peers that proved a key, and
    ///   only when the command acts for that key's participant
    ///   ([`authorize_guest_command`](crate::application::authorization::authorize_guest_command)).
    /// - Guests accept snapshots and host updates only from a peer whose
    ///   proven key derives the lobby's host participant, so a peer can claim
    ///   the host role (or a failover candidacy) only as itself.
    /// - Reclaim messages must be signed with the sender's proven key; use the
    ///   same key for signed history.
    pub fn enable_authentication(&mut self, secret: [u8; 32]) -> Result<Uuid> {
        if self.get_lobby().is_some() || self.join_participant.is_some() {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "Enable authentication before creating or joining the lobby".into(),
            ));
        }
        let public = ed25519_dalek::SigningKey::from_bytes(&secret)
            .verifying_key()
            .to_bytes();
        let id = crate::domain::participant_id(&public);
        self.enable_identity_exchange(secret);
        self.authenticated_as = Some(id);
        Ok(id)
    }

    /// The key-derived participant ID of `peer`, once it proved its key.
    fn bound_participant(&self, peer: crate::domain::PeerId) -> Option<Uuid> {
        self.verified_peer_identity(peer)
            .map(|identity| crate::domain::participant_id(identity.public_key()))
    }

    /// Authenticated guests trust `sender` as host only if it is the host
    /// they follow and its key derives the lobby's host participant.
    fn is_trusted_host(&self, sender: crate::domain::PeerId) -> bool {
        self.host_observation
            .as_ref()
            .is_some_and(|observation| observation.peer() == sender)
            && self.bound_participant(sender).is_some()
            && self.bound_participant(sender) == self.get_lobby().map(Lobby::host_id)
    }

    /// Authenticated sessions: a snapshot may name only its sender as host.
    fn snapshot_bound_to_sender(
        &self,
        sender: crate::domain::PeerId,
        snapshot: &serde_json::Value,
    ) -> bool {
        let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot.clone()) else {
            return false;
        };
        let host = snapshot.host();
        host.is_some() && host == self.bound_participant(sender)
    }

    /// A key proven by this connected peer, if the exchange has succeeded.
    pub fn verified_peer_identity(&self, peer: crate::domain::PeerId) -> Option<&VerifiedIdentity> {
        self.identity_exchange.as_ref()?.verified(peer)
    }

    fn send_identity_message(&mut self, peer: crate::domain::PeerId, message: IdentityMessage) {
        if let Ok(payload) = serde_json::to_value(message)
            && let Err(error) = self.transport.send_control_to(peer, payload)
        {
            tracing::debug!(%error, "Unable to send identity message");
        }
    }

    /// Main event loop
    pub fn poll(&mut self) -> usize {
        if !self.is_host {
            self.update_join_connection();
        }
        let mut processed = 0;

        // 1. Handle transport events
        for event in self.transport.drain_events() {
            match event {
                TransportEvent::PeerConnected(peer_id) => {
                    if self.is_host {
                        tracing::info!("🟢 HOST: Peer {} connected - sending snapshot", peer_id);
                        self.send_snapshot_to_peer(peer_id);
                    } else {
                        tracing::info!("🟢 GUEST: Connected to host");
                    }
                }
                TransportEvent::SnapshotRequested { from } => {
                    if self.is_host {
                        tracing::info!("📥 HOST: Snapshot requested by {}", from);
                        self.send_snapshot_to_peer(from);
                    }
                }
                TransportEvent::SnapshotReceived {
                    from,
                    snapshot,
                    as_of_sequence,
                } => {
                    self.receive_snapshot(from, snapshot, as_of_sequence);
                }
            }
            processed += 1;
        }

        // 2. Poll transport for messages
        let messages = self.transport.poll_with_sender();
        let disconnected = self.transport.drain_disconnected_peers();
        if let Some(entry) = &mut self.reconciliation
            && disconnected.contains(&entry.peer)
        {
            let _ = entry.attempt.cancel();
            entry.request_id = None;
            entry.received_history = None;
            entry.received_signed_history = None;
        }
        if self
            .history_offer
            .as_ref()
            .is_some_and(|offer| disconnected.contains(&offer.peer))
        {
            self.history_offer = None;
        }
        if self
            .signed_history_offer
            .as_ref()
            .is_some_and(|offer| disconnected.contains(&offer.peer))
        {
            self.signed_history_offer = None;
        }
        let now = instant::Instant::now();
        let rank = self.failover_rank();
        let mut promote = false;
        if let Some(observation) = &mut self.host_observation {
            for &peer in &disconnected {
                observation.disconnected(peer, now);
            }
            promote = observation.advance(now, rank);
        }
        if promote {
            self.promote_to_host();
        }
        if self
            .announced_to
            .is_some_and(|peer| disconnected.contains(&peer))
        {
            self.announced_to = None;
        }
        if self.is_host {
            for &peer in &disconnected {
                self.participant_left(peer);
            }
        }
        if let Some(exchange) = &mut self.identity_exchange {
            for peer in disconnected {
                exchange.disconnected(peer);
            }
        }
        if !self.is_host {
            self.update_join_connection();
        }

        let peers = self.transport.connected_peers();
        if let Some(exchange) = &mut self.identity_exchange {
            let outgoing = exchange.advance(self.lobby_id, &peers, instant::Instant::now());
            for (peer, message) in outgoing {
                self.send_identity_message(peer, message);
            }
        }

        if !messages.is_empty() {
            tracing::debug!("📥 Received {} messages from transport", messages.len());
        }

        for (sender, payload) in messages {
            processed += 1;
            tracing::trace!(%sender, "Received application message");

            if payload.get("reconciliation_history_v1").is_some() {
                continue;
            }
            if payload.get("reconciliation_history_v2").is_some() {
                if peers.contains(&sender)
                    && let Ok(signed) = serde_json::from_value::<SignedHistory>(payload)
                    && signed.reconciliation_history_v2.len() <= MAX_HISTORY_BYTES
                    && let Some(local) = self.transport.local_peer_id()
                    && let Some(exchange) = &self.identity_exchange
                    && exchange.issued_challenge(sender) == Some(signed.challenge)
                    && let Some(identity) = exchange.verified(sender)
                    && signed.verify(sender, local, identity.public_key())
                    && let Ok(message) =
                        serde_json::from_str::<HistoryMessage>(&signed.reconciliation_history_v2)
                {
                    self.receive_history_message(sender, message);
                }
                continue;
            }

            if payload.get("host_reclaim_v1").is_some() {
                if peers.contains(&sender)
                    && let Some(local) = self.transport.local_peer_id()
                    && let Ok(message) = serde_json::from_value::<ReclaimMessage>(payload)
                    && let Some((author, body)) = message.open(sender, local)
                    && body.lobby_id() == self.lobby_id
                    && (self.authenticated_as.is_none()
                        || self
                            .verified_peer_identity(sender)
                            .is_some_and(|identity| identity.public_key() == &author))
                {
                    self.receive_reclaim(sender, author, body);
                }
                continue;
            }

            if payload.get("signed_event_v1").is_some() {
                if let Some(replica) = &mut self.signed_replica
                    && let Ok(message) = serde_json::from_value::<SignedEventMessage>(payload)
                    && let Err(error) = replica.accept(message.signed_event_v1)
                {
                    tracing::debug!(%error, "Rejected shared signed event");
                }
                continue;
            }

            if let Ok(message) = serde_json::from_value::<IdentityMessage>(payload.clone()) {
                if peers.contains(&sender)
                    && let Some(local) = self.transport.local_peer_id()
                    && let Some(exchange) = &mut self.identity_exchange
                    && let Some(reply) = exchange.receive(
                        self.lobby_id,
                        local,
                        sender,
                        message,
                        instant::Instant::now(),
                    )
                {
                    self.send_identity_message(sender, reply);
                }
                continue;
            }

            if let Ok(cmd) = serde_json::from_value::<DomainCommand>(payload.clone()) {
                tracing::debug!("📥 Processing command: {:?}", std::mem::discriminant(&cmd));

                // Log details for important commands
                match &cmd {
                    DomainCommand::JoinLobby { guest_name, .. } => {
                        tracing::info!("👤 Guest '{}' wants to join", guest_name);
                    }
                    DomainCommand::SubmitResult { result, run_id, .. } => {
                        tracing::info!(
                            "📊 Result from participant {} for run {}",
                            result.participant_id,
                            run_id
                        );
                    }
                    _ => {}
                }

                if self.authenticated_as.is_some() {
                    let allowed = if self.is_host {
                        match self.bound_participant(sender) {
                            Some(actor) => {
                                crate::application::authorization::authorize_guest_command(
                                    &cmd,
                                    actor,
                                    self.lobby_id,
                                    self.get_lobby(),
                                )
                                .map_err(|rejection| rejection.to_string())
                            }
                            None => Err("sender has not proven a key".to_string()),
                        }
                    } else if self.is_trusted_host(sender) {
                        Ok(())
                    } else {
                        Err("sender is not the authenticated host".to_string())
                    };
                    if let Err(reason) = allowed {
                        tracing::warn!(%sender, %reason, "Rejected command");
                        continue;
                    }
                }

                if self.is_host
                    && let DomainCommand::AddParticipant { participant, .. } = &cmd
                {
                    self.link_participant(sender, participant.id());
                }

                let submitted = if self.is_host {
                    let id = Uuid::new_v4();
                    match self.domain.submit_tracked(id, cmd.clone()) {
                        Ok(()) => {
                            self.pending_guest_commands.insert(id, cmd);
                            Ok(())
                        }
                        Err(error) => Err(error),
                    }
                } else {
                    self.domain.submit(cmd)
                };
                if let Err(e) = submitted {
                    tracing::warn!("❌ Failed to submit command to domain: {:?}", e);
                }
            }
        }

        if self.is_host {
            self.expire_restore_deadline(now);
        }

        // Snapshots that arrived before their sender proved its key.
        self.deferred_snapshots
            .retain(|peer, _| peers.contains(peer));
        let ready: Vec<_> = self
            .deferred_snapshots
            .keys()
            .copied()
            .filter(|peer| self.bound_participant(*peer).is_some())
            .collect();
        for peer in ready {
            if let Some((snapshot, as_of_sequence)) = self.deferred_snapshots.remove(&peer) {
                self.receive_snapshot(peer, snapshot, as_of_sequence);
            }
        }

        // 3. Process domain commands
        let domain_processed = self.domain.poll();
        processed += domain_processed;

        if domain_processed > 0 {
            tracing::debug!("🔧 Domain processed {} commands", domain_processed);
        }

        // 4. Broadcast HOST-INITIATED events (not guest commands)
        if self.is_host {
            self.publish_host_events();
        } else {
            // Guests drain events (but don't broadcast)
            self.domain.drain_events();
            self.update_join_admission();
            self.send_join_request();
            self.update_synchronization_gap();
        }

        if processed > 0 {
            self.persist();
        }

        processed
    }

    /// Record, track and broadcast events the host's domain produced.
    fn publish_host_events(&mut self) {
        let mut run_completed = false;
        for (request_id, event) in self.domain.drain_events_with_ids() {
            // Guests cannot always derive completion (e.g. after a submitter
            // was removed), so a completed run is followed by a snapshot.
            run_completed |= matches!(
                event,
                CoreDomainEvent::RunEnded {
                    status: konnekt_session_core::RunStatus::Completed,
                    ..
                }
            );
            self.record_signed_event(&event);
            if let Some(tracked) = request_id.and_then(|id| self.command_outcomes.get_mut(&id)) {
                let outcome = if let CoreDomainEvent::CommandFailed { reason, .. } = &event {
                    tracked.rejection = Some(reason.clone());
                    CommandOutcomeEvent::Rejected
                } else {
                    CommandOutcomeEvent::Accepted
                };
                let _ = tracked.machine.handle(outcome);
            }
            if let Some(command) = request_id.and_then(|id| self.pending_guest_commands.remove(&id))
            {
                if matches!(event, CoreDomainEvent::CommandFailed { .. }) {
                    tracing::debug!("Guest command rejected by domain");
                    continue;
                }
                if !matches!(
                    command,
                    DomainCommand::JoinLobby { .. } | DomainCommand::AddParticipant { .. }
                ) {
                    if let Ok(payload) = serde_json::to_value(&command)
                        && let Err(error) = self.transport.send(payload)
                    {
                        tracing::warn!(%error, "Failed to broadcast accepted guest command");
                    }
                    continue;
                }
            }
            tracing::debug!(
                "📤 HOST: Processing domain event: {:?}",
                std::mem::discriminant(&event)
            );

            // Translate HOST-initiated events → commands for guests
            if let Some(cmd) = self.event_to_command(event) {
                tracing::debug!(
                    "   ↳ Broadcasting host-initiated event as command: {:?}",
                    std::mem::discriminant(&cmd)
                );

                if let Ok(payload) = serde_json::to_value(&cmd) {
                    let _ = self.transport.send(payload);
                }
            }
        }
        if run_completed {
            for peer in self.transport.connected_peers() {
                self.send_snapshot_to_peer(peer);
            }
        }
    }

    /// Send snapshot to a specific peer (HOST ONLY)
    fn send_snapshot_to_peer(&mut self, peer_id: crate::domain::PeerId) {
        if let Some(lobby) = self.get_lobby() {
            let snapshot = self.create_snapshot(lobby);
            if let Ok(snapshot_json) = serde_json::to_value(&snapshot) {
                let _ = self.transport.send_snapshot(peer_id, snapshot_json);
            }
        }
    }

    /// Create snapshot from current lobby state
    fn create_snapshot(&self, lobby: &Lobby) -> LobbySnapshot {
        LobbySnapshot {
            lobby_id: lobby.id(),
            name: lobby.name().to_string(),
            host_id: lobby.host_id(),
            participants: lobby.participants().values().cloned().collect(),
            activity_queue: lobby.activity_queue().to_vec(),
            version: LobbySnapshot::COMPLETE,
            lobby: Some(lobby.clone()),
            runs: self
                .domain
                .event_loop()
                .recent_runs_of(&lobby.id(), self.snapshot_run_limit)
                .into_iter()
                .cloned()
                .collect(),
        }
    }

    /// Validate and apply a host snapshot, or start a reclaim on a host.
    fn receive_snapshot(
        &mut self,
        from: crate::domain::PeerId,
        snapshot: serde_json::Value,
        as_of_sequence: u64,
    ) {
        tracing::info!("📥 GUEST: Received snapshot (seq: {})", as_of_sequence);
        if self.authenticated_as.is_some() && self.bound_participant(from).is_none() {
            // Keep the latest until the sender proves its key.
            self.deferred_snapshots
                .insert(from, (snapshot, as_of_sequence));
            return;
        }
        if self.authenticated_as.is_some() && !self.snapshot_bound_to_sender(from, &snapshot) {
            tracing::debug!(%from, "Ignoring snapshot not bound to its sender's key");
            return;
        }
        if self.is_host {
            self.start_reclaim(from, &snapshot);
            return;
        }
        let elected = self.snapshot_host_is_candidate(&snapshot);
        // Failover guests follow one host: snapshots from another
        // peer count only after an election or a handover.
        // An authenticated host that reconnected keeps its key and role.
        let same_host_key = self.authenticated_as.is_some()
            && self.bound_participant(from).is_some()
            && self.bound_participant(from)
                == self.get_lobby().map(Lobby::host_id).or(self.followed_host);
        if (self.failover.is_some() || self.authenticated_as.is_some())
            && !same_host_key
            && self.host_observation.as_ref().is_some_and(|observation| {
                observation.peer() != from && !(observation.is_lost() && elected)
            })
        {
            tracing::debug!(%from, "Ignoring snapshot from a peer that is not the host");
            return;
        }
        self.rejoin_if_missing(&snapshot);
        if self.apply_snapshot(snapshot) {
            self.followed_host = self.get_lobby().map(Lobby::host_id);
            self.transport.snapshot_accepted(from, as_of_sequence);
            self.announce_to(from);
            if same_host_key && let Some(observation) = &mut self.host_observation {
                observation.handed_over(from);
            }
            if self.transport.connected_peers().contains(&from) {
                let (grace, turn) = (self.host_grace, self.candidate_turn);
                self.host_observation
                    .get_or_insert_with(|| HostObservation::with_turn(from, grace, turn))
                    .snapshot_applied(from, elected);
            }
            if let Some(sync) = &mut self.synchronization {
                let _ = sync.handle(StateSynchronizationEvent::SnapshotApplied);
            }
            if let Some(join) = &mut self.join {
                let event = match join.current_state() {
                    JoinSessionState::Joining => Some(JoinSessionEvent::SnapshotApplied),
                    JoinSessionState::Synchronizing => Some(JoinSessionEvent::Synchronized),
                    _ => None,
                };
                if let Some(event) = event {
                    let _ = join.handle(event);
                }
            }
        }
    }

    /// A host that lost this guest (e.g. it reloaded) sends a snapshot
    /// without it: start over from that snapshot and join again.
    fn rejoin_if_missing(&mut self, snapshot: &serde_json::Value) {
        let Some(me) = self
            .join_participant
            .as_ref()
            .map(|participant| participant.id())
        else {
            return;
        };
        let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot.clone()) else {
            return;
        };
        if snapshot.has_participant(me) || self.join_state() != Some(JoinSessionState::Ready) {
            return;
        }
        tracing::info!("Host no longer lists this guest; joining again");
        self.domain = DomainLoop::new(10, 100);
        if let Some(join) = &mut self.join {
            let _ = join.handle(JoinSessionEvent::Disconnected);
            let _ = join.handle(JoinSessionEvent::Retry);
            let _ = join.handle(JoinSessionEvent::Connected);
        }
        self.join_retry_ticks = 0;
    }

    /// Whether a snapshot names a failover candidate of the current lobby as
    /// its host, i.e. an elected stand-in rather than an arbitrary peer.
    fn snapshot_host_is_candidate(&self, snapshot: &serde_json::Value) -> bool {
        let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot.clone()) else {
            return false;
        };
        let Some(host) = snapshot.host() else {
            return false;
        };
        self.get_lobby()
            .is_some_and(|lobby| failover_candidates(lobby).contains(&host))
    }

    /// Apply received snapshot (GUEST ONLY)
    fn apply_snapshot(&mut self, snapshot_json: serde_json::Value) -> bool {
        if let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot_json) {
            if snapshot.lobby_id != self.lobby_id {
                return false;
            }
            tracing::info!("📥 GUEST: Applying snapshot for lobby '{}'", snapshot.name);
            let Some(host_id) = snapshot.host() else {
                return false;
            };

            // A complete snapshot replaces the lobby and its runs atomically.
            if let Some(lobby) = snapshot.lobby {
                return match self
                    .domain
                    .event_loop_mut()
                    .restore_lobby(lobby, snapshot.runs)
                {
                    Ok(()) => {
                        tracing::info!("✅ GUEST: Complete snapshot restored");
                        true
                    }
                    Err(error) => {
                        tracing::warn!(%error, "Rejected inconsistent snapshot");
                        false
                    }
                };
            }

            // Legacy snapshot: participants and queue only.
            let Some(host_participant) = snapshot
                .participants
                .iter()
                .find(|p| p.id() == host_id)
                .cloned()
            else {
                return false;
            };

            // A restarted host has a new domain; the guest must discard its stale lobby.
            if self
                .get_lobby()
                .is_some_and(|lobby| lobby.host_id() != host_participant.id())
            {
                self.domain = DomainLoop::new(10, 100);
            }

            // Create lobby with host
            if self.get_lobby().is_none() {
                let create_cmd = DomainCommand::CreateLobbyWithHost {
                    lobby_id: snapshot.lobby_id,
                    lobby_name: snapshot.name,
                    host: host_participant,
                };
                if self.domain.submit(create_cmd).is_err() {
                    return false;
                }
                self.domain.poll();
            }

            // Add other participants
            for participant in snapshot.participants.iter() {
                if !participant.is_host() {
                    let add_cmd = DomainCommand::AddParticipant {
                        lobby_id: snapshot.lobby_id,
                        participant: participant.clone(),
                    };
                    let _ = self.domain.submit(add_cmd);
                }
            }

            // Queue entries the guest already has are rejected as duplicates.
            for config in snapshot.activity_queue {
                let _ = self.domain.submit(DomainCommand::QueueActivity {
                    lobby_id: snapshot.lobby_id,
                    config,
                });
            }

            while self.domain.poll() > 0 {}

            tracing::info!("✅ GUEST: Snapshot applied successfully");
            return true;
        }
        false
    }

    /// Translate domain event to command for guests
    fn event_to_command(&self, event: CoreDomainEvent) -> Option<DomainCommand> {
        match event {
            CoreDomainEvent::GuestLeft { participant_id, .. } => Some(DomainCommand::LeaveLobby {
                lobby_id: self.lobby_id,
                participant_id,
            }),
            CoreDomainEvent::GuestJoined { participant, .. } => {
                Some(DomainCommand::AddParticipant {
                    lobby_id: self.lobby_id,
                    participant,
                })
            }
            CoreDomainEvent::RunStarted { run_id, config, .. } => {
                let required_submitters = self
                    .domain
                    .event_loop()
                    .get_run(&run_id)
                    .map(|r| r.required_submitters().iter().cloned().collect())
                    .unwrap_or_default();
                Some(DomainCommand::SyncRunStarted {
                    lobby_id: self.lobby_id,
                    run_id,
                    config,
                    required_submitters,
                })
            }
            CoreDomainEvent::ActivityQueued { config, .. } => Some(DomainCommand::QueueActivity {
                lobby_id: self.lobby_id,
                config,
            }),
            CoreDomainEvent::ResultSubmitted { run_id, result, .. } => {
                Some(DomainCommand::SubmitResult {
                    lobby_id: self.lobby_id,
                    run_id,
                    result,
                })
            }
            CoreDomainEvent::RunEnded {
                run_id,
                status: konnekt_session_core::RunStatus::Cancelled,
                ..
            } => Some(DomainCommand::CancelRun {
                lobby_id: self.lobby_id,
                run_id,
            }),
            CoreDomainEvent::SubmitterRemoved {
                run_id,
                participant_id,
                ..
            } => Some(DomainCommand::RemoveSubmitter {
                lobby_id: self.lobby_id,
                run_id,
                participant_id,
            }),
            // Completion follows from replayed results; the host also sends a
            // snapshot afterwards (see `publish_host_events`).
            CoreDomainEvent::RunEnded { .. } => None,
            _ => None,
        }
    }

    /// Get current lobby
    pub fn get_lobby(&self) -> Option<&Lobby> {
        self.domain.event_loop().get_lobby(&self.lobby_id)
    }

    /// The participant this guest joined as, also after it took over as host.
    pub fn own_participant_id(&self) -> Option<Uuid> {
        self.join_participant
            .as_ref()
            .map(|participant| participant.id())
    }

    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }

    pub fn is_host(&self) -> bool {
        self.is_host
    }

    pub fn connected_peers(&self) -> Vec<crate::domain::PeerId> {
        self.transport.connected_peers()
    }

    pub fn local_peer_id(&self) -> Option<crate::domain::PeerId> {
        self.transport.local_peer_id()
    }

    pub fn get_active_run(&self) -> Option<&konnekt_session_core::ActivityRun> {
        let run_id = self.get_lobby()?.active_run_id()?;
        self.domain.event_loop().get_run(&run_id)
    }

    pub fn get_run(&self, run_id: &Uuid) -> Option<&konnekt_session_core::ActivityRun> {
        self.domain.event_loop().get_run(run_id)
    }
}

// Type alias for production use
pub type MatchboxSessionLoop = SessionLoopV2<crate::infrastructure::connection::MatchboxConnection>;

/// Snapshot of lobby state (for sync)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct LobbySnapshot {
    lobby_id: Uuid,
    name: String,
    host_id: Uuid,
    participants: Vec<konnekt_session_core::Participant>,
    /// Queued activities; older hosts omit it.
    #[serde(default)]
    activity_queue: Vec<konnekt_session_core::ActivityConfig>,
    /// `COMPLETE` when `lobby` and `runs` carry the full state; 0 for
    /// snapshots from older hosts.
    #[serde(default)]
    version: u8,
    /// The complete lobby: participants, roles, queue and active run.
    #[serde(default)]
    lobby: Option<Lobby>,
    /// Every run of the lobby with its results, finished ones included.
    #[serde(default)]
    runs: Vec<konnekt_session_core::ActivityRun>,
}

impl LobbySnapshot {
    const COMPLETE: u8 = 2;

    /// The host participant, or `None` if the snapshot contradicts itself.
    /// Every host check uses this, so the summary fields cannot claim a
    /// different host than the complete lobby.
    fn host(&self) -> Option<Uuid> {
        match &self.lobby {
            Some(lobby) => (self.version == Self::COMPLETE
                && lobby.id() == self.lobby_id
                && lobby.host_id() == self.host_id
                && self
                    .participants
                    .iter()
                    .filter(|p| p.is_host())
                    .all(|p| p.id() == self.host_id))
            .then_some(self.host_id),
            None => {
                let host = self.participants.iter().find(|p| p.is_host())?;
                (host.id() == self.host_id).then_some(self.host_id)
            }
        }
    }

    fn has_participant(&self, id: Uuid) -> bool {
        match &self.lobby {
            Some(lobby) => lobby.participants().contains_key(&id),
            None => self.participants.iter().any(|p| p.id() == id),
        }
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::LobbySnapshot;
    use konnekt_session_core::{Lobby, Participant};

    fn snapshot() -> (LobbySnapshot, uuid::Uuid) {
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let mut lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        let guest = Participant::new_guest("Guest".into()).unwrap_or_else(|e| panic!("{e}"));
        let guest_id = guest.id();
        lobby.add_guest(guest).unwrap_or_else(|e| panic!("{e}"));
        let snapshot = LobbySnapshot {
            lobby_id: lobby.id(),
            name: lobby.name().into(),
            host_id: lobby.host_id(),
            participants: lobby.participants().values().cloned().collect(),
            activity_queue: Vec::new(),
            version: LobbySnapshot::COMPLETE,
            lobby: Some(lobby),
            runs: Vec::new(),
        };
        (snapshot, guest_id)
    }

    #[test]
    fn complete_and_legacy_snapshots_name_one_consistent_host() {
        let (complete, guest) = snapshot();
        assert_eq!(complete.host(), Some(complete.host_id));
        assert!(complete.has_participant(guest));

        let mut legacy = complete.clone();
        legacy.lobby = None;
        legacy.version = 0;
        assert_eq!(legacy.host(), Some(complete.host_id));

        // Summary fields that claim another host than the complete lobby.
        let mut forged = complete.clone();
        forged.host_id = guest;
        assert_eq!(forged.host(), None);
        let mut forged = complete;
        forged.version = 0;
        assert_eq!(forged.host(), None);
    }

    #[test]
    fn snapshots_from_older_hosts_still_parse() {
        let (complete, _) = snapshot();
        let mut json = serde_json::to_value(&complete).unwrap_or_else(|e| panic!("{e}"));
        for field in ["version", "lobby", "runs", "activity_queue"] {
            json.as_object_mut().map(|object| object.remove(field));
        }
        let legacy: LobbySnapshot = serde_json::from_value(json).unwrap_or_else(|e| panic!("{e}"));
        assert!(legacy.lobby.is_none());
        assert_eq!(legacy.host(), Some(complete.host_id));
    }
}
