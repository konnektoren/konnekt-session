use crate::domain::PeerId;
use crate::protocol::host_failover::{DynamicHostFailover, HostFailoverEvent, HostFailoverState};
use instant::Instant;
use std::time::Duration;

pub(crate) const DEFAULT_GRACE: Duration = Duration::from_secs(30);

/// Observes the transport peer that supplied the first applied snapshot.
/// That peer is not authenticated or bound to a participant.
pub(crate) struct HostObservation {
    peer: PeerId,
    machine: DynamicHostFailover,
    lost_at: Option<Instant>,
    grace: Duration,
}

impl HostObservation {
    pub(crate) fn new(peer: PeerId, grace: Duration) -> Self {
        Self {
            peer,
            machine: DynamicHostFailover::new(()),
            lost_at: None,
            grace,
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

    pub(crate) fn snapshot_applied(&mut self, peer: PeerId) {
        if peer == self.peer && self.machine.handle(HostFailoverEvent::HostRestored).is_ok() {
            self.lost_at = None;
        }
    }

    pub(crate) fn advance(&mut self, now: Instant) {
        if self.state() == HostFailoverState::GracePeriod
            && self
                .lost_at
                .and_then(|lost| now.checked_duration_since(lost))
                .is_some_and(|elapsed| elapsed >= self.grace)
        {
            let _ = self.machine.handle(HostFailoverEvent::GraceExpired);
            // V2 has no authenticated candidate or authority transfer yet.
            let _ = self.machine.handle(HostFailoverEvent::NoCandidate);
        }
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
        host.advance(now + DEFAULT_GRACE);
        assert_eq!(host.state(), HostFailoverState::WaitingForCandidate);
        host.snapshot_applied(other);
        assert_eq!(host.state(), HostFailoverState::WaitingForCandidate);
        host.snapshot_applied(peer);
        assert_eq!(host.state(), HostFailoverState::HostAvailable);
    }
}
