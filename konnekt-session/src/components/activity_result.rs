use super::AvatarComp;
use crate::model::{ActivityResult, ActivityResultTrait, Named, Player, PlayerTrait};
use serde::Serialize;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityResultProps<P, T>
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
{
    pub player: Player<P>,
    pub result: ActivityResult<T>,
}

#[function_component(ActivityResultComp)]
pub fn activity_result_comp<P, T>(props: &ActivityResultProps<P, T>) -> Html
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
{
    let player = &props.player;
    let result = &props.result;

    let minutes = result.data.time_taken() / 60;
    let seconds = result.data.time_taken() % 60;

    html! {
        <div class="konnekt-session-activity-result">
            <div class="konnekt-session-activity-result__player">
                <AvatarComp player_id={player.id} />
                {player.name()}
            </div>
            <div class="konnekt-session-activity-result__score">
                <span><i class="fas fa-trophy konnekt-session-activity-result__score-icon"></i>{"Score"}</span>
                <span>{result.data.score()}</span>
            </div>
            <div class="konnekt-session-activity-result__time">
                <span><i class="fas fa-clock konnekt-session-activity-result__time-icon"></i>{"Time"}</span>
                <span>{format!("{}:{:02}", minutes, seconds)}</span>
            </div>
        </div>
    }
}
