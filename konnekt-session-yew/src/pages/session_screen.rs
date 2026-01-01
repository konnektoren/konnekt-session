use crate::components::{ActivityList, ParticipantList, SessionInfo};
use crate::hooks::use_session;
use konnekt_session_core::{DomainCommand, EchoChallenge, EchoResult};
use yew::prelude::*;

const ACTIVITY_TEMPLATES: &[(&str, &str)] = &[
    ("Echo: Hello Rust", "Hello Rust"),
    ("Echo: WebAssembly", "WebAssembly"),
    ("Echo: Konnekt", "Konnekt"),
    ("Echo: P2P Session", "P2P Session"),
    ("Echo: DDD + Hexagonal", "DDD + Hexagonal"),
];

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
    let selected_activity = use_state(|| 0usize);
    let view_mode = use_state(|| ViewMode::Lobby);
    let activity_response = use_state(String::new);

    // Determine current view based on lobby state
    {
        let view_mode = view_mode.clone();
        let lobby = session.lobby.clone();

        use_effect_with(lobby.clone(), move |lobby_opt| {
            if let Some(lobby) = lobby_opt {
                // Check for current in-progress activity
                if let Some(current) = lobby.current_activity() {
                    tracing::info!("Activity in progress: {}", current.name);
                    view_mode.set(ViewMode::ActivityInProgress);
                }
                // Check for completed activities
                else if lobby
                    .activities()
                    .iter()
                    .any(|a| a.status == konnekt_session_core::domain::ActivityStatus::Completed)
                {
                    tracing::info!("Activity completed, showing results");
                    view_mode.set(ViewMode::Results);
                }
                // Back to lobby view
                else {
                    tracing::info!("No active activity, showing lobby");
                    view_mode.set(ViewMode::Lobby);
                }
            }
            || ()
        });
    }

    // ===== CALLBACKS =====

    let on_plan_activity = {
        let selected = *selected_activity;
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(lobby) = &lobby {
                if let Some((name, prompt)) = ACTIVITY_TEMPLATES.get(selected) {
                    let challenge = EchoChallenge::new((*prompt).to_string());
                    let metadata = konnekt_session_core::domain::ActivityMetadata::new(
                        "echo-challenge-v1".to_string(),
                        (*name).to_string(),
                        challenge.to_config(),
                    );

                    send_command(DomainCommand::PlanActivity {
                        lobby_id: lobby.id(),
                        metadata,
                    });
                }
            }
        })
    };

    let on_select_activity = {
        let selected_activity = selected_activity.clone();
        Callback::from(move |idx: usize| {
            selected_activity.set(idx);
        })
    };

    let on_start_activity = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(lobby) = &lobby {
                if let Some(first_activity) = lobby.activities().first() {
                    send_command(DomainCommand::StartActivity {
                        lobby_id: lobby.id(),
                        activity_id: first_activity.id,
                    });
                }
            }
        })
    };

    let on_response_input = {
        let activity_response = activity_response.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            activity_response.set(input.value());
        })
    };

    let on_submit_response = {
        let activity_response = activity_response.clone();
        let lobby = session.lobby.clone();
        let send_command = session.send_command.clone();
        let participant_id = session.local_participant_id;

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            tracing::info!("📤 Attempting to submit response");

            if participant_id.is_none() {
                tracing::error!("❌ Cannot submit: No local participant ID");
                return;
            }

            if lobby.is_none() {
                tracing::error!("❌ Cannot submit: No lobby");
                return;
            }

            let lobby = lobby.as_ref().unwrap();
            let participant_id = participant_id.unwrap();

            if let Some(current_activity) = lobby.current_activity() {
                let response = (*activity_response).clone();

                tracing::info!(
                    "📤 Submitting response for activity {} by participant {}",
                    current_activity.id,
                    participant_id
                );

                if let Ok(challenge) = EchoChallenge::from_config(current_activity.config.clone()) {
                    let score = challenge.calculate_score(&response);
                    let echo_result = EchoResult::new(response.clone(), 1000);

                    tracing::info!(
                        "✅ Response '{}' scored: {} (expected: '{}')",
                        response,
                        score,
                        challenge.prompt
                    );

                    let result = konnekt_session_core::domain::ActivityResult::new(
                        current_activity.id,
                        participant_id,
                    )
                    .with_data(echo_result.to_json())
                    .with_score(score)
                    .with_time(1000);

                    send_command(DomainCommand::SubmitResult {
                        lobby_id: lobby.id(),
                        result,
                    });

                    activity_response.set(String::new());

                    tracing::info!("✅ Result submitted successfully");
                } else {
                    tracing::error!("❌ Failed to parse activity config");
                }
            } else {
                tracing::error!("❌ No current activity to submit to");
            }
        })
    };

    let on_cancel_activity = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(lobby) = &lobby {
                if let Some(current_activity) = lobby.current_activity() {
                    send_command(DomainCommand::CancelActivity {
                        lobby_id: lobby.id(),
                        activity_id: current_activity.id,
                    });
                }
            }
        })
    };

    let on_toggle_participation = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();
        let participant_id = session.local_participant_id;
        Callback::from(move |_: MouseEvent| {
            if let (Some(lobby), Some(participant_id)) = (&lobby, participant_id) {
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
                    *selected_activity,
                    on_select_activity,
                    on_plan_activity,
                    on_start_activity,
                    on_toggle_participation,
                ),
                ViewMode::ActivityInProgress => render_activity_view(
                    &session.lobby,
                    session.is_host,
                    &activity_response,
                    on_response_input,
                    on_submit_response,
                    on_cancel_activity,
                ),
                ViewMode::Results => render_results_view(&session.lobby),
            }}
        </div>
    }
}

// ===== VIEW RENDERERS =====

fn render_lobby_view(
    lobby: &Option<konnekt_session_core::Lobby>,
    is_host: bool,
    selected_activity: usize,
    on_select_activity: Callback<usize>,
    on_plan_activity: Callback<MouseEvent>,
    on_start_activity: Callback<MouseEvent>,
    on_toggle_participation: Callback<MouseEvent>,
) -> Html {
    if let Some(lobby) = lobby {
        let has_planned_activities = !lobby.activities().is_empty();

        html! {
            <div class="konnekt-session-screen__content">
                <div class="konnekt-session-screen__column">
                    <ParticipantList lobby={lobby.clone()} />

                    // Participation mode toggle (for all users)
                    <div class="konnekt-session-screen__participation">
                        <button
                            class="konnekt-btn konnekt-btn--secondary"
                            onclick={on_toggle_participation}
                        >
                            {"Toggle Active/Spectating"}
                        </button>
                    </div>

                    // Host controls
                    {if is_host {
                        html! {
                            <div class="konnekt-session-screen__activity-planner">
                                <h3>{"Plan Activity"}</h3>
                                <ul class="konnekt-activity-templates">
                                    {for ACTIVITY_TEMPLATES.iter().enumerate().map(|(idx, (name, _))| {
                                        let is_selected = idx == selected_activity;
                                        html! {
                                            <li
                                                class={classes!(
                                                    "konnekt-activity-template",
                                                    is_selected.then(|| "selected")
                                                )}
                                                onclick={let on_select = on_select_activity.clone(); move |_| on_select.emit(idx)}
                                            >
                                                {*name}
                                            </li>
                                        }
                                    })}
                                </ul>
                                <button
                                    class="konnekt-btn konnekt-btn--primary"
                                    onclick={on_plan_activity}
                                >
                                    {"Plan Selected Activity"}
                                </button>

                                {if has_planned_activities {
                                    html! {
                                        <button
                                            class="konnekt-btn konnekt-btn--success"
                                            onclick={on_start_activity}
                                        >
                                            {"Start First Activity"}
                                        </button>
                                    }
                                } else {
                                    html! {}
                                }}
                            </div>
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

fn render_activity_view(
    lobby: &Option<konnekt_session_core::Lobby>,
    is_host: bool,
    activity_response: &str,
    on_response_input: Callback<InputEvent>,
    on_submit_response: Callback<SubmitEvent>,
    on_cancel_activity: Callback<MouseEvent>,
) -> Html {
    if let Some(lobby) = lobby {
        if let Some(current_activity) = lobby.current_activity() {
            // Parse Echo challenge
            let (prompt, error) = match EchoChallenge::from_config(current_activity.config.clone())
            {
                Ok(challenge) => (Some(challenge.prompt.clone()), None),
                Err(e) => (None, Some(format!("Failed to load activity: {}", e))),
            };

            return html! {
                <div class="konnekt-activity-screen">
                    <div class="konnekt-activity-screen__header">
                        <h2 class="konnekt-activity-screen__title">
                            {"🎮 "}{&current_activity.name}
                        </h2>
                        {if is_host {
                            html! {
                                <button
                                    class="konnekt-btn konnekt-btn--danger"
                                    onclick={on_cancel_activity}
                                >
                                    {"Cancel Activity"}
                                </button>
                            }
                        } else {
                            html! {}
                        }}
                    </div>

                    {if let Some(err) = error {
                        html! {
                            <div class="konnekt-activity-screen__error">
                                {err}
                            </div>
                        }
                    } else if let Some(prompt_text) = prompt {
                        html! {
                            <div class="konnekt-activity-screen__content">
                                <div class="konnekt-activity-screen__prompt">
                                    <h3>{"Echo this:"}</h3>
                                    <div class="konnekt-activity-screen__prompt-text">
                                        {prompt_text}
                                    </div>
                                </div>

                                <form
                                    class="konnekt-activity-screen__form"
                                    onsubmit={on_submit_response}
                                >
                                    <label class="konnekt-activity-screen__label">
                                        {"Your Response:"}
                                        <input
                                            class="konnekt-activity-screen__input"
                                            type="text"
                                            value={activity_response.to_string()}
                                            oninput={on_response_input}
                                            placeholder="Type here..."
                                            autofocus={true}
                                        />
                                    </label>
                                    <button
                                        class="konnekt-btn konnekt-btn--primary konnekt-btn--large"
                                        type="submit"
                                        disabled={activity_response.is_empty()}
                                    >
                                        {"Submit Response"}
                                    </button>
                                </form>

                                <div class="konnekt-activity-screen__participants">
                                    <h4>{"Active Participants:"}</h4>
                                    <ul>
                                        {for lobby.active_participants().iter().map(|p| {
                                            html! {
                                                <li>{p.name()}</li>
                                            }
                                        })}
                                    </ul>
                                </div>
                            </div>
                        }
                    } else {
                        html! {}
                    }}
                </div>
            };
        }
    }

    html! {
        <div class="konnekt-session-screen__error">
            {"No activity in progress"}
        </div>
    }
}

fn render_results_view(lobby: &Option<konnekt_session_core::Lobby>) -> Html {
    if let Some(lobby) = lobby {
        let completed_activities: Vec<_> = lobby
            .activities()
            .iter()
            .filter(|a| a.status == konnekt_session_core::domain::ActivityStatus::Completed)
            .collect();

        if completed_activities.is_empty() {
            return html! {
                <div class="konnekt-results-screen">
                    <p>{"No completed activities yet"}</p>
                </div>
            };
        }

        html! {
            <div class="konnekt-results-screen">
                <div class="konnekt-results-screen__header">
                    <h2>{"🏆 Results"}</h2>
                </div>

                {for completed_activities.iter().map(|activity| {
                    let results = lobby.get_results(activity.id);

                    html! {
                        <div class="konnekt-results-screen__activity">
                            <h3>{&activity.name}</h3>
                            <ul class="konnekt-results-screen__list">
                                {for results.iter().map(|result| {
                                    let participant_name = lobby
                                        .participants()
                                        .get(&result.participant_id)
                                        .map(|p| p.name())
                                        .unwrap_or("Unknown");

                                    html! {
                                        <li class="konnekt-results-screen__item">
                                            <span class="konnekt-results-screen__name">
                                                {participant_name}
                                            </span>
                                            <span class="konnekt-results-screen__score">
                                                {format!("Score: {}", result.score.unwrap_or(0))}
                                            </span>
                                        </li>
                                    }
                                })}
                            </ul>
                        </div>
                    }
                })}

                <div class="konnekt-results-screen__footer">
                    <p class="konnekt-results-screen__note">
                        {"Activity completed! You can plan a new activity from the lobby."}
                    </p>
                </div>
            </div>
        }
    } else {
        html! {
            <div class="konnekt-results-screen">
                <p>{"Loading..."}</p>
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
    fn test_activity_templates_count() {
        assert_eq!(ACTIVITY_TEMPLATES.len(), 5);
        assert_eq!(ACTIVITY_TEMPLATES[0].0, "Echo: Hello Rust");
        assert_eq!(ACTIVITY_TEMPLATES[0].1, "Hello Rust");
    }

    #[test]
    fn test_view_mode_enum() {
        let mode = ViewMode::Lobby;
        assert_eq!(mode, ViewMode::Lobby);
        assert_ne!(mode, ViewMode::ActivityInProgress);
    }
}
