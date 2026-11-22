//! Lifecycle of one asset transfer between a sender and a receiver.
//!
//! The caller owns transfer identity, peer authentication, chunk validation,
//! reassembly, retries, deadlines and transport. Report `chunks_complete`
//! after the receiver has validated all chunks (or the sender has sent them),
//! and `acknowledged` only when the receiver sends its completion confirmation
//! or the sender receives that matching confirmation. A new transfer needs a new
//! machine; acknowledgement alone does not guarantee durable storage.

use state_machines::state_machine;

state_machine! {
    name: AssetTransfer,
    context: (),
    dynamic: true,
    initial: Prepared,
    states: [Prepared, Transferring, AwaitingAcknowledgement, Completed, TimedOut, Cancelled],
    final_states: [Completed, TimedOut, Cancelled],
    events {
        started { transition: { from: Prepared, to: Transferring } }
        chunks_complete { transition: { from: Transferring, to: AwaitingAcknowledgement } }
        acknowledged { transition: { from: AwaitingAcknowledgement, to: Completed } }
        timed_out {
            transition: { from: Transferring, to: TimedOut }
            transition: { from: AwaitingAcknowledgement, to: TimedOut }
        }
        cancelled {
            transition: { from: Prepared, to: Cancelled }
            transition: { from: Transferring, to: Cancelled }
            transition: { from: AwaitingAcknowledgement, to: Cancelled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetTransferEvent as Event, AssetTransferState as State, DynamicAssetTransfer};

    #[test]
    fn completion_requires_chunks_and_acknowledgement() {
        let mut transfer = DynamicAssetTransfer::new(());
        assert!(transfer.handle(Event::Acknowledged).is_err());
        assert!(transfer.handle(Event::Started).is_ok());
        assert!(transfer.handle(Event::Acknowledged).is_err());
        assert!(transfer.handle(Event::ChunksComplete).is_ok());
        assert!(transfer.handle(Event::Acknowledged).is_ok());
        assert_eq!(transfer.current_state(), State::Completed);
        assert!(transfer.handle(Event::TimedOut).is_err());
    }

    #[test]
    fn timeout_and_cancellation_are_final() {
        for event in [Event::TimedOut, Event::Cancelled] {
            let mut transfer = DynamicAssetTransfer::new(());
            assert!(transfer.handle(Event::Started).is_ok());
            assert!(transfer.handle(event).is_ok());
            assert!(transfer.handle(Event::ChunksComplete).is_err());
            assert!(transfer.handle(Event::Acknowledged).is_err());
        }
    }
}
