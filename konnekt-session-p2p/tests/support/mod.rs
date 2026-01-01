pub mod mock_connection;

use konnekt_session_core::DomainLoop;
use konnekt_session_p2p::SessionLoopV2; // ← Import from root
use konnekt_session_p2p::application::ConnectionEvent;
use konnekt_session_p2p::domain::PeerId;
use konnekt_session_p2p::infrastructure::error::{P2PError, Result};
use konnekt_session_p2p::infrastructure::transport::{NetworkConnection, P2PTransport};
use mock_connection::{MockConnection, MockNetwork, create_mock_network};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

// Implement NetworkConnection for MockConnection
impl NetworkConnection for MockConnection {
    fn local_peer_id(&self) -> Option<PeerId> {
        MockConnection::local_peer_id(self)
    }

    fn connected_peers(&self) -> Vec<PeerId> {
        MockConnection::connected_peers(self)
    }

    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        MockConnection::send_to(self, peer, data).map_err(|e| P2PError::SendFailed(e))
    }

    fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        MockConnection::broadcast(self, data).map_err(|e| P2PError::SendFailed(e))
    }

    fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        MockConnection::poll_events(self)
    }
}

/// Test fixture for P2P session
pub struct SessionFixture {
    pub host: SessionLoopV2<MockConnection>,
    pub guests: Vec<SessionLoopV2<MockConnection>>,
    pub lobby_id: Uuid,
    _network: Arc<Mutex<MockNetwork>>,
}

impl SessionFixture {
    /// Create a new test session with host + N guests
    pub fn new(guest_count: usize) -> Self {
        let network = create_mock_network();
        let lobby_id = Uuid::new_v4();

        let host = Self::create_host(network.clone(), lobby_id, "Test Lobby", "Host");

        let mut guests = Vec::new();
        for i in 0..guest_count {
            let guest = Self::create_guest(network.clone(), lobby_id, &format!("Guest{}", i + 1));
            guests.push(guest);
        }

        Self {
            host,
            guests,
            lobby_id,
            _network: network,
        }
    }

    fn create_host(
        network: Arc<Mutex<MockNetwork>>,
        lobby_id: Uuid,
        lobby_name: &str,
        host_name: &str,
    ) -> SessionLoopV2<MockConnection> {
        let mock_conn = MockConnection::new(network);
        let transport = P2PTransport::new_host(mock_conn, 100);

        let mut domain = DomainLoop::new(10, 100);

        let create_cmd = konnekt_session_core::DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: lobby_name.to_string(),
            host_name: host_name.to_string(),
        };

        domain.submit(create_cmd).unwrap();
        domain.poll();
        domain.drain_events();

        SessionLoopV2::new(domain, transport, true, lobby_id)
    }

    fn create_guest(
        network: Arc<Mutex<MockNetwork>>,
        lobby_id: Uuid,
        _guest_name: &str,
    ) -> SessionLoopV2<MockConnection> {
        let mock_conn = MockConnection::new(network);
        let transport = P2PTransport::new_guest(mock_conn, 100);
        let domain = DomainLoop::new(10, 100);

        SessionLoopV2::new(domain, transport, false, lobby_id)
    }

    pub fn poll_all(&mut self) {
        self.host.poll();
        for guest in self.guests.iter_mut() {
            guest.poll();
        }
    }
    /// Poll all peers N times with proper ordering
    pub fn tick(&mut self, count: usize) {
        for i in 0..count {
            // ✅ FIX: Poll in proper order - host first, then guests
            // This ensures host broadcasts are seen by guests in the same tick

            self.host.poll();

            for guest in self.guests.iter_mut() {
                guest.poll();
            }

            if i % 5 == 0 && i > 0 {
                tracing::trace!("🔄 Tick {}/{}", i, count);
            }
        }
    }

    /// Poll until lobby state stabilizes (same participant count for N iterations)
    pub fn poll_until_stable(&mut self, max_iterations: usize) -> usize {
        let mut last_state = self.get_participant_counts();
        let mut stable_count = 0;
        const STABLE_THRESHOLD: usize = 5; // Must be stable for 5 ticks

        for i in 0..max_iterations {
            self.tick(1); // Single tick

            let current_state = self.get_participant_counts();

            if current_state == last_state {
                stable_count += 1;
                if stable_count >= STABLE_THRESHOLD {
                    tracing::info!(
                        "✅ Stabilized after {} iterations (state: {:?})",
                        i + 1,
                        current_state
                    );
                    return i + 1;
                }
            } else {
                stable_count = 0; // Reset if state changed
            }

            last_state = current_state;
        }

        tracing::warn!(
            "⚠️  Did not stabilize after {} iterations (final state: {:?})",
            max_iterations,
            last_state
        );
        max_iterations
    }

    /// Get participant counts for all peers (for stability detection)
    fn get_participant_counts(&self) -> Vec<usize> {
        let mut counts = vec![
            self.host
                .get_lobby()
                .map(|l| l.participants().len())
                .unwrap_or(0),
        ];

        for guest in &self.guests {
            counts.push(
                guest
                    .get_lobby()
                    .map(|l| l.participants().len())
                    .unwrap_or(0),
            );
        }

        counts
    }

    /// Assert all peers have the same participant count
    pub fn assert_consistent_state(&self, expected_count: usize) {
        let host_count = self
            .host
            .get_lobby()
            .expect("Host should have lobby")
            .participants()
            .len();

        assert_eq!(
            host_count, expected_count,
            "Host should see {} participants, but sees {}",
            expected_count, host_count
        );

        for (i, guest) in self.guests.iter().enumerate() {
            let guest_count = guest
                .get_lobby()
                .unwrap_or_else(|| panic!("Guest {} should have lobby", i + 1))
                .participants()
                .len();

            assert_eq!(
                guest_count,
                expected_count,
                "Guest {} should see {} participants, but sees {}",
                i + 1,
                expected_count,
                guest_count
            );
        }
    }

    /// Print current state (for debugging)
    pub fn print_state(&self) {
        println!("\n📊 Current State:");
        if let Some(lobby) = self.host.get_lobby() {
            println!("   Host: {} participants", lobby.participants().len());
            for (id, p) in lobby.participants() {
                println!(
                    "      {} - {} ({})",
                    id,
                    p.name(),
                    if p.is_host() { "host" } else { "guest" }
                );
            }
        } else {
            println!("   Host: No lobby");
        }

        for (i, guest) in self.guests.iter().enumerate() {
            if let Some(lobby) = guest.get_lobby() {
                println!(
                    "   Guest {}: {} participants",
                    i + 1,
                    lobby.participants().len()
                );
                for (id, p) in lobby.participants() {
                    println!(
                        "      {} - {} ({})",
                        id,
                        p.name(),
                        if p.is_host() { "host" } else { "guest" }
                    );
                }
            } else {
                println!("   Guest {}: No lobby", i + 1);
            }
        }
    }
}
