#[allow(dead_code)]
mod support;

use ed25519_dalek::SigningKey;
use konnekt_session_core::{
    ActivityConfig, DomainCommand, DomainLoop, Participant,
    domain::{ActivityResult, RunStatus},
};
use konnekt_session_p2p::SessionLoopV2;
use konnekt_session_p2p::application::event_log_reconciliation::ReconciliationError;
use konnekt_session_p2p::application::merge_plan::DropReason;
use konnekt_session_p2p::application::signed_history::{event_hash, sign_event, verify_history};
use konnekt_session_p2p::domain::{
    DelegationReason, DomainEvent, LobbyEvent, SignedEvent, SignedHistoryBatch,
};
use konnekt_session_p2p::infrastructure::transport::P2PTransport;
use konnekt_session_p2p::protocol::event_log_reconciliation::EventLogReconciliationState as State;
use support::mock_connection::{MockConnection, create_mock_network};
use uuid::Uuid;

struct Split {
    host: SessionLoopV2<MockConnection>,
    guest: SessionLoopV2<MockConnection>,
    lobby_id: Uuid,
    host_key: [u8; 32],
    stand_in: SigningKey,
}

/// A returning host with a signed history and one connected guest.
fn returning_host() -> Split {
    let network = create_mock_network();
    let lobby_id = Uuid::new_v4();
    let key = SigningKey::from_bytes(&[11; 32]);
    let host_key = key.verifying_key().to_bytes();
    let mut host = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_host(MockConnection::new(network.clone()), 100),
        true,
        lobby_id,
    );
    host.enable_signed_history(key, 50)
        .unwrap_or_else(|error| panic!("{error}"));
    host.submit_command(DomainCommand::CreateLobby {
        lobby_id: Some(lobby_id),
        lobby_name: "Room".into(),
        host_name: "Host".into(),
    })
    .unwrap_or_else(|error| panic!("{error}"));
    let mut guest = SessionLoopV2::new(
        DomainLoop::new(10, 100),
        P2PTransport::new_guest(MockConnection::new(network), 100),
        false,
        lobby_id,
    );
    for _ in 0..5 {
        host.poll();
        guest.poll();
    }
    Split {
        host,
        guest,
        lobby_id,
        host_key,
        stand_in: SigningKey::from_bytes(&[12; 32]),
    }
}

/// The host's recorded history extended by events the stand-in signed.
fn stand_in_history(split: &Split, events: Vec<DomainEvent>) -> SignedHistoryBatch {
    let mut chain: Vec<SignedEvent> = split
        .host
        .signed_history()
        .unwrap_or_else(|| panic!("no recording"))
        .events()
        .iter()
        .cloned()
        .collect();
    for event in events {
        let sequence = chain.last().map_or(1, |last| last.sequence + 1);
        let signed = sign_event(
            &LobbyEvent::new(sequence, split.lobby_id, event),
            chain.last().map(event_hash),
            &split.stand_in,
        )
        .unwrap_or_else(|error| panic!("{error}"));
        chain.push(signed);
    }
    SignedHistoryBatch {
        anchor: None,
        events: chain,
    }
}

fn prepare(split: &mut Split, remote: &SignedHistoryBatch) {
    let peer = split.host.connected_peers()[0];
    let authors = [split.host_key, split.stand_in.verifying_key().to_bytes()];
    split
        .host
        .begin_reconciliation(peer)
        .unwrap_or_else(|error| panic!("{error}"));
    split
        .host
        .reconciliation_authority_checked(true)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        split
            .host
            .compare_signed_reconciliation_history(remote, None, &authors, split.host_key),
        Ok(State::PreparingMerge)
    );
}

#[test]
fn returning_host_replays_the_stand_in_history_and_broadcasts_it() {
    let mut split = returning_host();
    let alice = Participant::new_guest("Alice".into()).unwrap_or_else(|error| panic!("{error}"));
    let config = ActivityConfig::new("quiz".into(), "Quiz".into(), serde_json::Value::Null);
    let run_id = Uuid::new_v4();
    let result = ActivityResult::new(run_id, alice.id());
    let remote = stand_in_history(
        &split,
        vec![
            DomainEvent::GuestJoined {
                participant: alice.clone(),
            },
            DomainEvent::HostDelegated {
                from: Uuid::new_v4(),
                to: alice.id(),
                reason: DelegationReason::Timeout,
            },
            DomainEvent::ActivityQueued {
                config: config.clone(),
            },
            DomainEvent::RunStarted {
                run_id,
                config,
                required_submitters: vec![alice.id()],
            },
            DomainEvent::ResultSubmitted {
                run_id,
                result: result.clone(),
            },
            DomainEvent::RunEnded {
                run_id,
                status: RunStatus::Completed,
                results: vec![result],
            },
        ],
    );
    let before = split
        .host
        .signed_history()
        .and_then(|history| history.tip())
        .unwrap_or_else(|| panic!("no tip"));
    prepare(&mut split, &remote);

    let plan = split
        .host
        .prepare_reconciliation_merge(&remote)
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(plan.rebased.len(), 5);
    assert_eq!(plan.dropped.len(), 1);
    assert_eq!(plan.dropped[0].reason, DropReason::AuthorityChange);

    let outcome = split
        .host
        .apply_reconciliation_merge()
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(outcome.failed, vec![]);
    assert_eq!(outcome.applied.len(), 5);
    assert_eq!(split.host.reconciliation_state(), Some(State::Reconciled));

    // The returning host stays host and holds the stand-in's state.
    let lobby = split.host.get_lobby().unwrap_or_else(|| panic!("no lobby"));
    assert_ne!(lobby.host_id(), alice.id());
    assert!(lobby.participants().contains_key(&alice.id()));
    assert!(lobby.activity_queue().is_empty());
    assert_eq!(
        split.host.get_run(&run_id).map(|run| run.status()),
        Some(RunStatus::Completed)
    );

    // Replayed events are re-signed into the host's own chain.
    let history = split
        .host
        .signed_history()
        .unwrap_or_else(|| panic!("no recording"));
    let events: Vec<_> = history.events().iter().cloned().collect();
    assert!(events.iter().all(|event| event.author == split.host_key));
    assert!(
        history
            .tip()
            .is_some_and(|tip| tip.sequence > before.sequence)
    );
    assert!(verify_history(split.lobby_id, &events, &[split.host_key], None).is_ok());

    // Connected guests receive the replayed events like other host updates.
    for _ in 0..5 {
        split.host.poll();
        split.guest.poll();
    }
    let guest_lobby = split
        .guest
        .get_lobby()
        .unwrap_or_else(|| panic!("guest has no lobby"));
    assert!(guest_lobby.participants().contains_key(&alice.id()));
    assert!(guest_lobby.activity_queue().is_empty());

    assert_eq!(
        split.host.apply_reconciliation_merge(),
        Err(ReconciliationError::InvalidState)
    );
}

#[test]
fn a_plan_is_not_applied_after_the_host_history_changed() {
    let mut split = returning_host();
    let alice = Participant::new_guest("Alice".into()).unwrap_or_else(|error| panic!("{error}"));
    let remote = stand_in_history(
        &split,
        vec![DomainEvent::GuestJoined { participant: alice }],
    );
    prepare(&mut split, &remote);
    split
        .host
        .prepare_reconciliation_merge(&remote)
        .unwrap_or_else(|error| panic!("{error}"));
    split
        .host
        .submit_command(DomainCommand::QueueActivity {
            lobby_id: split.lobby_id,
            config: ActivityConfig::new("quiz".into(), "Late".into(), serde_json::Value::Null),
        })
        .unwrap_or_else(|error| panic!("{error}"));
    assert_eq!(
        split.host.apply_reconciliation_merge(),
        Err(ReconciliationError::HistoryMismatch)
    );
    assert_eq!(
        split.host.reconciliation_state(),
        Some(State::ApplyingMerge)
    );
}
