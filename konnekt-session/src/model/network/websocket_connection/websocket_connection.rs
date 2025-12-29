use crate::model::network::connection::{ConnectionHandler, ConnectionManager};

use super::super::{MessageCallback, NetworkError, Transport, TransportType};
use super::connection_manager::WsPeerId;
use super::WebSocketConnectionManager;
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::stream::{SplitSink, SplitStream};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::sync::{Arc, RwLock};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct WebSocketConnection {
    websocket_url: String,
    sender: UnboundedSender<String>,
    receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    connection_manager: Arc<RwLock<WebSocketConnectionManager>>,
    ws: Arc<RwLock<Option<WebSocket>>>,
}

impl PartialEq for WebSocketConnection {
    fn eq(&self, other: &Self) -> bool {
        self.websocket_url == other.websocket_url
    }
}

impl WebSocketConnection {
    pub fn new(websocket_url: String) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            websocket_url,
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            connection_manager: Arc::new(RwLock::new(WebSocketConnectionManager::new())),
            ws: Arc::new(RwLock::new(None)),
        }
    }
}

impl ConnectionHandler for WebSocketConnection {
    type InternMessageType = String;
    type ExternMessageType = Message;
    type CallbackType = MessageCallback;
    type ExternSenderType = SplitSink<WebSocket, Message>;
    type ExternReceiverType = SplitStream<WebSocket>;

    fn receiver(&self) -> Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>> {
        self.receiver.clone()
    }

    fn sender(&self) -> UnboundedSender<Self::InternMessageType> {
        self.sender.clone()
    }

    async fn next_message(
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) -> Option<Self::InternMessageType> {
        let mut receiver_guard = receiver.write().ok()?;
        receiver_guard.next().await
    }

    fn spawn_send_task(
        &self,
        mut sender: Self::ExternSenderType,
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) {
        spawn_local(async move {
            loop {
                let message = Self::next_message(receiver.clone()).await;

                match message {
                    Some(text) => {
                        if sender.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        });
    }

    fn spawn_receive_task(
        &self,
        mut receiver: Self::ExternReceiverType,
        callback: Arc<Self::CallbackType>,
    ) {
        spawn_local(async move {
            while let Some(message) = receiver.next().await {
                if let Ok(Message::Text(text)) = message {
                    callback(text);
                }
            }
        });
    }
}

impl Transport for WebSocketConnection {
    fn connect(&mut self) -> Result<(), NetworkError> {
        let ws = WebSocket::open(&self.websocket_url)
            .map_err(|e| NetworkError::ConnectionError(e.message))?;
        *self.ws.write().unwrap() = Some(ws);
        self.connection_manager
            .write()
            .unwrap()
            .add_peer(WsPeerId(self.websocket_url.to_string()));
        Ok(())
    }

    fn disconnect(&mut self) {
        *self.ws.write().unwrap() = None;
        self.connection_manager
            .write()
            .unwrap()
            .remove_peer(&WsPeerId(self.websocket_url.to_string()));
    }

    fn is_connected(&self) -> bool {
        self.connection_manager
            .read()
            .unwrap()
            .is_peer_connected(&WsPeerId(self.websocket_url.to_string()))
    }

    fn sender(&self) -> UnboundedSender<String> {
        self.sender.clone()
    }

    fn handle_messages(&self, callback: MessageCallback) {
        let ws_instance = self.ws.write().ok().and_then(|mut guard| guard.take());

        if let Some(ws) = ws_instance {
            let (write, read) = ws.split();
            let callback = Arc::new(callback);

            self.spawn_receive_task(read, callback.clone());
            self.spawn_send_task(write, self.receiver());
        }
    }

    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket(self.websocket_url.clone())
    }

    fn box_clone(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}
