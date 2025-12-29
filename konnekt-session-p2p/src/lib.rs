// Domain layer (core)
pub mod domain;

// Application layer (use cases)
pub mod application;

// Infrastructure layer (adapters)
pub mod infrastructure;

// Re-exports for convenience
pub use application::{
    ConnectionEvent, EventSyncManager, LobbySnapshot, P2PSession, SessionConfig, SyncError,
    SyncMessage, SyncResponse,
};
pub use domain::{
    DelegationReason, DomainEvent, EventLog, IceServer, LobbyEvent, PeerId, SessionId,
};
pub use infrastructure::error::{P2PError, Result};
