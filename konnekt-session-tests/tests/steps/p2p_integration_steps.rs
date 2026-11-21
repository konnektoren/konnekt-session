use cucumber::{given, then, when};
use konnekt_session_core::{DomainCommand, DomainEvent as CoreDomainEvent, Participant};
use konnekt_session_p2p::{DomainEvent as P2PDomainEvent, EventTranslator};
use konnekt_session_tests::SessionWorld;
use uuid::Uuid;

// ===== Given Steps =====

#[given("a P2P session is initialized")]
async fn p2p_session_initialized(world: &mut SessionWorld) {
    // Ensure we have a lobby ID for translation
    world.get_or_create_lobby_id();
}

#[given(expr = "the core domain emits a GuestJoined event")]
async fn core_emits_guest_joined(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let participant = Participant::new_guest("Alice".to_string()).unwrap();

    world.last_event = Some(CoreDomainEvent::GuestJoined {
        lobby_id,
        participant,
    });
}

#[given(expr = "a GuestJoined event is received from P2P")]
async fn p2p_event_received(world: &mut SessionWorld) {
    let participant = Participant::new_guest("Bob".to_string()).unwrap();

    // Store as P2P event (will be translated)
    world.last_error =
        Some(serde_json::to_string(&P2PDomainEvent::GuestJoined { participant }).unwrap());
}

#[given(expr = "a core GuestJoined event for {string}")]
async fn core_guest_joined_for(world: &mut SessionWorld, name: String) {
    let lobby_id = world.get_or_create_lobby_id();
    let participant = Participant::new_guest(name).unwrap();

    world.last_event = Some(CoreDomainEvent::GuestJoined {
        lobby_id,
        participant,
    });
}

#[given("the core domain emits a CommandFailed event")]
async fn core_emits_command_failed(world: &mut SessionWorld) {
    world.last_event = Some(CoreDomainEvent::CommandFailed {
        command: "TestCommand".to_string(),
        reason: "Test error".to_string(),
    });
}

#[given("a core HostDelegated event")]
async fn core_host_delegated(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let from = Uuid::new_v4();
    let to = Uuid::new_v4();

    world.last_event = Some(CoreDomainEvent::HostDelegated { lobby_id, from, to });
}

#[given("a core ParticipationModeChanged event")]
async fn core_participation_mode_changed(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let participant_id = Uuid::new_v4();

    world.last_event = Some(CoreDomainEvent::ParticipationModeChanged {
        lobby_id,
        participant_id,
        new_mode: konnekt_session_core::ParticipationMode::Spectating,
    });
}

// ===== When Steps =====

#[when("the P2P loop processes the event")]
async fn p2p_processes_event(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let translator = EventTranslator::new(lobby_id);

    if let Some(core_event) = world.last_event.clone() {
        let p2p_event = translator.to_p2p_event(core_event);

        // Store result for verification
        if let Some(event) = p2p_event {
            world.last_error = Some(serde_json::to_string(&event).unwrap());
        }
    }
}

#[when("the P2P loop polls")]
async fn p2p_loop_polls(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let translator = EventTranslator::new(lobby_id);

    // Parse P2P event from stored JSON
    if let Some(json) = &world.last_error {
        if let Ok(p2p_event) = serde_json::from_str::<P2PDomainEvent>(json) {
            let cmd = translator.to_domain_command(&p2p_event);

            if let Some(command) = cmd {
                world.last_command = Some(command);
            }
        }
    }
}

#[when("the event is translated to P2P and back to command")]
async fn roundtrip_translation(world: &mut SessionWorld) {
    let lobby_id = world.get_or_create_lobby_id();
    let translator = EventTranslator::new(lobby_id);

    if let Some(core_event) = world.last_event.clone() {
        // Core → P2P
        if let Some(p2p_event) = translator.to_p2p_event(core_event) {
            // P2P → Command
            if let Some(cmd) = translator.to_domain_command(&p2p_event) {
                world.last_command = Some(cmd);
            }
        }
    }
}

#[when("the event is broadcast via P2P")]
async fn event_broadcast_via_p2p(world: &mut SessionWorld) {
    // Same as "processes the event"
    p2p_processes_event(world).await;
}

// ===== Then Steps =====

#[then("the event should be broadcast to all peers")]
async fn event_should_be_broadcast(world: &mut SessionWorld) {
    // Verify we have a P2P event stored
    assert!(world.last_error.is_some(), "No P2P event was created");
}

#[then("the event should have a sequence number assigned")]
async fn event_has_sequence_number(_world: &mut SessionWorld) {
    // Sequence assignment happens in EventSyncManager during actual broadcast
    // This is tested in P2P layer unit tests
}

/// Peers add the joined participant as-is (`AddParticipant`), so every peer
/// keeps the same participant ID. `JoinLobby` would create a new one.
#[then("an AddParticipant command should be queued")]
async fn add_participant_command_queued(world: &mut SessionWorld) {
    match world.last_command.as_ref() {
        Some(DomainCommand::AddParticipant { .. }) => {}
        other => panic!("Expected AddParticipant, got: {:?}", other),
    }
}

#[then("the command should have the correct lobby ID")]
async fn command_has_correct_lobby_id(world: &mut SessionWorld) {
    let expected_lobby_id = world.get_or_create_lobby_id();
    match world.last_command.as_ref().unwrap() {
        DomainCommand::AddParticipant { lobby_id, .. } => {
            assert_eq!(*lobby_id, expected_lobby_id);
        }
        other => panic!("Expected AddParticipant command, got: {:?}", other),
    }
}

#[then("the command should keep the participant ID from P2P")]
async fn command_keeps_participant_id(world: &mut SessionWorld) {
    let json = world.last_error.as_ref().expect("No P2P event");
    let Ok(P2PDomainEvent::GuestJoined { participant: sent }) =
        serde_json::from_str::<P2PDomainEvent>(json)
    else {
        panic!("Expected GuestJoined P2P event");
    };
    match world.last_command.as_ref().unwrap() {
        DomainCommand::AddParticipant { participant, .. } => {
            assert_eq!(participant.id(), sent.id());
        }
        other => panic!("Expected AddParticipant command, got: {:?}", other),
    }
}

#[then(expr = "the resulting command should contain {string}")]
async fn command_contains_name(world: &mut SessionWorld, name: String) {
    match world.last_command.as_ref().unwrap() {
        DomainCommand::AddParticipant { participant, .. } => {
            assert_eq!(participant.name(), name);
        }
        other => panic!("Expected AddParticipant command, got: {:?}", other),
    }
}

#[then("no P2P event should be broadcast")]
async fn no_p2p_event_broadcast(world: &mut SessionWorld) {
    // CommandFailed should not produce a P2P event
    assert!(
        world.last_error.is_none(),
        "P2P event was created but shouldn't have been"
    );
}

#[then("peers should receive HostDelegated")]
async fn peers_receive_host_delegated(world: &mut SessionWorld) {
    assert!(world.last_error.is_some());

    let json = world.last_error.as_ref().unwrap();
    let p2p_event: P2PDomainEvent = serde_json::from_str(json).unwrap();

    match p2p_event {
        P2PDomainEvent::HostDelegated { .. } => {}
        _ => panic!("Expected HostDelegated event"),
    }
}

/// Translates the broadcast P2P event back into the command a peer would run.
fn peer_command(world: &mut SessionWorld) -> Option<DomainCommand> {
    let lobby_id = world.get_or_create_lobby_id();
    let json = world
        .last_error
        .as_ref()
        .expect("No P2P event was broadcast");
    let p2p_event: P2PDomainEvent = serde_json::from_str(json).unwrap();
    EventTranslator::new(lobby_id).to_domain_command(&p2p_event)
}

#[then("peers should translate to DelegateHost command")]
async fn peers_translate_to_delegate_host(world: &mut SessionWorld) {
    match peer_command(world) {
        Some(DomainCommand::DelegateHost { .. }) => {}
        other => panic!("Expected DelegateHost command, got: {:?}", other),
    }
}

#[then("peers should receive ParticipationModeChanged")]
async fn peers_receive_participation_mode_changed(world: &mut SessionWorld) {
    assert!(world.last_error.is_some());

    let json = world.last_error.as_ref().unwrap();
    let p2p_event: P2PDomainEvent = serde_json::from_str(json).unwrap();

    match p2p_event {
        P2PDomainEvent::ParticipationModeChanged { .. } => {}
        _ => panic!("Expected ParticipationModeChanged event"),
    }
}

/// Peers apply the already-validated mode (`UpdateParticipantMode`);
/// `ToggleParticipationMode` is the local, permission-checked request.
#[then("peers should translate to UpdateParticipantMode command")]
async fn peers_translate_to_update_participant_mode(world: &mut SessionWorld) {
    match peer_command(world) {
        Some(DomainCommand::UpdateParticipantMode { .. }) => {}
        other => panic!("Expected UpdateParticipantMode command, got: {:?}", other),
    }
}
