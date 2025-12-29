use std::fmt;

/// P2P infrastructure errors
#[derive(Debug, thiserror::Error)]
pub enum P2PError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),

    #[error("Invalid session ID: {0}")]
    InvalidSessionId(String),

    #[error("Peer not found: {0}")]
    PeerNotFound(String),

    #[error("Send failed: {0}")]
    SendFailed(String),

    #[error("Receive failed: {0}")]
    ReceiveFailed(String),

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("Channel closed")]
    ChannelClosed,
}

pub type Result<T> = std::result::Result<T, P2PError>;
