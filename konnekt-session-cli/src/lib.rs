pub mod infrastructure;

pub use infrastructure::{CliError, LogConfig, Result, SessionRuntime, SessionSnapshot};

#[cfg(feature = "tui")]
pub mod presentation;
