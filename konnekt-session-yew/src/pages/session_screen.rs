use crate::components::{ActivityList, ActivityPlanner, ActivitySubmission, ParticipantList, SessionInfo};
use crate::hooks::use_session;
use konnekt_session_core::{DomainCommand, RunStatus};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionScreenProps {
    pub on_leave: Callback<()>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ViewMode {
    Lobby,
    ActivityInProgress,
}

#[function_component(SessionScreen)]
pub fn session_screen(props: &SessionScreenProps) -> Html {
    let session = use_session();
    let view_mode = use_state(|| ViewMode::Lobby);

    {
        let view_mode = view_mode.clone();
        let active_run = session.active_run.clone();

        use_effect_with(active_run, move |run| {
            if let Some(run) = run {
                if run.status == RunStatus::InProgress {
                    view_mode.set(ViewMode::ActivityInProgress);
                } else {
                    view_mode.set(ViewMode::Lobby);
                }
            } else {
                view_mode.set(ViewMode::Lobby);
            }
            || ()
        });
    }

    let on_toggle_participation = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        let session_clone = session.clone();

        Callback::from(move |_: MouseEvent| {
            if let (Some(lobby), Some(participant_id)) = (&lobby, session_clone.get_local_participant_id())
            {
                send_command(DomainCommand::ToggleParticipationMode {
                    lobby_id: lobby.id(),
                    participant_id,
                    requester_id: participant_id,
                });
            }
        })
    };

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
                    &session.active_run,
                    session.is_host,
                    session.get_local_participant_id(),
                    on_toggle_participation,
                ),
                ViewMode::ActivityInProgress => html! {
                    <ActivitySubmission
                        lobby={session.lobby.clone()}
                        active_run={session.active_run.clone()}
                        is_host={session.is_host}
                        participant_id={session.get_local_participant_id()}
                    />
                },
            }}
        </div>
    }
}

fn render_lobby_view(
    lobby: &Option<konnekt_session_core::Lobby>,
    active_run: &Option<crate::hooks::ActiveRunSnapshot>,
    is_host: bool,
    local_participant_id: Option<uuid::Uuid>,
    on_toggle_participation: Callback<MouseEvent>,
) -> Html {
    if let Some(lobby) = lobby {
        let has_planned_activities = !lobby.activity_queue().is_empty();

        html! {
            <div class="konnekt-session-screen__content">
                <div class="konnekt-session-screen__column">
                    <ParticipantList
                        lobby={lobby.clone()}
                        local_participant_id={local_participant_id}
                    />

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
                    <ActivityList lobby={lobby.clone()} active_run={active_run.clone()} />

                    {if !is_host && !has_planned_activities && active_run.is_none() {
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
