pub mod infrastructure;

pub use infrastructure::{CliError, LogConfig, Result};

#[cfg(feature = "tui")]
pub mod presentation;
