//! # Konnekt Session Yew Components
//!
//! Reusable Yew components for building P2P session UIs.
//!
//! ## Features
//!
//! - 🎨 **Pre-built components** - Lobby, participants, activities
//! - 🔌 **Hooks** - `use_lobby()`, `use_session()`
//! - 🎯 **Type-safe** - Full Rust type safety
//! - 📦 **Modular** - Use only what you need
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use yew::prelude::*;
//! use konnekt_session_yew::{SessionProvider, LobbyView};
//!
//! #[function_component(App)]
//! fn app() -> Html {
//!     html! {
//!         <SessionProvider signalling_server="wss://match.konnektoren.help">
//!             <LobbyView />
//!         </SessionProvider>
//!     }
//! }
//! ```

pub mod components;
pub mod hooks;
pub mod providers;

// Re-exports for convenience
pub use components::{ActivityList, LobbyView, ParticipantList, SessionInfo};
pub use hooks::{use_lobby, use_session};
pub use providers::{SessionProvider, SessionProviderProps};
