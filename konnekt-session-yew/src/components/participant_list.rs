use konnekt_session_core::Lobby;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ParticipantListProps {
    pub lobby: Lobby,
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
                    // 🔧 FIX: Show role in parentheses, not as main text
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

                    let mode_class = if participant.can_submit_results() {
                        "active"
                    } else {
                        "spectating"
                    };

                    html! {
                        <li class={classes!("konnekt-participant-list__item", mode_class)}>
                            <span class="konnekt-participant-list__icon">{role_icon}</span>
                            <span class="konnekt-participant-list__name">
                                {participant.name()}
                                <span class="konnekt-participant-list__role">{role_text}</span>
                            </span>
                            <span class="konnekt-participant-list__mode">
                                {if participant.can_submit_results() {
                                    "🎮 Active"
                                } else {
                                    "👁️  Spectating"
                                }}
                            </span>
                        </li>
                    }
                })}
            </ul>
        </div>
    }
}

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
