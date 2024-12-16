use futures::channel::mpsc::UnboundedSender;
use std::fmt::Debug;

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
