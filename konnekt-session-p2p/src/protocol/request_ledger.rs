//! In-memory deduplication for requests from already authenticated participants.
//!
//! This ledger does not authenticate participants, authorize commands, persist
//! records, or send responses. Callers must establish those boundaries before
//! using a participant ID. It refuses new requests when full rather than
//! evicting records and accidentally executing a retry twice.

use konnekt_session_core::DomainCommand;
use std::collections::HashMap;
use uuid::Uuid;

/// A host's decision after processing a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestResult {
    Accepted,
    Rejected { reason: String },
}

/// Whether a request must be executed or has already been seen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestDisposition {
    New,
    Pending,
    Completed(RequestResult),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RequestLedgerError {
    #[error("request ID was already used for a different command")]
    ConflictingRequest,
    #[error("request ledger is full")]
    Full,
    #[error("request is not pending")]
    NotPending,
}

struct Entry {
    command: DomainCommand,
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
            return Ok(match &entry.result {
                Some(result) => RequestDisposition::Completed(result.clone()),
                None => RequestDisposition::Pending,
            });
        }
        if self.entries.len() >= self.capacity {
            return Err(RequestLedgerError::Full);
        }
        self.entries.insert(
            key,
            Entry {
                command,
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
        if entry.result.is_some() {
            return Err(RequestLedgerError::NotPending);
        }
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
