use crate::components::{
    use_lobby, ActivityResultDetailComp, LobbyComp, LobbyProvider, LobbyProviderConfig,
    RunningActivityComp,
};
use crate::config::Config;
use crate::example::{
    Challenge, ChallengeComp, ChallengeResult, ChallengeResultComp, PlayerProfile,
};
use crate::handler::NetworkHandler;
use crate::model::{
    network::TransportType, Activity, ActivityResult, ActivityStatus, CommandError, Lobby,
    LobbyCommand, LobbyCommandHandler, LobbyId, Player, PlayerId, Role,
};
use crate::prelude::use_lobby_handler;
use web_sys::Event;
use web_sys::HtmlSelectElement;
use yew::prelude::*;

fn init_lobby(
    lobby_id: LobbyId,
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

    let mut lobby =
        Lobby::<PlayerProfile, Challenge, ChallengeResult>::new_with_id(lobby_id, player, password);
    lobby.add_activity(activity1);
    lobby.add_activity(activity2);

    lobby
}

#[derive(Properties, PartialEq)]
pub struct LobbyPageProps {
    pub role: Role,
    pub player: Player<PlayerProfile>,
    pub lobby_id: LobbyId,
    pub password: Option<String>,
}

#[function_component(LobbyPage)]
pub fn lobby_page(props: &LobbyPageProps) -> Html {
    let config = use_state(Config::default);
    let role = use_state(|| props.player.role);

    let on_change = {
        let role = role.clone();
        move |e: Event| {
            let target = e.target_unchecked_into::<HtmlSelectElement>();
            let value = target.value();
            let selected_role = match value.as_str() {
                "Admin" => Role::Admin,
                "Player" => Role::Player,
                _ => Role::Player,
            };
            role.set(selected_role);
        }
    };

    let transport = TransportType::WebSocket(config.websocket_url.clone());

    /*
    let transport = TransportType::WebRTC(
        config.websocket_url.clone(),
        props.lobby_id.clone(),
        props.player.id.clone(),
        *role,
    );
    */

    /*
    let transport = TransportType::Matchbox(
        config.websocket_url.clone(),
        props.lobby_id.clone(),
        props.player.id.clone(),
        *role,
    );
    */

    let lobby_provider_config = LobbyProviderConfig {
        transport,
        player: props.player.clone(),
        lobby: init_lobby(props.lobby_id, props.player.clone(), props.password.clone()),
        role: *role,
        debug: true,
    };

    html! {
        <div>
            <LobbyProvider<PlayerProfile, Challenge, ChallengeResult>
                config={lobby_provider_config}
            >
            <select class="konnekt-session-lobby__role" onchange={on_change} value={role.to_string()}>
                <option value="Admin">{"Admin"}</option>
                <option value="Participant">{"Participant"}</option>
                <option value="Observer">{"Observer"}</option>
            </select>

            <LobbyInnerComp player={props.player.clone()} role={*role} />

            </LobbyProvider<PlayerProfile, Challenge, ChallengeResult>>
        </div>
    }
}

#[derive(Properties, PartialEq)]
pub struct LobbyInnerProps {
    pub role: Role,
    pub player: Player<PlayerProfile>,
}

#[function_component(LobbyInnerComp)]
pub fn lobby_inner_comp(props: &LobbyInnerProps) -> Html {
    let lobby = use_lobby::<_, _, _>();
    let lobby_handler = use_lobby_handler::<PlayerProfile, Challenge, ChallengeResult>();

    let current_lobby = (*lobby).clone();

    let selected_activity_result =
        use_state(|| None::<(PlayerId, ActivityResult<ChallengeResult>)>);

    let on_command = {
        let handler = lobby_handler.clone();
        Callback::from(move |command: LobbyCommand| {
            let handler: NetworkHandler<_, _, _> = (*handler).clone();
            if let Err(err) = handler.send_command(command) {
                log::error!("Command error: {:?}", err);
            }
        })
    };

    let on_error = {
        Callback::from(move |err: CommandError| {
            log::error!("Command error: {:?}", err);
        })
    };

    let activity_result_detail = {
        let lobby = lobby.clone();
        match selected_activity_result.as_ref() {
            Some((player_id, result)) => {
                let player = lobby
                    .participants
                    .iter()
                    .find(|p| p.id == *player_id)
                    .unwrap()
                    .clone();
                html! {
                    <ActivityResultDetailComp<_, _, ChallengeResultComp>
                        {player}
                        result={result.clone()}
                    />
                }
            }
            None => {
                html! {}
            }
        }
    };

    let on_activity_result_select = {
        let selected_activity_result = selected_activity_result.clone();

        Callback::from(
            move |(player_id, result): (PlayerId, ActivityResult<ChallengeResult>)| {
                selected_activity_result.set(Some((player_id, result.clone())));
            },
        )
    };

    html! {
        <div>
            <LobbyComp<PlayerProfile, Challenge, ChallengeResult>
                lobby={current_lobby.clone()}
                role={props.role}
                on_command={on_command.clone()}
                {on_error}
                {on_activity_result_select}
            />
            {activity_result_detail}
            <RunningActivityComp<Challenge, ChallengeComp>
                player_id={props.player.id}
                activities={current_lobby.activities.clone()}
                role={props.role}
                on_command={on_command}
            />
        </div>
    }
}
