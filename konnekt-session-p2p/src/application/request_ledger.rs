//! Bounded host-side request deduplication and execution outcomes.

use crate::domain::{RequestDisposition, RequestLedgerError, RequestResult};
use crate::protocol::request_ledger::{
    DynamicRequestExecution, RequestExecutionEvent, RequestExecutionState,
};
use konnekt_session_core::DomainCommand;
use std::collections::HashMap;
use uuid::Uuid;

struct Entry {
    command: DomainCommand,
    machine: DynamicRequestExecution,
    result: Option<RequestResult>,
}

/// A bounded ledger; request IDs are scoped to the authenticated participant.
pub struct RequestLedger {
    entries: HashMap<(Uuid, Uuid), Entry>,
    capacity: usize,
}

impl RequestLedger {
    pub fn new(capacity: usize) -> Self {
        Self {
            entries: HashMap::new(),
            capacity,
        }
    }

    /// Reserve a new request or look up an exact retry from the same participant.
    pub fn begin(
        &mut self,
        participant: Uuid,
        request: Uuid,
        command: DomainCommand,
    ) -> Result<RequestDisposition, RequestLedgerError> {
        let key = (participant, request);
        if let Some(entry) = self.entries.get(&key) {
            if entry.command != command {
                return Err(RequestLedgerError::ConflictingRequest);
            }
            return Ok(match entry.machine.current_state() {
                RequestExecutionState::Pending => RequestDisposition::Pending,
                RequestExecutionState::Accepted => {
                    RequestDisposition::Completed(RequestResult::Accepted)
                }
                RequestExecutionState::Rejected => RequestDisposition::Completed(
                    entry.result.clone().ok_or(RequestLedgerError::NotPending)?,
                ),
            });
        }
        if self.entries.len() >= self.capacity {
            return Err(RequestLedgerError::Full);
        }
        self.entries.insert(
            key,
            Entry {
                command,
                machine: DynamicRequestExecution::new(()),
                result: None,
            },
        );
        Ok(RequestDisposition::New)
    }

    /// Record an execution decision once; subsequent completion is rejected.
    pub fn complete(
        &mut self,
        participant: Uuid,
        request: Uuid,
        result: RequestResult,
    ) -> Result<(), RequestLedgerError> {
        let entry = self
            .entries
            .get_mut(&(participant, request))
            .ok_or(RequestLedgerError::NotPending)?;
        let event = match &result {
            RequestResult::Accepted => RequestExecutionEvent::Accepted,
            RequestResult::Rejected { .. } => RequestExecutionEvent::Rejected,
        };
        entry
            .machine
            .handle(event)
            .map_err(|_| RequestLedgerError::NotPending)?;
        entry.result = Some(result);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_return_the_original_result_and_conflicts_are_rejected() {
        let mut ledger = RequestLedger::new(1);
        let participant = Uuid::new_v4();
        let id = Uuid::new_v4();
        let command = DomainCommand::StartNextRun {
            lobby_id: Uuid::new_v4(),
        };
        assert_eq!(
            ledger.begin(participant, id, command.clone()),
            Ok(RequestDisposition::New)
        );
        assert_eq!(
            ledger.begin(participant, id, command.clone()),
            Ok(RequestDisposition::Pending)
        );
        let result = RequestResult::Rejected {
            reason: "no activity".into(),
        };
        assert_eq!(ledger.complete(participant, id, result.clone()), Ok(()));
        assert_eq!(
            ledger.begin(participant, id, command.clone()),
            Ok(RequestDisposition::Completed(result))
        );
        assert_eq!(
            ledger.complete(participant, id, RequestResult::Accepted),
            Err(RequestLedgerError::NotPending)
        );
        assert_eq!(
            ledger.begin(
                participant,
                id,
                DomainCommand::StartNextRun {
                    lobby_id: Uuid::new_v4()
                }
            ),
            Err(RequestLedgerError::ConflictingRequest)
        );
        assert_eq!(
            ledger.begin(Uuid::new_v4(), id, command),
            Err(RequestLedgerError::Full)
        );
    }

    #[test]
    fn request_ids_are_scoped_to_participants() {
        let mut ledger = RequestLedger::new(2);
        let first = Uuid::new_v4();
        let second = Uuid::new_v4();
        let id = Uuid::new_v4();
        let command = DomainCommand::StartNextRun {
            lobby_id: Uuid::new_v4(),
        };
        assert_eq!(
            ledger.begin(first, id, command.clone()),
            Ok(RequestDisposition::New)
        );
        assert_eq!(ledger.complete(first, id, RequestResult::Accepted), Ok(()));
        assert_eq!(
            ledger.begin(second, id, command),
            Ok(RequestDisposition::New)
        );
    }
}
