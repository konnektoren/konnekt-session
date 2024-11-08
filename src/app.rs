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

#[derive(PartialEq, Clone)]
struct Challenge {
    id: String,
    name: String,
}

impl Named for Challenge {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Identifiable for Challenge {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl ActivityData for Challenge {}

#[function_component(App)]
pub fn app() -> Html {
    let player_profile = PlayerProfile {
        id: "123".to_string(),
        name: "Admin".to_string(),
    };

    let player: Player<PlayerProfile> = Player::new(Role::Admin, player_profile);

    let activity = Activity {
        id: "789".to_string(),
        status: ActivityStatus::NotStarted,
        data: Challenge {
            id: "789".to_string(),
            name: "Challenge".to_string(),
        },
    };

    let mut lobby = Lobby::<PlayerProfile, Challenge>::new(player, None);
    lobby.add_activity(activity);

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
            <LobbyComp<PlayerProfile, Challenge> lobby={lobby} />
        </div>
    }
}
