pub mod infrastructure;

pub use infrastructure::{CliError, Result};

#[cfg(feature = "tui")]
pub mod presentation;
