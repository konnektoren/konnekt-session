use crate::components::PlayerListComp;
use crate::model::{Lobby, PlayerData};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LobbyProps<T>
where
    T: PlayerData + 'static,
{
    pub lobby: Lobby<T>,
}

#[function_component(LobbyComp)]
pub fn lobby_comp<T>(props: &LobbyProps<T>) -> Html
where
    T: PlayerData + 'static,
{
    html! {
        <div class="konnekt-session-lobby">
            <h1 class="konnekt-session-lobby__title">{"Lobby"}</h1>
            <div class="konnekt-session-lobby__players">
                <PlayerListComp<T> players={props.lobby.participants.clone()} />
            </div>
            <div class="konnekt-session-lobby__activities">
                <h2 class="konnekt-session-lobby__activities-title">{"Activities"}</h2>
                <ul class="konnekt-session-lobby__activities-list">
                    {for props.lobby.activities.iter().map(|activity| {
                        html! {
                            <li class="konnekt-session-lobby__activities-list-item">
                                {activity.name.clone()}
                            </li>
                        }
                    })}
                </ul>
            </div>
        </div>
    }
}
