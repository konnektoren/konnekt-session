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

impl PlayerData for PlayerProfile {}

#[function_component(App)]
pub fn app() -> Html {
    let player_profile = PlayerProfile {
        id: "123".to_string(),
        name: "Admin".to_string(),
    };

    let player: Player<PlayerProfile> = Player::new(Role::Admin, player_profile);

    let mut lobby = Lobby::new(player, None);

    let participant = Player::new(
        Role::Participant,
        PlayerProfile {
            id: "456".to_string(),
            name: "Participant".to_string(),
        },
    );

    lobby.add_participant(participant);

    html! {
        <div>
            <LobbyComp<PlayerProfile> lobby={lobby} />
        </div>
    }
}
