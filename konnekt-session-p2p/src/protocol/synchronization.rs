//! Guest-side lifecycle for synchronizing with the host.
//!
//! This machine tracks control state only. The caller validates snapshots,
//! applies events in order, and determines whether a gap has been recovered.
//! It does not authenticate the host or verify message contents.

use state_machines::state_machine;

state_machine! {
    name: StateSynchronization,
    context: (),
    dynamic: true,
    initial: AwaitingSnapshot,
    states: [AwaitingSnapshot, Current, Recovering, Disconnected],
    events {
        snapshot_applied {
            transition: { from: AwaitingSnapshot, to: Current }
            transition: { from: Recovering, to: Current }
        }
        gap_detected { transition: { from: Current, to: Recovering } }
        gap_recovered { transition: { from: Recovering, to: Current } }
        snapshot_required { transition: { from: Recovering, to: AwaitingSnapshot } }
        disconnected {
            transition: { from: AwaitingSnapshot, to: Disconnected }
            transition: { from: Current, to: Disconnected }
            transition: { from: Recovering, to: Disconnected }
        }
        reconnected { transition: { from: Disconnected, to: AwaitingSnapshot } }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicStateSynchronization, StateSynchronizationEvent as Event,
        StateSynchronizationState as State,
    };

    #[test]
    fn a_gap_requires_recovery_before_the_guest_is_current() {
        let mut sync = DynamicStateSynchronization::new(());
        assert!(sync.handle(Event::GapRecovered).is_err());
        assert!(sync.handle(Event::SnapshotApplied).is_ok());
        assert!(sync.handle(Event::GapDetected).is_ok());
        assert_eq!(sync.current_state(), State::Recovering);
        assert!(sync.handle(Event::GapRecovered).is_ok());
        assert_eq!(sync.current_state(), State::Current);
    }

    #[test]
    fn snapshot_fallback_and_reconnection_require_a_fresh_snapshot() {
        let mut sync = DynamicStateSynchronization::new(());
        assert!(sync.handle(Event::SnapshotApplied).is_ok());
        assert!(sync.handle(Event::GapDetected).is_ok());
        assert!(sync.handle(Event::SnapshotRequired).is_ok());
        assert!(sync.handle(Event::GapRecovered).is_err());
        assert!(sync.handle(Event::Disconnected).is_ok());
        assert!(sync.handle(Event::Reconnected).is_ok());
        assert_eq!(sync.current_state(), State::AwaitingSnapshot);
        assert!(sync.handle(Event::SnapshotApplied).is_ok());
    }
}
