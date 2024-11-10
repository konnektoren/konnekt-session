use crate::example::{LobbyPage, LoginCallback, LoginPage, PlayerProfile};
use crate::model::{Player, Role};
use std::str::FromStr;
use uuid::Uuid;
use yew::prelude::*;

#[derive(Debug, Default, Clone)]
enum AppState {
    #[default]
    Login,
    Lobby((Role, Player<PlayerProfile>, Uuid, Option<String>)),
}

#[function_component(App)]
pub fn app() -> Html {
    let state = use_state(|| AppState::Login);

    let on_login = {
        let state = state.clone();

        Callback::from(move |(player, role, lobby_id, password): LoginCallback| {
            let lobby_id: Uuid = Uuid::from_str(&lobby_id).unwrap();
            state.set(AppState::Lobby((role, player, lobby_id, password.clone())));
        })
    };

    match (&*state).clone() {
        AppState::Login => {
            html! {
                <LoginPage
                    on_login={on_login}
                />
            }
        }
        AppState::Lobby((role, player, lobby_id, password)) => {
            html! {
                <LobbyPage
                    role={role.clone()}
                    player={player.clone()}
                    lobby_id={lobby_id.clone()}
                    password={password.clone()}
                />
            }
        }
    }
}
