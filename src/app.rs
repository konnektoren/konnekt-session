use konnekt_session::prelude::*;
use yew::prelude::*;

#[derive(PartialEq, Clone)]
struct PlayerProfile {
    id: String,
    name: String,
}

impl Identifiable for PlayerProfile {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl Named for PlayerProfile {
    fn name(&self) -> &str {
        &self.name
    }
}

#[function_component(App)]
pub fn app() -> Html {
    // Define your PlayerProfile
    let player_profile = PlayerProfile {
        id: "123".to_string(),
        name: "Admin".to_string(),
    };

    // Create a Player using PlayerProfile
    let player: Player<PlayerProfile> = Player::new(Role::Admin, player_profile);

    html! {
        <div>
            // Pass the Player<PlayerProfile> to PlayerComp
            <PlayerComp<PlayerProfile> player={player} />
        </div>
    }
}
