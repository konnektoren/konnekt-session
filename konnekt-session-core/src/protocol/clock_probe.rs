//! Lifecycle of one clock probe initiated by a local participant.
//!
//! The caller matches a reply to its outstanding probe, validates timestamps,
//! calculates the sample, and supplies timeout or disconnect events. Each
//! refresh uses a new machine. This machine does not authenticate a peer or
//! establish a synchronized start.

use state_machines::state_machine;

state_machine! {
    name: ClockProbe,
    context: (),
    dynamic: true,
    initial: Prepared,
    states: [Prepared, AwaitingReply, Sampled, TimedOut, Cancelled],
    final_states: [Sampled, TimedOut, Cancelled],
    events {
        sent { transition: { from: Prepared, to: AwaitingReply } }
        sample_validated { transition: { from: AwaitingReply, to: Sampled } }
        timed_out { transition: { from: AwaitingReply, to: TimedOut } }
        disconnected {
            transition: { from: Prepared, to: Cancelled }
            transition: { from: AwaitingReply, to: Cancelled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ClockProbeEvent as Event, ClockProbeState as State, DynamicClockProbe};

    #[test]
    fn only_an_outstanding_probe_can_produce_a_sample() {
        let mut probe = DynamicClockProbe::new(());
        assert!(probe.handle(Event::SampleValidated).is_err());
        assert!(probe.handle(Event::Sent).is_ok());
        assert!(probe.handle(Event::SampleValidated).is_ok());
        assert_eq!(probe.current_state(), State::Sampled);
        assert!(probe.handle(Event::SampleValidated).is_err());
    }

    #[test]
    fn timeout_and_disconnect_make_late_replies_invalid() {
        for event in [Event::TimedOut, Event::Disconnected] {
            let mut probe = DynamicClockProbe::new(());
            assert!(probe.handle(Event::Sent).is_ok());
            assert!(probe.handle(event).is_ok());
            assert!(probe.handle(Event::SampleValidated).is_err());
        }
    }
}
