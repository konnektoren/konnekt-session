use thiserror::Error;

#[derive(Error, Debug)]
pub enum NetworkError {
    #[error("Connection refused")]
    ConnectionRefused { address: String },
    #[error("Connection reset by peer")]
    ConnectionReset { address: String },
    #[error("Connection timed out")]
    ConnectionTimeout { address: String },
    #[error("Invalid data received")]
    InvalidData,
    #[error("Unknown network error")]
    UnknownError(String),
    #[error("Failed to send message")]
    SendError,
    #[error("Internal error: {0}")]
    InternalError(String),
    #[error("Connection error: {0}")]
    ConnectionError(String),
}
