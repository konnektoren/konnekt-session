use super::{MessageCallback, NetworkError, Transport, TransportType};
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
    connected: Arc<RwLock<bool>>,
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
            connected: Arc::new(RwLock::new(false)),
            ws: Arc::new(RwLock::new(None)),
        }
    }

    fn take_websocket(&self) -> Option<WebSocket> {
        self.ws.write().ok().and_then(|mut guard| guard.take())
    }

    pub fn receiver(&self) -> Arc<RwLock<UnboundedReceiver<String>>> {
        self.receiver.clone()
    }

    fn spawn_read_task(&self, mut read: SplitStream<WebSocket>, callback: Arc<MessageCallback>) {
        spawn_local(async move {
            while let Some(message) = read.next().await {
                if let Ok(Message::Text(text)) = message {
                    callback(text);
                }
            }
        });
    }

    fn spawn_write_task(
        &self,
        mut write: SplitSink<WebSocket, Message>,
        receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    ) {
        spawn_local(async move {
            loop {
                let message = Self::get_next_message(&receiver).await;

                match message {
                    Some(text) => {
                        if write.send(Message::Text(text)).await.is_err() {
                            break;
                        }
                    }
                    None => break,
                }
            }
        });
    }

    async fn get_next_message(receiver: &Arc<RwLock<UnboundedReceiver<String>>>) -> Option<String> {
        let mut receiver_guard = receiver.write().ok()?;
        receiver_guard.next().await
    }
}

impl Transport for WebSocketConnection {
    fn connect(&mut self) -> Result<(), NetworkError> {
        let ws = WebSocket::open(&self.websocket_url)
            .map_err(|e| NetworkError::ConnectionError(e.message))?;
        *self.ws.write().unwrap() = Some(ws);
        *self.connected.write().unwrap() = true;
        Ok(())
    }

    fn disconnect(&mut self) {
        *self.connected.write().unwrap() = false;
        *self.ws.write().unwrap() = None;
    }

    fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    fn sender(&self) -> UnboundedSender<String> {
        self.sender.clone()
    }

    fn handle_messages(&self, callback: MessageCallback) {
        let ws_instance = self.take_websocket();

        if let Some(ws) = ws_instance {
            let (write, read) = ws.split();
            let callback = Arc::new(callback);

            self.spawn_read_task(read, callback.clone());
            self.spawn_write_task(write, self.receiver());
        }
    }

    fn transport_type(&self) -> TransportType {
        TransportType::WebSocket(self.websocket_url.clone())
    }

    fn box_clone(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}
