use crate::components::PlayerComp;
use crate::model::{Player, PlayerId, PlayerTrait};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct PlayerListProps<T>
where
    T: PlayerTrait + 'static,
{
    pub players: Vec<Player<T>>,
    #[prop_or_default]
    pub on_select: Callback<PlayerId>,
}

#[function_component(PlayerListComp)]
pub fn player_list_comp<T>(props: &PlayerListProps<T>) -> Html
where
    T: PlayerTrait + 'static,
{
    html! {
        <div class="konnekt-session-player-list">
            {for props.players.iter().map(|player| {
                let player_id = player.id.clone();
                html! {
                    <div onclick={props.on_select.reform(move |_| player_id)}>
                        <PlayerComp<T> player={player.clone()} />
                    </div>
                }
            })}
        </div>
    }
}
