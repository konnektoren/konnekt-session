use crate::application::ConnectionEvent;
use crate::domain::PeerId;
use crate::infrastructure::error::Result;

/// Trait for P2P connection (allows mocking in tests)
pub trait Connection {
    fn local_peer_id(&self) -> Option<PeerId>;
    fn connected_peers(&self) -> Vec<PeerId>;
    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()>;
    fn broadcast(&mut self, data: Vec<u8>) -> Result<()>;
    fn poll_events(&mut self) -> Vec<ConnectionEvent>;
}

// Implement for MatchboxConnection
impl Connection for super::connection::MatchboxConnection {
    fn local_peer_id(&self) -> Option<PeerId> {
        super::connection::MatchboxConnection::local_peer_id(self)
    }

    fn connected_peers(&self) -> Vec<PeerId> {
        super::connection::MatchboxConnection::connected_peers(self)
    }

    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        super::connection::MatchboxConnection::send_to(self, peer, data)
    }

    fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        super::connection::MatchboxConnection::broadcast(self, data)
    }

    fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        super::connection::MatchboxConnection::poll_events(self)
    }
}
