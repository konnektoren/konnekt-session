use futures::channel::mpsc;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::{SinkExt, StreamExt};
use gloo_net::websocket::futures::WebSocket;
use gloo_net::websocket::Message;
use std::sync::{Arc, Mutex, RwLock};
use wasm_bindgen_futures::spawn_local;

#[derive(Clone)]
pub struct WebSocketConnection {
    websocket_url: String,
    sender: UnboundedSender<String>,
    receiver: Arc<Mutex<UnboundedReceiver<String>>>,
    connected: Arc<RwLock<bool>>,
    ws: Arc<RwLock<Option<WebSocket>>>,
}

impl WebSocketConnection {
    pub fn new(websocket_url: String) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            websocket_url,
            sender,
            receiver: Arc::new(Mutex::new(receiver)),
            connected: Arc::new(RwLock::new(false)),
            ws: Arc::new(RwLock::new(None)),
        }
    }

    pub fn connect(&mut self) -> Result<(), crate::model::NetworkError> {
        let ws = WebSocket::open(&self.websocket_url)
            .map_err(|_| crate::model::NetworkError::ConnectionError)?;
        *self.ws.write().unwrap() = Some(ws);
        *self.connected.write().unwrap() = true;
        Ok(())
    }

    pub fn disconnect(&mut self) {
        *self.connected.write().unwrap() = false;
        *self.ws.write().unwrap() = None;
    }

    pub fn ws(&self) -> &Arc<RwLock<Option<WebSocket>>> {
        &self.ws
    }

    pub fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    pub fn sender(&self) -> UnboundedSender<String> {
        self.sender.clone()
    }

    pub fn receiver(&self) -> Arc<Mutex<UnboundedReceiver<String>>> {
        self.receiver.clone()
    }

    pub fn handle_messages<F>(&self, callback: F)
    where
        F: Fn(String) + 'static,
    {
        let ws = self.ws.clone();
        let receiver = self.receiver();
        let callback = Arc::new(callback);

        let ws_instance = {
            if let Ok(mut guard) = ws.write() {
                guard.take()
            } else {
                None
            }
        };

        if let Some(ws) = ws_instance {
            let (mut write, mut read) = ws.split();

            // Spawn read task
            let read_callback = callback.clone();
            spawn_local(async move {
                while let Some(Ok(Message::Text(message))) = read.next().await {
                    read_callback(message);
                }
            });

            // Spawn write task
            spawn_local(async move {
                loop {
                    let message = {
                        let mut receiver = receiver.lock().unwrap();
                        receiver.next().await
                    };

                    match message {
                        Some(message) => {
                            if write.send(Message::Text(message)).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
            });
        }
    }
}
