pub mod application;
pub mod domain;
pub mod infrastructure;

pub use infrastructure::{CliError, Result};

#[cfg(feature = "tui")]
pub mod presentation;

pub use application::{
    DualLoopRuntime, MessageTranslator, RuntimeBuilder, RuntimeStats, StateSynchronizer,
};
