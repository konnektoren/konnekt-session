use crate::application::runtime::CommandQueue;
use crate::application::{DomainCommand, DomainEvent, DomainEventLoop};

/// Domain event loop - processes commands in batches
pub struct DomainLoop {
    /// Stateful event loop (owns lobbies)
    event_loop: DomainEventLoop,

    /// Inbound command queue
    inbound: CommandQueue,

    /// Outbound event queue (caller drains this)
    outbound: Vec<DomainEvent>,

    /// Max commands to process per poll
    batch_size: usize,
}

impl DomainLoop {
    /// Create a new domain loop
    ///
    /// # Arguments
    /// * `batch_size` - Max commands to process per `poll()` call
    /// * `max_queue_size` - Max commands that can be queued
    pub fn new(batch_size: usize, max_queue_size: usize) -> Self {
        Self {
            event_loop: DomainEventLoop::new(),
            inbound: CommandQueue::new(max_queue_size),
            outbound: Vec::new(),
            batch_size,
        }
    }

    /// Submit a command (non-blocking)
    ///
    /// Returns error if queue is full (backpressure)
    pub fn submit(
        &mut self,
        cmd: DomainCommand,
    ) -> Result<(), crate::application::runtime::QueueError> {
        self.inbound.push(cmd)
    }

    /// Process up to `batch_size` commands
    ///
    /// Returns number of commands processed
    pub fn poll(&mut self) -> usize {
        let mut processed = 0;

        while processed < self.batch_size {
            match self.inbound.pop() {
                Some(cmd) => {
                    let event = self.event_loop.handle_command(cmd);
                    self.outbound.push(event);
                    processed += 1;
                }
                None => break,
            }
        }

        processed
    }

    /// Drain all emitted events (caller's responsibility to handle)
    pub fn drain_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.outbound)
    }

    /// Get reference to event loop (for queries)
    pub fn event_loop(&self) -> &DomainEventLoop {
        &self.event_loop
    }

    /// Get mutable reference to event loop (for P2P sync)
    pub fn event_loop_mut(&mut self) -> &mut DomainEventLoop {
        &mut self.event_loop
    }

    /// Get pending command count
    pub fn pending_commands(&self) -> usize {
        self.inbound.len()
    }

    /// Get pending event count
    pub fn pending_events(&self) -> usize {
        self.outbound.len()
    }
}

impl Default for DomainLoop {
    fn default() -> Self {
        Self::new(10, 100) // batch_size: 10, max_queue: 100
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_submit_and_poll() {
        let mut loop_ = DomainLoop::new(10, 100);

        // Submit command
        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "Test Lobby".to_string(),
                host_name: "Alice".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        assert_eq!(loop_.pending_commands(), 1);

        // Process it
        let count = loop_.poll();
        assert_eq!(count, 1);
        assert_eq!(loop_.pending_commands(), 0);
        assert_eq!(loop_.pending_events(), 1);

        // Drain events
        let events = loop_.drain_events();
        assert_eq!(events.len(), 1);
        assert_eq!(loop_.pending_events(), 0);

        match &events[0] {
            DomainEvent::LobbyCreated { lobby } => {
                assert_eq!(lobby.name(), "Test Lobby");
            }
            _ => panic!("Expected LobbyCreated"),
        }
    }

    #[test]
    fn test_batch_processing() {
        let mut loop_ = DomainLoop::new(3, 100); // batch_size = 3

        // Submit 5 commands
        for i in 0..5 {
            loop_
                .submit(DomainCommand::CreateLobby {
                    lobby_name: format!("Lobby{}", i),
                    host_name: "Host".to_string(),
                    lobby_id: None, // 🆕 Let the system generate an ID
                })
                .unwrap();
        }

        assert_eq!(loop_.pending_commands(), 5);

        // First poll: process 3
        let count = loop_.poll();
        assert_eq!(count, 3);
        assert_eq!(loop_.pending_commands(), 2);
        assert_eq!(loop_.drain_events().len(), 3);

        // Second poll: process remaining 2
        let count = loop_.poll();
        assert_eq!(count, 2);
        assert_eq!(loop_.pending_commands(), 0);
        assert_eq!(loop_.drain_events().len(), 2);
    }

    #[test]
    fn test_join_lobby() {
        let mut loop_ = DomainLoop::new(10, 100);

        // Create lobby
        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "Test".to_string(),
                host_name: "Alice".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();
        loop_.poll();
        let events = loop_.drain_events();

        let lobby_id = match &events[0] {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        // Join lobby
        loop_
            .submit(DomainCommand::JoinLobby {
                lobby_id,
                guest_name: "Bob".to_string(),
            })
            .unwrap();
        loop_.poll();

        let events = loop_.drain_events();
        match &events[0] {
            DomainEvent::GuestJoined { participant, .. } => {
                assert_eq!(participant.name(), "Bob");
            }
            _ => panic!("Expected GuestJoined"),
        }
    }

    #[test]
    fn test_queue_overflow() {
        let mut loop_ = DomainLoop::new(10, 2); // max_queue_size = 2

        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "L1".to_string(),
                host_name: "A".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "L2".to_string(),
                host_name: "B".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        // Third submit should fail
        let result = loop_.submit(DomainCommand::CreateLobby {
            lobby_name: "L3".to_string(),
            host_name: "C".to_string(),
            lobby_id: None, // 🆕 Let the system generate an ID
        });

        assert!(result.is_err());
    }

    #[test]
    fn test_poll_empty_queue() {
        let mut loop_ = DomainLoop::new(10, 100);

        let count = loop_.poll();
        assert_eq!(count, 0);
        assert!(loop_.drain_events().is_empty());
    }

    #[test]
    fn test_multiple_lobbies() {
        let mut loop_ = DomainLoop::new(10, 100);

        // Create two lobbies
        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "Lobby 1".to_string(),
                host_name: "Alice".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "Lobby 2".to_string(),
                host_name: "Bob".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        loop_.poll();
        let events = loop_.drain_events();

        assert_eq!(events.len(), 2);
        assert_eq!(loop_.event_loop().lobby_count(), 2);
    }

    #[test]
    fn test_default() {
        let loop_ = DomainLoop::default();
        assert_eq!(loop_.pending_commands(), 0);
        assert_eq!(loop_.pending_events(), 0);
    }

    #[test]
    fn test_event_query() {
        let mut loop_ = DomainLoop::new(10, 100);

        loop_
            .submit(DomainCommand::CreateLobby {
                lobby_name: "Test".to_string(),
                host_name: "Alice".to_string(),
                lobby_id: None, // 🆕 Let the system generate an ID
            })
            .unwrap();

        loop_.poll();
        let events = loop_.drain_events();

        let lobby_id = match &events[0] {
            DomainEvent::LobbyCreated { lobby } => lobby.id(),
            _ => panic!("Expected LobbyCreated"),
        };

        // Query via event loop
        let lobby = loop_.event_loop().get_lobby(&lobby_id);
        assert!(lobby.is_some());
        assert_eq!(lobby.unwrap().name(), "Test");
    }
}
