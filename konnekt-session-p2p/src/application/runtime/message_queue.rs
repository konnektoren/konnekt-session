use crate::domain::LobbyEvent;
use std::collections::VecDeque;

/// Synchronous message queue for P2P events
#[derive(Debug)]
pub struct MessageQueue {
    queue: VecDeque<LobbyEvent>,
    max_size: usize,
}

impl MessageQueue {
    pub fn new(max_size: usize) -> Self {
        Self {
            queue: VecDeque::with_capacity(max_size),
            max_size,
        }
    }

    /// Push a message (returns error if full)
    pub fn push(&mut self, msg: LobbyEvent) -> Result<(), QueueError> {
        if self.queue.len() >= self.max_size {
            return Err(QueueError::Full { max: self.max_size });
        }
        self.queue.push_back(msg);
        Ok(())
    }

    /// Pop next message
    pub fn pop(&mut self) -> Option<LobbyEvent> {
        self.queue.pop_front()
    }

    /// Drain all messages (for batch processing)
    pub fn drain(&mut self) -> Vec<LobbyEvent> {
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

impl Default for MessageQueue {
    fn default() -> Self {
        Self::new(100)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::DomainEvent;
    use uuid::Uuid;

    fn create_test_event() -> LobbyEvent {
        let lobby_id = Uuid::new_v4();
        LobbyEvent::new(
            1,
            lobby_id,
            DomainEvent::LobbyCreated {
                lobby_id,
                host_id: Uuid::new_v4(),
                name: "Test".to_string(),
            },
        )
    }

    #[test]
    fn test_push_pop() {
        let mut queue = MessageQueue::new(10);
        let event = create_test_event();

        queue.push(event.clone()).unwrap();
        assert_eq!(queue.len(), 1);

        let popped = queue.pop().unwrap();
        assert_eq!(popped.sequence, event.sequence);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_queue_full() {
        let mut queue = MessageQueue::new(2);

        queue.push(create_test_event()).unwrap();
        queue.push(create_test_event()).unwrap();

        let result = queue.push(create_test_event());
        assert_eq!(result, Err(QueueError::Full { max: 2 }));
    }

    #[test]
    fn test_drain() {
        let mut queue = MessageQueue::new(10);

        for _ in 0..3 {
            queue.push(create_test_event()).unwrap();
        }

        let drained = queue.drain();
        assert_eq!(drained.len(), 3);
        assert!(queue.is_empty());
    }

    #[test]
    fn test_fifo_order() {
        let mut queue = MessageQueue::new(10);
        let lobby_id = Uuid::new_v4();

        for seq in 1..=5 {
            queue
                .push(LobbyEvent::new(
                    seq,
                    lobby_id,
                    DomainEvent::GuestLeft {
                        participant_id: Uuid::new_v4(),
                    },
                ))
                .unwrap();
        }

        for seq in 1..=5 {
            let event = queue.pop().unwrap();
            assert_eq!(event.sequence, seq);
        }
    }

    #[test]
    fn test_default() {
        let queue = MessageQueue::default();
        assert_eq!(queue.capacity(), 100);
        assert!(queue.is_empty());
    }
}
