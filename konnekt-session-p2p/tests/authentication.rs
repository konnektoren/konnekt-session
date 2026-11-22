#[allow(dead_code)]
mod support;

use ed25519_dalek::SigningKey;
use konnekt_session_core::{ActivityConfig, DomainCommand, DomainLoop, Participant};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::domain::participant_id;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use konnekt_session_p2p::protocol::join::JoinSessionState;
use std::sync::{Arc, Mutex};
use support::mock_connection::{MockConnection, MockNetwork, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;

fn id_of(secret: [u8; 32]) -> Uuid {
    participant_id(&SigningKey::from_bytes(&secret).verifying_key().to_bytes())
}

fn host(network: &Arc<Mutex<MockNetwork>>, lobby_id: Uuid, secret: [u8; 32]) -> Loop {
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 100),
        true,
        lobby_id,
    );
    host.enable_authentication(secret)
        .unwrap_or_else(|error| panic!("{error}"));
    host
}

fn guest(network: &Arc<Mutex<MockNetwork>>, lobby_id: Uuid, secret: [u8; 32], name: &str) -> Loop {
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(MockConnection::new(network.clone()), 100),
        false,
        lobby_id,
    );
    let id = guest
        .enable_authentication(secret)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        guest
            .join(name.into())
            .unwrap_or_else(|error| panic!("{error}")),
        id
    );
    guest
}

struct Session {
    network: Arc<Mutex<MockNetwork>>,
    lobby_id: Uuid,
    host: Loop,
    guests: Vec<Loop>,
    others: Vec<Loop>,
}

impl Session {
    fn new() -> Self {
        let network = create_mock_network();
        let lobby_id = Uuid::new_v4();
        let mut host = host(&network, lobby_id, [31; 32]);
        host.submit_command(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Room".into(),
            host_name: "Host".into(),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        let guests = vec![
            guest(&network, lobby_id, [32; 32], "Ada"),
            guest(&network, lobby_id, [33; 32], "Bo"),
        ];
        let mut session = Self {
            network,
            lobby_id,
            host,
            guests,
            others: Vec::new(),
        };
        session.tick(20);
        session
    }

    fn tick(&mut self, count: usize) {
        for _ in 0..count {
            self.host.poll();
            for peer in self.guests.iter_mut().chain(self.others.iter_mut()) {
                peer.poll();
            }
        }
    }

    fn queue(&mut self, guest: usize, name: &str) {
        self.guests[guest]
            .submit_command(DomainCommand::QueueActivity {
                lobby_id: self.lobby_id,
                config: ActivityConfig::new("quiz".into(), name.into(), serde_json::Value::Null),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }
}

fn queued(session: &Loop) -> Vec<String> {
    session
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"))
        .activity_queue()
        .iter()
        .map(|activity| activity.name.clone())
        .collect()
}

#[test]
fn participants_are_the_ids_derived_from_their_keys() {
    let session = Session::new();
    let lobby = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"));
    assert_eq!(lobby.host_id(), id_of([31; 32]));
    for (guest, secret) in session.guests.iter().zip([[32; 32], [33; 32]]) {
        assert_eq!(guest.own_participant_id(), Some(id_of(secret)));
        assert_eq!(guest.join_state(), Some(JoinSessionState::Ready));
        assert!(lobby.participants().contains_key(&id_of(secret)));
        assert_eq!(
            guest.get_lobby().map(|lobby| lobby.participants().len()),
            Some(3)
        );
    }
}

#[test]
fn the_host_rejects_commands_that_act_for_someone_else() {
    let mut session = Session::new();
    let (ada, bo) = (id_of([32; 32]), id_of([33; 32]));
    let lobby_id = session.lobby_id;
    let forged = Participant::guest_with_id(Uuid::new_v4(), "Ghost".into())
        .unwrap_or_else(|error| panic!("{error}"));
    for command in [
        DomainCommand::LeaveLobby {
            lobby_id,
            participant_id: bo,
        },
        DomainCommand::AddParticipant {
            lobby_id,
            participant: forged.clone(),
        },
        DomainCommand::DelegateHost {
            lobby_id,
            current_host_id: id_of([31; 32]),
            new_host_id: ada,
        },
    ] {
        session.guests[0]
            .submit_command(command)
            .unwrap_or_else(|error| panic!("{error}"));
    }
    session.tick(10);
    let lobby = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"));
    assert!(lobby.participants().contains_key(&bo));
    assert!(!lobby.participants().contains_key(&forged.id()));
    assert_eq!(lobby.host_id(), id_of([31; 32]));

    // Acting as oneself still works.
    session.guests[0]
        .submit_command(DomainCommand::LeaveLobby {
            lobby_id,
            participant_id: ada,
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(10);
    let lobby = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"));
    assert!(!lobby.participants().contains_key(&ada));
}

#[test]
fn guests_ignore_a_peer_that_impersonates_or_replaces_the_host() {
    let mut session = Session::new();
    let host_id = id_of([31; 32]);

    // One rogue claims the real host's participant ID with another key; the
    // other names itself host of the same lobby.
    for (secret, claimed) in [([40; 32], host_id), ([41; 32], id_of([41; 32]))] {
        let mut rogue = host(&session.network, session.lobby_id, secret);
        rogue
            .submit_command(DomainCommand::CreateLobbyWithHost {
                lobby_id: session.lobby_id,
                lobby_name: "Room".into(),
                host: Participant::host_with_id(claimed, "Host".into())
                    .unwrap_or_else(|error| panic!("{error}")),
            })
            .unwrap_or_else(|error| panic!("{error}"));
        rogue
            .submit_command(DomainCommand::QueueActivity {
                lobby_id: session.lobby_id,
                config: ActivityConfig::new("quiz".into(), "Rogue".into(), serde_json::Value::Null),
            })
            .unwrap_or_else(|error| panic!("{error}"));
        session.others.push(rogue);
    }
    session.tick(30);
    for rogue in &mut session.others {
        rogue
            .submit_command(DomainCommand::QueueActivity {
                lobby_id: session.lobby_id,
                config: ActivityConfig::new("quiz".into(), "Rogue".into(), serde_json::Value::Null),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }
    session.tick(10);

    for guest in &session.guests {
        let lobby = guest.get_lobby().unwrap_or_else(|| panic!("no lobby"));
        assert_eq!(lobby.host_id(), host_id);
        assert_eq!(lobby.participants().len(), 3);
        assert!(!queued(guest).contains(&"Rogue".to_string()));
    }
    // Guest commands still reach the real host and come back to everyone.
    session.queue(0, "Legit");
    session.tick(10);
    assert_eq!(queued(&session.host), vec!["Legit".to_string()]);
    for guest in &session.guests {
        assert_eq!(queued(guest), vec!["Legit".to_string()]);
    }
}

#[test]
fn a_reloaded_host_with_the_same_key_is_followed_again() {
    let mut session = Session::new();
    session.queue(0, "Before");
    session.tick(10);
    // The host reloads: new connection and new domain, same key.
    let old_peer = session
        .host
        .local_peer_id()
        .unwrap_or_else(|| panic!("no peer"));
    session
        .network
        .lock()
        .unwrap_or_else(|error| panic!("{error}"))
        .peers
        .remove(&old_peer);
    session.host = host(&session.network, session.lobby_id, [31; 32]);
    session
        .host
        .submit_command(DomainCommand::CreateLobby {
            lobby_id: Some(session.lobby_id),
            lobby_name: "Room".into(),
            host_name: "Host".into(),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(80);
    session.queue(1, "After");
    session.tick(10);
    assert!(queued(&session.host).contains(&"After".to_string()));
    for guest in &session.guests {
        assert!(queued(guest).contains(&"After".to_string()));
        assert_eq!(
            guest.get_lobby().map(|lobby| lobby.host_id()),
            Some(id_of([31; 32]))
        );
    }
}

#[test]
fn guests_that_saw_the_host_leave_follow_it_after_a_restore() {
    use konnekt_session_p2p::ConnectionEvent;
    use konnekt_session_p2p::application::persistence::MemoryStore;
    let mut session = Session::new();
    // A lone guest: with no peer left it drops its replica.
    let bo = session.guests.pop().unwrap_or_else(|| panic!("no guest"));
    let bo_peer = bo.local_peer_id().unwrap_or_else(|| panic!("no peer"));
    drop(bo);
    session
        .network
        .lock()
        .unwrap_or_else(|error| panic!("{error}"))
        .peers
        .remove(&bo_peer);
    let store = MemoryStore::new();
    session.host.enable_persistence(Box::new(store.clone()));
    session.queue(0, "Before");
    session.tick(10);
    let old_peer = session
        .host
        .local_peer_id()
        .unwrap_or_else(|| panic!("no peer"));
    {
        let mut network = session
            .network
            .lock()
            .unwrap_or_else(|error| panic!("{error}"));
        network.peers.remove(&old_peer);
        for guest in &session.guests {
            let peer = guest.local_peer_id().unwrap_or_else(|| panic!("no peer"));
            network
                .events
                .push_back((peer, ConnectionEvent::PeerDisconnected(old_peer)));
        }
    }
    // The guests drop their replica while the host is gone.
    session.tick(2);
    for guest in &session.guests {
        assert!(guest.get_lobby().is_none());
    }
    session.host = host(&session.network, session.lobby_id, [31; 32]);
    session.host.enable_persistence(Box::new(store));
    let state = session
        .host
        .stored_state()
        .and_then(Result::ok)
        .unwrap_or_else(|| panic!("nothing stored"));
    session
        .host
        .restore_state(state)
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(80);
    session.queue(0, "After");
    session.tick(10);
    assert_eq!(
        queued(&session.host),
        vec!["Before".to_string(), "After".to_string()]
    );
    for guest in &session.guests {
        assert_eq!(guest.join_state(), Some(JoinSessionState::Ready));
        assert_eq!(queued(guest), queued(&session.host));
    }
}
