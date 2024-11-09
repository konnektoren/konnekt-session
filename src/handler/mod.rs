mod local_handler;

#[cfg(feature = "yew")]
mod websocket_handler;

pub use local_handler::LocalLobbyCommandHandler;

#[cfg(feature = "yew")]
pub use websocket_handler::WebSocketLobbyCommandHandler;
