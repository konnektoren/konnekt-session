#![cfg(feature = "server")]

use konnekt_session::server::WebSocketListener;
use konnekt_session::server::WebSocketServerImpl;
use std::net::SocketAddr;
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
    let websocket_server = WebSocketServerImpl::new();
    let websocket_listener = WebSocketListener::new(websocket_server, listener);
    websocket_listener.run().await;
}
