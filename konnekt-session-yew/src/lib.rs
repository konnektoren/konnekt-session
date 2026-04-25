//! # Konnekt Session Yew Components
//!
//! Reusable Yew components for building P2P session UIs.

pub mod app;
pub mod components;
pub mod hooks;
pub mod pages;
#[cfg(feature = "preview")]
pub mod preview;
pub mod providers;

// Re-exports for convenience
pub use app::App;
pub use components::{ActivityList, LobbyView, ParticipantList, SessionInfo};
pub use hooks::{
    HostConnectivityOptions, HostConnectivityState, use_host_connectivity, use_lobby, use_session,
};
pub use pages::{LoginScreen, SessionScreen};
pub use providers::{SessionProvider, SessionProviderProps};
