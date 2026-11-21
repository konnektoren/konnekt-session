//! Readiness of one player for the currently selected activity.
//!
//! Activity selection, player membership, and delivery of messages are owned
//! by the caller. Callers must reset each player when the selection changes or
//! a run starts. A local ready state is not proof of a peer's identity.

use state_machines::state_machine;

state_machine! {
    name: PlayerReadiness,
    context: (),
    dynamic: true,
    initial: NotReady,
    states: [NotReady, Ready],
    events {
        ready {
            transition: { from: NotReady, to: Ready }
            transition: { from: Ready, to: Ready }
        }
        not_ready {
            transition: { from: Ready, to: NotReady }
            transition: { from: NotReady, to: NotReady }
        }
        reset {
            transition: { from: Ready, to: NotReady }
            transition: { from: NotReady, to: NotReady }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicPlayerReadiness, PlayerReadinessEvent as Event, PlayerReadinessState as State,
    };

    #[test]
    fn readiness_is_idempotent_and_reset_clears_it() {
        let mut player = DynamicPlayerReadiness::new(());
        assert_eq!(player.current_state(), State::NotReady);
        assert!(player.handle(Event::Ready).is_ok());
        assert!(player.handle(Event::Ready).is_ok());
        assert_eq!(player.current_state(), State::Ready);
        assert!(player.handle(Event::Reset).is_ok());
        assert_eq!(player.current_state(), State::NotReady);
        assert!(player.handle(Event::NotReady).is_ok());
    }
}
