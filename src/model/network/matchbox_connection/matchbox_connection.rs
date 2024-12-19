use super::super::{MessageCallback, NetworkError, Transport, TransportType};
use super::MatchboxConnectionManager;
use crate::model::network::connection::ConnectionManager;
use crate::model::{ClientId, LobbyId, Role};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::stream::AbortHandle;
use futures::Future;
use futures::{select, FutureExt, StreamExt};
use futures_timer::Delay;
use matchbox_socket::{PeerState, WebRtcSocket};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use wasm_bindgen_futures::spawn_local;

const CHANNEL_ID: usize = 0;
const DELAY_MS: u64 = 20;

type MessagePacket = Box<[u8]>;

#[derive(Clone)]
pub struct MatchboxConnection {
    signaling_url: String,
    client_id: ClientId,
    lobby_id: LobbyId,
    role: Role,
    sender: UnboundedSender<String>,
    receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    bridge_sender: UnboundedSender<String>,
    bridge_receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    connected: Arc<RwLock<bool>>,
    peer_manager: Arc<MatchboxConnectionManager>,
    socket_handle: Arc<RwLock<Option<AbortHandle>>>,
}

impl MatchboxConnection {
    pub fn new(signaling_url: String, client_id: ClientId, lobby_id: LobbyId, role: Role) -> Self {
        let (sender, receiver) = mpsc::unbounded();

        let (bridge_sender, bridge_receiver) = mpsc::unbounded();

        Self {
            signaling_url,
            client_id,
            lobby_id,
            role,
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            connected: Arc::new(RwLock::new(false)),
            peer_manager: Arc::new(MatchboxConnectionManager::new()),
            socket_handle: Arc::new(RwLock::new(None)),
            bridge_sender,
            bridge_receiver: Arc::new(RwLock::new(bridge_receiver)),
        }
    }

    async fn run_socket_loop(
        mut socket: WebRtcSocket,
        setup_future: impl Future<Output = Result<(), matchbox_socket::Error>>,
        peer_manager: Arc<MatchboxConnectionManager>,
        receiver: Arc<RwLock<UnboundedReceiver<String>>>,
        bridge_sender: UnboundedSender<String>,
        connected: Arc<RwLock<bool>>,
    ) {
        let setup_fut = setup_future.fuse();
        futures::pin_mut!(setup_fut);

        loop {
            let delay = Delay::new(Duration::from_millis(DELAY_MS)).fuse();
            futures::pin_mut!(delay);

            // Handle outgoing messages from the application
            if let Ok(mut receiver_guard) = receiver.write() {
                if let Some(message) = receiver_guard.next().now_or_never() {
                    if let Some(message) = message {
                        let packet = message.as_bytes().to_vec().into_boxed_slice();
                        let peers: Vec<_> = socket.connected_peers().collect();
                        for peer in peers {
                            socket.channel_mut(CHANNEL_ID).send(packet.clone(), peer);
                        }
                    }
                }
            }

            // Handle peer updates
            for (peer, state) in socket.update_peers() {
                match state {
                    PeerState::Connected => {
                        log::info!("Peer joined: {peer}");
                        if let Ok(mut peers) = peer_manager.peers.write() {
                            peers.insert(peer.clone(), true);
                        }
                        peer_manager.add_peer(peer.clone());
                        let packet = "I'm a teapot!".as_bytes().to_vec().into_boxed_slice();
                        socket.channel_mut(CHANNEL_ID).send(packet, peer);

                        *connected.write().unwrap() = true;
                    }
                    PeerState::Disconnected => {
                        log::info!("Peer left: {peer}");
                        peer_manager.remove_peer(&peer);
                    }
                }
            }

            // Handle incoming messages from WebRTC and forward to bridge
            for (peer, packet) in socket.channel_mut(CHANNEL_ID).receive() {
                if let Ok(text) = String::from_utf8(packet.to_vec()) {
                    if let Err(e) = bridge_sender.unbounded_send(text) {
                        log::error!("Failed to forward message to bridge: {}", e);
                    }
                }
            }

            select! {
                _ = &mut delay => {},
                _ = &mut setup_fut => break,
            }
        }
    }

    // Add a method to spawn the bridge receive task
    fn spawn_bridge_receive_task(&self, callback: MessageCallback) {
        let bridge_receiver = self.bridge_receiver.clone();

        spawn_local(async move {
            if let Ok(mut receiver) = bridge_receiver.write() {
                while let Some(text) = receiver.next().await {
                    callback(text);
                }
            }
        });
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
        let (socket, setup_future) = WebRtcSocket::new_reliable(&room_url);

        *self.connected.write().unwrap() = true;
        log::info!("Connecting to signaling server: {}", room_url);

        let peer_manager = self.peer_manager.clone();
        let receiver = self.receiver.clone();
        let bridge_sender = self.bridge_sender.clone();
        let connected = self.connected.clone();

        let (future, handle) = futures::future::abortable(async move {
            Self::run_socket_loop(
                socket,
                setup_future,
                peer_manager,
                receiver,
                bridge_sender,
                connected,
            )
            .await;
        });

        *self.socket_handle.write().unwrap() = Some(handle);
        *self.connected.write().unwrap() = true;

        spawn_local(async move {
            let _ = future.await;
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
        self.spawn_bridge_receive_task(callback);
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
