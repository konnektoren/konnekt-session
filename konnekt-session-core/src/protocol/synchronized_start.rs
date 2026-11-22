//! Lifecycle of one start announcement from its sender's perspective.
//!
//! The caller owns the run and announcement IDs, selected start time, required
//! peers, clock conversion, transmission, authentication and timeout policy.
//! Report `acknowledged` only after every required peer has acknowledged this
//! announcement. Acknowledgement does not prove a peer started on time.

use state_machines::state_machine;

state_machine! {
    name: SynchronizedStart,
    context: (),
    dynamic: true,
    initial: Prepared,
    states: [Prepared, AwaitingAcknowledgements, Acknowledged, TimedOut, Cancelled],
    final_states: [Acknowledged, TimedOut, Cancelled],
    events {
        announced { transition: { from: Prepared, to: AwaitingAcknowledgements } }
        acknowledged { transition: { from: AwaitingAcknowledgements, to: Acknowledged } }
        timed_out { transition: { from: AwaitingAcknowledgements, to: TimedOut } }
        cancelled {
            transition: { from: Prepared, to: Cancelled }
            transition: { from: AwaitingAcknowledgements, to: Cancelled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicSynchronizedStart, SynchronizedStartEvent as Event, SynchronizedStartState as State,
    };

    #[test]
    fn acknowledgement_requires_an_announcement_and_is_final() {
        let mut start = DynamicSynchronizedStart::new(());
        assert!(start.handle(Event::Acknowledged).is_err());
        assert!(start.handle(Event::Announced).is_ok());
        assert!(start.handle(Event::Acknowledged).is_ok());
        assert_eq!(start.current_state(), State::Acknowledged);
        assert!(start.handle(Event::TimedOut).is_err());
    }

    #[test]
    fn late_acknowledgements_cannot_reverse_timeout_or_cancellation() {
        for event in [Event::TimedOut, Event::Cancelled] {
            let mut start = DynamicSynchronizedStart::new(());
            assert!(start.handle(Event::Announced).is_ok());
            assert!(start.handle(event).is_ok());
            assert!(start.handle(Event::Acknowledged).is_err());
        }
    }
}
