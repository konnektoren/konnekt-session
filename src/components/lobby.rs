use crate::components::PlayerListComp;
use crate::model::{ActivityData, Lobby, Named, PlayerData};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LobbyProps<P, A>
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    pub lobby: Lobby<P, A>,
}

#[function_component(LobbyComp)]
pub fn lobby_comp<P, A>(props: &LobbyProps<P, A>) -> Html
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    html! {
        <div class="konnekt-session-lobby">
            <h1 class="konnekt-session-lobby__title">{"Lobby"}</h1>
            <div class="konnekt-session-lobby__players">
                <PlayerListComp<P> players={props.lobby.participants.clone()} />
            </div>
            <div class="konnekt-session-lobby__activities">
                <h2 class="konnekt-session-lobby__activities-title">{"Activities"}</h2>
                <ul class="konnekt-session-lobby__activities-list">
                    {for props.lobby.activities.iter().map(|activity| {
                        html! {
                            <li class="konnekt-session-lobby__activities-list-item">
                                {activity.name().to_string()}
                            </li>
                        }
                    })}
                </ul>
            </div>
        </div>
    }
}
