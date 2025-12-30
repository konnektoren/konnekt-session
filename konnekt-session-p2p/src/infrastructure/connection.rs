use crate::application::ConnectionEvent;
use crate::domain::{IceServer, PeerId};
use crate::infrastructure::error::{P2PError, Result};
use matchbox_socket::{RtcIceServerConfig, WebRtcSocket, WebRtcSocketBuilder};
use std::sync::{Arc, Mutex};

/// Infrastructure adapter: Manages WebRTC connection via Matchbox signalling
pub struct MatchboxConnection {
    socket: Arc<Mutex<WebRtcSocket>>,
    local_peer_id: Option<PeerId>,
}

impl MatchboxConnection {
    /// Connect to Matchbox signalling server (default config)
    pub async fn connect_default(signalling_url: &str) -> Result<Self> {
        Self::connect(signalling_url, IceServer::default_stun_servers()).await
    }

    //// Connect to Matchbox signalling server with custom ICE servers
    pub async fn connect(signalling_url: &str, ice_servers: Vec<IceServer>) -> Result<Self> {
        tracing::info!("Connecting to signalling server: {}", signalling_url);
        tracing::info!("Configured with {} ICE servers", ice_servers.len());

        for (i, server) in ice_servers.iter().enumerate() {
            if server.username.is_some() {
                tracing::info!(
                    "  ICE Server {}: {} (with auth)",
                    i + 1,
                    server.urls.join(", ")
                );
            } else {
                tracing::info!("  ICE Server {}: {}", i + 1, server.urls.join(", "));
            }
        }

        let ice_server_config = build_ice_server_config(&ice_servers);

        let (mut socket, loop_fut) = WebRtcSocketBuilder::new(signalling_url)
            .ice_server(ice_server_config)
            .add_channel(matchbox_socket::ChannelConfig::reliable())
            .build();

        // 🔧 FIX: Simple spawn with instrument span
        let matchbox_span = tracing::info_span!("matchbox::webrtc_loop");
        tokio::spawn(async move {
            let _enter = matchbox_span.enter();
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

/// Build ICE server configuration for Matchbox
fn build_ice_server_config(ice_servers: &[IceServer]) -> RtcIceServerConfig {
    if ice_servers.is_empty() {
        // Use default if none provided
        return RtcIceServerConfig::default();
    }

    // Convert first server (Matchbox only supports one ICE server config currently)
    let first_server = &ice_servers[0];

    RtcIceServerConfig {
        urls: first_server.urls.clone(),
        username: first_server.username.clone(),
        credential: first_server.credential.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_ice_server_config_stun_only() {
        let servers = vec![IceServer::stun("stun:stun.l.google.com:19302".to_string())];

        let config = build_ice_server_config(&servers);
        assert_eq!(config.urls, vec!["stun:stun.l.google.com:19302"]);
        assert!(config.username.is_none());
        assert!(config.credential.is_none());
    }

    #[test]
    fn test_build_ice_server_config_with_turn() {
        let servers = vec![IceServer::turn(
            "turn:turn.example.com:3478".to_string(),
            "user".to_string(),
            "pass".to_string(),
        )];

        let config = build_ice_server_config(&servers);
        assert_eq!(config.urls, vec!["turn:turn.example.com:3478"]);
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.credential, Some("pass".to_string()));
    }

    #[test]
    fn test_build_ice_server_config_multiple_urls() {
        let servers = vec![IceServer::from_urls(vec![
            "stun:stun1.example.com:3478".to_string(),
            "stun:stun2.example.com:3478".to_string(),
        ])];

        let config = build_ice_server_config(&servers);
        assert_eq!(
            config.urls,
            vec!["stun:stun1.example.com:3478", "stun:stun2.example.com:3478"]
        );
    }

    #[test]
    fn test_build_ice_server_config_empty() {
        let servers = vec![];
        let config = build_ice_server_config(&servers);

        // Should return default config
        assert!(!config.urls.is_empty());
    }
}
