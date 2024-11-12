use crate::model::{ActivityResult, ActivityResultTrait};
use serde::Serialize;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityResultProps<T>
where
    T: ActivityResultTrait + Serialize + 'static,
{
    pub result: ActivityResult<T>,
}

#[function_component(ActivityResultComp)]
pub fn activity_result_comp<T>(props: &ActivityResultProps<T>) -> Html
where
    T: ActivityResultTrait + Serialize + 'static,
{
    let result = &props.result;

    html! {
        <div class="konnekt-session-activity-result">
            <div class="konnekt-session-activity-result__player">
                {result.data.identifier()}
            </div>
            <div class="konnekt-session-activity-result__score">
                {result.data.score()}
            </div>
            <div class="konnekt-session-activity-result__time">
                {result.data.time_taken()}
            </div>
        </div>
    }
}
