use crate::components::{
    ActivityList, ActivityPlanner, ActivitySubmission, ParticipantList, ResultsView, SessionInfo,
};
use crate::hooks::use_session;
use konnekt_session_core::DomainCommand;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionScreenProps {
    pub on_leave: Callback<()>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Lobby,
    ActivityInProgress,
    Results,
}

#[function_component(SessionScreen)]
pub fn session_screen(props: &SessionScreenProps) -> Html {
    let session = use_session();
    let view_mode = use_state(|| ViewMode::Lobby);

    // ✅ NO LOCAL STATE - determine view mode from Core only
    {
        let view_mode = view_mode.clone();
        let lobby = session.lobby.clone();

        use_effect_with(lobby.clone(), move |lobby_opt| {
            if let Some(lobby) = lobby_opt {
                // Check Core's activity status
                if let Some(current) = lobby.current_activity() {
                    if current.status == konnekt_session_core::domain::ActivityStatus::InProgress {
                        view_mode.set(ViewMode::ActivityInProgress);
                    } else if current.status
                        == konnekt_session_core::domain::ActivityStatus::Completed
                    {
                        view_mode.set(ViewMode::Results);
                    }
                } else if lobby
                    .activities()
                    .iter()
                    .any(|a| a.status == konnekt_session_core::domain::ActivityStatus::Completed)
                {
                    view_mode.set(ViewMode::Results);
                } else {
                    view_mode.set(ViewMode::Lobby);
                }
            }
            || ()
        });
    }

    // ===== CALLBACKS =====

    let on_toggle_participation = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        let session_clone = session.clone();

        Callback::from(move |_: MouseEvent| {
            if let (Some(lobby), Some(participant_id)) =
                (&lobby, session_clone.get_local_participant_id())
            {
                let activity_in_progress = lobby.current_activity().is_some();
                send_command(DomainCommand::ToggleParticipationMode {
                    lobby_id: lobby.id(),
                    participant_id,
                    requester_id: participant_id,
                    activity_in_progress,
                });
            }
        })
    };

    // ===== RENDER =====

    html! {
        <div class="konnekt-session-screen">
            <header class="konnekt-session-screen__header">
                <h1 class="konnekt-session-screen__title">
                    {if let Some(lobby) = session.lobby.as_ref() {
                        lobby.name().to_string()
                    } else {
                        "Loading...".to_string()
                    }}
                </h1>
                <button
                    class="konnekt-session-screen__leave-btn"
                    onclick={let on_leave = props.on_leave.clone(); move |_| on_leave.emit(())}
                >
                    {"Leave Lobby"}
                </button>
            </header>

            <SessionInfo
                session_id={session.session_id.to_string()}
                peer_count={session.peer_count}
                is_host={session.is_host}
            />

            {match *view_mode {
                ViewMode::Lobby => render_lobby_view(
                    &session.lobby,
                    session.is_host,
                    on_toggle_participation,
                ),
                ViewMode::ActivityInProgress => html! {
                    <ActivitySubmission
                        lobby={session.lobby.clone()}
                        is_host={session.is_host}
                        participant_id={session.get_local_participant_id()}
                    />
                },
                ViewMode::Results => html! {
                    <ResultsView
                        lobby={session.lobby.clone()}
                        is_host={session.is_host}
                    />
                },
            }}
        </div>
    }
}

fn render_lobby_view(
    lobby: &Option<konnekt_session_core::Lobby>,
    is_host: bool,
    on_toggle_participation: Callback<MouseEvent>,
) -> Html {
    if let Some(lobby) = lobby {
        let has_planned_activities = !lobby.activities().is_empty();

        html! {
            <div class="konnekt-session-screen__content">
                <div class="konnekt-session-screen__column">
                    <ParticipantList lobby={lobby.clone()} />

                    <div class="konnekt-session-screen__participation">
                        <button
                            class="konnekt-btn konnekt-btn--secondary"
                            onclick={on_toggle_participation}
                        >
                            {"Toggle Active/Spectating"}
                        </button>
                    </div>

                    {if is_host {
                        html! {
                            <ActivityPlanner lobby_id={lobby.id()} />
                        }
                    } else {
                        html! {}
                    }}
                </div>

                <div class="konnekt-session-screen__column">
                    <ActivityList lobby={lobby.clone()} />

                    {if !is_host && !has_planned_activities {
                        html! {
                            <div class="konnekt-session-screen__waiting">
                                <p>{"Waiting for host to plan activities..."}</p>
                            </div>
                        }
                    } else {
                        html! {}
                    }}
                </div>
            </div>
        }
    } else {
        html! {
            <div class="konnekt-session-screen__loading">
                <p>{"Syncing lobby from host..."}</p>
                <div class="konnekt-spinner"></div>
            </div>
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_screen_has_leave_callback() {
        let on_leave = Callback::from(|_: ()| {});
        let _props = yew::props!(SessionScreenProps { on_leave });
        assert!(true);
    }

    #[test]
    fn test_view_mode_enum() {
        let mode = ViewMode::Lobby;
        assert_eq!(mode, ViewMode::Lobby);
        assert_ne!(mode, ViewMode::ActivityInProgress);
    }
}
