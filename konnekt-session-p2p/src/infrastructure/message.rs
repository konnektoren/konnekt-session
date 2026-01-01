use serde::{Deserialize, Serialize};

/// Generic P2P message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2PMessage {
    /// Sequence number for ordering
    pub sequence: u64,

    /// Message type discriminator
    #[serde(flatten)]
    pub kind: MessageKind,
}

/// Message types (control + application)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum MessageKind {
    /// Application payload (opaque to P2P layer)
    #[serde(rename = "app")]
    Application { payload: serde_json::Value },

    /// Request full snapshot (guest → host)
    #[serde(rename = "snapshot_req")]
    SnapshotRequest,

    /// Full snapshot response (host → guest)
    #[serde(rename = "snapshot_resp")]
    SnapshotResponse {
        /// Opaque snapshot payload
        snapshot: serde_json::Value,
        /// Sequence number this snapshot represents
        as_of_sequence: u64,
    },

    /// Request resend of missing messages
    #[serde(rename = "resend_req")]
    ResendRequest { from: u64, to: u64 },

    /// Response with missing messages
    #[serde(rename = "resend_resp")]
    ResendResponse { messages: Vec<P2PMessage> },
}

impl P2PMessage {
    /// Create an application message (no sequence yet)
    pub fn application(payload: serde_json::Value) -> Self {
        Self {
            sequence: 0, // Will be assigned by transport
            kind: MessageKind::Application { payload },
        }
    }

    /// Create a snapshot request
    pub fn snapshot_request() -> Self {
        Self {
            sequence: 0,
            kind: MessageKind::SnapshotRequest,
        }
    }

    /// Create a snapshot response
    pub fn snapshot_response(snapshot: serde_json::Value, as_of_sequence: u64) -> Self {
        Self {
            sequence: 0,
            kind: MessageKind::SnapshotResponse {
                snapshot,
                as_of_sequence,
            },
        }
    }

    /// Create a resend request
    pub fn resend_request(from: u64, to: u64) -> Self {
        Self {
            sequence: 0,
            kind: MessageKind::ResendRequest { from, to },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_serialization() {
        let msg = P2PMessage::application(serde_json::json!({
            "command": "JoinLobby",
            "guest_name": "Alice"
        }));

        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: P2PMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sequence, 0);
        assert!(matches!(deserialized.kind, MessageKind::Application { .. }));
    }

    #[test]
    fn test_snapshot_request() {
        let msg = P2PMessage::snapshot_request();
        assert!(matches!(msg.kind, MessageKind::SnapshotRequest));
    }
}
