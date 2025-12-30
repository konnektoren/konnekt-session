use cucumber::{given, then, when};
use konnekt_session_core::{
    DomainCommand, DomainEvent as CoreDomainEvent, Lobby, Participant, ParticipationMode,
};
use konnekt_session_p2p::application::EventTranslator;
use konnekt_session_p2p::domain::DomainEvent as P2PDomainEvent;
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, cucumber::World)]
pub struct TranslatorWorld {
    lobby_id: Uuid,
    translator: Option<EventTranslator>,

    // For creating test data
    participant_ids: HashMap<String, Uuid>,

    // P2P events
    current_p2p_event: Option<P2PDomainEvent>,

    // Core events
    current_core_event: Option<CoreDomainEvent>,

    // Translation results
    translated_command: Option<Option<DomainCommand>>,
    translated_p2p_event: Option<Option<P2PDomainEvent>>,
}

impl Default for TranslatorWorld {
    fn default() -> Self {
        let lobby_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
        Self {
            lobby_id,
            translator: None,
            participant_ids: HashMap::new(),
            current_p2p_event: None,
            current_core_event: None,
            translated_command: None,
            translated_p2p_event: None,
        }
    }
}

impl TranslatorWorld {
    fn get_or_create_participant_id(&mut self, name: &str) -> Uuid {
        if let Some(id) = self.participant_ids.get(name) {
            *id
        } else {
            let id = Uuid::new_v4();
            self.participant_ids.insert(name.to_string(), id);
            id
        }
    }
}

// ===== Given Steps =====

#[given(expr = "a lobby with ID {string}")]
async fn lobby_with_id(world: &mut TranslatorWorld, lobby_id_str: String) {
    // For simplicity, we use a fixed UUID format
    world.lobby_id = Uuid::parse_str("00000000-0000-0000-0000-000000000123").unwrap();
}

#[given(expr = "an event translator for lobby {string}")]
async fn create_translator(world: &mut TranslatorWorld, _lobby_id_str: String) {
    world.translator = Some(EventTranslator::new(world.lobby_id));
}

// P2P Events

#[given(expr = "a P2P event {string} for participant {string}")]
async fn p2p_event_for_participant(
    world: &mut TranslatorWorld,
    event_type: String,
    participant_name: String,
) {
    match event_type.as_str() {
        "GuestJoined" => {
            let participant = Participant::new_guest(participant_name).unwrap();
            world.current_p2p_event = Some(P2PDomainEvent::GuestJoined { participant });
        }
        "GuestLeft" => {
            let participant_id = world.get_or_create_participant_id(&participant_name);
            world.current_p2p_event = Some(P2PDomainEvent::GuestLeft { participant_id });
        }
        "ParticipationModeChanged" => {
            let participant_id = world.get_or_create_participant_id(&participant_name);
            world.current_p2p_event = Some(P2PDomainEvent::ParticipationModeChanged {
                participant_id,
                new_mode: "Spectating".to_string(),
            });
        }
        _ => panic!("Unknown P2P event type: {}", event_type),
    }
}

#[given(expr = "a P2P event {string} with guest {string} kicked by {string}")]
async fn p2p_event_guest_kicked(
    world: &mut TranslatorWorld,
    event_type: String,
    guest_name: String,
    host_name: String,
) {
    assert_eq!(event_type, "GuestKicked");

    let guest_id = world.get_or_create_participant_id(&guest_name);
    let host_id = world.get_or_create_participant_id(&host_name);

    world.current_p2p_event = Some(P2PDomainEvent::GuestKicked {
        participant_id: guest_id,
        kicked_by: host_id,
    });
}

#[given(expr = "a P2P event {string} from {string} to {string}")]
async fn p2p_event_host_delegated(
    world: &mut TranslatorWorld,
    event_type: String,
    from_name: String,
    to_name: String,
) {
    assert_eq!(event_type, "HostDelegated");

    let from = world.get_or_create_participant_id(&from_name);
    let to = world.get_or_create_participant_id(&to_name);

    world.current_p2p_event = Some(P2PDomainEvent::HostDelegated {
        from,
        to,
        reason: konnekt_session_p2p::DelegationReason::Manual,
    });
}

#[given(expr = "a P2P event {string} for lobby {string}")]
async fn p2p_event_lobby_created(
    world: &mut TranslatorWorld,
    event_type: String,
    _lobby_name: String,
) {
    assert_eq!(event_type, "LobbyCreated");

    world.current_p2p_event = Some(P2PDomainEvent::LobbyCreated {
        lobby_id: world.lobby_id,
        host_id: Uuid::new_v4(),
        name: "Test Lobby".to_string(),
    });
}

// Core Events

#[given(expr = "a core event {string} for lobby {string}")]
async fn core_event_lobby_created(
    world: &mut TranslatorWorld,
    event_type: String,
    lobby_name: String,
) {
    assert_eq!(event_type, "LobbyCreated");

    let host = Participant::new_host("Host".to_string()).unwrap();
    let lobby = Lobby::with_id(world.lobby_id, lobby_name, host).unwrap();

    world.current_core_event = Some(CoreDomainEvent::LobbyCreated { lobby });
}

#[given(expr = "a core event {string} for participant {string}")]
async fn core_event_for_participant(
    world: &mut TranslatorWorld,
    event_type: String,
    participant_name: String,
) {
    match event_type.as_str() {
        "GuestJoined" => {
            let participant = Participant::new_guest(participant_name).unwrap();
            world.current_core_event = Some(CoreDomainEvent::GuestJoined {
                lobby_id: world.lobby_id,
                participant,
            });
        }
        "GuestLeft" => {
            let participant_id = world.get_or_create_participant_id(&participant_name);
            world.current_core_event = Some(CoreDomainEvent::GuestLeft {
                lobby_id: world.lobby_id,
                participant_id,
            });
        }
        _ => panic!("Unknown core event type: {}", event_type),
    }
}

#[given(expr = "a core event {string} with guest {string} kicked by {string}")]
async fn core_event_guest_kicked(
    world: &mut TranslatorWorld,
    event_type: String,
    guest_name: String,
    host_name: String,
) {
    assert_eq!(event_type, "GuestKicked");

    let guest_id = world.get_or_create_participant_id(&guest_name);
    let host_id = world.get_or_create_participant_id(&host_name);

    world.current_core_event = Some(CoreDomainEvent::GuestKicked {
        lobby_id: world.lobby_id,
        participant_id: guest_id,
        kicked_by: host_id,
    });
}

#[given(expr = "a core event {string} from {string} to {string}")]
async fn core_event_host_delegated(
    world: &mut TranslatorWorld,
    event_type: String,
    from_name: String,
    to_name: String,
) {
    assert_eq!(event_type, "HostDelegated");

    let from = world.get_or_create_participant_id(&from_name);
    let to = world.get_or_create_participant_id(&to_name);

    world.current_core_event = Some(CoreDomainEvent::HostDelegated {
        lobby_id: world.lobby_id,
        from,
        to,
    });
}

#[given(expr = "a core event {string} for participant {string} to {string}")]
async fn core_event_participation_mode_changed(
    world: &mut TranslatorWorld,
    event_type: String,
    participant_name: String,
    mode: String,
) {
    assert_eq!(event_type, "ParticipationModeChanged");

    let participant_id = world.get_or_create_participant_id(&participant_name);
    let new_mode = if mode == "Spectating" {
        ParticipationMode::Spectating
    } else {
        ParticipationMode::Active
    };

    world.current_core_event = Some(CoreDomainEvent::ParticipationModeChanged {
        lobby_id: world.lobby_id,
        participant_id,
        new_mode,
    });
}

#[given(expr = "a core event {string} with reason {string}")]
async fn core_event_command_failed(
    world: &mut TranslatorWorld,
    event_type: String,
    reason: String,
) {
    assert_eq!(event_type, "CommandFailed");

    world.current_core_event = Some(CoreDomainEvent::CommandFailed {
        command: "Test".to_string(),
        reason,
    });
}

// ===== When Steps =====

#[when("I translate the P2P event to a domain command")]
async fn translate_p2p_to_command(world: &mut TranslatorWorld) {
    let translator = world
        .translator
        .as_ref()
        .expect("Translator not initialized");
    let p2p_event = world.current_p2p_event.as_ref().expect("No P2P event set");

    let result = translator.to_domain_command(p2p_event);
    world.translated_command = Some(result);
}

#[when("I translate the core event to a P2P event")]
async fn translate_core_to_p2p(world: &mut TranslatorWorld) {
    let translator = world
        .translator
        .as_ref()
        .expect("Translator not initialized");
    let core_event = world.current_core_event.take().expect("No core event set");

    let result = translator.to_p2p_event(core_event);
    world.translated_p2p_event = Some(result);
}

#[when("I translate to a command and back to a P2P event")]
async fn roundtrip_translation(world: &mut TranslatorWorld) {
    let translator = world
        .translator
        .as_ref()
        .expect("Translator not initialized");
    let p2p_event = world.current_p2p_event.as_ref().expect("No P2P event set");

    // Step 1: P2P → Command
    let command = translator
        .to_domain_command(p2p_event)
        .expect("Should produce command");

    // Step 2: Simulate domain processing (would execute command and produce event)
    // For testing, we manually create the corresponding core event
    let core_event = match p2p_event {
        P2PDomainEvent::GuestJoined { participant } => CoreDomainEvent::GuestJoined {
            lobby_id: world.lobby_id,
            participant: participant.clone(),
        },
        P2PDomainEvent::GuestLeft { participant_id } => CoreDomainEvent::GuestLeft {
            lobby_id: world.lobby_id,
            participant_id: *participant_id,
        },
        _ => panic!("Roundtrip not implemented for this event type"),
    };

    // Step 3: Core → P2P
    let result = translator.to_p2p_event(core_event);
    world.translated_p2p_event = Some(result);
}

// ===== Then Steps =====

#[then(expr = "the command should be {string}")]
async fn command_type_is(world: &mut TranslatorWorld, expected_type: String) {
    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    let actual_type = match command {
        DomainCommand::CreateLobby { .. } => "CreateLobby",
        DomainCommand::JoinLobby { .. } => "JoinLobby",
        DomainCommand::LeaveLobby { .. } => "LeaveLobby",
        DomainCommand::KickGuest { .. } => "KickGuest",
        DomainCommand::ToggleParticipationMode { .. } => "ToggleParticipationMode",
        DomainCommand::DelegateHost { .. } => "DelegateHost",
        _ => todo!(),
    };

    assert_eq!(actual_type, expected_type, "Command type mismatch");
}

#[then(expr = "the command lobby ID should be {string}")]
async fn command_lobby_id_is(world: &mut TranslatorWorld, _expected_id: String) {
    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    let lobby_id = match command {
        DomainCommand::JoinLobby { lobby_id, .. } => lobby_id,
        DomainCommand::LeaveLobby { lobby_id, .. } => lobby_id,
        DomainCommand::KickGuest { lobby_id, .. } => lobby_id,
        DomainCommand::ToggleParticipationMode { lobby_id, .. } => lobby_id,
        DomainCommand::DelegateHost { lobby_id, .. } => lobby_id,
        _ => panic!("Command doesn't have lobby_id"),
    };

    assert_eq!(*lobby_id, world.lobby_id, "Lobby ID mismatch");
}

#[then(expr = "the command should contain guest name {string}")]
async fn command_contains_guest_name(world: &mut TranslatorWorld, expected_name: String) {
    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::JoinLobby { guest_name, .. } => {
            assert_eq!(*guest_name, expected_name);
        }
        _ => panic!("Command doesn't have guest_name"),
    }
}

#[then(expr = "the command should contain participant ID {string}")]
async fn command_contains_participant_id(world: &mut TranslatorWorld, participant_name: String) {
    let expected_id = world
        .participant_ids
        .get(&participant_name)
        .expect("Participant not found");

    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::LeaveLobby { participant_id, .. } => {
            assert_eq!(*participant_id, *expected_id);
        }
        DomainCommand::ToggleParticipationMode { participant_id, .. } => {
            assert_eq!(*participant_id, *expected_id);
        }
        _ => panic!("Command doesn't have participant_id"),
    }
}

#[then(expr = "the command should contain guest ID {string}")]
async fn command_contains_guest_id(world: &mut TranslatorWorld, guest_name: String) {
    let expected_id = world
        .participant_ids
        .get(&guest_name)
        .expect("Guest not found");

    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::KickGuest { guest_id, .. } => {
            assert_eq!(*guest_id, *expected_id);
        }
        _ => panic!("Command doesn't have guest_id"),
    }
}

#[then(expr = "the command should contain host ID {string}")]
async fn command_contains_host_id(world: &mut TranslatorWorld, host_name: String) {
    let expected_id = world
        .participant_ids
        .get(&host_name)
        .expect("Host not found");

    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::KickGuest { host_id, .. } => {
            assert_eq!(*host_id, *expected_id);
        }
        _ => panic!("Command doesn't have host_id"),
    }
}

#[then(expr = "the command should contain current host {string}")]
async fn command_contains_current_host(world: &mut TranslatorWorld, host_name: String) {
    let expected_id = world
        .participant_ids
        .get(&host_name)
        .expect("Host not found");

    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::DelegateHost {
            current_host_id, ..
        } => {
            assert_eq!(*current_host_id, *expected_id);
        }
        _ => panic!("Command doesn't have current_host_id"),
    }
}

#[then(expr = "the command should contain new host {string}")]
async fn command_contains_new_host(world: &mut TranslatorWorld, guest_name: String) {
    let expected_id = world
        .participant_ids
        .get(&guest_name)
        .expect("Guest not found");

    let command = world
        .translated_command
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match command {
        DomainCommand::DelegateHost { new_host_id, .. } => {
            assert_eq!(*new_host_id, *expected_id);
        }
        _ => panic!("Command doesn't have new_host_id"),
    }
}

#[then("the translation should return None")]
async fn translation_returns_none(world: &mut TranslatorWorld) {
    if let Some(result) = &world.translated_command {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else if let Some(result) = &world.translated_p2p_event {
        assert!(result.is_none(), "Expected None, got {:?}", result);
    } else {
        panic!("No translation was performed");
    }
}

// P2P Event Assertions

#[then(expr = "the P2P event should be {string}")]
async fn p2p_event_type_is(world: &mut TranslatorWorld, expected_type: String) {
    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    let actual_type = match event {
        P2PDomainEvent::LobbyCreated { .. } => "LobbyCreated",
        P2PDomainEvent::GuestJoined { .. } => "GuestJoined",
        P2PDomainEvent::GuestLeft { .. } => "GuestLeft",
        P2PDomainEvent::GuestKicked { .. } => "GuestKicked",
        P2PDomainEvent::HostDelegated { .. } => "HostDelegated",
        P2PDomainEvent::ParticipationModeChanged { .. } => "ParticipationModeChanged",
        _ => todo!(),
    };

    assert_eq!(actual_type, expected_type, "P2P event type mismatch");
}

#[then(expr = "the P2P event should contain lobby ID {string}")]
async fn p2p_event_contains_lobby_id(world: &mut TranslatorWorld, _expected_id: String) {
    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::LobbyCreated { lobby_id, .. } => {
            assert_eq!(*lobby_id, world.lobby_id);
        }
        _ => panic!("Event doesn't have lobby_id"),
    }
}

#[then(expr = "the P2P event should contain lobby name {string}")]
async fn p2p_event_contains_lobby_name(world: &mut TranslatorWorld, expected_name: String) {
    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::LobbyCreated { name, .. } => {
            assert_eq!(*name, expected_name);
        }
        _ => panic!("Event doesn't have lobby name"),
    }
}

#[then(expr = "the P2P event should contain participant name {string}")]
async fn p2p_event_contains_participant_name(world: &mut TranslatorWorld, expected_name: String) {
    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::GuestJoined { participant } => {
            assert_eq!(participant.name(), expected_name);
        }
        _ => panic!("Event doesn't have participant name"),
    }
}

#[then(expr = "the P2P event should contain participant ID {string}")]
async fn p2p_event_contains_participant_id(world: &mut TranslatorWorld, participant_name: String) {
    let expected_id = world
        .participant_ids
        .get(&participant_name)
        .expect("Participant not found");

    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::GuestLeft { participant_id } => {
            assert_eq!(*participant_id, *expected_id);
        }
        P2PDomainEvent::ParticipationModeChanged { participant_id, .. } => {
            assert_eq!(*participant_id, *expected_id);
        }
        _ => panic!("Event doesn't have participant_id"),
    }
}

#[then(expr = "the P2P event should contain guest ID {string}")]
async fn p2p_event_contains_guest_id(world: &mut TranslatorWorld, guest_name: String) {
    let expected_id = world
        .participant_ids
        .get(&guest_name)
        .expect("Guest not found");

    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::GuestKicked { participant_id, .. } => {
            assert_eq!(*participant_id, *expected_id);
        }
        _ => panic!("Event doesn't have guest_id"),
    }
}

#[then(expr = "the P2P event should contain kicked by {string}")]
async fn p2p_event_contains_kicked_by(world: &mut TranslatorWorld, host_name: String) {
    let expected_id = world
        .participant_ids
        .get(&host_name)
        .expect("Host not found");

    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::GuestKicked { kicked_by, .. } => {
            assert_eq!(*kicked_by, *expected_id);
        }
        _ => panic!("Event doesn't have kicked_by"),
    }
}

#[then(expr = "the P2P event should contain from {string}")]
async fn p2p_event_contains_from(world: &mut TranslatorWorld, from_name: String) {
    let expected_id = world
        .participant_ids
        .get(&from_name)
        .expect("From ID not found");

    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::HostDelegated { from, .. } => {
            assert_eq!(*from, *expected_id);
        }
        _ => panic!("Event doesn't have from"),
    }
}

#[then(expr = "the P2P event should contain to {string}")]
async fn p2p_event_contains_to(world: &mut TranslatorWorld, to_name: String) {
    let expected_id = world
        .participant_ids
        .get(&to_name)
        .expect("To ID not found");

    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::HostDelegated { to, .. } => {
            assert_eq!(*to, *expected_id);
        }
        _ => panic!("Event doesn't have to"),
    }
}

#[then(expr = "the P2P event should contain mode {string}")]
async fn p2p_event_contains_mode(world: &mut TranslatorWorld, expected_mode: String) {
    let event = world
        .translated_p2p_event
        .as_ref()
        .expect("No translation result")
        .as_ref()
        .expect("Translation returned None");

    match event {
        P2PDomainEvent::ParticipationModeChanged { new_mode, .. } => {
            assert_eq!(*new_mode, expected_mode);
        }
        _ => panic!("Event doesn't have mode"),
    }
}

#[then(expr = "the final P2P event should be {string}")]
async fn final_p2p_event_type(world: &mut TranslatorWorld, expected_type: String) {
    p2p_event_type_is(world, expected_type).await;
}

#[then(expr = "the participant name should be preserved as {string}")]
async fn participant_name_preserved(world: &mut TranslatorWorld, expected_name: String) {
    p2p_event_contains_participant_name(world, expected_name).await;
}

#[then(expr = "the participant ID should be preserved as {string}")]
async fn participant_id_preserved(world: &mut TranslatorWorld, participant_name: String) {
    p2p_event_contains_participant_id(world, participant_name).await;
}
