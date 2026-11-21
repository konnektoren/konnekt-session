//! Host-side lifecycle for a single identity challenge.
//!
//! The caller issues a fresh challenge and verifies its response, including
//! its session and peer binding, before reporting the outcome. A machine is
//! scoped to one challenge and cannot be reused after reaching a final state.

use state_machines::state_machine;

state_machine! {
    name: IdentityVerification,
    context: (),
    dynamic: true,
    initial: AwaitingChallenge,
    states: [AwaitingChallenge, AwaitingProof, Verified, Rejected, Expired, Cancelled],
    final_states: [Verified, Rejected, Expired, Cancelled],
    events {
        challenge_issued { transition: { from: AwaitingChallenge, to: AwaitingProof } }
        proof_verified { transition: { from: AwaitingProof, to: Verified } }
        proof_rejected { transition: { from: AwaitingProof, to: Rejected } }
        timed_out { transition: { from: AwaitingProof, to: Expired } }
        disconnected {
            transition: { from: AwaitingChallenge, to: Cancelled }
            transition: { from: AwaitingProof, to: Cancelled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicIdentityVerification, IdentityVerificationEvent as Event,
        IdentityVerificationState as State,
    };

    #[test]
    fn verification_requires_an_issued_challenge() {
        let mut machine = DynamicIdentityVerification::new(());
        assert!(machine.handle(Event::ProofVerified).is_err());
        assert!(machine.handle(Event::ChallengeIssued).is_ok());
        assert!(machine.handle(Event::ProofVerified).is_ok());
        assert_eq!(machine.current_state(), State::Verified);
        assert!(machine.handle(Event::ProofVerified).is_err());
    }

    #[test]
    fn expired_challenges_cannot_be_verified() {
        let mut machine = DynamicIdentityVerification::new(());
        assert!(machine.handle(Event::ChallengeIssued).is_ok());
        assert!(machine.handle(Event::TimedOut).is_ok());
        assert!(machine.handle(Event::ProofVerified).is_err());
        assert_eq!(machine.current_state(), State::Expired);
    }
}
