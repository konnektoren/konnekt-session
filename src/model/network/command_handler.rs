use super::NetworkCommand;
use async_trait::async_trait;

#[async_trait]
pub trait NetworkCommandHandler<T> {
    async fn handle_command(&self, command: NetworkCommand<T>);

    async fn send_command(&self, command: NetworkCommand<T>);
}
