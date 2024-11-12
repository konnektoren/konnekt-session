use crate::components::ActivityResultComp;
use crate::model::{ActivityResult, ActivityResultTrait};
use serde::Serialize;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityResultListProps<T>
where
    T: ActivityResultTrait + Serialize + 'static,
{
    pub results: Vec<ActivityResult<T>>,
}

#[function_component(ActivityResultListComp)]
pub fn activity_result_list_comp<T>(props: &ActivityResultListProps<T>) -> Html
where
    T: ActivityResultTrait + Serialize + 'static,
{
    html! {
        <div class="konnekt-session-activity-result-list">
            {for props.results.iter().map(|result| {
                let result: ActivityResult<T> = result.clone();
                html! {
                    <ActivityResultComp<T> {result} />
                }
            })}
        </div>
    }
}
