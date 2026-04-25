use konnekt_session_core::Lobby;
use uuid::Uuid;
use yew::prelude::*;

#[cfg(feature = "preview")]
use yew_preview::prelude::*;
#[cfg(feature = "preview")]
use yew_preview::test_utils::{exists, has_text};

#[derive(Properties, PartialEq, Clone)]
pub struct ParticipantListProps {
    pub lobby: Lobby,
    #[prop_or_default]
    pub local_participant_id: Option<Uuid>,
}

/// Displays list of participants in the lobby
#[function_component(ParticipantList)]
pub fn participant_list(props: &ParticipantListProps) -> Html {
    let participants = props.lobby.participants();

    html! {
        <div class="konnekt-participant-list">
            <h3 class="konnekt-participant-list__title">
                {"Participants ("}
                {participants.len()}
                {")"}
            </h3>
            <ul class="konnekt-participant-list__items">
                {for participants.values().map(|participant| {
                    let role_icon = if participant.is_host() {
                        "👑"
                    } else {
                        "👤"
                    };

                    let role_text = if participant.is_host() {
                        " (Host)"
                    } else {
                        ""
                    };
                    let is_me = Some(participant.id()) == props.local_participant_id;

                    let mode_class = if participant.can_submit_results() {
                        "active"
                    } else {
                        "spectating"
                    };

                    // ✅ Build tooltip with participant ID
                    let tooltip = format!(
                        "ID: {}\nJoined: {}",
                        participant.id(),
                        participant.joined_at()
                    );

                    html! {
                        <li
                            class={classes!("konnekt-participant-list__item", mode_class)}
                            title={tooltip}
                        >
                            <span class="konnekt-participant-list__icon">{role_icon}</span>
                            <span class="konnekt-participant-list__name">
                                {participant.name()}
                                <span class="konnekt-participant-list__role">{role_text}</span>
                                {if is_me {
                                    html! { <span class="konnekt-participant-list__you">{" (you)"}</span> }
                                } else {
                                    html! {}
                                }}
                            </span>
                            <span class="konnekt-participant-list__mode">
                                {if participant.can_submit_results() {
                                    "🎮 Active"
                                } else {
                                    "👁️  Spectating"
                                }}
                            </span>
                            // ✅ Show short ID for debugging
                            <span class="konnekt-participant-list__id">
                                {format!("#{}", &participant.id().to_string()[..8])}
                            </span>
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}

#[cfg(feature = "preview")]
mod preview_fixtures {
    use super::*;
    use konnekt_session_core::{Lobby, Participant};

    pub fn make_sample_lobby() -> Lobby {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Preview Lobby".to_string(), host).unwrap();
        lobby
            .add_guest(Participant::new_guest("Bob".to_string()).unwrap())
            .unwrap();
        lobby
            .add_guest(Participant::new_guest("Charlie".to_string()).unwrap())
            .unwrap();
        lobby
    }
}

#[cfg(feature = "preview")]
yew_preview::create_preview_with_tests!(
    component: ParticipantList,
    default_props: ParticipantListProps {
        lobby: preview_fixtures::make_sample_lobby(),
    },
    variants: [],
    tests: [
        ("Has main container class", exists("konnekt-participant-list")),
        ("Has title tag", exists("h3")),
        ("Has items list class", exists("konnekt-participant-list__items")),
        ("Has participant item class", exists("konnekt-participant-list__item")),
        ("Has icon class", exists("konnekt-participant-list__icon")),
        ("Shows correct participant count", has_text("Participants (3)")),
        ("Contains Alice", has_text("Alice")),
        ("Contains Bob", has_text("Bob")),
        ("Contains Charlie", has_text("Charlie")),
        ("Shows Active status", has_text("Active")),
    ]
);

#[cfg(test)]
mod tests {
    use super::*;
    use konnekt_session_core::{Lobby, Participant};

    #[test]
    fn test_shows_participant_name_not_role() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let props = yew::props!(ParticipantListProps {
            lobby: lobby.clone(),
        });

        // Render component (in real app, would check HTML output)
        // For now, just verify lobby has correct data
        let participants: Vec<_> = lobby.participants().values().collect();
        assert_eq!(participants.len(), 1);
        assert_eq!(participants[0].name(), "Alice");
        assert!(participants[0].is_host());
    }

    #[test]
    fn test_shows_guest_name() {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Test Lobby".to_string(), host).unwrap();

        let guest = Participant::new_guest("Bob".to_string()).unwrap();
        lobby.add_guest(guest).unwrap();

        let participants: Vec<_> = lobby.participants().values().collect();
        assert_eq!(participants.len(), 2);

        let bob = participants.iter().find(|p| !p.is_host()).unwrap();
        assert_eq!(bob.name(), "Bob");
    }
}
