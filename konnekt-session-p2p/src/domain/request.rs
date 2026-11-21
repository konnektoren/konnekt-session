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
