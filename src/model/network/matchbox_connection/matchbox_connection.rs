use super::super::{MessageCallback, NetworkError, Transport, TransportType};
use super::MatchboxPeerManager;
use crate::model::{ClientId, LobbyId, Role};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
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
    peer_manager: Arc<MatchboxPeerManager>,
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
            peer_manager: Arc::new(MatchboxPeerManager::new()),
        }
    }

    async fn run_socket_loop(
        mut socket: WebRtcSocket,
        setup_future: impl std::future::Future<Output = Result<(), matchbox_socket::Error>>,
        sender: UnboundedSender<String>,
        receiver: Arc<RwLock<UnboundedReceiver<String>>>,
        peer_manager: Arc<MatchboxPeerManager>,
    ) {
        let setup_fut = setup_future.fuse();
        futures::pin_mut!(setup_fut);

        let timeout = Delay::new(Duration::from_millis(100));
        futures::pin_mut!(timeout);

        // Spawn write task
        let (write_tx, mut write_rx) =
            mpsc::unbounded::<(MessagePacket, Vec<matchbox_socket::PeerId>)>();

        // Spawn write task
        let peer_manager_write = peer_manager.clone();
        spawn_local(async move {
            loop {
                let message = Self::get_next_message(&receiver).await;

                match message {
                    Some(text) => {
                        log::info!("next message {}", text);
                        let packet = text.as_bytes().to_vec().into_boxed_slice();
                        let peers = peer_manager_write.get_connected_peers();
                        if !peers.is_empty() {
                            if write_tx.unbounded_send((packet, peers)).is_err() {
                                log::error!("Failed to send message to main loop");
                                break;
                            }
                        }
                    }
                    None => break,
                }
            }
        });

        loop {
            // Handle peer updates
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        log::info!("Peer joined: {peer}");
                        peer_manager.add_peer(peer);
                        let packet = "hello friend!".as_bytes().to_vec().into_boxed_slice();
                        socket.send(packet, peer);
                    }
                    PeerState::Disconnected => {
                        log::info!("Peer left: {peer}");
                        peer_manager.remove_peer(&peer);
                    }
                }
            }

            // Handle incoming messages
            for (peer, packet) in socket.receive() {
                if let Ok(text) = String::from_utf8(packet.to_vec()) {
                    log::debug!("Received message from peer {}", peer);
                    if sender.unbounded_send(text).is_err() {
                        log::warn!("Failed to forward message - channel closed");
                        return;
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

    async fn get_next_message(receiver: &Arc<RwLock<UnboundedReceiver<String>>>) -> Option<String> {
        let mut receiver_guard = receiver.write().ok()?;
        receiver_guard.next().await
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

        let sender = self.sender.clone();
        let receiver = self.receiver.clone();
        let peer_manager = self.peer_manager.clone();

        spawn_local(async move {
            Self::run_socket_loop(socket, setup_future, sender, receiver, peer_manager).await;
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
        let receiver = self.receiver.clone();
        let callback = Arc::new(callback);

        spawn_local(async move {
            let mut receiver_guard = receiver.write().unwrap();
            while let Some(message) = receiver_guard.next().await {
                callback(message);
            }
        });
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
