#[allow(dead_code)]
mod support;

use ed25519_dalek::SigningKey;
use konnekt_session_core::{ActivityConfig, DomainCommand, DomainLoop};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::application::ConnectionEvent;
use konnekt_session_p2p::application::event_log_reconciliation::{
    HistorySide, ReconciliationAttempt, SignedHistoryView,
};
use konnekt_session_p2p::application::host_failover::failover_candidates;
use konnekt_session_p2p::application::merge_plan::DropReason;
use konnekt_session_p2p::application::signed_history::{event_hash, verify_history};
use konnekt_session_p2p::domain::DomainEvent;
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use konnekt_session_p2p::protocol::event_log_reconciliation::EventLogReconciliationState;
use konnekt_session_p2p::protocol::host_failover::HostFailoverState;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use support::mock_connection::{MockConnection, MockNetwork, create_mock_network};
use uuid::Uuid;

type Loop = SessionLoopV2<MockConnection>;

struct Session {
    network: Arc<Mutex<MockNetwork>>,
    host: Loop,
    host_peer: konnekt_session_p2p::domain::PeerId,
    guests: Vec<Loop>,
    keys: Vec<SigningKey>,
    lobby_id: Uuid,
    away: Vec<(konnekt_session_p2p::domain::PeerId, Inbox)>,
}

type Inbox = Arc<Mutex<std::collections::VecDeque<(konnekt_session_p2p::domain::PeerId, Vec<u8>)>>>;

impl Session {
    /// A recording host and two guests that follow its signed chain and may
    /// take over. Keys: 0 = host, 1 and 2 = guests.
    fn new(candidate_turn: Duration) -> Self {
        Self::build(candidate_turn, false)
    }

    /// Every peer proves its key; the same key signs its history.
    fn authenticated(candidate_turn: Duration) -> Self {
        Self::build(candidate_turn, true)
    }

    fn build(candidate_turn: Duration, authenticated: bool) -> Self {
        let network = create_mock_network();
        let lobby_id = Uuid::new_v4();
        let keys: Vec<_> = (21..24)
            .map(|seed| SigningKey::from_bytes(&[seed; 32]))
            .collect();
        let trusted: Vec<_> = keys
            .iter()
            .map(|key| key.verifying_key().to_bytes())
            .collect();
        let connection = MockConnection::new(network.clone());
        let host_peer = connection
            .local_peer_id()
            .unwrap_or_else(|| panic!("no peer id"));
        let mut host = SessionLoopV2::new(
            DomainLoop::new(10, 100),
            P2PTransport::new_host(connection, 100),
            true,
            lobby_id,
        );
        if authenticated {
            host.enable_authentication(keys[0].to_bytes())
                .unwrap_or_else(|error| panic!("{error}"));
        }
        host.enable_signed_history(keys[0].clone(), 50)
            .unwrap_or_else(|error| panic!("{error}"));
        assert!(host.enable_reclaim(trusted.clone()));
        host.submit_command(DomainCommand::CreateLobby {
            lobby_id: Some(lobby_id),
            lobby_name: "Room".into(),
            host_name: "Host".into(),
        })
        .unwrap_or_else(|error| panic!("{error}"));
        host.poll();
        let mut session = Self {
            network,
            host,
            host_peer,
            guests: Vec::new(),
            keys,
            lobby_id,
            away: Vec::new(),
        };
        for (index, name) in ["Ada", "Bo"].into_iter().enumerate() {
            let mut guest = SessionLoopV2::new(
                DomainLoop::new(10, 100),
                P2PTransport::new_guest(MockConnection::new(session.network.clone()), 100),
                false,
                lobby_id,
            );
            if authenticated {
                guest
                    .enable_authentication(session.keys[index + 1].to_bytes())
                    .unwrap_or_else(|error| panic!("{error}"));
            }
            guest
                .follow_signed_history(trusted.clone(), 50)
                .unwrap_or_else(|error| panic!("{error}"));
            assert!(guest.set_host_grace_period(Duration::ZERO));
            assert!(guest.enable_failover(Some(session.keys[index + 1].clone()), candidate_turn));
            guest
                .join(name.into())
                .unwrap_or_else(|error| panic!("{error}"));
            session.guests.push(guest);
            session.tick(true, 20);
        }
        session
    }

    fn tick(&mut self, with_host: bool, count: usize) {
        for _ in 0..count {
            if with_host {
                self.host.poll();
            }
            for guest in &mut self.guests {
                guest.poll();
            }
        }
    }

    fn drop_host(&mut self) {
        self.drop_peer(self.host_peer);
    }

    fn drop_peer(&mut self, gone: konnekt_session_p2p::domain::PeerId) {
        let mut network = self
            .network
            .lock()
            .unwrap_or_else(|error| panic!("{error}"));
        if let Some(inbox) = network.peers.remove(&gone) {
            self.away.push((gone, inbox));
        }
        let others: Vec<_> = network.peers.keys().copied().collect();
        for peer in others {
            network
                .events
                .push_back((peer, ConnectionEvent::PeerDisconnected(gone)));
        }
    }

    /// Reconnect the original host to everyone still connected.
    fn return_host(&mut self) {
        let (peer, inbox) = self
            .away
            .iter()
            .position(|(peer, _)| *peer == self.host_peer)
            .map(|index| self.away.remove(index))
            .unwrap_or_else(|| panic!("host is not away"));
        let mut network = self
            .network
            .lock()
            .unwrap_or_else(|error| panic!("{error}"));
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

    fn participant_id(&self, guest: usize) -> Uuid {
        self.guests[guest]
            .own_participant_id()
            .unwrap_or_else(|| panic!("guest has not joined"))
    }
}

#[test]
fn the_first_candidate_takes_over_and_continues_the_signed_chain() {
    let mut session = Session::new(Duration::from_secs(60));
    session
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: session.lobby_id,
            config: ActivityConfig::new("quiz".into(), "Before".into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(true, 5);

    // Guests hold a verified copy of the host's chain.
    let host_tip = session
        .host
        .signed_history()
        .and_then(|history| history.tip());
    for guest in &session.guests {
        assert_eq!(
            guest
                .signed_history_replica()
                .and_then(|replica| replica.tip()),
            host_tip
        );
    }

    // Both replicas agree on who takes over.
    let order = failover_candidates(
        session.guests[0]
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby")),
    );
    let stand_in = (0..2)
        .find(|&guest| session.participant_id(guest) == order[0])
        .unwrap_or_else(|| panic!("no candidate"));
    let follower = 1 - stand_in;
    let stand_in_id = session.participant_id(stand_in);

    session.drop_host();
    session.tick(false, 10);

    assert!(session.guests[stand_in].is_host());
    assert!(!session.guests[follower].is_host());
    assert_eq!(
        session.guests[follower].host_failover_state(),
        Some(HostFailoverState::HostAvailable)
    );
    let follower_lobby = session.guests[follower]
        .get_lobby()
        .unwrap_or_else(|| panic!("follower lost the lobby"));
    assert_eq!(follower_lobby.host_id(), stand_in_id);

    // The stand-in continues the previous host's chain.
    let recorder = session.guests[stand_in]
        .signed_history()
        .unwrap_or_else(|| panic!("stand-in does not record"));
    let stand_in_key = session.keys[stand_in + 1].verifying_key().to_bytes();
    let host_key = session.keys[0].verifying_key().to_bytes();
    let chain: Vec<_> = recorder.events().iter().cloned().collect();
    assert!(
        verify_history(
            session.lobby_id,
            &chain,
            &[host_key, stand_in_key],
            recorder.anchor()
        )
        .is_ok()
    );
    let handover = chain
        .iter()
        .position(|event| event.author == stand_in_key)
        .unwrap_or_else(|| panic!("stand-in recorded nothing"));
    assert_eq!(
        chain[handover].previous_hash,
        host_tip.map(|tip| tip.hash),
        "the first stand-in event links to the previous host's tip"
    );

    // The follower's commands now reach the stand-in, and the follower's
    // replica keeps following the chain.
    session.guests[follower]
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: session.lobby_id,
            config: ActivityConfig::new("quiz".into(), "After".into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(false, 10);
    for guest in &session.guests {
        let queue = guest
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby"))
            .activity_queue();
        assert!(queue.iter().any(|activity| activity.name == "After"));
    }
    let stand_in_tip = session.guests[stand_in]
        .signed_history()
        .and_then(|history| history.tip());
    assert_eq!(
        session.guests[follower]
            .signed_history_replica()
            .and_then(|replica| replica.tip()),
        stand_in_tip
    );

    // The returning host can fast-forward over the stand-in's history.
    let host_history = session
        .host
        .signed_history()
        .unwrap_or_else(|| panic!("no host history"));
    let local: Vec<_> = host_history.events().iter().cloned().collect();
    let recorder = session.guests[stand_in]
        .signed_history()
        .unwrap_or_else(|| panic!("no stand-in history"));
    let remote: Vec<_> = recorder.events().iter().cloned().collect();
    let checkpoint = recorder.anchor();
    assert!(checkpoint.is_none_or(|anchor| {
        local
            .iter()
            .any(|event| event.sequence == anchor.sequence && event_hash(event) == anchor.hash)
    }));
    let mut attempt = ReconciliationAttempt::new(session.lobby_id);
    attempt
        .authority_checked(true)
        .unwrap_or_else(|error| panic!("{error}"));
    let trusted = [host_key];
    let both = [host_key, stand_in_key];
    assert_eq!(
        attempt.compare_signed(
            SignedHistoryView {
                events: &local,
                checkpoint: host_history.anchor(),
                trusted_authors: &trusted,
            },
            SignedHistoryView {
                events: &remote,
                checkpoint,
                trusted_authors: &both,
            },
            host_key,
        ),
        Ok(EventLogReconciliationState::PreparingMerge)
    );
    let choice = attempt
        .fork_choice()
        .unwrap_or_else(|| panic!("no fork choice"));
    assert_eq!(choice.selected, HistorySide::Remote);
    assert!(choice.applies_locally());
    let plan = attempt
        .prepare_merge(&local, &remote)
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(plan.dropped.iter().any(|dropped| matches!(
        dropped.event,
        DomainEvent::HostDelegated { .. }
    ) && dropped.reason == DropReason::AuthorityChange));
    assert!(plan.rebased.iter().any(|event| matches!(
        event,
        DomainEvent::ActivityQueued { config } if config.name == "After"
    )));
}

#[test]
fn a_later_candidate_waits_for_its_turn() {
    let mut session = Session::new(Duration::from_secs(60));
    session.drop_host();
    session.tick(false, 10);
    let hosts = session
        .guests
        .iter()
        .filter(|guest| guest.is_host())
        .count();
    assert_eq!(hosts, 1, "exactly one candidate takes over");
}

#[test]
fn the_next_candidate_takes_over_when_the_first_is_gone_too() {
    let mut session = Session::new(Duration::from_millis(20));
    let order = failover_candidates(
        session.guests[0]
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby")),
    );
    let first = (0..2)
        .find(|&guest| session.participant_id(guest) == order[0])
        .unwrap_or_else(|| panic!("no candidate"));
    let second = 1 - first;
    let first_peer = session.guests[first]
        .local_peer_id()
        .unwrap_or_else(|| panic!("no peer id"));
    session.drop_host();
    session.drop_peer(first_peer);
    session.guests[second].poll();
    assert!(!session.guests[second].is_host());
    assert_eq!(
        session.guests[second].host_failover_state(),
        Some(HostFailoverState::Electing)
    );
    std::thread::sleep(Duration::from_millis(40));
    session.guests[second].poll();
    assert!(session.guests[second].is_host());
}

#[test]
fn the_returning_host_reclaims_its_role_and_keeps_the_stand_in_changes() {
    reclaim_scenario(Session::new(Duration::from_secs(60)));
}

#[test]
fn failover_and_reclaim_work_with_authentication() {
    reclaim_scenario(Session::authenticated(Duration::from_secs(60)));
}

fn reclaim_scenario(mut session: Session) {
    let host_participant = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"))
        .host_id();
    session.drop_host();
    session.tick(false, 10);
    let stand_in = (0..2)
        .find(|&guest| session.guests[guest].is_host())
        .unwrap_or_else(|| panic!("nobody took over"));
    let follower = 1 - stand_in;
    session.guests[follower]
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: session.lobby_id,
            config: ActivityConfig::new("quiz".into(), "Meanwhile".into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(false, 10);

    session.return_host();
    session.tick(true, 30);

    assert_eq!(
        session.host.reclaim_state(),
        Some(HostFailoverState::HostAvailable)
    );
    assert!(session.host.is_host());
    assert!(session.guests.iter().all(|guest| !guest.is_host()));
    for guest in &session.guests {
        let lobby = guest.get_lobby().unwrap_or_else(|| panic!("no lobby"));
        assert_eq!(
            lobby.host_id(),
            host_participant,
            "guests follow the original host"
        );
        assert_eq!(
            guest.host_failover_state(),
            Some(HostFailoverState::HostAvailable)
        );
    }
    // The stand-in's changes survived the handback.
    let names = |lobby: &konnekt_session_core::Lobby| -> Vec<String> {
        lobby
            .activity_queue()
            .iter()
            .map(|activity| activity.name.clone())
            .collect()
    };
    let host_lobby = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"));
    assert!(names(host_lobby).contains(&"Meanwhile".to_string()));
    assert_eq!(
        host_lobby.participants().len(),
        3,
        "both guests are still participants"
    );

    // Commands from both former guests reach the original host again, and
    // every peer keeps following its signed chain.
    for (index, guest) in session.guests.iter_mut().enumerate() {
        guest
            .submit_command(DomainCommand::QueueActivity {
                lobby_id: session.lobby_id,
                config: ActivityConfig::new(
                    "quiz".into(),
                    format!("Back {index}"),
                    serde_json::Value::Null,
                ),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    }
    session.tick(true, 10);
    let host_queue = names(
        session
            .host
            .get_lobby()
            .unwrap_or_else(|| panic!("no lobby")),
    );
    for guest in &session.guests {
        let queue = names(guest.get_lobby().unwrap_or_else(|| panic!("no lobby")));
        assert_eq!(queue, host_queue);
    }
    assert!(host_queue.contains(&"Back 0".to_string()));
    assert!(host_queue.contains(&"Back 1".to_string()));
    let host_key = session.keys[0].verifying_key().to_bytes();
    let history = session
        .host
        .signed_history()
        .unwrap_or_else(|| panic!("no history"));
    assert!(
        history
            .events()
            .iter()
            .all(|event| event.author == host_key)
    );
    for guest in &session.guests {
        assert_eq!(
            guest
                .signed_history_replica()
                .and_then(|replica| replica.tip()),
            history.tip()
        );
    }
}

#[test]
fn a_run_in_progress_survives_failover_without_waiting_for_the_old_host() {
    use konnekt_session_core::{RunStatus, domain::ActivityResult};
    let mut session = Session::new(Duration::from_secs(60));
    let lobby_id = session.lobby_id;
    let old_host = session
        .host
        .get_lobby()
        .unwrap_or_else(|| panic!("no lobby"))
        .host_id();
    session
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id,
            config: ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    session
        .host
        .submit_command(DomainCommand::StartNextRun { lobby_id })
        .unwrap_or_else(|error| panic!("{error}"));
    session.tick(true, 5);
    let run = session
        .host
        .get_active_run()
        .unwrap_or_else(|| panic!("no run"));
    let run_id = run.id();
    assert!(
        run.required_submitters().contains(&old_host),
        "the departed host must be removed for the run to complete"
    );
    let submit = |session: &mut Session, guest: usize| {
        let me = session.participant_id(guest);
        session.guests[guest]
            .submit_command(DomainCommand::SubmitResult {
                lobby_id,
                run_id,
                result: ActivityResult::new(run_id, me),
            })
            .unwrap_or_else(|error| panic!("{error}"));
    };
    submit(&mut session, 0);
    session.tick(true, 5);

    session.drop_host();
    session.tick(false, 10);
    let stand_in = (0..2)
        .find(|&guest| session.guests[guest].is_host())
        .unwrap_or_else(|| panic!("nobody took over"));
    let follower = 1 - stand_in;
    for guest in &session.guests {
        let run = guest
            .get_active_run()
            .unwrap_or_else(|| panic!("run lost in failover"));
        assert_eq!(run.id(), run_id);
        assert!(run.results().contains_key(&session.participant_id(0)));
    }

    // The remaining guest submits; nobody waits for the departed host.
    let pending = if session.guests[0]
        .get_active_run()
        .is_some_and(|run| run.results().contains_key(&session.participant_id(1)))
    {
        None
    } else {
        Some(1)
    };
    if let Some(guest) = pending {
        submit(&mut session, guest);
        session.tick(false, 5);
    }
    // The stand-in does not wait for the departed host.
    assert!(
        session.guests[stand_in]
            .get_run(&run_id)
            .is_some_and(|run| !run.required_submitters().contains(&old_host))
    );
    session.tick(false, 10);
    for guest in [stand_in, follower] {
        assert_eq!(
            session.guests[guest]
                .get_run(&run_id)
                .map(|run| run.status()),
            Some(RunStatus::Completed),
            "run completed for guest {guest}"
        );
        assert!(session.guests[guest].get_active_run().is_none());
    }
}
