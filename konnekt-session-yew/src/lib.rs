//! # Konnekt Session Yew Components
//!
//! Reusable Yew components for building P2P session UIs.

pub mod app;
pub mod components;
pub mod hooks;
pub mod pages;
pub mod providers;

// Re-exports for convenience
pub use app::App;
pub use components::{ActivityList, LobbyView, ParticipantList, SessionInfo};
pub use hooks::{use_lobby, use_session};
pub use pages::{LoginScreen, SessionScreen};
pub use providers::{SessionProvider, SessionProviderProps};
