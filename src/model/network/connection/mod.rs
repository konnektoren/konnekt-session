use super::MessageCallback;
use futures::channel::mpsc::{UnboundedReceiver, UnboundedSender};
use futures::stream::{SplitSink, SplitStream};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

pub trait PeerId: Clone + Debug + Eq + std::hash::Hash {}

pub trait ConnectionReader {
    type Peer: PeerId;
    fn spawn_read_task(&self, message_sender: UnboundedSender<(Self::Peer, String)>);
}

pub trait ConnectionWriter {
    type Peer: PeerId;
    fn spawn_write_task(&self, peer: Self::Peer);
}

pub trait ConnectionManager {
    type Peer: PeerId;

    fn add_peer(&self, peer: Self::Peer);
    fn remove_peer(&self, peer: &Self::Peer);
    fn get_connected_peers(&self) -> Vec<Self::Peer>;
    fn is_peer_connected(&self, peer: &Self::Peer) -> bool;
}

pub trait ConnectionHandler {
    type InternMessageType: Clone + Debug;
    type ExternMessageType: Clone + Debug;
    type SocketType: Sized;
    type CallbackType: Sized;

    fn take_socket(&self) -> Option<Self::SocketType>;

    fn receiver(&self) -> Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>;

    async fn next_message(
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    ) -> Option<Self::InternMessageType>;

    fn spawn_send_task(
        &self,
        sender: SplitSink<Self::SocketType, Self::ExternMessageType>,
        receiver: Arc<RwLock<UnboundedReceiver<Self::InternMessageType>>>,
    );

    fn spawn_receive_task(
        &self,
        receiver: SplitStream<Self::SocketType>,
        callback: Arc<Self::CallbackType>,
    );
}
