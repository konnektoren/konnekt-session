use crate::application::ConnectionEvent;
use crate::domain::PeerId;
use crate::infrastructure::error::{P2PError, Result};
use matchbox_socket::WebRtcSocket;
use std::sync::{Arc, Mutex};

/// Infrastructure adapter: Manages WebRTC connection via Matchbox signalling
pub struct MatchboxConnection {
    socket: Arc<Mutex<WebRtcSocket>>,
    local_peer_id: Option<PeerId>,
}

impl MatchboxConnection {
    /// Connect to Matchbox signalling server
    pub async fn connect(signalling_url: &str) -> Result<Self> {
        tracing::info!("Connecting to signalling server: {}", signalling_url);

        let (mut socket, loop_fut) = WebRtcSocket::new_reliable(signalling_url);

        // Spawn the loop future to drive the socket
        tokio::spawn(async move {
            let _ = loop_fut.await;
        });

        // Wait for peer ID to be assigned
        let peer_id = wait_for_peer_id(&mut socket).await?;

        tracing::info!("Connected with peer ID: {}", peer_id);

        Ok(MatchboxConnection {
            socket: Arc::new(Mutex::new(socket)),
            local_peer_id: Some(peer_id),
        })
    }

    /// Get our local peer ID
    pub fn local_peer_id(&self) -> Option<PeerId> {
        self.local_peer_id
    }

    /// Get list of currently connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        let socket = self.socket.lock().unwrap();
        socket.connected_peers().map(PeerId::new).collect()
    }

    /// Send data to a specific peer
    pub fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        let mut socket = self.socket.lock().unwrap();
        socket.send(data.clone().into_boxed_slice(), peer.inner());

        tracing::debug!("Sent {} bytes to peer {}", data.len(), peer);
        Ok(())
    }

    /// Broadcast data to all connected peers
    pub fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        let peers = self.connected_peers();
        let peer_count = peers.len();

        for peer in peers {
            self.send_to(peer, data.clone())?;
        }

        tracing::debug!("Broadcast {} bytes to {} peers", data.len(), peer_count);
        Ok(())
    }

    /// Poll for events (call this regularly in your event loop)
    pub fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        let mut events = Vec::new();
        let mut socket = self.socket.lock().unwrap();

        // Check for new peers
        for (peer_id, state) in socket.update_peers() {
            let peer = PeerId::new(peer_id);
            match state {
                matchbox_socket::PeerState::Connected => {
                    tracing::info!("Peer connected: {}", peer);
                    events.push(ConnectionEvent::PeerConnected(peer));
                }
                matchbox_socket::PeerState::Disconnected => {
                    tracing::info!("Peer disconnected: {}", peer);
                    events.push(ConnectionEvent::PeerDisconnected(peer));
                }
            }
        }

        // Check for messages
        for (peer_id, packet) in socket.receive() {
            let peer = PeerId::new(peer_id);
            tracing::debug!("Received {} bytes from peer {}", packet.len(), peer);

            events.push(ConnectionEvent::MessageReceived {
                from: peer,
                data: packet.to_vec(),
            });
        }

        events
    }
}

/// Wait for the socket to receive a peer ID from the signalling server
async fn wait_for_peer_id(socket: &mut WebRtcSocket) -> Result<PeerId> {
    use tokio::time::{Duration, timeout};

    let wait_duration = Duration::from_secs(5);

    timeout(wait_duration, async {
        loop {
            // Update peers to process signalling messages
            socket.update_peers();

            if let Some(id) = socket.id() {
                return Ok(PeerId::new(id));
            }

            // Small delay between checks
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .map_err(|_| P2PError::ConnectionFailed("Timeout waiting for peer ID".to_string()))?
}
