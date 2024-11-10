#![cfg(feature = "server")]

use axum::Router;
use konnekt_session::server::{create_session_route, MemoryStorage, WebSocketServer};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::fmt;
use tracing_subscriber::EnvFilter;

#[tokio::main]
pub async fn main() {
    // Initialize tracing
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("konnekt_session=debug"));

    fmt::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_ansi(true)
        .init();

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    let memory_storage = Arc::new(MemoryStorage::new());
    let websocket_server = WebSocketServer::new(memory_storage.clone(), memory_storage.clone());

    let app = Router::new().nest("/", create_session_route(websocket_server));

    log::info!("Server running at http://{}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .expect("Failed to start server.");
}
