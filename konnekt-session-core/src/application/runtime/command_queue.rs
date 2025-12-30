use crate::application::DomainCommand;
use std::collections::VecDeque;

/// Synchronous command queue (no async, works in any runtime)
#[derive(Debug)]
pub struct CommandQueue {
    queue: VecDeque<DomainCommand>,
    max_size: usize,
}

impl CommandQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push a command (returns error if full)
    pub fn push(&mut self, cmd: DomainCommand) -> Result<(), QueueError> {
        if self.queue.len() >= self.max_size {
            return Err(QueueError::Full { max: self.max_size });
        }
        self.queue.push_back(cmd);
        Ok(())
    }

    /// Pop next command
    pub fn pop(&mut self) -> Option<DomainCommand> {
        self.queue.pop_front()
    }

    /// Drain all commands (for batch processing)
    pub fn drain(&mut self) -> Vec<DomainCommand> {
        self.queue.drain(..).collect()
    }

    pub fn len(&self) -> usize {
        self.queue.len()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    pub fn capacity(&self) -> usize {
        self.max_size
    }
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum QueueError {
    #[error("Queue is full (max size: {max})")]
    Full { max: usize },
}

impl Default for CommandQueue {
    fn default() -> Self {
        Self::new(100) // Default max size
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_push_pop() {
        let mut queue = CommandQueue::new(10);

        let cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
        };

        queue.push(cmd.clone()).unwrap();
        assert_eq!(queue.len(), 1);

        let popped = queue.pop().unwrap();
        assert_eq!(popped, cmd);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_full() {
        let mut queue = CommandQueue::new(2);

        queue
            .push(DomainCommand::CreateLobby {
                lobby_name: "L1".to_string(),
                host_name: "A".to_string(),
            })
            .unwrap();

        queue
            .push(DomainCommand::CreateLobby {
                lobby_name: "L2".to_string(),
                host_name: "B".to_string(),
            })
            .unwrap();

        // Third push should fail
        let result = queue.push(DomainCommand::CreateLobby {
            lobby_name: "L3".to_string(),
            host_name: "C".to_string(),
        });

        assert!(matches!(result, Err(QueueError::Full { max: 2 })));
    }

    #[test]
    fn test_drain() {
        let mut queue = CommandQueue::new(10);

        for i in 0..3 {
            queue
                .push(DomainCommand::CreateLobby {
                    lobby_name: format!("L{}", i),
                    host_name: "Host".to_string(),
                })
                .unwrap();
        }

        let drained = queue.drain();
        assert_eq!(drained.len(), 3);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_fifo_order() {
        let mut queue = CommandQueue::new(10);

        for i in 0..5 {
            queue
                .push(DomainCommand::CreateLobby {
                    lobby_name: format!("L{}", i),
                    host_name: "Host".to_string(),
                })
                .unwrap();
        }

        // Should pop in FIFO order
        for i in 0..5 {
            let cmd = queue.pop().unwrap();
            if let DomainCommand::CreateLobby { lobby_name, .. } = cmd {
                assert_eq!(lobby_name, format!("L{}", i));
            }
        }
    }

    #[test]
    fn test_default() {
        let queue = CommandQueue::default();
        assert_eq!(queue.capacity(), 100);
        assert!(queue.is_empty());
    }
}
