use crate::components::{LobbyComp, RunningActivityComp};
use crate::config::Config;
use crate::example::{Challenge, ChallengeComp, ChallengeResult, PlayerProfile};
use crate::handler::{LocalLobbyCommandHandler, WebSocketLobbyCommandHandler};
use crate::model::{
    Activity, ActivityResultTrait, ActivityStatus, ActivityTrait, CommandError, Lobby,
    LobbyCommand, LobbyCommandHandler, Player, PlayerTrait, Role,
};
use serde::Serialize;
use std::cell::RefCell;
use std::hash::Hash;
use std::hash::Hasher;
use uuid::Uuid;
use web_sys::Event;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

fn init_lobby(
    player: Player<PlayerProfile>,
    password: Option<String>,
) -> Lobby<PlayerProfile, Challenge, ChallengeResult> {
    let activity1 = Activity {
        id: "456".to_string(),
        status: ActivityStatus::NotStarted,
        data: Challenge {
            id: "456".to_string(),
            name: "Challenge 1".to_string(),
        },
    };

    let activity2 = Activity {
        id: "789".to_string(),
        status: ActivityStatus::NotStarted,
        data: Challenge {
            id: "789".to_string(),
            name: "Challenge 2".to_string(),
        },
    };

    let mut lobby = Lobby::<PlayerProfile, Challenge, ChallengeResult>::new(player, password);
    lobby.add_activity(activity1);
    lobby.add_activity(activity2);

    lobby
}

fn hash_lobby<
    P: PlayerTrait + Hash,
    A: ActivityTrait + Hash,
    AR: ActivityResultTrait + Hash + Serialize,
>(
    lobby: &Lobby<P, A, AR>,
) -> u64 {
    let mut hasher = std::hash::DefaultHasher::new();
    lobby.hash(&mut hasher);
    hasher.finish()
}

#[derive(Properties, PartialEq)]
pub struct LobbyProps {
    pub role: Role,
    pub player: Player<PlayerProfile>,
    pub lobby_id: Uuid,
    pub password: Option<String>,
}

#[function_component(LobbyPage)]
pub fn lobby_page(props: &LobbyProps) -> Html {
    let config = use_state(Config::default);
    let role = use_state(|| props.player.role);
    let lobby_id = use_state(|| props.lobby_id);

    let player = use_state(|| RefCell::new(props.player.clone()));
    let lobby =
        use_state(|| RefCell::new(init_lobby(props.player.clone(), props.password.clone())));

    let last_event = use_state(|| 0);

    // Create WebSocket handler
    let websocket_handler = use_state(|| {
        let local_handler =
            LocalLobbyCommandHandler::<PlayerProfile, Challenge, ChallengeResult>::new(
                |data: &str| serde_json::from_str(data).expect("Failed to deserialize player data"),
                |data: &str| {
                    serde_json::from_str(data).expect("Failed to deserialize challenge data")
                },
                |data: &str| {
                    serde_json::from_str(data).expect("Failed to deserialize challenge result data")
                },
            );

        let update_ui = Callback::from(
            move |lobby: Lobby<PlayerProfile, Challenge, ChallengeResult>| {
                last_event.set(hash_lobby(&lobby));
            },
        );

        let player = player.clone();
        let password = props.password.clone();

        WebSocketLobbyCommandHandler::new(
            &config.websocket_url,
            *lobby_id,
            player.clone(),
            password,
            local_handler.clone(),
            lobby.clone(),
            update_ui,
        )
    });

    let on_command = {
        let handler = websocket_handler.clone();
        Callback::from(move |command: LobbyCommand| {
            if let Err(err) = handler.send_command(command) {
                log::info!("Command error: {:?}", err);
            }
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
                "Participant" => Role::Player,
                _ => Role::Player,
            };
            role.set(selected_role);
        }
    };

    // Get current lobby state
    let current_lobby = (*lobby.borrow()).clone();

    html! {
        <div>
            <div>{"Connected to lobby: "}{lobby_id.to_string()}</div>
            <select onchange={on_change} value={role.to_string()}>
                <option value="Admin">{"Admin"}</option>
                <option value="Participant">{"Participant"}</option>
                <option value="Observer">{"Observer"}</option>
            </select>
            <LobbyComp<PlayerProfile, Challenge, ChallengeResult>
                lobby={current_lobby.clone()}
                role={*role}
                on_command={on_command.clone()}
                {on_error}
            />
            <RunningActivityComp<Challenge, ChallengeComp>
                player_id={props.player.id}
                activities={current_lobby.activities.clone()}
                role={*role}
                on_command={on_command}
            />
        </div>
    }
}
