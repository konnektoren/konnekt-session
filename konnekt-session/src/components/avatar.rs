use crate::model::PlayerId;
use colorid::colorid;
use std::hash::{DefaultHasher, Hash, Hasher};
use yew::prelude::*;

const AVATARS: &str = include_str!("../assets/avatars.txt");

#[derive(Properties, PartialEq, Clone)]
pub struct AvatarProps {
    pub player_id: PlayerId,
    #[prop_or(false)]
    pub color: bool,
}

fn calculate_avatar_index(player_id: &PlayerId) -> usize {
    let mut hasher = DefaultHasher::new();
    player_id.hash(&mut hasher);
    hasher.finish() as usize
}

#[function_component(AvatarComp)]
pub fn avatar(props: &AvatarProps) -> Html {
    let avatars: Vec<&str> = AVATARS.lines().collect();

    let avatar_index = calculate_avatar_index(&props.player_id) % avatars.len();
    let avatar = avatars[avatar_index];

    match props.color {
        true => avatar_with_color(avatar, avatar_index),
        false => avatar_without_color(avatar),
    }
}

fn avatar_with_color(avatar: &str, avatar_index: usize) -> Html {
    let color = colorid(1 + avatar_index);
    let colors: Vec<&str> = color.split('-').collect();
    let foreground = colors[0];

    html! {
        <div class="konnekt-session-player__icon">
            <i class={format!("fa-solid {}", avatar)} style={format!("color: {};", foreground)}></i>
        </div>
    }
}

fn avatar_without_color(avatar: &str) -> Html {
    html! {
        <div class="konnekt-session-player__icon">
            <i class={format!("fa-solid {}", avatar)}></i>
        </div>
    }
}
