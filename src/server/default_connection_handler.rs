use crate::server::ConnectionHandler;

pub struct DefaultConnectionHandler;

impl ConnectionHandler for DefaultConnectionHandler {
    fn handle_connect(
        &self,
        player_id: Uuid,
        role: Role,
        data: &str,
        password: &str,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
        ws_sender: WebSocketStream<tokio::net::TcpStream>,
        tx: mpsc::UnboundedSender<Message>,
    ) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            // Handle outgoing messages
            let mut send_task = tokio::spawn(async move {
                while let Some(message) = tx.recv().await {
                    if let Err(e) = ws_sender.send(message).await {
                        debug!("Error sending message: {}", e);
                        break;
                    }
                }
            });

            // Handle incoming messages
            let connections_clone = connections.clone();
            let mut recv_task = tokio::spawn(async move {
                loop {
                    // Keep receiving messages until an error occurs
                    match ws_receiver.next().await {
                        Some(Ok(message)) => {
                            match message {
                                Message::Text(text) => {
                                    if let Ok(command_wrapper) =
                                        serde_json::from_str::<LobbyCommandWrapper>(&text)
                                    {
                                        broadcast_command(
                                            connections_clone.clone(),
                                            &command_wrapper,
                                            lobby_id,
                                        )
                                        .await;
                                    }
                                }
                                Message::Close(_) => break, // Handle close message
                                _ => {}
                            }
                        }
                        Some(Err(e)) => {
                            debug!("Error receiving message: {}", e);
                            break; // Handle receiving errors
                        }
                        None => {
                            debug!("Connection closed");
                            break; // Connection closed gracefully
                        }
                    }
                }
                // Remove connection on disconnect
                let mut connections_write = connections_clone.write().await;
                if let Some(lobby_connections) = connections_write.get_mut(&lobby_id) {
                    lobby_connections.remove(&player_id);
                    if lobby_connections.is_empty() {
                        connections_write.remove(&lobby_id);
                    }
                }
            });

            // Run tasks concurrently
            let _ = tokio::join!(send_task, recv_task);
        })
    }

    fn handle_close(
        &self,
        player_id: Uuid,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    ) {
        let mut connections_write = connections.write().unwrap();
        if let Some(lobby_connections) = connections_write.get_mut(&lobby_id) {
            lobby_connections.remove(&player_id);
            if lobby_connections.is_empty() {
                connections_write.remove(&lobby_id);
            }
        }
    }

    fn handle_message(
        &self,
        message: &str,
        player_id: Uuid,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    ) -> Option<Message> {
        // Echo the message back to the client
        Some(Message::Text(message.to_string()))
    }
}

async fn broadcast_command(
    connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    command_wrapper: &LobbyCommandWrapper,
    lobby_id: Uuid,
) {
    let connections = connections.read().await;
    let message = Message::Text(serde_json::to_string(command_wrapper).unwrap());

    if let Some(lobby_connections) = connections.get(&lobby_id) {
        for connection in lobby_connections.values() {
            if let Err(e) = connection.sender.send(message.clone()) {
                debug!(
                    "Error broadcasting message to player {}: {}",
                    connection.player_id, e
                );
            }
        }
    }
}
