mod local_handler;

#[cfg(feature = "yew")]
mod network_handler;

pub use local_handler::LocalLobbyCommandHandler;

#[cfg(feature = "yew")]
pub use network_handler::NetworkHandler;
