//! Lifecycle of one command request from the caller's perspective.
//!
//! Sending a request is not acceptance. A timeout or disconnect leaves the
//! outcome unknown: the host may already have applied the command. Retrying
//! is safe only when the caller preserves the request ID and the host
//! deduplicates requests. This machine does not implement those guarantees,
//! authenticate replies, or execute commands.

use state_machines::state_machine;

state_machine! {
    name: CommandOutcome,
    context: (),
    dynamic: true,
    initial: Prepared,
    states: [Prepared, Pending, OutcomeUnknown, Accepted, Rejected, Abandoned],
    final_states: [Accepted, Rejected, Abandoned],
    events {
        sent { transition: { from: Prepared, to: Pending } }
        response_lost { transition: { from: Pending, to: OutcomeUnknown } }
        retried { transition: { from: OutcomeUnknown, to: Pending } }
        accepted {
            transition: { from: Pending, to: Accepted }
            transition: { from: OutcomeUnknown, to: Accepted }
        }
        rejected {
            transition: { from: Pending, to: Rejected }
            transition: { from: OutcomeUnknown, to: Rejected }
        }
        abandoned {
            transition: { from: Prepared, to: Abandoned }
            transition: { from: Pending, to: Abandoned }
            transition: { from: OutcomeUnknown, to: Abandoned }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandOutcomeEvent as Event, CommandOutcomeState as State, DynamicCommandOutcome,
    };

    #[test]
    fn sending_is_not_acceptance_and_terminal_outcomes_cannot_change() {
        for (response, expected) in [
            (Event::Accepted, State::Accepted),
            (Event::Rejected, State::Rejected),
        ] {
            let mut request = DynamicCommandOutcome::new(());
            assert!(request.handle(Event::Accepted).is_err());
            assert!(request.handle(Event::Sent).is_ok());
            assert_eq!(request.current_state(), State::Pending);
            assert!(request.handle(response).is_ok());
            assert_eq!(request.current_state(), expected);
            assert!(request.handle(Event::Retried).is_err());
        }
    }

    #[test]
    fn a_late_reply_can_resolve_an_unknown_outcome() {
        let mut request = DynamicCommandOutcome::new(());
        assert!(request.handle(Event::Sent).is_ok());
        assert!(request.handle(Event::ResponseLost).is_ok());
        assert_eq!(request.current_state(), State::OutcomeUnknown);
        assert!(request.handle(Event::Accepted).is_ok());
        assert_eq!(request.current_state(), State::Accepted);
    }

    #[test]
    fn retry_and_abandon_do_not_claim_host_cancellation() {
        let mut request = DynamicCommandOutcome::new(());
        assert!(request.handle(Event::Sent).is_ok());
        assert!(request.handle(Event::ResponseLost).is_ok());
        assert!(request.handle(Event::Retried).is_ok());
        assert!(request.handle(Event::Abandoned).is_ok());
        assert_eq!(request.current_state(), State::Abandoned);
        assert!(request.handle(Event::Rejected).is_err());
    }
}
