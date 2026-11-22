#[allow(dead_code)]
mod support;

use konnekt_session_core::{
    ActivityConfig, DomainCommand, DomainLoop, RunStatus, domain::ActivityResult,
};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use std::sync::{Arc, Mutex};
use support::mock_connection::{MockConnection, MockNetwork, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;

fn guest(network: &Arc<Mutex<MockNetwork>>, lobby_id: Uuid, name: &str) -> (Loop, Uuid) {
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(MockConnection::new(network.clone()), 100),
        false,
        lobby_id,
    );
    let id = guest
        .join(name.into())
        .unwrap_or_else(|error| panic!("{error}"));
    (guest, id)
}

fn tick(host: &mut Loop, guests: &mut [Loop], count: usize) {
    for _ in 0..count {
        host.poll();
        for guest in guests.iter_mut() {
            guest.poll();
        }
    }
}

#[test]
fn late_guests_receive_the_active_run_and_finished_results() {
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
    let (early, early_id) = guest(&network, lobby_id, "Early");
    let mut guests = vec![early];
    tick(&mut host, &mut guests, 20);

    for name in ["Quiz", "Next"] {
        host.submit_command(DomainCommand::QueueActivity {
            lobby_id,
            config: ActivityConfig::new("quiz".into(), name.into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    }
    host.submit_command(DomainCommand::StartNextRun { lobby_id })
        .unwrap_or_else(|error| panic!("{error}"));
    tick(&mut host, &mut guests, 5);
    let run = host
        .get_active_run()
        .unwrap_or_else(|| panic!("no active run"));
    let run_id = run.id();
    let required: Vec<Uuid> = run.required_submitters().iter().copied().collect();
    assert!(required.contains(&early_id));
    guests[0]
        .submit_command(DomainCommand::SubmitResult {
            lobby_id,
            run_id,
            result: ActivityResult::new(run_id, early_id),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    tick(&mut host, &mut guests, 5);

    // Joining during the run: the run, its submitters and results arrive.
    let (mid, _) = guest(&network, lobby_id, "Mid");
    guests.push(mid);
    tick(&mut host, &mut guests, 20);
    let seen = guests[1]
        .get_active_run()
        .unwrap_or_else(|| panic!("late guest has no active run"));
    assert_eq!(seen.id(), run_id);
    assert!(seen.results().contains_key(&early_id));
    assert_eq!(
        seen.required_submitters(),
        host.get_active_run()
            .unwrap_or_else(|| panic!("no run"))
            .required_submitters()
    );
    let queue = |session: &Loop| -> Vec<String> {
        session
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby"))
            .activity_queue()
            .iter()
            .map(|activity| activity.name.clone())
            .collect()
    };
    assert_eq!(queue(&guests[1]), vec!["Next".to_string()]);

    // Finish the run; a guest joining afterwards sees the finished results.
    for participant in required.iter().filter(|id| **id != early_id) {
        host.submit_command(DomainCommand::SubmitResult {
            lobby_id,
            run_id,
            result: ActivityResult::new(run_id, *participant),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    }
    tick(&mut host, &mut guests, 5);
    assert_eq!(
        host.get_run(&run_id).map(|run| run.status()),
        Some(RunStatus::Completed)
    );
    let (late, _) = guest(&network, lobby_id, "Late");
    guests.push(late);
    tick(&mut host, &mut guests, 20);
    let finished = guests[2]
        .get_run(&run_id)
        .unwrap_or_else(|| panic!("late guest has no finished run"));
    assert_eq!(finished.status(), RunStatus::Completed);
    assert_eq!(finished.results().len(), required.len());
    assert!(guests[2].get_active_run().is_none());
    assert_eq!(
        guests[2]
            .get_lobby()
            .map(|lobby| lobby.participants().len()),
        Some(4)
    );
}

#[test]
fn a_guest_recovers_through_a_snapshot_when_missed_updates_are_not_cached() {
    use konnekt_session_p2p::protocol::synchronization::StateSynchronizationState as Sync;
    let network = create_mock_network();
    let lobby_id = Uuid::new_v4();
    // The host keeps only the last two updates for resends.
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 2),
        true,
        lobby_id,
    );
    host.submit_command(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name: "Room".into(),
        host_name: "Host".into(),
    })
    .unwrap_or_else(|error| panic!("{error}"));
    let (guest, _) = guest(&network, lobby_id, "Guest");
    let mut guests = vec![guest];
    tick(&mut host, &mut guests, 20);
    let guest_peer = guests[0]
        .local_peer_id()
        .unwrap_or_else(|| panic!("no peer"));
    let queue = |host: &mut Loop, name: &str| {
        host.submit_command(DomainCommand::QueueActivity {
            lobby_id,
            config: ActivityConfig::new("quiz".into(), name.into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        host.poll();
    };

    // The guest loses four updates; the host cache holds only two.
    for name in ["A", "B", "C", "D"] {
        queue(&mut host, name);
    }
    network
        .lock()
        .unwrap_or_else(|error| panic!("{error}"))
        .peers
        .get(&guest_peer)
        .unwrap_or_else(|| panic!("guest not on network"))
        .lock()
        .unwrap_or_else(|error| panic!("{error}"))
        .clear();
    queue(&mut host, "E");
    guests[0].poll();
    assert_eq!(guests[0].synchronization_state(), Some(Sync::Recovering));

    tick(&mut host, &mut guests, 5);
    assert_eq!(guests[0].synchronization_state(), Some(Sync::Current));
    let names: Vec<_> = guests[0]
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"))
        .activity_queue()
        .iter()
        .map(|activity| activity.name.clone())
        .collect();
    assert_eq!(names, ["A", "B", "C", "D", "E"]);

    // Later updates continue normally from the snapshot's sequence.
    queue(&mut host, "F");
    tick(&mut host, &mut guests, 3);
    assert_eq!(
        guests[0]
            .get_lobby()
            .map(|lobby| lobby.activity_queue().len()),
        Some(6)
    );
}

#[test]
fn snapshots_carry_only_the_most_recent_runs() {
    use konnekt_session_p2p::application::runtime::DEFAULT_SNAPSHOT_RUNS;
    assert_eq!(DEFAULT_SNAPSHOT_RUNS, 32);
    let network = create_mock_network();
    let lobby_id = Uuid::new_v4();
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 100),
        true,
        lobby_id,
    );
    host.set_snapshot_run_limit(2);
    host.submit_command(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name: "Room".into(),
        host_name: "Host".into(),
    })
    .unwrap_or_else(|error| panic!("{error}"));
    host.poll();
    let host_id = host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"))
        .host_id();
    let mut runs = Vec::new();
    for name in ["One", "Two", "Three"] {
        host.submit_command(DomainCommand::QueueActivity {
            lobby_id,
            config: ActivityConfig::new("quiz".into(), name.into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        host.submit_command(DomainCommand::StartNextRun { lobby_id })
            .unwrap_or_else(|error| panic!("{error}"));
        host.poll();
        let run_id = host
            .get_active_run()
            .unwrap_or_else(|| panic!("no run"))
            .id();
        host.submit_command(DomainCommand::SubmitResult {
            lobby_id,
            run_id,
            result: ActivityResult::new(run_id, host_id),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        host.poll();
        runs.push(run_id);
    }
    let (late, _) = guest(&network, lobby_id, "Late");
    let mut guests = vec![late];
    tick(&mut host, &mut guests, 20);
    assert!(guests[0].get_run(&runs[0]).is_none());
    for run_id in &runs[1..] {
        assert_eq!(
            guests[0].get_run(run_id).map(|run| run.status()),
            Some(RunStatus::Completed)
        );
    }
}
