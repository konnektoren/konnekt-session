#[allow(dead_code)]
mod support;

use ed25519_dalek::SigningKey;
use konnekt_session_core::{
    ActivityConfig, DomainCommand, DomainLoop, RunStatus, domain::ActivityResult,
};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::application::ConnectionEvent;
use konnekt_session_p2p::application::persistence::{MemoryStore, PersistenceError};
use konnekt_session_p2p::application::signed_history::verify_history;
use konnekt_session_p2p::domain::participant_id;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use konnekt_session_p2p::protocol::join::JoinSessionState;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use support::mock_connection::{MockConnection, MockNetwork, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;
const HOST: [u8; 32] = [61; 32];
const GUESTS: [[u8; 32]; 2] = [[62; 32], [63; 32]];

fn id_of(secret: [u8; 32]) -> Uuid {
    participant_id(&SigningKey::from_bytes(&secret).verifying_key().to_bytes())
}

fn new_host(
    network: &Arc<Mutex<MockNetwork>>,
    lobby_id: Uuid,
    secret: [u8; 32],
    store: &MemoryStore,
) -> Loop {
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 100),
        true,
        lobby_id,
    );
    host.enable_authentication(secret)
        .unwrap_or_else(|error| panic!("{error}"));
    host.enable_signed_history(SigningKey::from_bytes(&secret), 50)
        .unwrap_or_else(|error| panic!("{error}"));
    host.enable_persistence(Box::new(store.clone()));
    host
}

fn new_guest(
    network: &Arc<Mutex<MockNetwork>>,
    lobby_id: Uuid,
    secret: [u8; 32],
    store: &MemoryStore,
) -> Loop {
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(MockConnection::new(network.clone()), 100),
        false,
        lobby_id,
    );
    guest
        .enable_authentication(secret)
        .unwrap_or_else(|error| panic!("{error}"));
    guest.enable_persistence(Box::new(store.clone()));
    guest
        .join("Guest".into())
        .unwrap_or_else(|error| panic!("{error}"));
    guest
}

struct World {
    network: Arc<Mutex<MockNetwork>>,
    lobby_id: Uuid,
    host_store: MemoryStore,
    guest_stores: Vec<MemoryStore>,
    host: Loop,
    guests: Vec<Loop>,
    run_id: Uuid,
}

impl World {
    /// A host and two guests; one activity queued and a run in progress in
    /// which the first guest already submitted.
    fn new() -> Self {
        let network = create_mock_network();
        let lobby_id = Uuid::new_v4();
        let host_store = MemoryStore::new();
        let guest_stores = vec![MemoryStore::new(), MemoryStore::new()];
        let mut host = new_host(&network, lobby_id, HOST, &host_store);
        host.submit_command(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Room".into(),
            host_name: "Host".into(),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        let guests = (0..2)
            .map(|index| new_guest(&network, lobby_id, GUESTS[index], &guest_stores[index]))
            .collect();
        let mut world = Self {
            network,
            lobby_id,
            host_store,
            guest_stores,
            host,
            guests,
            run_id: Uuid::nil(),
        };
        world.tick(30);
        for name in ["Quiz", "Kept"] {
            world
                .host
                .submit_command(DomainCommand::QueueActivity {
                    lobby_id,
                    config: ActivityConfig::new(
                        "quiz".into(),
                        name.into(),
                        serde_json::Value::Null,
                    ),
                })
                .unwrap_or_else(|error| panic!("{error}"));
        }
        world
            .host
            .submit_command(DomainCommand::StartNextRun { lobby_id })
            .unwrap_or_else(|error| panic!("{error}"));
        world.tick(5);
        world.run_id = world
            .host
            .get_active_run()
            .unwrap_or_else(|| panic!("no run"))
            .id();
        world.submit(0);
        world.tick(5);
        world
    }

    fn tick(&mut self, count: usize) {
        for _ in 0..count {
            self.host.poll();
            for guest in &mut self.guests {
                guest.poll();
            }
        }
    }

    fn submit(&mut self, guest: usize) {
        let me = id_of(GUESTS[guest]);
        self.guests[guest]
            .submit_command(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                run_id: self.run_id,
                result: ActivityResult::new(self.run_id, me),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }

    fn drop_peer(&self, session: &Loop) {
        let gone = session.local_peer_id().unwrap_or_else(|| panic!("no peer"));
        let mut network = self.network.lock().unwrap_or_else(|e| panic!("{e}"));
        network.peers.remove(&gone);
        let others: Vec<_> = network.peers.keys().copied().collect();
        for other in others {
            network
                .events
                .push_back((other, ConnectionEvent::PeerDisconnected(gone)));
        }
    }
}

#[test]
fn a_reloaded_host_reopens_its_lobby_and_continues_its_signed_history() {
    let mut world = World::new();
    let old_tip = world
        .host
        .signed_history()
        .and_then(|history| history.tip())
        .unwrap_or_else(|| panic!("no history"));
    world.drop_peer(&world.host);

    // Reload: new session, same key, state from the store.
    let state = world
        .host_store
        .state(world.lobby_id)
        .unwrap_or_else(|| panic!("nothing stored"))
        .unwrap_or_else(|error| panic!("{error}"));
    world.host = new_host(&world.network, world.lobby_id, HOST, &world.host_store);
    world
        .host
        .restore_state(state)
        .unwrap_or_else(|error| panic!("{error}"));
    let lobby = world.host.get_lobby().unwrap_or_else(|| panic!("no lobby"));
    assert_eq!(lobby.host_id(), id_of(HOST));
    assert_eq!(lobby.participants().len(), 3);
    assert_eq!(lobby.activity_queue().len(), 1);
    let run = world
        .host
        .get_active_run()
        .unwrap_or_else(|| panic!("run lost on reload"));
    assert!(run.results().contains_key(&id_of(GUESTS[0])));

    // Guests follow the reloaded host and finish the run with it.
    world.tick(80);
    for guest in &world.guests {
        assert_eq!(guest.join_state(), Some(JoinSessionState::Ready));
        assert_eq!(
            guest.get_lobby().map(|lobby| lobby.host_id()),
            Some(id_of(HOST))
        );
    }
    world.submit(1);
    let run_id = world.run_id;
    world
        .host
        .submit_command(DomainCommand::SubmitResult {
            lobby_id: world.lobby_id,
            run_id,
            result: ActivityResult::new(run_id, id_of(HOST)),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    world.tick(10);
    for session in std::iter::once(&world.host).chain(&world.guests) {
        assert_eq!(
            session.get_run(&run_id).map(|run| run.status()),
            Some(RunStatus::Completed)
        );
    }

    // The signed chain continued from before the reload.
    let history = world
        .host
        .signed_history()
        .unwrap_or_else(|| panic!("no history"));
    let events: Vec<_> = history.events().iter().cloned().collect();
    assert!(
        history
            .tip()
            .is_some_and(|tip| tip.sequence > old_tip.sequence)
    );
    let host_key = SigningKey::from_bytes(&HOST).verifying_key().to_bytes();
    assert!(verify_history(world.lobby_id, &events, &[host_key], history.anchor()).is_ok());
    assert!(
        events
            .iter()
            .any(|event| event.sequence == old_tip.sequence)
    );
}

#[test]
fn a_guest_reopens_a_lobby_nobody_answers_in_as_its_host() {
    let mut world = World::new();
    world.drop_peer(&world.host);
    world.drop_peer(&world.guests[1]);
    let reopener = world.guests.remove(0);
    world.drop_peer(&reopener);
    world.guests.clear();

    let state = world.guest_stores[0]
        .state(world.lobby_id)
        .unwrap_or_else(|| panic!("guest stored nothing"))
        .unwrap_or_else(|error| panic!("{error}"));
    world.host = new_host(
        &world.network,
        world.lobby_id,
        GUESTS[0],
        &world.guest_stores[0],
    );
    world
        .host
        .restore_state(state)
        .unwrap_or_else(|error| panic!("{error}"));
    let lobby = world.host.get_lobby().unwrap_or_else(|| panic!("no lobby"));
    assert_eq!(lobby.host_id(), id_of(GUESTS[0]));
    assert!(lobby.participants().contains_key(&id_of(HOST)));
    assert_eq!(lobby.participants().len(), 3);

    // The other guest comes back as the same participant.
    world.guests.push(new_guest(
        &world.network,
        world.lobby_id,
        GUESTS[1],
        &world.guest_stores[1],
    ));
    world.tick(80);
    assert_eq!(world.guests[0].join_state(), Some(JoinSessionState::Ready));
    assert_eq!(
        world
            .host
            .get_lobby()
            .map(|lobby| lobby.participants().len()),
        Some(3)
    );
}

#[test]
fn after_a_restore_the_run_stops_waiting_for_players_who_do_not_return() {
    let mut world = World::new();
    world.drop_peer(&world.host);
    for guest in &world.guests {
        world.drop_peer(guest);
    }
    world.guests.clear();
    let state = world
        .host_store
        .state(world.lobby_id)
        .unwrap_or_else(|| panic!("nothing stored"))
        .unwrap_or_else(|error| panic!("{error}"));
    world.host = new_host(&world.network, world.lobby_id, HOST, &world.host_store);
    assert!(world.host.set_host_grace_period(Duration::from_millis(20)));
    world
        .host
        .restore_state(state)
        .unwrap_or_else(|error| panic!("{error}"));
    let absent = id_of(GUESTS[1]);
    world.host.poll();
    assert!(
        world
            .host
            .get_active_run()
            .is_some_and(|run| run.required_submitters().contains(&absent)),
        "players get the grace period to come back"
    );
    std::thread::sleep(Duration::from_millis(40));
    world.host.poll();
    world.host.poll();
    assert!(
        world
            .host
            .get_run(&world.run_id)
            .is_some_and(|run| !run.required_submitters().contains(&absent))
    );
}

#[test]
fn only_participants_can_reopen_a_lobby_and_only_before_it_exists() {
    let world = World::new();
    let state = || {
        world
            .host_store
            .state(world.lobby_id)
            .unwrap_or_else(|| panic!("nothing stored"))
            .unwrap_or_else(|error| panic!("{error}"))
    };
    let stranger_store = MemoryStore::new();
    let mut stranger = new_host(&world.network, world.lobby_id, [70; 32], &stranger_store);
    assert!(matches!(
        stranger.restore_state(state()),
        Err(PersistenceError::NotAParticipant)
    ));
    let mut other_lobby = new_host(&world.network, Uuid::new_v4(), HOST, &stranger_store);
    assert!(matches!(
        other_lobby.restore_state(state()),
        Err(PersistenceError::WrongLobby)
    ));
    let mut host = new_host(&world.network, world.lobby_id, HOST, &stranger_store);
    assert!(host.restore_state(state()).is_ok());
    assert!(matches!(
        host.restore_state(state()),
        Err(PersistenceError::LobbyExists)
    ));
}
