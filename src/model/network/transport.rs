use super::NetworkError;
use async_trait::async_trait;
use futures::channel::mpsc::UnboundedSender;

#[async_trait(?Send)]
pub trait Transport: PartialEq {
    fn connect(&mut self) -> Result<(), NetworkError>;
    fn disconnect(&mut self);
    fn is_connected(&self) -> bool;
    fn sender(&self) -> UnboundedSender<String>;
    fn handle_messages<F>(&self, callback: F)
    where
        F: Fn(String) + 'static;
}
