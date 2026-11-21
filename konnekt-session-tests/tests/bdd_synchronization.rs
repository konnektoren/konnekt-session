use cucumber::{World, given, then, when};
use konnekt_session_core::{ActivityConfig, DomainCommand, DomainLoop};
use konnekt_session_p2p::{
    ConnectionEvent, SessionLoopV2,
    domain::PeerId,
    infrastructure::{
        error::{P2PError, Result},
        message::P2PMessage,
        transport::{NetworkConnection, P2PTransport},
    },
    protocol::synchronization::StateSynchronizationState,
};
use std::fmt;
use uuid::Uuid;

#[path = "../../konnekt-session-p2p/tests/support/mock_connection.rs"]
mod mock_connection;
use mock_connection::{MockConnection, create_mock_network};

impl NetworkConnection for MockConnection {
    fn local_peer_id(&self) -> Option<PeerId> {
        MockConnection::local_peer_id(self)
    }
    fn connected_peers(&self) -> Vec<PeerId> {
        MockConnection::connected_peers(self)
    }
    fn send_to(&mut self, peer: PeerId, data: Vec<u8>) -> Result<()> {
        MockConnection::send_to(self, peer, data).map_err(P2PError::SendFailed)
    }
    fn broadcast(&mut self, data: Vec<u8>) -> Result<()> {
        MockConnection::broadcast(self, data).map_err(P2PError::SendFailed)
    }
    fn poll_events(&mut self) -> Vec<ConnectionEvent> {
        MockConnection::poll_events(self)
    }
}

struct Fixture {
    host: MockConnection,
    guest: SessionLoopV2<MockConnection>,
    guest_id: PeerId,
    lobby_id: Uuid,
}

impl fmt::Debug for Fixture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Fixture").finish_non_exhaustive()
    }
}

impl Fixture {
    fn send(&mut self, sequence: u64) {
        let command = DomainCommand::QueueActivity {
            lobby_id: self.lobby_id,
            config: ActivityConfig::new(
                "test".into(),
                format!("Activity {sequence}"),
                serde_json::Value::Null,
            ),
        };
        let payload = serde_json::to_value(command).unwrap_or_else(|error| panic!("{error}"));
        let mut message = P2PMessage::application(payload);
        message.sequence = sequence;
        let data = serde_json::to_vec(&message).unwrap_or_else(|error| panic!("{error}"));
        if let Err(error) = self.host.send_to(self.guest_id, data) {
            panic!("{error}");
        }
        self.guest.poll();
    }
}

#[derive(Debug, Default, World)]
struct SyncWorld {
    fixture: Option<Fixture>,
}

#[given("a guest has synchronized with the host")]
fn synchronized(world: &mut SyncWorld) {
    let network = create_mock_network();
    let host = MockConnection::new(network.clone());
    let host_sender = host.clone();
    let guest = MockConnection::new(network);
    let guest_id = guest
        .local_peer_id()
        .unwrap_or_else(|| panic!("missing guest peer"));
    let lobby_id = Uuid::new_v4();
    let mut domain = DomainLoop::new(10, 100);
    if let Err(error) = domain.submit(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name: "Room".into(),
        host_name: "Host".into(),
    }) {
        panic!("{error}");
    }
    domain.poll();
    let mut host_loop =
        SessionLoopV2::new(domain, P2PTransport::new_host(host, 100), true, lobby_id);
    let mut guest_loop = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(guest, 100),
        false,
        lobby_id,
    );
    for _ in 0..5 {
        host_loop.poll();
        guest_loop.poll();
    }
    assert_eq!(
        guest_loop.synchronization_state(),
        Some(StateSynchronizationState::Current)
    );
    world.fixture = Some(Fixture {
        host: host_sender,
        guest: guest_loop,
        guest_id,
        lobby_id,
    });
}

#[when("a later host update arrives before an earlier update")]
#[given("a later host update arrived before an earlier update")]
fn gap(world: &mut SyncWorld) {
    if let Some(fixture) = &mut world.fixture {
        fixture.send(2);
    } else {
        panic!("missing session");
    }
}

#[when("the missing host update arrives")]
fn recovered(world: &mut SyncWorld) {
    if let Some(fixture) = &mut world.fixture {
        fixture.send(1);
    } else {
        panic!("missing session");
    }
}

#[then("the guest is recovering its session")]
fn recovering(world: &mut SyncWorld) {
    assert_eq!(
        world
            .fixture
            .as_ref()
            .and_then(|f| f.guest.synchronization_state()),
        Some(StateSynchronizationState::Recovering)
    );
    if let Some(lobby) = world.fixture.as_ref().and_then(|f| f.guest.get_lobby()) {
        assert!(lobby.activity_queue().is_empty());
    } else {
        panic!("missing guest lobby");
    }
}

#[then("the guest is current again")]
fn current(world: &mut SyncWorld) {
    assert_eq!(
        world
            .fixture
            .as_ref()
            .and_then(|f| f.guest.synchronization_state()),
        Some(StateSynchronizationState::Current)
    );
    if let Some(lobby) = world.fixture.as_ref().and_then(|f| f.guest.get_lobby()) {
        let names: Vec<_> = lobby
            .activity_queue()
            .iter()
            .map(|activity| activity.name.as_str())
            .collect();
        assert_eq!(names, ["Activity 1", "Activity 2"]);
    } else {
        panic!("missing guest lobby");
    }
}

#[tokio::main]
async fn main() {
    SyncWorld::cucumber()
        .run_and_exit("tests/synchronization_features")
        .await;
}
