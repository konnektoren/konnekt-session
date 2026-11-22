use serde::{Deserialize, Serialize};

use super::Offer;

/// Messages sent over an authenticated discovery connection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Message {
    Offer(Offer),
    Accept(Offer),
    Commit(Offer),
    CommitAck(Offer),
    Busy(Offer),
    Cancel(Offer),
}
