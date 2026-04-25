use cucumber::{given, then, when};
use konnekt_session_core::{DomainCommand, DomainEvent, LobbyRole, ParticipationMode};
use konnekt_session_p2p::SessionId;
use konnekt_session_tests::{SessionWorld, WhoAmIObservation};
use konnekt_session_yew::hooks::{P2PRole, SessionContext};
use std::rc::Rc;

#[given(expr = "a lobby named {string} with host {string}")]
async fn lobby_named_with_host(world: &mut SessionWorld, lobby_name: String, host_name: String) {
    let event = world
        .execute(DomainCommand::CreateLobby {
            lobby_id: None,
            lobby_name: lobby_name.clone(),
            host_name: host_name.clone(),
        })
        .clone();

    if let DomainEvent::LobbyCreated { lobby } = event {
        world.lobby_ids.insert(lobby_name, lobby.id());
        world.participant_ids.insert(host_name, lobby.host_id());
    } else {
        panic!("Expected LobbyCreated event");
    }
}

#[given(expr = "guest {string} has joined that lobby")]
async fn guest_has_joined(world: &mut SessionWorld, guest_name: String) {
    let lobby_id = *world
        .lobby_ids
        .values()
        .next()
        .expect("Expected a lobby to exist");

    let event = world
        .execute(DomainCommand::JoinLobby {
            lobby_id,
            guest_name: guest_name.clone(),
        })
        .clone();

    if let DomainEvent::GuestJoined { participant, .. } = event {
        world.participant_ids.insert(guest_name, participant.id());
    } else {
        panic!("Expected GuestJoined event");
    }
}

#[when(expr = "I resolve who am i for {string} as p2p role {string} with peer id {string}")]
async fn resolve_who_am_i_for(
    world: &mut SessionWorld,
    participant_name: String,
    p2p_role: String,
    peer_id: String,
) {
    let lobby = world
        .lobby_ids
        .keys()
        .next()
        .and_then(|name| world.get_lobby(name))
        .expect("Expected lobby to exist")
        .clone();

    let participant_id = world.get_participant_id(&participant_name);
    let is_host = match p2p_role.as_str() {
        "Host" => true,
        "Guest" => false,
        other => panic!("Unsupported p2p role '{}'", other),
    };

    let ctx = SessionContext {
        session_id: SessionId::new(),
        lobby: Some(lobby),
        peer_count: 1,
        is_host,
        active_run: None,
        local_participant_id: Some(participant_id),
        local_peer_id: Some(peer_id),
        send_command: Rc::new(|_| {}),
        local_participant_name: None, // explicit: identity should not rely on name tracking
    };

    let info = ctx.who_am_i_info();
    world.last_who_am_i = Some(WhoAmIObservation {
        local_peer_id: info.local_peer_id,
        p2p_role: match info.p2p_role {
            P2PRole::Host => "Host".to_string(),
            P2PRole::Guest => "Guest".to_string(),
        },
        participant_id: info.participant_id,
        participant_name: info.participant_name,
        lobby_role: info.lobby_role,
        participation_mode: info.participation_mode,
    });
}

#[then(expr = "who am i should report participant name {string}")]
async fn who_am_i_reports_participant_name(world: &mut SessionWorld, expected_name: String) {
    let actual = world
        .last_who_am_i
        .as_ref()
        .and_then(|i| i.participant_name.clone());
    assert_eq!(actual, Some(expected_name));
}

#[then(expr = "who am i should report lobby role {string}")]
async fn who_am_i_reports_lobby_role(world: &mut SessionWorld, expected_role: String) {
    let expected = match expected_role.as_str() {
        "Host" => LobbyRole::Host,
        "Guest" => LobbyRole::Guest,
        other => panic!("Unsupported lobby role '{}'", other),
    };
    let actual = world.last_who_am_i.as_ref().and_then(|i| i.lobby_role);
    assert_eq!(actual, Some(expected));
}

#[then(expr = "who am i should report participation mode {string}")]
async fn who_am_i_reports_participation_mode(world: &mut SessionWorld, expected_mode: String) {
    let expected = match expected_mode.as_str() {
        "Active" => ParticipationMode::Active,
        "Spectating" => ParticipationMode::Spectating,
        other => panic!("Unsupported participation mode '{}'", other),
    };
    let actual = world.last_who_am_i.as_ref().and_then(|i| i.participation_mode);
    assert_eq!(actual, Some(expected));
}

#[then(expr = "who am i should report p2p role {string}")]
async fn who_am_i_reports_p2p_role(world: &mut SessionWorld, expected_role: String) {
    let actual = world
        .last_who_am_i
        .as_ref()
        .map(|i| i.p2p_role.clone())
        .expect("Expected who_am_i observation");
    assert_eq!(actual, expected_role);
}

#[then(expr = "who am i should report local peer id {string}")]
async fn who_am_i_reports_local_peer_id(world: &mut SessionWorld, expected_peer_id: String) {
    let actual = world
        .last_who_am_i
        .as_ref()
        .and_then(|i| i.local_peer_id.clone());
    assert_eq!(actual, Some(expected_peer_id));
}

#[then(expr = "who am i should report participant id for {string}")]
async fn who_am_i_reports_participant_id(world: &mut SessionWorld, participant_name: String) {
    let expected = world.get_participant_id(&participant_name);
    let actual = world
        .last_who_am_i
        .as_ref()
        .and_then(|i| i.participant_id)
        .expect("Expected participant id in who_am_i");
    assert_eq!(actual, expected);
}
