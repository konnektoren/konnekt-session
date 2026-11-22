use crate::application::event_log_reconciliation::{
    HistoryMessage, MAX_HISTORY_BYTES, ReconciliationAttempt, ReconciliationError, SignedHistory,
    valid_history,
};
use crate::application::host_failover::{DEFAULT_GRACE, HostObservation};
use crate::application::identity_exchange::{IdentityExchange, IdentityMessage};
use crate::domain::VerifiedIdentity;
use crate::infrastructure::error::Result;
use crate::infrastructure::transport::{NetworkConnection, P2PTransport, TransportEvent};
use crate::protocol::command_outcome::{
    CommandOutcomeEvent, CommandOutcomeState, DynamicCommandOutcome,
};
use crate::protocol::event_log_reconciliation::EventLogReconciliationState;
use crate::protocol::host_failover::HostFailoverState;
use crate::protocol::join::{DynamicJoinSession, JoinSessionEvent, JoinSessionState};
use crate::protocol::synchronization::{
    DynamicStateSynchronization, StateSynchronizationEvent, StateSynchronizationState,
};
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, DomainLoop, Lobby};
use std::collections::HashMap;
use uuid::Uuid;

const RECOVERY_RETRY_POLLS: u8 = 50;

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
}

struct PeerReconciliation {
    peer: crate::domain::PeerId,
    attempt: ReconciliationAttempt,
    request_id: Option<Uuid>,
    expected_key: Option<[u8; 32]>,
    received_history: Option<Vec<crate::domain::LobbyEvent>>,
}

struct HistoryOffer {
    peer: crate::domain::PeerId,
    expected_key: [u8; 32],
    events: Vec<crate::domain::LobbyEvent>,
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
        }
    }

    /// Request admission using an ID that remains stable for retries in this session.
    pub fn join(&mut self, name: String) -> Result<Uuid> {
        if self.is_host || self.join_participant.is_some() {
            return Err(crate::infrastructure::error::P2PError::SendFailed(
                "A guest join is already configured or this session is the host".into(),
            ));
        }
        let participant = konnekt_session_core::Participant::new_guest(name).map_err(|error| {
            crate::infrastructure::error::P2PError::SendFailed(error.to_string())
        })?;
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

    pub fn cancel_reconciliation(&mut self) -> std::result::Result<(), ReconciliationError> {
        let entry = self
            .reconciliation
            .as_mut()
            .ok_or(ReconciliationError::InvalidState)?;
        entry.attempt.cancel()?;
        entry.request_id = None;
        entry.received_history = None;
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
        Ok(request_id)
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
            if self.join_participant.is_some() {
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
        self.join_retry_ticks = 49;
    }

    /// Submit a domain command
    pub fn submit_command(&mut self, cmd: DomainCommand) -> Result<()> {
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
        if let Some(entry) = &mut self.reconciliation {
            let _ = entry.attempt.cancel();
            entry.request_id = None;
            entry.expected_key = None;
            entry.received_history = None;
        }
        self.identity_exchange = Some(IdentityExchange::new(secret));
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
                    tracing::info!("📥 GUEST: Received snapshot (seq: {})", as_of_sequence);
                    if !self.is_host && self.apply_snapshot(snapshot) {
                        if self.transport.connected_peers().contains(&from) {
                            self.host_observation
                                .get_or_insert_with(|| HostObservation::new(from, self.host_grace))
                                .snapshot_applied(from);
                        }
                        if let Some(sync) = &mut self.synchronization {
                            let _ = sync.handle(StateSynchronizationEvent::SnapshotApplied);
                        }
                        if let Some(join) = &mut self.join {
                            let event = match join.current_state() {
                                JoinSessionState::Joining => {
                                    Some(JoinSessionEvent::SnapshotApplied)
                                }
                                JoinSessionState::Synchronizing => {
                                    Some(JoinSessionEvent::Synchronized)
                                }
                                _ => None,
                            };
                            if let Some(event) = event {
                                let _ = join.handle(event);
                            }
                        }
                    }
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
        }
        if self
            .history_offer
            .as_ref()
            .is_some_and(|offer| disconnected.contains(&offer.peer))
        {
            self.history_offer = None;
        }
        let now = instant::Instant::now();
        if let Some(observation) = &mut self.host_observation {
            for &peer in &disconnected {
                observation.disconnected(peer, now);
            }
            observation.advance(now);
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

        // 3. Process domain commands
        let domain_processed = self.domain.poll();
        processed += domain_processed;

        if domain_processed > 0 {
            tracing::debug!("🔧 Domain processed {} commands", domain_processed);
        }

        // 4. Broadcast HOST-INITIATED events (not guest commands)
        if self.is_host {
            for (request_id, event) in self.domain.drain_events_with_ids() {
                if let Some(tracked) = request_id.and_then(|id| self.command_outcomes.get_mut(&id))
                {
                    let outcome = if let CoreDomainEvent::CommandFailed { reason, .. } = &event {
                        tracked.rejection = Some(reason.clone());
                        CommandOutcomeEvent::Rejected
                    } else {
                        CommandOutcomeEvent::Accepted
                    };
                    let _ = tracked.machine.handle(outcome);
                }
                if let Some(command) =
                    request_id.and_then(|id| self.pending_guest_commands.remove(&id))
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
        } else {
            // Guests drain events (but don't broadcast)
            self.domain.drain_events();
            self.update_join_admission();
            self.send_join_request();
            self.update_synchronization_gap();
        }

        processed
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
        }
    }

    /// Apply received snapshot (GUEST ONLY)
    fn apply_snapshot(&mut self, snapshot_json: serde_json::Value) -> bool {
        if let Ok(snapshot) = serde_json::from_value::<LobbySnapshot>(snapshot_json) {
            if snapshot.lobby_id != self.lobby_id {
                return false;
            }
            tracing::info!("📥 GUEST: Applying snapshot for lobby '{}'", snapshot.name);

            // Find host participant
            let Some(host_participant) =
                snapshot.participants.iter().find(|p| p.is_host()).cloned()
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

            self.domain.poll();

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
            CoreDomainEvent::RunEnded { .. } => {
                // Guests auto-complete when they process all SubmitResult commands
                None // Guest will auto-complete when they receive all results
            }
            _ => None,
        }
    }

    /// Get current lobby
    pub fn get_lobby(&self) -> Option<&Lobby> {
        self.domain.event_loop().get_lobby(&self.lobby_id)
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
}
