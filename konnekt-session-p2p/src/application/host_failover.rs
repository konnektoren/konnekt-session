use crate::domain::PeerId;
use crate::protocol::host_failover::{DynamicHostFailover, HostFailoverEvent, HostFailoverState};
use instant::Instant;
use std::time::Duration;

pub(crate) const DEFAULT_GRACE: Duration = Duration::from_secs(30);
/// Delay between successive candidates' turns to take over.
pub(crate) const DEFAULT_CANDIDATE_TURN: Duration = Duration::from_secs(10);

/// Deterministic failover order: every active guest of the lost host, oldest join
/// first, ties broken by participant ID. All peers holding the same lobby
/// replica compute the same order without a vote. The candidate at rank `k`
/// takes over after `grace + k * turn` unless a new host appeared earlier, so
/// absent candidates are skipped. Spectators never take over.
pub fn failover_candidates(lobby: &konnekt_session_core::Lobby) -> Vec<uuid::Uuid> {
    let mut guests: Vec<_> = lobby
        .participants()
        .values()
        .filter(|participant| {
            participant.id() != lobby.host_id() && participant.can_submit_results()
        })
        .map(|participant| (participant.joined_at(), participant.id()))
        .collect();
    guests.sort_unstable();
    guests.into_iter().map(|(_, id)| id).collect()
}

/// Observes the transport peer that supplied the first applied snapshot.
/// That peer is not authenticated or bound to a participant.
pub(crate) struct HostObservation {
    peer: PeerId,
    machine: DynamicHostFailover,
    lost_at: Option<Instant>,
    grace: Duration,
    turn: Duration,
}

impl HostObservation {
    #[cfg(test)]
    pub(crate) fn new(peer: PeerId, grace: Duration) -> Self {
        Self::with_turn(peer, grace, DEFAULT_CANDIDATE_TURN)
    }

    pub(crate) fn with_turn(peer: PeerId, grace: Duration, turn: Duration) -> Self {
        Self {
            peer,
            machine: DynamicHostFailover::new(()),
            lost_at: None,
            grace,
            turn,
        }
    }

    pub(crate) fn state(&self) -> HostFailoverState {
        self.machine.current_state()
    }

    pub(crate) fn disconnected(&mut self, peer: PeerId, now: Instant) {
        if peer == self.peer && self.machine.handle(HostFailoverEvent::HostLost).is_ok() {
            self.lost_at = Some(now);
        }
    }

    /// A snapshot from the observed host restores it. While the host is
    /// lost, a snapshot from another peer is accepted only when the caller
    /// checked that its host is a failover candidate (`elected`); that peer
    /// becomes the observed host. Snapshots are not authenticated.
    pub(crate) fn snapshot_applied(&mut self, peer: PeerId, elected: bool) {
        let lost = elected
            && matches!(
                self.state(),
                HostFailoverState::GracePeriod
                    | HostFailoverState::Electing
                    | HostFailoverState::WaitingForCandidate
            );
        if (peer == self.peer || lost)
            && self.machine.handle(HostFailoverEvent::HostRestored).is_ok()
        {
            self.peer = peer;
            self.lost_at = None;
        }
    }

    /// Advance deadlines. `rank` is this client's position in
    /// [`failover_candidates`], or `None` if it does not take over. Returns
    /// true once when this client must take over as host.
    pub(crate) fn advance(&mut self, now: Instant, rank: Option<usize>) -> bool {
        let Some(elapsed) = self
            .lost_at
            .and_then(|lost| now.checked_duration_since(lost))
        else {
            return false;
        };
        if self.state() == HostFailoverState::GracePeriod && elapsed >= self.grace {
            let _ = self.machine.handle(HostFailoverEvent::GraceExpired);
        }
        if self.state() != HostFailoverState::Electing {
            return false;
        }
        let Some(rank) = rank else {
            let _ = self.machine.handle(HostFailoverEvent::NoCandidate);
            return false;
        };
        let turn = self
            .turn
            .checked_mul(u32::try_from(rank).unwrap_or(u32::MAX))
            .and_then(|delay| self.grace.checked_add(delay));
        if turn.is_some_and(|turn| elapsed >= turn) {
            return self
                .machine
                .handle(HostFailoverEvent::CandidateSelected)
                .is_ok();
        }
        false
    }

    pub(crate) fn peer(&self) -> PeerId {
        self.peer
    }

    /// True between losing the host and accepting a new one.
    pub(crate) fn is_lost(&self) -> bool {
        matches!(
            self.state(),
            HostFailoverState::GracePeriod
                | HostFailoverState::Electing
                | HostFailoverState::WaitingForCandidate
        )
    }

    /// The observed host handed its role to `peer`.
    pub(crate) fn handed_over(&mut self, peer: PeerId) {
        self.peer = peer;
        self.lost_at = None;
    }

    /// The local candidate finished taking over.
    pub(crate) fn promoted(&mut self) {
        let _ = self.machine.handle(HostFailoverEvent::Synchronized);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MatchboxPeerId;
    use uuid::Uuid;

    #[test]
    fn only_the_observed_host_loss_starts_grace_and_its_snapshot_restores_it() {
        let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let other = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let now = Instant::now();
        let mut host = HostObservation::new(peer, DEFAULT_GRACE);
        host.disconnected(other, now);
        assert_eq!(host.state(), HostFailoverState::HostAvailable);
        host.disconnected(peer, now);
        assert!(!host.advance(now + DEFAULT_GRACE, None));
        assert_eq!(host.state(), HostFailoverState::WaitingForCandidate);
        host.snapshot_applied(other, false);
        assert_eq!(host.state(), HostFailoverState::WaitingForCandidate);
        host.snapshot_applied(peer, false);
        assert_eq!(host.state(), HostFailoverState::HostAvailable);
    }

    #[test]
    fn candidates_take_over_in_rank_order_unless_a_new_host_appears() {
        let host = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let stand_in = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
        let grace = Duration::from_secs(5);
        let turn = Duration::from_secs(10);
        let now = Instant::now();

        let mut first = HostObservation::with_turn(host, grace, turn);
        first.disconnected(host, now);
        assert!(!first.advance(now + grace - Duration::from_millis(1), Some(0)));
        assert!(first.advance(now + grace, Some(0)));
        assert_eq!(first.state(), HostFailoverState::Synchronizing);
        assert!(!first.advance(now + grace, Some(0)));
        first.promoted();
        assert_eq!(first.state(), HostFailoverState::HostAvailable);

        let mut second = HostObservation::with_turn(host, grace, turn);
        second.disconnected(host, now);
        assert!(!second.advance(now + grace, Some(1)));
        assert_eq!(second.state(), HostFailoverState::Electing);
        second.snapshot_applied(stand_in, true);
        assert_eq!(second.state(), HostFailoverState::HostAvailable);
        // The stand-in is now observed; the old host's snapshot is ignored.
        second.disconnected(host, now);
        assert_eq!(second.state(), HostFailoverState::HostAvailable);

        let mut late = HostObservation::with_turn(host, grace, turn);
        late.disconnected(host, now);
        assert!(late.advance(now + grace + turn, Some(1)));
    }

    #[test]
    fn candidates_are_ordered_by_join_time_and_exclude_the_host() {
        use konnekt_session_core::{Lobby, Participant, Timestamp};
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let host_id = host.id();
        let mut lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        let late = Participant::with_timestamp(
            "Late".into(),
            konnekt_session_core::LobbyRole::Guest,
            Timestamp::from_millis(2_000),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let early = Participant::with_timestamp(
            "Early".into(),
            konnekt_session_core::LobbyRole::Guest,
            Timestamp::from_millis(1_000),
        )
        .unwrap_or_else(|e| panic!("{e}"));
        let (late_id, early_id) = (late.id(), early.id());
        lobby.add_guest(late).unwrap_or_else(|e| panic!("{e}"));
        lobby.add_guest(early).unwrap_or_else(|e| panic!("{e}"));
        let order = failover_candidates(&lobby);
        assert_eq!(order, vec![early_id, late_id]);
        assert!(!order.contains(&host_id));
    }

    #[test]
    fn spectators_are_not_failover_candidates() {
        use konnekt_session_core::{Lobby, Participant, ParticipationMode};
        let host = Participant::new_host("Host".into()).unwrap_or_else(|e| panic!("{e}"));
        let mut lobby = Lobby::new("Room".into(), host).unwrap_or_else(|e| panic!("{e}"));
        let observer = Participant::new_guest("Observer".into())
            .unwrap_or_else(|e| panic!("{e}"))
            .with_participation_mode(ParticipationMode::Spectating);
        let player = Participant::new_guest("Player".into()).unwrap_or_else(|e| panic!("{e}"));
        let player_id = player.id();
        lobby.add_guest(observer).unwrap_or_else(|e| panic!("{e}"));
        lobby.add_guest(player).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(failover_candidates(&lobby), vec![player_id]);
    }
}
