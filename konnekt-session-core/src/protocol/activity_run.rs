//! Lifecycle of an activity after it has been started.
//!
//! The aggregate owns participants and results and reports when completion or
//! cancellation is warranted. Queued activities are not runs yet.

use state_machines::state_machine;

state_machine! {
    name: ActivityRunLifecycle,
    context: (),
    dynamic: true,
    initial: InProgress,
    states: [InProgress, Completed, Cancelled],
    final_states: [Completed, Cancelled],
    events {
        completed { transition: { from: InProgress, to: Completed } }
        cancelled { transition: { from: InProgress, to: Cancelled } }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        ActivityRunLifecycleEvent as Event, ActivityRunLifecycleState as State,
        DynamicActivityRunLifecycle,
    };

    #[test]
    fn completed_and_cancelled_runs_are_terminal() {
        for (event, expected) in [
            (Event::Completed, State::Completed),
            (Event::Cancelled, State::Cancelled),
        ] {
            let mut machine = DynamicActivityRunLifecycle::new(());
            assert_eq!(machine.current_state(), State::InProgress);
            assert!(machine.handle(event).is_ok());
            assert_eq!(machine.current_state(), expected);
            assert!(machine.handle(Event::Completed).is_err());
            assert!(machine.handle(Event::Cancelled).is_err());
        }
    }
}
