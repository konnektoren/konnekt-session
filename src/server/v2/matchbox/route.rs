use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use crossbeam_channel::{bounded, unbounded};
use futures_util::{SinkExt, StreamExt};
use matchbox_signaling::SignalingServer;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{error, info};

static CHANNEL_CAPACITY: usize = 32;

#[derive(Clone, Serialize, Deserialize)]
enum SharedMessage {
    Text(String),
    Binary(Vec<u8>),
    Ping(Vec<u8>),
    Pong(Vec<u8>),
    Close,
}

pub fn create_matchbox_route() -> Router {
    // Start matchbox server
    let matchbox_addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3001));
    let matchbox_server = SignalingServer::full_mesh_builder(matchbox_addr)
        .on_connection_request(|meta| {
            info!("Matchbox connection request from {:?}", meta.origin);
            Ok(true)
        })
        .on_peer_connected(|peer_id| {
            info!("Peer connected to matchbox: {:?}", peer_id);
        })
        .on_peer_disconnected(|peer_id| {
            info!("Peer disconnected from matchbox: {:?}", peer_id);
        })
        .build();

    // Spawn matchbox server in a separate task
    tokio::spawn(async move {
        if let Err(e) = matchbox_server.serve().await {
            error!("Matchbox server error: {}", e);
        }
    });

    // Create proxy route
    Router::new().route("/", get(handle_websocket_proxy))
}

async fn handle_websocket_proxy(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(client_ws: WebSocket) {
    let (sender, receiver) = bounded::<SharedMessage>(CHANNEL_CAPACITY);
    let (feedback_sender, feedback_receiver) = unbounded::<SharedMessage>();

    let sender = Arc::new(sender);
    let receiver = Arc::new(receiver);

    let (mut client_tx, mut client_rx) = client_ws.split();
    let (tx, mut rx) = mpsc::channel::<Message>(32);

    // Handle client to channel
    let sender_clone = Arc::clone(&sender);
    let client_to_channel = tokio::spawn(async move {
        while let Some(Ok(msg)) = client_rx.next().await {
            let shared_msg = match msg {
                Message::Text(t) => SharedMessage::Text(t),
                Message::Binary(b) => SharedMessage::Binary(b),
                Message::Ping(p) => SharedMessage::Ping(p),
                Message::Pong(p) => SharedMessage::Pong(p),
                Message::Close(_) => SharedMessage::Close,
            };

            if let Err(e) = sender_clone.send(shared_msg) {
                error!("Failed to send message to channel: {}", e);
                break;
            }
        }
    });

    // Handle channel to client
    let receiver_clone = Arc::clone(&receiver);
    let channel_to_client = tokio::spawn(async move {
        loop {
            match receiver_clone.try_recv() {
                Ok(shared_msg) => {
                    let ws_msg = match shared_msg {
                        SharedMessage::Text(t) => Message::Text(t),
                        SharedMessage::Binary(b) => Message::Binary(b),
                        SharedMessage::Ping(p) => Message::Ping(p),
                        SharedMessage::Pong(p) => Message::Pong(p),
                        SharedMessage::Close => break,
                    };

                    if let Err(e) = tx.send(ws_msg).await {
                        error!("Failed to send message to client: {}", e);
                        break;
                    }
                }
                Err(crossbeam_channel::TryRecvError::Empty) => {
                    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
                    continue;
                }
                Err(e) => {
                    error!("Channel receive error: {}", e);
                    break;
                }
            }
        }
    });

    // Forward messages from channel to client
    let forward_to_client = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if let Err(e) = client_tx.send(msg).await {
                error!("Failed to forward message to client: {}", e);
                break;
            }
        }
    });

    // Connect to matchbox server and handle messages
    let feedback_sender = Arc::new(feedback_sender);
    let matchbox_handler = tokio::spawn(async move {
        if let Ok(mut matchbox_ws) = tokio_tungstenite::connect_async("ws://127.0.0.1:3001").await {
            info!("Connected to matchbox server");

            while let Some(Ok(msg)) = matchbox_ws.0.next().await {
                let shared_msg = match msg {
                    tokio_tungstenite::tungstenite::Message::Text(t) => {
                        SharedMessage::Text(t.to_string())
                    }
                    tokio_tungstenite::tungstenite::Message::Binary(b) => {
                        SharedMessage::Binary(b.as_slice().to_vec())
                    }
                    tokio_tungstenite::tungstenite::Message::Ping(p) => {
                        SharedMessage::Ping(p.as_slice().to_vec())
                    }
                    tokio_tungstenite::tungstenite::Message::Pong(p) => {
                        SharedMessage::Pong(p.as_slice().to_vec())
                    }
                    tokio_tungstenite::tungstenite::Message::Close(_) => SharedMessage::Close,
                    _ => continue,
                };

                if let Err(e) = feedback_sender.send(shared_msg) {
                    error!("Failed to send matchbox message to channel: {}", e);
                    break;
                }
            }
        }
    });

    // Wait for any task to finish
    tokio::select! {
        _ = client_to_channel => info!("Client to channel connection ended"),
        _ = channel_to_client => info!("Channel to client connection ended"),
        _ = forward_to_client => info!("Forward to client task ended"),
        _ = matchbox_handler => info!("Matchbox handler ended"),
    }
}
