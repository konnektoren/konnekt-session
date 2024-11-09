use crate::components::{ActivityCatalogComp, PlayerListComp};
use crate::model::{Activity, ActivityData, Lobby, Named, PlayerData, Role};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LobbyProps<P, A>
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    pub lobby: Lobby<P, A>,
    pub role: Role,
}

#[function_component(LobbyComp)]
pub fn lobby_comp<P, A>(props: &LobbyProps<P, A>) -> Html
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    let lobby = use_state(|| props.lobby.clone());

    let is_admin = props.role == Role::Admin;

    let on_select = {
        let lobby = lobby.clone();
        Callback::from(move |activity: Activity<A>| {
            let mut new_lobby = (&*lobby).clone();
            new_lobby.select_activity(&activity.id);

            lobby.set(new_lobby);
        })
    };

    let catalog = (*lobby).catalog.clone();
    let players = (*lobby).participants.clone();
    let activities = (*lobby).activities.clone();

    html! {
        <div class="konnekt-session-lobby">
            <h1 class="konnekt-session-lobby__title">{"Lobby"}</h1>
            <div class="konnekt-session-lobby__players">
                <PlayerListComp<P> {players} />
            </div>
            if is_admin {
                <ActivityCatalogComp<A> {catalog} {on_select} />
            }
            <div class="konnekt-session-lobby__activities">
                <h2 class="konnekt-session-lobby__activities-title">{"Activities"}</h2>
                <ul class="konnekt-session-lobby__activities-list">
                    {for activities.iter().map(|activity| {
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
