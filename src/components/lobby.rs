use crate::components::{
    ActivityCatalogComp, ActivityComp, ActivityResultListComp, PlayerListComp,
};
use crate::model::{
    Activity, ActivityResult, ActivityResultTrait, ActivityTrait, CommandError, Lobby,
    LobbyCommand, PlayerTrait, Role,
};
use crate::prelude::PlayerId;
use serde::Serialize;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct LobbyProps<P, A, AR>
where
    P: PlayerTrait + 'static,
    A: ActivityTrait + 'static,
    AR: ActivityResultTrait + Serialize + 'static,
{
    pub lobby: Lobby<P, A, AR>,
    pub role: Role,
    pub on_command: Callback<LobbyCommand>,
    #[prop_or_default]
    pub on_error: Callback<CommandError>,
    #[prop_or_default]
    pub on_activity_result_select: Callback<(PlayerId, ActivityResult<AR>)>,
}

#[function_component(LobbyComp)]
pub fn lobby_comp<P, A, AR>(props: &LobbyProps<P, A, AR>) -> Html
where
    P: PlayerTrait + 'static,
    A: ActivityTrait + 'static,
    AR: ActivityResultTrait + Serialize + 'static,
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
            <div class="konnekt-session-lobby__content">
                <div class="konnekt-session-lobby__players">
                    <PlayerListComp<P> players={props.lobby.participants.clone()} />
                </div>
                <div class="konnekt-session-lobby__activities">
                    if is_admin {
                        <ActivityCatalogComp<A>
                            catalog={props.lobby.catalog.clone()}
                            {on_select}
                        />
                    }
                    <h2 class="konnekt-session-lobby__activities-title">{"Activities"}</h2>
                    <div class="konnekt-session-lobby__activities-list">
                        {for props.lobby.activities.iter().map(|activity| {
                            let activity = activity.clone();
                            html! {
                                <ActivityComp<A>
                                    player_id={props.lobby.player_id}
                                    {activity}
                                    role={props.role}
                                    on_command={props.on_command.clone()}
                                />
                            }
                        })}
                    </div>
                </div>
                <div class="konnekt-session-lobby_results">
                <ActivityResultListComp<P, AR> players={props.lobby.participants.clone()} results={props.lobby.results.clone()}
                    on_select={props.on_activity_result_select.clone()}
                />
                </div>
            </div>
        </div>
    }
}
