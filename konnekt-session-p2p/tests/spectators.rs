#[allow(dead_code)]
mod support;

use konnekt_session_core::{ActivityConfig, DomainCommand, DomainLoop, ParticipationMode};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use support::mock_connection::{MockConnection, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;

fn tick(host: &mut Loop, guests: &mut [Loop], count: usize) {
    for _ in 0..count {
        host.poll();
        for guest in guests.iter_mut() {
            guest.poll();
        }
    }
}

#[test]
fn a_guest_can_join_as_spectator_and_is_not_a_required_submitter() {
    let network = create_mock_network();
    let lobby_id = Uuid::new_v4();
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 100),
        true,
        lobby_id,
    );
    host.submit_command(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name: "Room".into(),
        host_name: "Host".into(),
    })
    .unwrap_or_else(|error| panic!("{error}"));
    let new_guest = || {
        SessionLoopV2::new(
            DomainLoop::new(10, 100),
            P2PTransport::new_guest(MockConnection::new(network.clone()), 100),
            false,
            lobby_id,
        )
    };
    let mut player = new_guest();
    let player_id = player
        .join("Player".into())
        .unwrap_or_else(|error| panic!("{error}"));
    let mut observer = new_guest();
    let observer_id = observer
        .join_as("Observer".into(), ParticipationMode::Spectating)
        .unwrap_or_else(|error| panic!("{error}"));
    let mut guests = vec![player, observer];
    tick(&mut host, &mut guests, 20);

    for session in std::iter::once(&host).chain(guests.iter()) {
        let lobby = session
            .get_lobby()
            .unwrap_or_else(|| panic!("lobby not synchronized"));
        let mode = |id: &Uuid| lobby.participants()[id].participation_mode();
        assert_eq!(mode(&player_id), ParticipationMode::Active);
        assert_eq!(mode(&observer_id), ParticipationMode::Spectating);
        assert!(!lobby.participants()[&observer_id].is_host());
    }

    host.submit_command(DomainCommand::QueueActivity {
        lobby_id,
        config: ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null),
    })
    .unwrap_or_else(|error| panic!("{error}"));
    host.submit_command(DomainCommand::StartNextRun { lobby_id })
        .unwrap_or_else(|error| panic!("{error}"));
    tick(&mut host, &mut guests, 5);
    let run = host
        .get_active_run()
        .unwrap_or_else(|| panic!("no active run"));
    assert!(run.required_submitters().contains(&player_id));
    assert!(!run.required_submitters().contains(&observer_id));
    assert!(
        guests[1].get_active_run().is_some(),
        "spectator follows the run"
    );
}
