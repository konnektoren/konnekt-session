use cucumber::{World, given, then, when};
use konnekt_session_core::{ActivityConfig, DomainCommand, DomainLoop};
use konnekt_session_p2p::{
    ConnectionEvent, SessionLoopV2,
    domain::{MatchboxPeerId, PeerId, PeerRegistry},
    infrastructure::{
        error::{P2PError, Result},
        message::P2PMessage,
        transport::{NetworkConnection, P2PTransport},
    },
    protocol::{command_outcome::CommandOutcomeState, synchronization::StateSynchronizationState},
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
    host_loop: SessionLoopV2<MockConnection>,
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
    request: Option<Uuid>,
    grace: Option<(PeerRegistry, PeerId)>,
    timed_out: Vec<PeerId>,
    asset: Option<AssetFixture>,
}

struct AssetFixture {
    sender: konnekt_session_core::application::asset_transfer::AssetSender,
    receiver: konnekt_session_core::application::asset_transfer::AssetReceiver,
}

impl fmt::Debug for AssetFixture {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AssetFixture").finish_non_exhaustive()
    }
}

#[given("a player has an asset to share")]
fn asset_to_share(world: &mut SyncWorld) {
    use konnekt_session_core::application::asset_transfer::{
        AssetReceiver, AssetSender, TransferLimits,
    };
    let limits = TransferLimits {
        max_bytes: 32,
        chunk_bytes: 3,
    };
    let sender = AssetSender::new(b"music".to_vec(), limits).unwrap_or_else(|e| panic!("{e}"));
    let receiver =
        AssetReceiver::new(sender.chunks().len(), limits).unwrap_or_else(|e| panic!("{e}"));
    world.asset = Some(AssetFixture { sender, receiver });
}

#[when("the asset arrives out of order with a repeated chunk")]
fn receive_asset(world: &mut SyncWorld) {
    let Some(asset) = &mut world.asset else {
        panic!("missing asset")
    };
    let chunks: Vec<_> = asset.sender.chunks().map(<[u8]>::to_vec).collect();
    assert_eq!(asset.receiver.receive(1, chunks[1].clone()), Ok(false));
    assert_eq!(asset.receiver.receive(1, chunks[1].clone()), Ok(false));
    assert_eq!(asset.receiver.receive(0, chunks[0].clone()), Ok(true));
    assert!(asset.receiver.acknowledge().is_ok());
    assert!(asset.sender.chunks_complete().is_ok());
    assert!(asset.sender.acknowledge().is_ok());
}

#[then("both players complete the transfer with the original asset")]
fn asset_complete(world: &mut SyncWorld) {
    use konnekt_session_core::protocol::asset_transfer::AssetTransferState;
    let Some(asset) = &world.asset else {
        panic!("missing asset")
    };
    assert_eq!(asset.receiver.asset().as_deref(), Some(&b"music"[..]));
    assert_eq!(asset.receiver.state(), AssetTransferState::Completed);
    assert_eq!(asset.sender.state(), AssetTransferState::Completed);
}

#[when("the receiving player cancels the transfer")]
fn cancel_asset(world: &mut SyncWorld) {
    let Some(asset) = &mut world.asset else {
        panic!("missing asset")
    };
    assert!(asset.receiver.cancel().is_ok());
}

#[then("later chunks cannot complete the asset")]
fn cancelled_asset(world: &mut SyncWorld) {
    use konnekt_session_core::application::asset_transfer::TransferError;
    let Some(asset) = &mut world.asset else {
        panic!("missing asset")
    };
    assert_eq!(
        asset.receiver.receive(0, b"mus".to_vec()),
        Err(TransferError::InvalidState)
    );
    assert!(asset.receiver.asset().is_none());
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
        host_loop,
        guest: guest_loop,
        guest_id,
        lobby_id,
    });
}

#[when("the host queues an activity")]
fn host_queues_activity(world: &mut SyncWorld) {
    let Some(fixture) = &mut world.fixture else {
        panic!("missing session");
    };
    let command = DomainCommand::QueueActivity {
        lobby_id: fixture.lobby_id,
        config: ActivityConfig::new("test".into(), "Activity".into(), serde_json::Value::Null),
    };
    world.request = Some(
        fixture
            .host_loop
            .submit_tracked_command(command)
            .unwrap_or_else(|error| panic!("{error}")),
    );
}

#[when("the host starts a session with no planned activity")]
fn host_starts_empty_session(world: &mut SyncWorld) {
    let Some(fixture) = &mut world.fixture else {
        panic!("missing session");
    };
    let command = DomainCommand::StartNextRun {
        lobby_id: fixture.lobby_id,
    };
    world.request = Some(
        fixture
            .host_loop
            .submit_tracked_command(command)
            .unwrap_or_else(|error| panic!("{error}")),
    );
    fixture.host_loop.poll();
}

#[when("the host queues and processes an activity")]
fn host_processes_activity(world: &mut SyncWorld) {
    host_queues_activity(world);
    if let Some(fixture) = &mut world.fixture {
        fixture.host_loop.poll();
    }
}

fn command_state(world: &SyncWorld) -> Option<(CommandOutcomeState, Option<&str>)> {
    world
        .fixture
        .as_ref()?
        .host_loop
        .command_outcome(world.request?)
}

#[then("the host request is pending")]
fn request_pending(world: &mut SyncWorld) {
    assert_eq!(
        command_state(world),
        Some((CommandOutcomeState::Pending, None))
    );
}

#[then("the host request is rejected with a reason")]
fn request_rejected(world: &mut SyncWorld) {
    let Some((state, reason)) = command_state(world) else {
        panic!("missing request");
    };
    assert_eq!(state, CommandOutcomeState::Rejected);
    assert!(reason.is_some_and(|value| !value.is_empty()));
}

#[then("the host request is accepted")]
fn request_accepted(world: &mut SyncWorld) {
    assert_eq!(
        command_state(world),
        Some((CommandOutcomeState::Accepted, None))
    );
}

#[when("both players exchange identity proofs")]
fn exchange_identity_proofs(world: &mut SyncWorld) {
    let Some(fixture) = &mut world.fixture else {
        panic!("missing session");
    };
    fixture.host_loop.enable_identity_exchange([1; 32]);
    fixture.guest.enable_identity_exchange([2; 32]);
    for _ in 0..8 {
        fixture.host_loop.poll();
        fixture.guest.poll();
    }
}

#[then("each player can see the peer's proven key")]
fn peer_keys_are_verified(world: &mut SyncWorld) {
    let Some(fixture) = &world.fixture else {
        panic!("missing session");
    };
    let Some(host_peer) = fixture.host_loop.local_peer_id() else {
        panic!("missing host peer");
    };
    assert!(
        fixture
            .host_loop
            .verified_peer_identity(fixture.guest_id)
            .is_some()
    );
    assert!(fixture.guest.verified_peer_identity(host_peer).is_some());
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

#[given("a connected peer enters the grace period")]
fn peer_enters_grace(world: &mut SyncWorld) {
    let peer = PeerId::new(MatchboxPeerId(Uuid::new_v4()));
    let mut registry = PeerRegistry::with_grace_period(std::time::Duration::ZERO);
    registry.add_peer(peer);
    registry.mark_peer_disconnected(&peer);
    world.grace = Some((registry, peer));
}

#[when("the peer reconnects before the grace period is checked")]
fn peer_reconnects(world: &mut SyncWorld) {
    let (registry, peer) = world
        .grace
        .as_mut()
        .unwrap_or_else(|| panic!("missing peer"));
    registry.add_peer(*peer);
    world.timed_out = registry.check_grace_periods();
}

#[when("the disconnected peer's grace period expires")]
fn peer_grace_expires(world: &mut SyncWorld) {
    let (registry, _) = world
        .grace
        .as_mut()
        .unwrap_or_else(|| panic!("missing peer"));
    world.timed_out = registry.check_grace_periods();
}

#[then("the peer remains connected")]
fn peer_remains_connected(world: &mut SyncWorld) {
    let (registry, peer) = world
        .grace
        .as_ref()
        .unwrap_or_else(|| panic!("missing peer"));
    assert!(world.timed_out.is_empty());
    assert!(
        registry
            .get_peer(peer)
            .is_some_and(|p| !p.is_disconnected())
    );
}

#[then("the peer is reported timed out once")]
fn peer_times_out_once(world: &mut SyncWorld) {
    let (registry, peer) = world
        .grace
        .as_mut()
        .unwrap_or_else(|| panic!("missing peer"));
    assert_eq!(world.timed_out, vec![*peer]);
    assert!(registry.check_grace_periods().is_empty());
}

#[tokio::main]
async fn main() {
    SyncWorld::cucumber()
        .run_and_exit("tests/synchronization_features")
        .await;
}
