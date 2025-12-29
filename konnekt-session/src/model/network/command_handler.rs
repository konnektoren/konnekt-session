use super::{NetworkCommand, NetworkError};
use async_trait::async_trait;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait NetworkCommandHandler<T> {
    async fn handle_command(&self, command: NetworkCommand<T>) -> Result<(), NetworkError>;

    async fn send_command(&self, command: NetworkCommand<T>) -> Result<(), NetworkError>;
}
