use super::SignalingSession;
use crate::model::network::SignalingMessage;
use axum::{
    extract::{ws::Message, Path, State, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info};
use uuid::Uuid;

pub fn create_signaling_route() -> Router {
    let session = SignalingSession::new();

    Router::new()
        .route(
            "/signaling/:lobby_id/:client_id",
            get(handle_signaling_connection),
        )
        .with_state(session)
}

async fn handle_signaling_connection(
    ws: WebSocketUpgrade,
    State(session): State<SignalingSession>,
    Path((lobby_id, client_id)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    info!(
        "New signaling connection for lobby: {}, client: {}",
        lobby_id, client_id
    );
    ws.on_upgrade(move |socket| handle_socket(socket, session, lobby_id, client_id))
}

async fn handle_socket(
    socket: axum::extract::ws::WebSocket,
    session: SignalingSession,
    lobby_id: Uuid,
    client_id: Uuid,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel(32);

    session.add_session(lobby_id, client_id, tx).await;

    // Handle outgoing messages
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            if sender.send(message).await.is_err() {
                break;
            }
        }
    });

    // Handle incoming messages
    while let Some(Ok(message)) = receiver.next().await {
        if let Message::Text(text) = message {
            match serde_json::from_str::<SignalingMessage>(&text) {
                Ok(msg) => {
                    if let Err(e) = session.forward_message(lobby_id, &msg).await {
                        error!("Failed to forward message: {}", e);
                    }
                }
                Err(e) => {
                    error!("Failed to parse signaling message: {}", e);
                }
            }
        }
    }

    // Cleanup
    session.remove_session(lobby_id, client_id).await;
    send_task.abort();
}
