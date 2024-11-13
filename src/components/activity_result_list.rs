use crate::components::ActivityResultComp;
use crate::model::{ActivityResult, ActivityResultTrait, Player, PlayerTrait};
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
                        html! {
                            <ActivityResultComp<P, T> {player} {result} />
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
