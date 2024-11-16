use crate::components::activity_result::ActivityResultProps;
use crate::model::{ActivityResult, ActivityResultTrait, Player, PlayerTrait};
use serde::Serialize;
use std::marker::PhantomData;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivityResultDetailProps<P, T, C>
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
    C: Component<Properties = ActivityResultProps<P, T>> + PartialEq + 'static,
{
    pub player: Player<P>,
    pub result: ActivityResult<T>,
    #[prop_or_default]
    pub _phantom: PhantomData<C>,
}

#[function_component(ActivityResultDetailComp)]
pub fn activity_result_detail_comp<P, T, C>(props: &ActivityResultDetailProps<P, T, C>) -> Html
where
    P: PlayerTrait + 'static,
    T: ActivityResultTrait + Serialize + 'static,
    C: Component<Properties = ActivityResultProps<P, T>> + PartialEq + 'static,
{
    html! {
        <div class="konnekt-session-activity-result-detail">
            <C player={props.player.clone()} result={props.result.clone()} />
        </div>
    }
}
