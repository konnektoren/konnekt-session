use konnekt_session::prelude::*;
use web_sys::Event;
use web_sys::HtmlSelectElement;
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
    let role = use_state(|| Role::Admin);
    let lobby = use_state(|| {
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
        lobby
    });

    let handler = LocalLobbyCommandHandler;

    let on_command = {
        let lobby = lobby.clone();
        Callback::from(move |command: LobbyCommand| {
            let mut new_lobby = (*lobby).clone();
            match handler.handle_command(&mut new_lobby, command) {
                Ok(()) => lobby.set(new_lobby),
                Err(err) => {
                    log::error!("Command error: {:?}", err);
                    // You might want to show this error to the user
                }
            }
        })
    };

    let on_error = {
        Callback::from(move |err: CommandError| {
            log::error!("Command error: {:?}", err);
            // Handle error (e.g., show toast notification)
        })
    };

    let on_change = {
        let role = role.clone();
        move |e: Event| {
            let target = e.target_unchecked_into::<HtmlSelectElement>();
            let value = target.value();
            let selected_role = match value.as_str() {
                "Admin" => Role::Admin,
                "Participant" => Role::Participant,
                _ => Role::Participant,
            };
            role.set(selected_role);
        }
    };

    html! {
        <div>
            <select onchange={on_change}>
                <option value="Admin">{"Admin"}</option>
                <option value="Participant">{"Participant"}</option>
            </select>
            <LobbyComp<PlayerProfile, Challenge>
                lobby={(*lobby).clone()}
                role={*role}
                on_command={on_command}
                {on_error}
            />
        </div>
    }
}
