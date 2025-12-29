use crate::model::{NetworkCommand, NetworkCommandHandler};
use crate::server::v2::ConnectionHandler;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::Receiver;
use tracing::{debug, error, info, instrument, warn};

#[instrument(skip(ws, connection_handler), fields(client_id))]
pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    connection_handler: ConnectionHandler,
) -> impl IntoResponse {
    debug!("New WebSocket upgrade request");
    ws.on_upgrade(move |socket| listen(socket, connection_handler))
}

#[instrument(skip(socket, connection_handler), fields(client_id))]
async fn listen(socket: WebSocket, connection_handler: ConnectionHandler) {
    debug!("WebSocket connection established");
    let (ws_sender, ws_receiver) = socket.split();
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let connection_handler = connection_handler.with_sender(tx);

    let sender_task = handle_outgoing_messages(rx, ws_sender);
    let receiver_task = handle_incoming_messages(ws_receiver, &connection_handler);

    tokio::select! {
        _ = sender_task => {
            info!(client_id = ?connection_handler.client_id(), "Sender task completed");
        }
        _ = receiver_task => {
            info!(client_id = ?connection_handler.client_id(), "Receiver task completed");
        }
    }
    if let Err(e) = connection_handler.disconnect().await {
        error!(error = ?e, "Failed to disconnect");
    }
}

#[instrument(skip(rx, ws_sender))]
pub async fn handle_outgoing_messages(
    mut rx: Receiver<Message>,
    mut ws_sender: SplitSink<WebSocket, Message>,
) {
    debug!("Started handling outgoing messages");
    while let Some(msg) = rx.recv().await {
        debug!(?msg, "Sending message");
        if let Err(e) = ws_sender.send(msg).await {
            error!(error = ?e, "Failed to send message");
            break;
        }
    }
}

#[instrument(skip(receiver, connection_handler))]
pub async fn handle_incoming_messages(
    mut receiver: SplitStream<WebSocket>,
    connection_handler: &ConnectionHandler,
) {
    debug!("Started handling incoming messages");
    let connection_handler = connection_handler.clone();

    while let Some(message) = receiver.next().await {
        match message {
            Ok(message) => {
                debug!(?message, "Received message");
                handle_message(message, &connection_handler).await;
            }
            Err(e) => {
                error!(error = ?e, "Failed to receive message");
                break;
            }
        }
    }
}

#[instrument(skip(message, connection_handler), fields(client_id = ?connection_handler.client_id()))]
pub async fn handle_message(message: Message, connection_handler: &ConnectionHandler) {
    match message {
        Message::Text(text) => {
            debug!(?text, "Handling text message");
            match serde_json::from_str::<NetworkCommand<String>>(&text) {
                Ok(command) => {
                    debug!(?command, "Parsed command");
                    if let Err(e) = connection_handler.handle_command(command).await {
                        error!(error = ?e, "Failed to handle command");
                    }
                }
                Err(e) => {
                    error!(error = ?e, text = ?text, "Failed to parse command");
                }
            }
        }
        Message::Close(_) => {
            info!(client_id = ?connection_handler.client_id(), "Client disconnected");
            if let Err(e) = connection_handler.disconnect().await {
                error!(error = ?e, "Failed to handle disconnect");
            }
        }
        _ => {
            warn!(
                message = ?message,
                client_id = ?connection_handler.client_id(),
                "Unsupported message type"
            );
        }
    }
}
