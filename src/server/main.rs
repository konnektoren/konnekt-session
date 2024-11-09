#![cfg(feature = "server")]

use std::net::SocketAddr;

use konnekt_session::server::WebSocketListener;
use konnekt_session::server::WebSocketServerImpl;
use tracing::debug;
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
    let server = WebSocketServerImpl::new();
    let listener = WebSocketListener::new(server, addr);
    listener.run().await;
}
