use crate::server::{WebSocketListener, WebSocketServer};
use axum::extract::WebSocketUpgrade;
use axum::{routing::get, Router};
use uuid::Uuid;

pub fn create_session_route(websocket_server: WebSocketServer) -> Router {
    Router::new().route(
        "/session",
        get(move |ws: WebSocketUpgrade| {
            let lobby_id = Uuid::new_v4();
            WebSocketListener::handle_websocket(ws, Some(lobby_id), websocket_server.clone())
        }),
    )
}
