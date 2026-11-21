//! Guest connection and admission lifecycle.
//!
//! Admission and initial synchronization can arrive in either order. The
//! guest is ready only after both have succeeded on the current connection.

use state_machines::state_machine;

state_machine! {
    name: JoinSession,
    context: (),
    dynamic: true,
    initial: Connecting,
    states: [Connecting, Joining, Synchronizing, AwaitingAdmission, Ready, Disconnected, Failed],
    events {
        connected { transition: { from: Connecting, to: Joining } }
        admitted { transition: { from: Joining, to: Synchronizing } }
        snapshot_applied { transition: { from: Joining, to: AwaitingAdmission } }
        synchronized { transition: { from: Synchronizing, to: Ready } }
        admission_confirmed { transition: { from: AwaitingAdmission, to: Ready } }
        failed {
            transition: { from: Connecting, to: Failed }
            transition: { from: Joining, to: Failed }
            transition: { from: Synchronizing, to: Failed }
            transition: { from: AwaitingAdmission, to: Failed }
        }
        disconnected {
            transition: { from: Joining, to: Disconnected }
            transition: { from: Synchronizing, to: Disconnected }
            transition: { from: AwaitingAdmission, to: Disconnected }
            transition: { from: Ready, to: Disconnected }
        }
        retry {
            transition: { from: Failed, to: Connecting }
            transition: { from: Disconnected, to: Connecting }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DynamicJoinSession, JoinSessionEvent as Event, JoinSessionState as State};

    #[test]
    fn admission_and_snapshot_can_arrive_in_either_order() {
        for events in [
            [Event::Admitted, Event::Synchronized],
            [Event::SnapshotApplied, Event::AdmissionConfirmed],
        ] {
            let mut join = DynamicJoinSession::new(());
            assert!(join.handle(Event::Connected).is_ok());
            for event in events {
                assert!(join.handle(event).is_ok());
            }
            assert_eq!(join.current_state(), State::Ready);
        }
    }

    #[test]
    fn reconnect_requires_a_fresh_admission_and_snapshot() {
        let mut join = DynamicJoinSession::new(());
        assert!(join.handle(Event::Connected).is_ok());
        assert!(join.handle(Event::SnapshotApplied).is_ok());
        assert!(join.handle(Event::Disconnected).is_ok());
        assert!(join.handle(Event::Retry).is_ok());
        assert!(join.handle(Event::Connected).is_ok());
        assert!(join.handle(Event::AdmissionConfirmed).is_err());
        assert_eq!(join.current_state(), State::Joining);
    }
}
