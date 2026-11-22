//! Bounded in-memory asset transfer helpers. Transport and peer authentication
//! belong to the caller, as do retries, deadlines and asset-specific validation.

use crate::protocol::asset_transfer::{
    AssetTransferEvent as Event, AssetTransferState as State, DynamicAssetTransfer,
};

/// Resource limits for one transfer. Both values must be nonzero.
#[derive(Debug, Clone, Copy)]
pub struct TransferLimits {
    pub max_bytes: usize,
    pub chunk_bytes: usize,
}

impl Default for TransferLimits {
    fn default() -> Self {
        Self {
            max_bytes: 10 * 1024 * 1024,
            chunk_bytes: 4096,
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TransferError {
    #[error("invalid transfer limits or asset size")]
    InvalidSize,
    #[error("invalid chunk index or size")]
    InvalidChunk,
    #[error("chunk conflicts with an earlier chunk")]
    ConflictingChunk,
    #[error("transfer is not in the required state")]
    InvalidState,
}

fn started() -> DynamicAssetTransfer {
    let mut machine = DynamicAssetTransfer::new(());
    let _ = machine.handle(Event::Started);
    machine
}

/// A single outgoing asset, split into bounded chunks for the caller to send.
pub struct AssetSender {
    machine: DynamicAssetTransfer,
    bytes: Vec<u8>,
    chunk_bytes: usize,
}

impl AssetSender {
    pub fn new(bytes: Vec<u8>, limits: TransferLimits) -> Result<Self, TransferError> {
        if limits.chunk_bytes == 0 || bytes.is_empty() || bytes.len() > limits.max_bytes {
            return Err(TransferError::InvalidSize);
        }
        Ok(Self {
            machine: started(),
            bytes,
            chunk_bytes: limits.chunk_bytes,
        })
    }

    pub fn chunks(&self) -> impl ExactSizeIterator<Item = &[u8]> {
        self.bytes.chunks(self.chunk_bytes)
    }

    pub fn state(&self) -> State {
        self.machine.current_state()
    }

    /// Call only after sending every chunk or receiving a matching completion confirmation.
    pub fn chunks_complete(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::ChunksComplete)
            .map_err(|_| TransferError::InvalidState)
    }

    /// Call after authenticating and matching the receiver's confirmation.
    pub fn acknowledge(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::Acknowledged)
            .map_err(|_| TransferError::InvalidState)
    }

    pub fn cancel(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::Cancelled)
            .map_err(|_| TransferError::InvalidState)
    }

    pub fn timeout(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::TimedOut)
            .map_err(|_| TransferError::InvalidState)
    }
}

/// Reassembles a single asset in memory, accepting identical duplicate chunks.
pub struct AssetReceiver {
    machine: DynamicAssetTransfer,
    limits: TransferLimits,
    chunks: Vec<Option<Vec<u8>>>,
    received: usize,
    size: usize,
}

impl AssetReceiver {
    pub fn new(total: usize, limits: TransferLimits) -> Result<Self, TransferError> {
        if limits.chunk_bytes == 0
            || limits.max_bytes == 0
            || total == 0
            || total > limits.max_bytes.div_ceil(limits.chunk_bytes)
        {
            return Err(TransferError::InvalidSize);
        }
        Ok(Self {
            machine: started(),
            limits,
            chunks: vec![None; total],
            received: 0,
            size: 0,
        })
    }

    pub fn state(&self) -> State {
        self.machine.current_state()
    }
    pub fn received(&self) -> usize {
        self.received
    }
    pub fn total(&self) -> usize {
        self.chunks.len()
    }

    /// Returns whether the complete asset is ready for application validation.
    pub fn receive(&mut self, index: usize, bytes: Vec<u8>) -> Result<bool, TransferError> {
        if self.state() != State::Transferring {
            return Err(TransferError::InvalidState);
        }
        if index >= self.chunks.len() || bytes.is_empty() || bytes.len() > self.limits.chunk_bytes {
            return Err(TransferError::InvalidChunk);
        }
        if let Some(existing) = &self.chunks[index] {
            return if *existing == bytes {
                Ok(false)
            } else {
                Err(TransferError::ConflictingChunk)
            };
        }
        let size = self
            .size
            .checked_add(bytes.len())
            .ok_or(TransferError::InvalidSize)?;
        if size > self.limits.max_bytes {
            return Err(TransferError::InvalidSize);
        }
        self.size = size;
        self.chunks[index] = Some(bytes);
        self.received += 1;
        if self.received == self.chunks.len() {
            self.machine
                .handle(Event::ChunksComplete)
                .map_err(|_| TransferError::InvalidState)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Returns the assembled bytes after all chunks arrive.
    pub fn asset(&self) -> Option<Vec<u8>> {
        (self.received == self.chunks.len())
            .then(|| self.chunks.iter().flatten().flatten().copied().collect())
    }

    /// Call after validating the asset and sending a completion confirmation.
    pub fn acknowledge(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::Acknowledged)
            .map_err(|_| TransferError::InvalidState)
    }

    pub fn cancel(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::Cancelled)
            .map_err(|_| TransferError::InvalidState)
    }

    pub fn timeout(&mut self) -> Result<(), TransferError> {
        self.machine
            .handle(Event::TimedOut)
            .map_err(|_| TransferError::InvalidState)
    }
}

#[cfg(test)]
mod tests {
    use super::{AssetReceiver, AssetSender, TransferError, TransferLimits};

    #[test]
    fn limits_and_conflicting_chunks_are_enforced() {
        let limits = TransferLimits {
            max_bytes: 5,
            chunk_bytes: 3,
        };
        assert!(matches!(
            AssetSender::new(vec![0; 6], limits),
            Err(TransferError::InvalidSize)
        ));
        assert!(matches!(
            AssetReceiver::new(3, limits),
            Err(TransferError::InvalidSize)
        ));
        let mut receiver = AssetReceiver::new(2, limits).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(receiver.receive(1, vec![1, 2, 3]), Ok(false));
        assert_eq!(
            receiver.receive(1, vec![9]),
            Err(TransferError::ConflictingChunk)
        );
        assert_eq!(
            receiver.receive(0, vec![1, 2, 3]),
            Err(TransferError::InvalidSize)
        );
        assert_eq!(receiver.receive(0, vec![4, 5]), Ok(true));
        assert_eq!(receiver.asset(), Some(vec![4, 5, 1, 2, 3]));
    }
}
