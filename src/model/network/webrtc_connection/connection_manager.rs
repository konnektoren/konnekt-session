use crate::model::{ClientId, LobbyId};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use web_sys::RtcDataChannel;
use web_sys::RtcDataChannelState;

#[derive(Default, Clone)]
pub struct WebRTCConnectionManager {
    connections: Arc<RwLock<HashMap<(ClientId, LobbyId), RtcDataChannel>>>, // Changed to store both ClientId and LobbyId as key
    lobby_clients: Arc<RwLock<HashMap<LobbyId, Vec<ClientId>>>>,
}

impl WebRTCConnectionManager {
    pub fn new() -> Self {
        Self {
            connections: Arc::new(RwLock::new(HashMap::new())),
            lobby_clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn remove_connection(&self, client_id: &ClientId, lobby_id: &LobbyId) {
        self.connections
            .write()
            .unwrap()
            .remove(&(*client_id, *lobby_id));
        let mut lobby_clients = self.lobby_clients.write().unwrap();
        if let Some(clients) = lobby_clients.get_mut(lobby_id) {
            clients.retain(|id| id != client_id);
        }
    }

    pub fn broadcast_to_lobby(
        &self,
        lobby_id: &LobbyId,
        message: &str,
        exclude_client: Option<ClientId>,
    ) {
        let connections = self.connections.read().unwrap();
        log::info!(
            "📢 Broadcasting to lobby {} (excluding {:?}), total connections: {}",
            lobby_id,
            exclude_client,
            connections.len()
        );

        for ((client_id, client_lobby_id), data_channel) in connections.iter() {
            if client_lobby_id == lobby_id && Some(*client_id) != exclude_client {
                log::info!("Attempting to broadcast to client {}", client_id);

                if data_channel.ready_state() == RtcDataChannelState::Open {
                    match data_channel.send_with_str(message) {
                        Ok(_) => {
                            log::info!("✅ Successfully broadcasted to client {}", client_id);
                        }
                        Err(e) => {
                            log::error!("❌ Failed to broadcast to client {}: {:?}", client_id, e);
                        }
                    }
                } else {
                    log::warn!("⚠️ Data channel not open for client {}", client_id);
                }
            }
        }
    }

    pub fn add_connection(&self, client_id: ClientId, lobby_id: LobbyId, channel: RtcDataChannel) {
        log::info!(
            "Adding connection for client {} in lobby {}",
            client_id,
            lobby_id
        );
        self.connections
            .write()
            .unwrap()
            .insert((client_id, lobby_id), channel);
        self.lobby_clients
            .write()
            .unwrap()
            .entry(lobby_id)
            .or_default()
            .push(client_id);
    }

    pub fn get_connection(
        &self,
        client_id: &ClientId,
        lobby_id: &LobbyId,
    ) -> Option<RtcDataChannel> {
        self.connections
            .read()
            .unwrap()
            .get(&(*client_id, *lobby_id))
            .cloned()
    }
}
