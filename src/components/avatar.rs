use crate::model::PlayerId;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use yew::prelude::*;

const AVATARS: &str = include_str!("../assets/avatars.txt");

#[derive(Properties, PartialEq, Clone)]
pub struct AvatarProps {
    pub player_id: PlayerId,
}

fn calculate_avatar_index(player_id: &PlayerId, avatars_len: usize) -> usize {
    let mut hasher = DefaultHasher::new();
    player_id.hash(&mut hasher);
    (hasher.finish() as usize) % avatars_len
}

#[function_component(AvatarComp)]
pub fn avatar(props: &AvatarProps) -> Html {
    let avatars: Vec<&str> = AVATARS.lines().collect();

    let avatar_index = calculate_avatar_index(&props.player_id, avatars.len());
    let avatar = avatars[avatar_index];

    html! {
        <div class="konnekt-session-player__icon">
            <i class={format!("fa-solid {}", avatar)}></i>
        </div>
    }
}
