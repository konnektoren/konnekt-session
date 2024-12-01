use crate::server::v2::{websocket_listener, ConnectionHandler};
use axum::extract::WebSocketUpgrade;
use axum::{routing::get, Router};
use tracing::{debug, instrument};

#[instrument(skip(connection_handler))]
pub fn create_session_route(connection_handler: ConnectionHandler) -> Router {
    debug!("Creating session route");
    Router::new().route(
        "/session",
        get(move |ws: WebSocketUpgrade| {
            debug!("Received WebSocket upgrade request");
            websocket_listener::handle_websocket(
                ws,
                ConnectionHandler::new_from(&connection_handler),
            )
        }),
    )
}
