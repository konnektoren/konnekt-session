use crate::model::SignalingMessage;
use axum::extract::ws::Message;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc::Sender, RwLock};
use tracing::{debug, error, info, instrument};
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SignalingSession {
    sessions: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Sender<Message>>>>>,
}

impl SignalingSession {
    pub fn new() -> Self {
        debug!("Creating new SignalingSession");
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    #[instrument(skip(self, sender))]
    pub async fn add_session(&self, lobby_id: Uuid, client_id: Uuid, sender: Sender<Message>) {
        info!(?lobby_id, ?client_id, "Adding new signaling session");
        let mut sessions = self.sessions.write().await;
        sessions
            .entry(lobby_id)
            .or_insert_with(HashMap::new)
            .insert(client_id, sender);
        debug!(?lobby_id, ?client_id, "Session added successfully");
    }

    #[instrument(skip(self))]
    pub async fn remove_session(&self, lobby_id: Uuid, client_id: Uuid) {
        info!(?lobby_id, ?client_id, "Removing signaling session");
        let mut sessions = self.sessions.write().await;
        if let Some(lobby_sessions) = sessions.get_mut(&lobby_id) {
            lobby_sessions.remove(&client_id);
            if lobby_sessions.is_empty() {
                sessions.remove(&lobby_id);
                debug!(?lobby_id, "Removed empty lobby");
            }
        }
        debug!(?lobby_id, ?client_id, "Session removed successfully");
    }

    #[instrument(skip(self))]
    pub async fn get_available_peer(&self, lobby_id: Uuid, client_id: Uuid) -> Option<Uuid> {
        debug!(?lobby_id, ?client_id, "Looking for available peer");
        let sessions = self.sessions.read().await;
        if let Some(lobby_sessions) = sessions.get(&lobby_id) {
            let peer = lobby_sessions.keys().find(|&&id| id != client_id).copied();
            if let Some(peer_id) = peer {
                info!(?lobby_id, ?client_id, ?peer_id, "Found available peer");
            } else {
                debug!(?lobby_id, ?client_id, "No available peers found");
            }
            return peer;
        }
        debug!(?lobby_id, "Lobby not found");
        None
    }

    #[instrument(skip(self, message))]
    pub async fn forward_message(
        &self,
        lobby_id: Uuid,
        message: &SignalingMessage,
    ) -> Result<(), String> {
        debug!(?lobby_id, from = ?message.from, to = ?message.to, "Forwarding message");
        let sessions = self.sessions.read().await;

        if let Some(lobby_sessions) = sessions.get(&lobby_id) {
            // If the target is Uuid::nil(), find an available peer
            let target_id = if message.to == Uuid::nil() {
                match self.get_available_peer(lobby_id, message.from).await {
                    Some(peer_id) => peer_id,
                    None => {
                        error!(?lobby_id, "No available peers found");
                        return Err("No available peers".to_string());
                    }
                }
            } else {
                message.to
            };

            if let Some(sender) = lobby_sessions.get(&target_id) {
                // Create a new message with the correct target ID
                let forward_message = SignalingMessage {
                    from: message.from,
                    to: target_id,
                    content: message.content.clone(),
                };

                match serde_json::to_string(&forward_message) {
                    Ok(msg_str) => {
                        if let Err(e) = sender.send(Message::Text(msg_str)).await {
                            error!(?lobby_id, error = ?e, "Failed to send message");
                            return Err(e.to_string());
                        }
                        info!(?lobby_id, from = ?message.from, to = ?target_id, "Message forwarded successfully");
                        Ok(())
                    }
                    Err(e) => {
                        error!(?lobby_id, error = ?e, "Failed to serialize message");
                        Err(e.to_string())
                    }
                }
            } else {
                error!(?lobby_id, to = ?target_id, "Recipient not found");
                Err("Recipient not found".to_string())
            }
        } else {
            error!(?lobby_id, "Lobby not found");
            Err("Lobby not found".to_string())
        }
    }
}
