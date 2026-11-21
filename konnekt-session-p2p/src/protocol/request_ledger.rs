//! Host-side request execution state machine.

use state_machines::state_machine;

pub use crate::application::RequestLedger;
pub use crate::domain::{RequestDisposition, RequestLedgerError, RequestResult};

state_machine! {
    name: RequestExecution,
    context: (),
    dynamic: true,
    initial: Pending,
    states: [Pending, Accepted, Rejected],
    final_states: [Accepted, Rejected],
    events {
        accepted { transition: { from: Pending, to: Accepted } }
        rejected { transition: { from: Pending, to: Rejected } }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicRequestExecution, RequestExecutionEvent as Event, RequestExecutionState as State,
    };

    #[test]
    fn decisions_are_terminal() {
        for (event, state) in [
            (Event::Accepted, State::Accepted),
            (Event::Rejected, State::Rejected),
        ] {
            let mut machine = DynamicRequestExecution::new(());
            assert_eq!(machine.current_state(), State::Pending);
            assert!(machine.handle(event).is_ok());
            assert_eq!(machine.current_state(), state);
            assert!(machine.handle(Event::Accepted).is_err());
            assert!(machine.handle(Event::Rejected).is_err());
        }
    }
}
