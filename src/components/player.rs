use crate::model::{Named, Player, PlayerTrait};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct PlayerProps<T>
where
    T: PlayerTrait + 'static,
{
    pub player: Player<T>,
}

#[function_component(PlayerComp)]
pub fn player_comp<T>(props: &PlayerProps<T>) -> Html
where
    T: PlayerTrait + 'static,
{
    html! {
        <div class="konnekt-session-player">
            <h2 class="konnekt-session-player__name">{props.player.name()}</h2>
        </div>
    }
}
