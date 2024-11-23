use crate::components::ActivityResultComp;
use crate::model::{ActivityResult, ActivityResultTrait, Player, PlayerId, PlayerTrait};
use serde::Serialize;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityResultListProps<P, T>
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
{
    pub players: Vec<Player<P>>,
    pub results: Vec<ActivityResult<T>>,
    #[prop_or_default]
    pub on_select: Callback<(PlayerId, ActivityResult<T>)>,
}

#[function_component(ActivityResultListComp)]
pub fn activity_result_list_comp<P, T>(props: &ActivityResultListProps<P, T>) -> Html
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
{
    let players = props.players.clone();

    html! {
        <div class="konnekt-session-activity-result-list">
            {for props.results.iter().map(|result| {
                let result: ActivityResult<T> = result.clone();
                match players.iter().find(|player| player.id == result.player_id) {
                    Some(player) => {
                        let player = player.clone();
                        let on_select = {

                            let player_id = player.id;
                            let activity_result = result.clone();
                            let callback = props.on_select.clone();
                            Callback::from(move |_| {
                                callback.emit((player_id, activity_result.clone()))
                            })
                        };
                        html! {
                            <div onclick={on_select} >
                                <ActivityResultComp<P, T> player={player} result={result} />
                            </div>
                        }
                    }
                    None => {
                        html! {
                            <div class="konnekt-session-activity-result">
                                <div class="konnekt-session-activity-result__player">
                                    {"Unknown Player"}
                                </div>
                            </div>
                        }
                    }
                }
            })}
        </div>
    }
}
