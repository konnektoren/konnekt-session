mod client;
mod command;
mod command_handler;
mod connection;
mod error;
mod signaling;
mod transport;
mod transport_factory;

#[cfg(feature = "websocket")]
mod websocket_connection;

#[cfg(feature = "webrtc")]
mod webrtc_connection;

#[cfg(feature = "matchbox")]
mod matchbox_connection;

pub use client::{Client, ClientId};
pub use command::NetworkCommand;
pub use command_handler::NetworkCommandHandler;
pub use error::NetworkError;
pub use signaling::{SignalingContent, SignalingMessage};
pub use transport::{MessageCallback, Transport, TransportType};
pub use transport_factory::create_transport;

#[cfg(feature = "websocket")]
pub use websocket_connection::WebSocketConnection;

#[cfg(feature = "webrtc")]
pub use webrtc_connection::WebRTCConnection;
