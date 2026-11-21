mod support;

use konnekt_session_core::{DomainCommand, domain::ActivityConfig};
use support::SessionFixture;

#[test]
fn joining_guests_with_the_same_name_keep_distinct_ids() {
    use konnekt_session_p2p::protocol::join::JoinSessionState;

    let mut fixture = SessionFixture::new(2);
    let first = fixture.guests[0].join("Player".into()).unwrap();
    let second = fixture.guests[1].join("Player".into()).unwrap();
    assert_ne!(first, second);
    fixture.tick(50);
    for guest in &fixture.guests {
        assert_eq!(guest.join_state(), Some(JoinSessionState::Ready));
        let lobby = guest.get_lobby().unwrap();
        assert!(lobby.participants().contains_key(&first));
        assert!(lobby.participants().contains_key(&second));
    }
}

#[test]
fn test_guest_joins_and_syncs_lobby() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(10);

    let guest_lobby = fixture.guests[0].get_lobby();
    assert!(guest_lobby.is_some(), "Guest should have synced lobby");

    let lobby = guest_lobby.unwrap();
    assert_eq!(lobby.name(), "Test Lobby");
    assert_eq!(lobby.participants().len(), 1);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(10);

    let host_lobby = fixture.host.get_lobby().unwrap();
    let guest_lobby = fixture.guests[0].get_lobby().unwrap();

    assert_eq!(host_lobby.participants().len(), 2);
    assert_eq!(guest_lobby.participants().len(), 2);
}

#[test]
fn test_multiple_guests() {
    let mut fixture = SessionFixture::new(3);

    fixture.tick(500);

    for (i, guest) in fixture.guests.iter_mut().enumerate() {
        let cmd = DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: format!("Guest{}", i + 1),
        };
        guest.submit_command(cmd).unwrap();
    }

    fixture.tick(900);

    let host_count = fixture
        .host
        .get_lobby()
        .expect("Host should have lobby")
        .participants()
        .len();
    assert_eq!(host_count, 4, "Host should see 4 participants");

    for (index, guest) in fixture.guests.iter().enumerate() {
        let guest_count = guest
            .get_lobby()
            .unwrap_or_else(|| panic!("Guest {} should have lobby", index + 1))
            .participants()
            .len();
        assert_eq!(
            guest_count,
            4,
            "Guest {} should see 4 participants",
            index + 1
        );
    }
}

#[test]
fn test_activity_queue_and_start_run() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(10);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(10);

    // Host queues activity
    let config = ActivityConfig::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        serde_json::json!({}),
    );

    fixture
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: fixture.lobby_id,
            config,
        })
        .unwrap();

    fixture.tick(10);

    // Both should see queued activity
    assert_eq!(fixture.host.get_lobby().unwrap().activity_queue().len(), 1);

    // Host starts next run
    fixture
        .host
        .submit_command(DomainCommand::StartNextRun {
            lobby_id: fixture.lobby_id,
        })
        .unwrap();

    fixture.tick(10);

    // Host should have active run, queue should be empty
    assert!(fixture.host.get_lobby().unwrap().has_active_run());
    assert!(
        fixture
            .host
            .get_lobby()
            .unwrap()
            .activity_queue()
            .is_empty()
    );
}

#[test]
fn test_activity_completion() {
    let mut fixture = SessionFixture::new(1);

    fixture.tick(200);

    fixture.guests[0]
        .submit_command(DomainCommand::JoinLobby {
            lobby_id: fixture.lobby_id,
            guest_name: "Guest1".to_string(),
        })
        .unwrap();

    fixture.tick(250);

    // IDs must come from HOST's lobby — the host assigns UUIDs.
    // Guests receive echoed JoinLobby commands and create different UUIDs locally.
    let host_lobby = fixture.host.get_lobby().unwrap();
    let host_participant_id = host_lobby
        .participants()
        .values()
        .find(|p| p.is_host())
        .expect("Host participant")
        .id();
    let guest_participant_id = host_lobby
        .participants()
        .values()
        .find(|p| !p.is_host())
        .expect("Guest participant on host")
        .id();

    // Queue and start
    let config = ActivityConfig::new(
        "echo-challenge-v1".to_string(),
        "Echo Test".to_string(),
        serde_json::json!({}),
    );

    fixture
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: fixture.lobby_id,
            config,
        })
        .unwrap();

    fixture.tick(250);

    fixture
        .host
        .submit_command(DomainCommand::StartNextRun {
            lobby_id: fixture.lobby_id,
        })
        .unwrap();

    fixture.tick(250);

    let run_id = fixture
        .host
        .get_lobby()
        .unwrap()
        .active_run_id()
        .expect("Run should be active");

    // Host submits result
    fixture
        .host
        .submit_command(DomainCommand::SubmitResult {
            lobby_id: fixture.lobby_id,
            run_id,
            result: konnekt_session_core::domain::ActivityResult::new(run_id, host_participant_id)
                .with_score(100),
        })
        .unwrap();

    fixture.tick(250);

    // Guest submits result
    fixture.guests[0]
        .submit_command(DomainCommand::SubmitResult {
            lobby_id: fixture.lobby_id,
            run_id,
            result: konnekt_session_core::domain::ActivityResult::new(run_id, guest_participant_id)
                .with_score(80),
        })
        .unwrap();

    fixture.tick(400);

    // Run should be completed on host
    // After completion, active_run is cleared from lobby
    assert!(!fixture.host.get_lobby().unwrap().has_active_run());
}

#[test]
fn host_removal_is_replicated_and_guest_can_join_again() -> Result<(), Box<dyn std::error::Error>> {
    let mut fixture = SessionFixture::new(1);
    fixture.tick(10);
    fixture.guests[0].submit_command(DomainCommand::JoinLobby {
        lobby_id: fixture.lobby_id,
        guest_name: "Returning guest".into(),
    })?;
    fixture.tick(10);
    let guest = fixture
        .host
        .get_lobby()
        .ok_or("missing lobby")?
        .participants()
        .values()
        .find(|p| !p.is_host())
        .ok_or("missing guest")?
        .id();
    fixture.host.submit_command(DomainCommand::LeaveLobby {
        lobby_id: fixture.lobby_id,
        participant_id: guest,
    })?;
    fixture.tick(10);
    assert_eq!(
        fixture.guests[0]
            .get_lobby()
            .ok_or("missing guest lobby")?
            .participants()
            .len(),
        1
    );
    fixture.guests[0].submit_command(DomainCommand::JoinLobby {
        lobby_id: fixture.lobby_id,
        guest_name: "Returning guest".into(),
    })?;
    fixture.tick(10);
    assert_eq!(
        fixture
            .host
            .get_lobby()
            .ok_or("missing lobby")?
            .participants(),
        fixture.guests[0]
            .get_lobby()
            .ok_or("missing guest lobby")?
            .participants()
    );
    Ok(())
}

#[test]
fn guest_accepts_restarted_host_snapshot_and_new_sequence() -> Result<(), Box<dyn std::error::Error>>
{
    use konnekt_session_core::DomainLoop;
    use konnekt_session_p2p::{SessionLoopV2, infrastructure::transport::P2PTransport};
    use support::mock_connection::{MockConnection, create_mock_network};
    let network = create_mock_network();
    let lobby_id = uuid::Uuid::new_v4();
    let host_connection = MockConnection::new(network.clone());
    let old_peer = host_connection.local_peer_id().ok_or("missing host peer")?;
    let make_host = |connection| -> Result<_, Box<dyn std::error::Error>> {
        let mut domain = DomainLoop::new(10, 100);
        domain.submit(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Reload".into(),
            host_name: "Host".into(),
        })?;
        domain.poll();
        Ok(SessionLoopV2::new(
            domain,
            P2PTransport::new_host(connection, 100),
            true,
            lobby_id,
        ))
    };
    let mut host = make_host(host_connection)?;
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(MockConnection::new(network.clone()), 100),
        false,
        lobby_id,
    );
    for _ in 0..10 {
        host.poll();
        guest.poll();
    }
    guest.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "Guest".into(),
    })?;
    for _ in 0..10 {
        host.poll();
        guest.poll();
    }
    let old_host = host.get_lobby().ok_or("missing lobby")?.host_id();
    drop(host);
    network
        .lock()
        .map_err(|_| "network lock poisoned")?
        .peers
        .remove(&old_peer);
    let mut host = make_host(MockConnection::new(network.clone()))?;
    for _ in 0..10 {
        host.poll();
        guest.poll();
    }
    assert_ne!(
        guest.get_lobby().ok_or("missing guest lobby")?.host_id(),
        old_host
    );
    guest.submit_command(DomainCommand::JoinLobby {
        lobby_id,
        guest_name: "Guest".into(),
    })?;
    for _ in 0..10 {
        host.poll();
        guest.poll();
    }
    assert_eq!(
        host.get_lobby()
            .ok_or("missing lobby")?
            .participants()
            .len(),
        2
    );
    assert_eq!(
        host.get_lobby().ok_or("missing lobby")?.participants(),
        guest
            .get_lobby()
            .ok_or("missing guest lobby")?
            .participants()
    );
    Ok(())
}
#[test]
fn transport_reports_the_peer_that_sent_a_guest_command() {
    use konnekt_session_p2p::infrastructure::transport::P2PTransport;
    use support::mock_connection::{MockConnection, create_mock_network};

    let network = create_mock_network();
    let host_connection = MockConnection::new(network.clone());
    let guest_connection = MockConnection::new(network);
    let guest_id = guest_connection.local_peer_id().unwrap();
    let mut host = P2PTransport::new_host(host_connection, 10);
    let mut guest = P2PTransport::new_guest(guest_connection, 10);
    let payload = serde_json::json!({"request": "join"});

    guest.send_to_host(payload.clone()).unwrap();
    assert_eq!(host.poll_with_sender(), vec![(guest_id, payload)]);
}

#[test]
fn guest_reports_a_gap_until_missing_host_messages_arrive() -> Result<(), Box<dyn std::error::Error>>
{
    use konnekt_session_core::DomainLoop;
    use konnekt_session_p2p::SessionLoopV2;
    use konnekt_session_p2p::infrastructure::message::P2PMessage;
    use konnekt_session_p2p::infrastructure::transport::P2PTransport;
    use konnekt_session_p2p::protocol::synchronization::StateSynchronizationState as State;
    use support::mock_connection::{MockConnection, create_mock_network};

    let network = create_mock_network();
    let mut host_connection = MockConnection::new(network.clone());
    let guest_connection = MockConnection::new(network);
    let guest_id = guest_connection
        .local_peer_id()
        .ok_or("missing guest peer")?;
    let lobby_id = uuid::Uuid::new_v4();
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(guest_connection, 10),
        false,
        lobby_id,
    );
    let host = konnekt_session_core::Participant::new_host("Host".into())?;
    let snapshot = P2PMessage::snapshot_response(
        serde_json::json!({
            "lobby_id": lobby_id,
            "name": "Lobby",
            "host_id": host.id(),
            "participants": [host],
        }),
        0,
    );
    host_connection.send_to(guest_id, serde_json::to_vec(&snapshot)?)?;
    guest.poll();
    guest.poll();
    assert_eq!(guest.synchronization_state(), Some(State::Current));

    let mut message = P2PMessage::application(serde_json::json!({}));
    message.sequence = 2;
    host_connection.send_to(guest_id, serde_json::to_vec(&message)?)?;
    guest.poll();
    assert_eq!(guest.synchronization_state(), Some(State::Recovering));

    message.sequence = 1;
    host_connection.send_to(guest_id, serde_json::to_vec(&message)?)?;
    guest.poll();
    assert_eq!(guest.synchronization_state(), Some(State::Current));
    Ok(())
}

#[test]
fn guest_retries_missing_host_updates_and_requests_new_gaps()
-> Result<(), Box<dyn std::error::Error>> {
    use konnekt_session_core::DomainLoop;
    use konnekt_session_p2p::SessionLoopV2;
    use konnekt_session_p2p::application::ConnectionEvent;
    use konnekt_session_p2p::infrastructure::message::{MessageKind, P2PMessage};
    use konnekt_session_p2p::infrastructure::transport::P2PTransport;
    use support::mock_connection::{MockConnection, create_mock_network};

    let network = create_mock_network();
    let mut host = MockConnection::new(network.clone());
    let guest_connection = MockConnection::new(network);
    let guest_id = guest_connection.local_peer_id().ok_or("missing guest")?;
    let lobby_id = uuid::Uuid::new_v4();
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(guest_connection, 10),
        false,
        lobby_id,
    );
    let participant = konnekt_session_core::Participant::new_host("Host".into())?;
    let snapshot = P2PMessage::snapshot_response(
        serde_json::json!({
            "lobby_id": lobby_id, "name": "Lobby", "host_id": participant.id(), "participants": [participant],
        }),
        0,
    );
    host.send_to(guest_id, serde_json::to_vec(&snapshot)?)?;
    guest.poll();
    guest.poll();
    host.poll_events();
    let mut message = P2PMessage::application(serde_json::json!({}));
    message.sequence = 3;
    host.send_to(guest_id, serde_json::to_vec(&message)?)?;
    guest.poll();
    let requests = |events: Vec<ConnectionEvent>| -> Vec<(u64, u64)> {
        events
            .into_iter()
            .filter_map(|event| {
                let ConnectionEvent::MessageReceived { data, .. } = event else {
                    return None;
                };
                let message: P2PMessage = serde_json::from_slice(&data).ok()?;
                match message.kind {
                    MessageKind::ResendRequest { from, to } => Some((from, to)),
                    _ => None,
                }
            })
            .collect()
    };
    assert_eq!(requests(host.poll_events()), vec![(1, 2)]);
    for _ in 0..51 {
        guest.poll();
    }
    assert_eq!(requests(host.poll_events()), vec![(1, 2)]);
    message.sequence = 1;
    host.send_to(guest_id, serde_json::to_vec(&message)?)?;
    guest.poll();
    assert_eq!(requests(host.poll_events()), vec![(2, 2)]);
    Ok(())
}

#[test]
fn guest_retries_a_lost_initial_snapshot_request() -> Result<(), Box<dyn std::error::Error>> {
    use konnekt_session_core::DomainLoop;
    use konnekt_session_p2p::SessionLoopV2;
    use konnekt_session_p2p::application::ConnectionEvent;
    use konnekt_session_p2p::infrastructure::message::{MessageKind, P2PMessage};
    use konnekt_session_p2p::infrastructure::transport::P2PTransport;
    use support::mock_connection::{MockConnection, create_mock_network};

    let network = create_mock_network();
    let mut host = MockConnection::new(network.clone());
    let guest_connection = MockConnection::new(network);
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(guest_connection, 10),
        false,
        uuid::Uuid::new_v4(),
    );
    let snapshot_requests = |events: Vec<ConnectionEvent>| {
        events
            .into_iter()
            .filter(|event| {
                let ConnectionEvent::MessageReceived { data, .. } = event else {
                    return false;
                };
                serde_json::from_slice::<P2PMessage>(data)
                    .is_ok_and(|message| matches!(message.kind, MessageKind::SnapshotRequest))
            })
            .count()
    };
    guest.poll();
    assert_eq!(snapshot_requests(host.poll_events()), 1);
    for _ in 0..51 {
        guest.poll();
    }
    assert_eq!(snapshot_requests(host.poll_events()), 1);
    Ok(())
}
