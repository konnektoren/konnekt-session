use konnekt_session_core::Lobby;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ParticipantListProps {
    pub lobby: Lobby,
}

/// Displays list of participants in the lobby
///
/// Shows:
/// - Host (with crown icon)
/// - Active participants (can submit results)
/// - Spectators (view-only)
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

                    let mode_class = if participant.can_submit_results() {
                        "active"
                    } else {
                        "spectating"
                    };

                    html! {
                        <li class={classes!("konnekt-participant-list__item", mode_class)}>
                            <span class="konnekt-participant-list__icon">{role_icon}</span>
                            <span class="konnekt-participant-list__name">{participant.name()}</span>
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
