//! Local observation of host failover and reclaim.
//!
//! The caller detects host loss, owns deadlines, chooses and validates a
//! candidate, proves identity, transfers authority and synchronizes history.
//! Events report those outcomes; this machine does not establish agreement
//! across peers or authorize an election or reclaim.

use state_machines::state_machine;

state_machine! {
    name: HostFailover,
    context: (),
    dynamic: true,
    initial: HostAvailable,
    states: [HostAvailable, GracePeriod, Electing, WaitingForCandidate, Synchronizing, ReclaimPending],
    events {
        host_lost { transition: { from: HostAvailable, to: GracePeriod } }
        host_restored {
            transition: { from: GracePeriod, to: HostAvailable }
            transition: { from: Electing, to: HostAvailable }
            transition: { from: WaitingForCandidate, to: HostAvailable }
        }
        grace_expired { transition: { from: GracePeriod, to: Electing } }
        no_candidate { transition: { from: Electing, to: WaitingForCandidate } }
        retry_election { transition: { from: WaitingForCandidate, to: Electing } }
        candidate_selected { transition: { from: Electing, to: Synchronizing } }
        candidate_lost { transition: { from: Synchronizing, to: WaitingForCandidate } }
        synchronized { transition: { from: Synchronizing, to: HostAvailable } }
        reclaim_requested { transition: { from: HostAvailable, to: ReclaimPending } }
        reclaim_accepted { transition: { from: ReclaimPending, to: Synchronizing } }
        reclaim_rejected { transition: { from: ReclaimPending, to: HostAvailable } }
        host_lost_during_reclaim { transition: { from: ReclaimPending, to: GracePeriod } }
    }
}

#[cfg(test)]
mod tests {
    use super::{DynamicHostFailover, HostFailoverEvent as Event, HostFailoverState as State};

    #[test]
    fn failover_requires_grace_election_and_synchronization() {
        let mut host = DynamicHostFailover::new(());
        assert!(host.handle(Event::CandidateSelected).is_err());
        assert!(host.handle(Event::HostLost).is_ok());
        assert!(host.handle(Event::CandidateSelected).is_err());
        assert!(host.handle(Event::GraceExpired).is_ok());
        assert!(host.handle(Event::CandidateSelected).is_ok());
        assert!(host.handle(Event::Synchronized).is_ok());
        assert_eq!(host.current_state(), State::HostAvailable);
    }

    #[test]
    fn restored_host_and_reclaim_follow_separate_paths() {
        let mut host = DynamicHostFailover::new(());
        assert!(host.handle(Event::HostLost).is_ok());
        assert!(host.handle(Event::HostRestored).is_ok());
        assert!(host.handle(Event::ReclaimRequested).is_ok());
        assert!(host.handle(Event::ReclaimAccepted).is_ok());
        assert!(host.handle(Event::ReclaimAccepted).is_err());
        assert!(host.handle(Event::Synchronized).is_ok());
        assert_eq!(host.current_state(), State::HostAvailable);
    }
}
