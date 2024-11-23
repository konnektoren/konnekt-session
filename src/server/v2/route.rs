use crate::server::v2::{websocket_listener, ConnectionHandler};
use axum::extract::WebSocketUpgrade;
use axum::{routing::get, Router};

pub fn create_session_route(connection_handler: ConnectionHandler) -> Router {
    Router::new().route(
        "/session",
        get(move |ws: WebSocketUpgrade| {
            websocket_listener::handle_websocket(
                ws,
                ConnectionHandler::new_from(&connection_handler),
            )
        }),
    )
}
