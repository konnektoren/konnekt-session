use uuid::Uuid;

/// SHA-256 digest of a complete signed event envelope.
pub type EventHash = [u8; 32];

/// A previously verified checkpoint supplied by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HistoryAnchor {
    pub lobby_id: Uuid,
    pub sequence: u64,
    pub hash: EventHash,
}

/// A retained signed segment and its sender-provided, untrusted checkpoint.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedHistoryBatch {
    pub anchor: Option<HistoryAnchor>,
    pub events: Vec<SignedEvent>,
}

/// Versioned envelope. `payload` is the exact JSON byte string that was signed;
/// consumers must not reserialize it when checking signatures or hashes.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SignedEvent {
    pub version: u8,
    pub lobby_id: Uuid,
    pub sequence: u64,
    pub previous_hash: Option<EventHash>,
    pub author: [u8; 32],
    pub payload: Vec<u8>,
    pub signature: Vec<u8>,
}
