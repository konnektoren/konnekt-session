use crate::components::{LobbyComp, RunningActivityComp};
use crate::config::Config;
use crate::example::{Challenge, ChallengeComp, PlayerProfile};
use crate::handler::{LocalLobbyCommandHandler, WebSocketLobbyCommandHandler};
use crate::model::{
    Activity, ActivityStatus, CommandError, Lobby, LobbyCommand, LobbyCommandHandler, Player, Role,
};
use std::cell::RefCell;
use std::rc::Rc;
use uuid::Uuid;
use web_sys::Event;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

fn init_lobby() -> Lobby<PlayerProfile, Challenge> {
    let player_profile = PlayerProfile {
        id: "123".to_string(),
        name: "Admin".to_string(),
    };

    let player: Player<PlayerProfile> = Player::new(Role::Admin, player_profile);

    let activity = Activity {
        id: "789".to_string(),
        status: ActivityStatus::NotStarted,
        data: Challenge {
            id: "789".to_string(),
            name: "Challenge".to_string(),
        },
    };

    let mut lobby = Lobby::<PlayerProfile, Challenge>::new(player, None);
    lobby.add_activity(activity);

    let participant = Player::new(
        Role::Participant,
        PlayerProfile {
            id: "456".to_string(),
            name: "Participant".to_string(),
        },
    );

    lobby.add_participant(participant);

    lobby
}

#[function_component(App)]
pub fn app() -> Html {
    let config = use_state(|| Config::default());
    let role = use_state(|| Role::Admin);
    let lobby_id = use_state(|| Uuid::new_v4());
    let lobby = use_state(|| Rc::new(RefCell::new(init_lobby())));

    let event_counter = use_state(|| 0);

    // Create local handler for deserializing player data
    let local_handler = LocalLobbyCommandHandler::<PlayerProfile>::new(|data: &str| {
        serde_json::from_str(data).expect("Failed to deserialize player data")
    });

    // Create WebSocket handler
    let websocket_handler = use_state(|| {
        WebSocketLobbyCommandHandler::new(
            &config.websocket_url,
            *lobby_id,
            local_handler.clone(),
            (*lobby).clone(),
        )
    });

    let on_command = {
        let handler = (*websocket_handler).clone();
        let lobby = (*lobby).clone();
        let event_counter = event_counter.clone();
        Callback::from(move |command: LobbyCommand| {
            if let Err(err) = handler.handle_command(&mut lobby.borrow_mut(), command) {
                log::info!("Command error: {:?}", err);
            }

            event_counter.set(*event_counter + 1);
        })
    };

    let on_error = {
        Callback::from(move |err: CommandError| {
            log::error!("Command error: {:?}", err);
        })
    };

    let on_change = {
        let role = role.clone();
        move |e: Event| {
            let target = e.target_unchecked_into::<HtmlSelectElement>();
            let value = target.value();
            let selected_role = match value.as_str() {
                "Admin" => Role::Admin,
                "Participant" => Role::Participant,
                _ => Role::Participant,
            };
            role.set(selected_role);
        }
    };

    // Get current lobby state
    let current_lobby = {
        let lobby_ref = lobby.borrow();
        (*lobby_ref).clone()
    };

    html! {
        <div>
            <div>{"Connected to lobby: "}{lobby_id.to_string()}</div>
            <select onchange={on_change}>
                <option value="Admin">{"Admin"}</option>
                <option value="Participant">{"Participant"}</option>
            </select>
            <LobbyComp<PlayerProfile, Challenge>
                lobby={current_lobby.clone()}
                role={*role}
                on_command={on_command.clone()}
                {on_error}
            />
            <RunningActivityComp<Challenge, ChallengeComp>
                activities={current_lobby.activities.clone()}
                role={*role}
                on_command={on_command}
            />
        </div>
    }
}
