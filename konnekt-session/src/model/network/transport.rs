use crate::model::{LobbyId, Role};

use super::{ClientId, NetworkError};
use futures::channel::mpsc::UnboundedSender;
use std::fmt::Debug;

pub type MessageCallback = Box<dyn Fn(String) + 'static>;

#[derive(Clone, PartialEq, Debug)]
pub enum TransportType {
    #[cfg(feature = "websocket")]
    WebSocket(String),
    #[cfg(feature = "webrtc")]
    WebRTC(String, LobbyId, ClientId, Role),
    #[cfg(feature = "matchbox")]
    Matchbox(String, LobbyId, ClientId, Role),
    #[cfg(not(any(feature = "websocket", feature = "webrtc")))]
    None,
}

pub trait Transport: 'static {
    fn connect(&mut self) -> Result<(), NetworkError>;
    fn disconnect(&mut self);
    fn is_connected(&self) -> bool;
    fn sender(&self) -> UnboundedSender<String>;
    fn handle_messages(&self, callback: MessageCallback);
    fn transport_type(&self) -> TransportType;
    fn box_clone(&self) -> Box<dyn Transport>;
}

impl Clone for Box<dyn Transport> {
    fn clone(&self) -> Self {
        self.box_clone()
    }
}
