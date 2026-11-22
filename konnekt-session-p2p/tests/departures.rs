#[allow(dead_code)]
mod support;

use konnekt_session_core::{
    ActivityConfig, DomainCommand, DomainLoop, RunStatus, domain::ActivityResult,
};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::application::ConnectionEvent;
use konnekt_session_p2p::domain::PeerId;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use konnekt_session_p2p::protocol::join::JoinSessionState;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use support::mock_connection::{MockConnection, MockNetwork, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;
type Inbox = Arc<Mutex<VecDeque<(PeerId, Vec<u8>)>>>;

struct Lobby {
    network: Arc<Mutex<MockNetwork>>,
    lobby_id: Uuid,
    host: Loop,
    guests: Vec<Loop>,
    secrets: Option<Vec<[u8; 32]>>,
}

impl Lobby {
    fn new(authenticated: bool) -> Self {
        let network = create_mock_network();
        let lobby_id = Uuid::new_v4();
        let mut host = SessionLoopV2::new(
            DomainLoop::new(10, 100),
            P2PTransport::new_host(MockConnection::new(network.clone()), 100),
            true,
            lobby_id,
        );
        let secrets = authenticated.then(|| vec![[51; 32], [52; 32]]);
        if authenticated {
            host.enable_authentication([50; 32])
                .unwrap_or_else(|error| panic!("{error}"));
        }
        host.submit_command(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Room".into(),
            host_name: "Host".into(),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        let mut lobby = Self {
            network,
            lobby_id,
            host,
            guests: Vec::new(),
            secrets,
        };
        for index in 0..2 {
            let guest = lobby.new_guest(index);
            lobby.guests.push(guest);
            lobby.tick(20);
        }
        lobby
    }

    fn new_guest(&self, index: usize) -> Loop {
        let mut guest = SessionLoopV2::new(
            DomainLoop::new(10, 100),
            P2PTransport::new_guest(MockConnection::new(self.network.clone()), 100),
            false,
            self.lobby_id,
        );
        if let Some(secrets) = &self.secrets {
            guest
                .enable_authentication(secrets[index])
                .unwrap_or_else(|error| panic!("{error}"));
        }
        guest
            .join(format!("Guest{index}"))
            .unwrap_or_else(|error| panic!("{error}"));
        guest
    }

    fn tick(&mut self, count: usize) {
        for _ in 0..count {
            self.host.poll();
            for guest in &mut self.guests {
                guest.poll();
            }
        }
    }

    fn peer(&self, guest: usize) -> PeerId {
        self.guests[guest]
            .local_peer_id()
            .unwrap_or_else(|| panic!("no peer"))
    }

    fn participant(&self, guest: usize) -> Uuid {
        self.guests[guest]
            .own_participant_id()
            .unwrap_or_else(|| panic!("not joined"))
    }

    /// Cut a guest's connection in both directions.
    fn disconnect(&mut self, guest: usize) -> Inbox {
        let gone = self.peer(guest);
        let mut network = self.network.lock().unwrap_or_else(|e| panic!("{e}"));
        let inbox = network
            .peers
            .remove(&gone)
            .unwrap_or_else(|| panic!("not connected"));
        let others: Vec<_> = network.peers.keys().copied().collect();
        for other in others {
            network
                .events
                .push_back((other, ConnectionEvent::PeerDisconnected(gone)));
            network
                .events
                .push_back((gone, ConnectionEvent::PeerDisconnected(other)));
        }
        inbox
    }

    fn reconnect(&mut self, guest: usize, inbox: Inbox) {
        let peer = self.peer(guest);
        let mut network = self.network.lock().unwrap_or_else(|e| panic!("{e}"));
        let others: Vec<_> = network.peers.keys().copied().collect();
        network.peers.insert(peer, inbox);
        for other in others {
            network
                .events
                .push_back((other, ConnectionEvent::PeerConnected(peer)));
            network
                .events
                .push_back((peer, ConnectionEvent::PeerConnected(other)));
        }
    }

    /// Queue and start a run; returns its ID and required submitters.
    fn start_run(&mut self, name: &str) -> (Uuid, Vec<Uuid>) {
        self.host
            .submit_command(DomainCommand::QueueActivity {
                lobby_id: self.lobby_id,
                config: ActivityConfig::new("quiz".into(), name.into(), serde_json::Value::Null),
            })
            .unwrap_or_else(|error| panic!("{error}"));
        self.host
            .submit_command(DomainCommand::StartNextRun {
                lobby_id: self.lobby_id,
            })
            .unwrap_or_else(|error| panic!("{error}"));
        self.tick(5);
        let run = self
            .host
            .get_active_run()
            .unwrap_or_else(|| panic!("no run"));
        (
            run.id(),
            run.required_submitters().iter().copied().collect(),
        )
    }

    fn host_submits(&mut self, run_id: Uuid) {
        let host_id = self
            .host
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby"))
            .host_id();
        self.host
            .submit_command(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                run_id,
                result: ActivityResult::new(run_id, host_id),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }

    fn guest_submits(&mut self, guest: usize, run_id: Uuid) {
        let me = self.participant(guest);
        self.guests[guest]
            .submit_command(DomainCommand::SubmitResult {
                lobby_id: self.lobby_id,
                run_id,
                result: ActivityResult::new(run_id, me),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }

    fn participants(&self) -> usize {
        self.host
            .get_lobby()
            .map(|lobby| lobby.participants().len())
            .unwrap_or_default()
    }
}

#[test]
fn the_host_does_not_wait_for_a_player_who_left_and_relinks_them_on_return() {
    let mut lobby = Lobby::new(false);
    let leaver = lobby.participant(1);
    assert!(lobby.host.connected_participants().contains(&leaver));
    let (run_id, required) = lobby.start_run("Quiz");
    assert!(required.contains(&leaver));
    lobby.guest_submits(0, run_id);
    lobby.tick(5);

    let inbox = lobby.disconnect(1);
    lobby.tick(5);
    let run = lobby
        .host
        .get_run(&run_id)
        .unwrap_or_else(|| panic!("no run"));
    assert!(!run.required_submitters().contains(&leaver));
    assert_eq!(lobby.participants(), 3, "the player stays in the lobby");
    assert!(!lobby.host.connected_participants().contains(&leaver));

    lobby.host_submits(run_id);
    lobby.tick(5);
    for session in [&lobby.host, &lobby.guests[0]] {
        assert_eq!(
            session.get_run(&run_id).map(|run| run.status()),
            Some(RunStatus::Completed)
        );
    }

    // The player comes back: same participant, no duplicate, up to date.
    lobby.reconnect(1, inbox);
    lobby.tick(80);
    assert_eq!(lobby.participants(), 3);
    assert!(lobby.host.connected_participants().contains(&leaver));
    assert_eq!(lobby.guests[1].join_state(), Some(JoinSessionState::Ready));
    assert_eq!(
        lobby.guests[1].get_run(&run_id).map(|run| run.status()),
        Some(RunStatus::Completed)
    );

    // The next run counts on them again, and a second departure is handled.
    let (second, required) = lobby.start_run("Again");
    assert!(required.contains(&leaver));
    lobby.disconnect(1);
    lobby.tick(5);
    assert!(
        lobby
            .host
            .get_run(&second)
            .is_some_and(|run| !run.required_submitters().contains(&leaver))
    );
}

#[test]
fn a_reloaded_player_returns_as_the_same_participant() {
    let mut lobby = Lobby::new(true);
    let player = lobby.participant(1);
    let (run_id, _) = lobby.start_run("Quiz");
    lobby.disconnect(1);
    lobby.tick(5);
    assert!(
        lobby
            .host
            .get_run(&run_id)
            .is_some_and(|run| !run.required_submitters().contains(&player))
    );

    // A fresh session with the same key: no new participant is created.
    lobby.guests[1] = lobby.new_guest(1);
    assert_eq!(lobby.participant(1), player);
    lobby.tick(80);
    assert_eq!(lobby.participants(), 3);
    assert!(lobby.host.connected_participants().contains(&player));
    assert_eq!(lobby.guests[1].join_state(), Some(JoinSessionState::Ready));
    let seen = lobby.guests[1]
        .get_active_run()
        .unwrap_or_else(|| panic!("reloaded player does not see the run"));
    assert_eq!(seen.id(), run_id);
}
