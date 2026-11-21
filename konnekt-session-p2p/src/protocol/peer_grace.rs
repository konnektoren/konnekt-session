//! Connection grace period for one transport peer, not participant recovery.
use state_machines::state_machine;

state_machine! {
    name: PeerGrace,
    context: (),
    dynamic: true,
    initial: Connected,
    states: [Connected, Disconnected, TimedOut],
    final_states: [TimedOut],
    events {
        disconnected { transition: { from: Connected, to: Disconnected } }
        reconnected { transition: { from: Disconnected, to: Connected } }
        grace_expired { transition: { from: Disconnected, to: TimedOut } }
    }
}

#[cfg(test)]
mod tests {
    use super::{DynamicPeerGrace, PeerGraceEvent as Event, PeerGraceState as State};

    #[test]
    fn reconnect_before_timeout_and_reject_late_reconnect() {
        let mut machine = DynamicPeerGrace::new(());
        assert!(machine.handle(Event::GraceExpired).is_err());
        assert!(machine.handle(Event::Disconnected).is_ok());
        assert!(machine.handle(Event::Reconnected).is_ok());
        assert_eq!(machine.current_state(), State::Connected);
        assert!(machine.handle(Event::Disconnected).is_ok());
        assert!(machine.handle(Event::GraceExpired).is_ok());
        assert!(machine.handle(Event::Reconnected).is_err());
        assert_eq!(machine.current_state(), State::TimedOut);
    }
}
