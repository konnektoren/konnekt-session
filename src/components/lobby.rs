use crate::components::{ActivityCatalogComp, ActivityComp, PlayerListComp};
use crate::model::{Activity, ActivityData, CommandError, Lobby, LobbyCommand, PlayerData, Role};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LobbyProps<P, A>
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    pub lobby: Lobby<P, A>,
    pub role: Role,
    pub on_command: Callback<LobbyCommand>,
    #[prop_or_default]
    pub on_error: Callback<CommandError>,
}

#[function_component(LobbyComp)]
pub fn lobby_comp<P, A>(props: &LobbyProps<P, A>) -> Html
where
    P: PlayerData + 'static,
    A: ActivityData + 'static,
{
    let is_admin = props.role == Role::Admin;

    let on_select = {
        let on_command = props.on_command.clone();
        Callback::from(move |activity: Activity<A>| {
            on_command.emit(LobbyCommand::SelectActivity {
                activity_id: activity.id.clone(),
            });
        })
    };

    html! {
        <div class="konnekt-session-lobby">
            <h1 class="konnekt-session-lobby__title">{"Lobby"}</h1>
            <div class="konnekt-session-lobby__players">
                <PlayerListComp<P> players={props.lobby.participants.clone()} />
            </div>
            if is_admin {
                <ActivityCatalogComp<A>
                    catalog={props.lobby.catalog.clone()}
                    {on_select}
                />
            }
            <div class="konnekt-session-lobby__activities">
                <h2 class="konnekt-session-lobby__activities-title">{"Activities"}</h2>
                <div class="konnekt-session-lobby__activities-list">
                    {for props.lobby.activities.iter().map(|activity| {
                        let activity = activity.clone();
                        html! {
                            <ActivityComp<A>
                                {activity}
                                role={props.role}
                                on_command={props.on_command.clone()}
                            />
                        }
                    })}
                </div>
            </div>
        </div>
    }
}
