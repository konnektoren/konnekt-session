pub mod error;

pub use error::CliError;

pub type Result<T> = std::result::Result<T, CliError>;
