#![cfg(feature = "server")]

use axum::Router;
use konnekt_session::server::telemetry::{init_telemetry, shutdown_telemetry};
use konnekt_session::server::v2::{
    create_session_route, signaling::create_signaling_route, ConnectionHandler, MemoryStorage,
};
use std::error::Error;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{info, instrument};

#[tokio::main]
#[instrument]
pub async fn main() -> Result<(), Box<dyn Error>> {
    // Initialize telemetry
    init_telemetry().await?;

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(&addr).await?;

    let memory_storage = Arc::new(MemoryStorage::new());
    let connection_handler = ConnectionHandler::new(memory_storage.clone(), memory_storage.clone());

    let app = Router::new()
        .nest("/", create_session_route(connection_handler))
        .merge(create_signaling_route());

    info!("Server running at http://{}", addr);

    axum::serve(listener, app.into_make_service())
        .await
        .expect("Failed to start server.");

    // Shutdown telemetry on server shutdown
    shutdown_telemetry();
    Ok(())
}
