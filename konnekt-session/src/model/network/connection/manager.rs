pub trait PeerId: Clone + std::fmt::Debug {}

pub trait ConnectionManager {
    type Peer: PeerId;

    fn add_peer(&self, peer: Self::Peer);
    fn remove_peer(&self, peer: &Self::Peer);
    fn get_connected_peers(&self) -> Vec<Self::Peer>;
    fn is_peer_connected(&self, peer: &Self::Peer) -> bool;
}
