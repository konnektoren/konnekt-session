use super::super::{MessageCallback, NetworkError, Transport, TransportType};
use super::MatchboxConnectionManager;
use crate::model::network::connection::{ConnectionHandler, ConnectionManager};
use crate::model::{ClientId, LobbyId, Role};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::stream::{SplitSink, SplitStream};
use futures::{select, FutureExt, StreamExt};
use futures_timer::Delay;
use matchbox_socket::{ChannelConfig, PeerState, WebRtcSocket, WebRtcSocketBuilder};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;

type MessagePacket = Box<[u8]>;

#[derive(Clone)]
pub struct MatchboxConnection {
    signaling_url: String,
    client_id: ClientId,
    lobby_id: LobbyId,
    role: Role,
    sender: UnboundedSender<String>,
    receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    connected: Arc<RwLock<bool>>,
    peer_manager: Arc<MatchboxConnectionManager>,
    socket: Arc<RwLock<Option<WebRtcSocket>>>,
}

impl MatchboxConnection {
    pub fn new(signaling_url: String, client_id: ClientId, lobby_id: LobbyId, role: Role) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        Self {
            signaling_url,
            client_id,
            lobby_id,
            role,
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            connected: Arc::new(RwLock::new(false)),
            peer_manager: Arc::new(MatchboxConnectionManager::new()),
            socket: Arc::new(RwLock::new(None)),
        }
    }

    async fn run_socket_loop(
        socket: Arc<RwLock<Option<WebRtcSocket>>>,
        setup_future: impl std::future::Future<Output = Result<(), matchbox_socket::Error>>,
        peer_manager: Arc<MatchboxConnectionManager>,
    ) {
        let setup_fut = setup_future.fuse();
        futures::pin_mut!(setup_fut);

        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);

        loop {
            if let Ok(mut socket_guard) = socket.write() {
                if let Some(socket) = &mut *socket_guard {
                    for (peer, state) in socket.update_peers() {
                        match state {
                            PeerState::Connected => {
                                log::info!("Peer joined: {peer}");
                                peer_manager.add_peer(peer.clone());
                            }
                            PeerState::Disconnected => {
                                log::info!("Peer left: {peer}");
                                peer_manager.remove_peer(&peer);
                            }
                        }
                    }
                }
            }

            select! {
                _ = (&mut timeout).fuse() => {
                    timeout.reset(Duration::from_millis(100));
                }
                _ = &mut setup_fut => {
                    break;
                }
            }
        }
    }

    pub fn spawn_socket_send_task(&self) {
        let peer_manager = self.peer_manager.clone();
        let socket = self.socket.clone();
        let receiver = self.receiver();

        spawn_local(async move {
            while let Some(message) = Self::next_message(receiver.clone()).await {
                let packet = message.as_bytes().to_vec().into_boxed_slice();
                let peers = peer_manager.get_connected_peers();

                for peer in peers {
                    if let Ok(mut socket_guard) = socket.write() {
                        if let Some(socket) = &mut *socket_guard {
                            log::debug!("Sending message to peer {} ({})", peer, packet.len());
                            socket.send(packet.clone(), peer);
                        }
                    }
                }
                Delay::new(Duration::from_millis(10)).await;
            }

            log::warn!("Send task ended");
        });
    }

    pub fn spawn_socket_receive_task(&self, callback: MessageCallback) {
        let socket = self.socket.clone();
        spawn_local(async move {
            loop {
                if let Ok(mut socket_guard) = socket.write() {
                    if let Some(socket) = &mut *socket_guard {
                        for (peer, packet) in socket.receive() {
                            log::debug!("Received message from peer {} ({})", peer, packet.len());
                            if let Ok(text) = String::from_utf8(packet.to_vec()) {
                                callback(text);
                            }
                        }
                    }
                }
                Delay::new(Duration::from_millis(10)).await;
            }
        });
    }
}

impl ConnectionHandler for MatchboxConnection {
    type SocketType = Arc<RwLock<WebRtcSocket>>;
    type InternMessageType = String;
    type ExternMessageType = MessagePacket;
    type CallbackType = MessageCallback;

    fn take_socket(&self) -> Option<Self::SocketType> {
        if let Ok(mut socket_guard) = self.socket.write() {
            if let Some(socket) = socket_guard.take() {
                return Some(Arc::new(RwLock::new(socket)));
            }
        }
        None
    }

    fn receiver(&self) -> Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>> {
        self.receiver.clone()
    }

    async fn next_message(
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) -> Option<Self::InternMessageType> {
        if let Ok(mut receiver_guard) = receiver.write() {
            receiver_guard.next().await
        } else {
            None
        }
    }

    fn spawn_send_task(
        &self,
        _sender: SplitSink<Self::SocketType, Self::ExternMessageType>,
        _receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) {
        unimplemented!("unable to provide splitsink")
    }

    fn spawn_receive_task(
        &self,
        _receiver: SplitStream<Self::SocketType>,
        _callback: Arc<Self::CallbackType>,
    ) {
        unimplemented!("unable to provide splitstream")
    }
}

impl Drop for MatchboxConnection {
    fn drop(&mut self) {
        self.disconnect();
    }
}

impl Transport for MatchboxConnection {
    fn connect(&mut self) -> Result<(), NetworkError> {
        let room_url = format!("{}/{}", self.signaling_url, self.lobby_id);
        let channel_config = ChannelConfig::unreliable();

        let (socket, setup_future) = WebRtcSocketBuilder::new(&room_url)
            .add_channel(channel_config)
            .build();

        *self.connected.write().unwrap() = true;

        log::info!("Connecting to signaling server: {}", room_url);

        let peer_manager = self.peer_manager.clone();
        self.socket = Arc::new(RwLock::new(Some(socket)));
        let socket = self.socket.clone();

        spawn_local(async move {
            Self::run_socket_loop(socket.clone(), setup_future, peer_manager).await;
        });

        Ok(())
    }

    fn disconnect(&mut self) {
        *self.connected.write().unwrap() = false;
    }

    fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    fn sender(&self) -> UnboundedSender<String> {
        self.sender.clone()
    }

    fn handle_messages(&self, callback: MessageCallback) {
        self.spawn_socket_receive_task(callback);
        self.spawn_socket_send_task();
    }

    fn transport_type(&self) -> TransportType {
        TransportType::Matchbox(
            self.signaling_url.clone(),
            self.lobby_id,
            self.client_id,
            self.role,
        )
    }

    fn box_clone(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}

impl PartialEq for MatchboxConnection {
    fn eq(&self, other: &Self) -> bool {
        self.signaling_url == other.signaling_url && self.lobby_id == other.lobby_id
    }
}
