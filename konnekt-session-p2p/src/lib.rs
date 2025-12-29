// Domain layer (core)
pub mod domain;

// Application layer (use cases)
pub mod application;

// Infrastructure layer (adapters)
pub mod infrastructure;

// Re-exports for convenience
pub use application::{ConnectionEvent, P2PSession, SessionConfig};
pub use domain::{IceServer, PeerId, SessionId};
pub use infrastructure::error::{P2PError, Result};
