use futures::channel::mpsc::UnboundedReceiver;
use futures::stream::{SplitSink, SplitStream};
use std::fmt::Debug;
use std::sync::{Arc, RwLock};

pub trait PeerId: Clone + Debug {}

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
