use crate::model::{NetworkCommand, NetworkCommandHandler};
use crate::server::v2::ConnectionHandler;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::WebSocketUpgrade;
use axum::response::IntoResponse;
use futures_util::stream::{SplitSink, SplitStream};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc::Receiver;

pub async fn handle_websocket(
    ws: WebSocketUpgrade,
    connection_handler: ConnectionHandler,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| listen(socket, connection_handler))
}

async fn listen(socket: WebSocket, connection_handler: ConnectionHandler) {
    let (ws_sender, ws_receiver) = socket.split();
    let (tx, rx) = tokio::sync::mpsc::channel(10);
    let connection_handler = connection_handler.with_sender(tx);

    let sender_task = handle_outgoing_messages(rx, ws_sender);
    let receiver_task = handle_incoming_messages(ws_receiver, &connection_handler);

    tokio::select! {
        _ = sender_task => {
            log::info!("Sender task completed for client {:?}", connection_handler.client_id());
        }
        _ = receiver_task => {
            log::info!("Receiver task completed for client {:?}", connection_handler.client_id());
        }
    }
    if let Err(e) = connection_handler.disconnect().await {
        log::error!("Failed to disconnect: {:?}", e);
    }
}

pub async fn handle_outgoing_messages(
    mut rx: Receiver<Message>,
    mut ws_sender: SplitSink<WebSocket, Message>,
) {
    while let Some(msg) = rx.recv().await {
        if let Err(e) = ws_sender.send(msg).await {
            log::error!("Failed to send message: {:?}", e);
            break;
        }
    }
}

pub async fn handle_incoming_messages(
    mut receiver: SplitStream<WebSocket>,
    connection_handler: &ConnectionHandler,
) {
    let connection_handler = connection_handler.clone();

    while let Some(message) = receiver.next().await {
        match message {
            Ok(message) => {
                handle_message(message, &connection_handler).await;
            }
            Err(e) => {
                log::error!("Failed to receive message: {:?}", e);
                break;
            }
        }
    }
}

pub async fn handle_message(message: Message, connection_handler: &ConnectionHandler) {
    match message {
        Message::Text(text) => match serde_json::from_str::<NetworkCommand<String>>(&text) {
            Ok(command) => {
                if let Err(e) = connection_handler.handle_command(command).await {
                    log::error!("Failed to handle command: {:?}", e);
                }
            }
            Err(e) => {
                log::error!("Failed to parse command: {:?}", e);
            }
        },
        Message::Close(_) => {
            log::info!("Client disconnected {:?}", connection_handler.client_id());
            if let Err(e) = connection_handler.disconnect().await {
                log::error!("Failed to handle disconnect: {:?}", e);
            }
        }
        _ => {
            log::warn!(
                "Unsupported message type: {:?} from {:?}",
                message,
                connection_handler.client_id()
            );
        }
    }
}
