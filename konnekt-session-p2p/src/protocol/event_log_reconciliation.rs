//! Lifecycle of one attempt to reconcile two session histories.
//!
//! The caller authenticates participants, validates history and authority,
//! finds a common ancestor, selects a history, validates and applies a merge
//! plan, and handles timeouts. Events report those outcomes. This machine
//! neither establishes distributed agreement nor merges or verifies events.
//!
//! History whose ancestry cannot be verified (for example a bounded segment
//! without a trusted checkpoint) is insufficient before any comparison.

use state_machines::state_machine;

state_machine! {
    name: EventLogReconciliation,
    context: (),
    dynamic: true,
    initial: VerifyingAuthority,
    states: [VerifyingAuthority, ComparingHistories, FindingAncestor, SelectingHistory, PreparingMerge, ApplyingMerge, Reconciled, Rejected, InsufficientHistory, Cancelled],
    final_states: [Reconciled, Rejected, InsufficientHistory, Cancelled],
    events {
        authority_verified { transition: { from: VerifyingAuthority, to: ComparingHistories } }
        histories_match { transition: { from: ComparingHistories, to: Reconciled } }
        divergence_detected { transition: { from: ComparingHistories, to: FindingAncestor } }
        ancestor_found { transition: { from: FindingAncestor, to: SelectingHistory } }
        history_unavailable {
            transition: { from: ComparingHistories, to: InsufficientHistory }
            transition: { from: FindingAncestor, to: InsufficientHistory }
        }
        history_selected { transition: { from: SelectingHistory, to: PreparingMerge } }
        merge_plan_validated { transition: { from: PreparingMerge, to: ApplyingMerge } }
        merge_applied { transition: { from: ApplyingMerge, to: Reconciled } }
        rejected {
            transition: { from: VerifyingAuthority, to: Rejected }
            transition: { from: ComparingHistories, to: Rejected }
            transition: { from: FindingAncestor, to: Rejected }
            transition: { from: SelectingHistory, to: Rejected }
            transition: { from: PreparingMerge, to: Rejected }
            transition: { from: ApplyingMerge, to: Rejected }
        }
        cancelled {
            transition: { from: VerifyingAuthority, to: Cancelled }
            transition: { from: ComparingHistories, to: Cancelled }
            transition: { from: FindingAncestor, to: Cancelled }
            transition: { from: SelectingHistory, to: Cancelled }
            transition: { from: PreparingMerge, to: Cancelled }
            transition: { from: ApplyingMerge, to: Cancelled }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DynamicEventLogReconciliation, EventLogReconciliationEvent as Event,
        EventLogReconciliationState as State,
    };

    #[test]
    fn divergent_histories_require_a_validated_merge_before_completion() {
        let mut reconciliation = DynamicEventLogReconciliation::new(());
        assert!(reconciliation.handle(Event::HistoriesMatch).is_err());
        assert!(reconciliation.handle(Event::AuthorityVerified).is_ok());
        assert!(reconciliation.handle(Event::DivergenceDetected).is_ok());
        assert!(reconciliation.handle(Event::MergeApplied).is_err());
        assert!(reconciliation.handle(Event::AncestorFound).is_ok());
        assert!(reconciliation.handle(Event::HistorySelected).is_ok());
        assert!(reconciliation.handle(Event::MergePlanValidated).is_ok());
        assert!(reconciliation.handle(Event::MergeApplied).is_ok());
        assert_eq!(reconciliation.current_state(), State::Reconciled);
        assert!(reconciliation.handle(Event::Rejected).is_err());
    }

    #[test]
    fn missing_history_and_untrusted_authority_cannot_be_reconciled() {
        let mut missing = DynamicEventLogReconciliation::new(());
        assert!(missing.handle(Event::AuthorityVerified).is_ok());
        assert!(missing.handle(Event::DivergenceDetected).is_ok());
        assert!(missing.handle(Event::HistoryUnavailable).is_ok());
        assert!(missing.handle(Event::AncestorFound).is_err());
        let mut untrusted = DynamicEventLogReconciliation::new(());
        assert!(untrusted.handle(Event::Rejected).is_ok());
        assert!(untrusted.handle(Event::AuthorityVerified).is_err());
    }
}
