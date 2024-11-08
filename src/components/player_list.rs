use crate::components::PlayerComp;
use crate::model::{Player, PlayerData};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct PlayerListProps<T>
where
    T: PlayerData + 'static,
{
    pub players: Vec<Player<T>>,
}

#[function_component(PlayerListComp)]
pub fn player_list_comp<T>(props: &PlayerListProps<T>) -> Html
where
    T: PlayerData + 'static,
{
    html! {
        <div class="konnekt-session-player-list">
            {for props.players.iter().map(|player| {
                let player: Player<T> = player.clone();
                html! {
                    <PlayerComp<T> {player} />
                }
            })}
        </div>
    }
}
