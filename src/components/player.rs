use crate::model::{Identifiable, Named, Player};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct PlayerProps<T>
where
    T: Identifiable + Named + PartialEq,
{
    pub player: Player<T>,
}

#[function_component(PlayerComp)]
pub fn player_comp<T>(props: &PlayerProps<T>) -> Html
where
    T: Identifiable + Named + PartialEq,
{
    html! {
        <div class="konnekt-session-player">
            <h2 class="konnekt-session-player__name">{props.player.name()}</h2>
        </div>
    }
}
