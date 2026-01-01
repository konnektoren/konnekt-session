use crate::hooks::use_session;
use konnekt_session_core::{DomainCommand, EchoChallenge, EchoResult, Lobby};
use uuid::Uuid;
use yew::prelude::*;

use super::submission_status::SubmissionStatus;

#[derive(Properties, PartialEq)]
pub struct ActivitySubmissionProps {
    pub lobby: Option<Lobby>,
    pub is_host: bool,
    pub participant_id: Option<Uuid>,
    // ✅ REMOVED: has_submitted (was local state)
    // ✅ REMOVED: on_submit callback (no longer needed)
}

#[function_component(ActivitySubmission)]
pub fn activity_submission(props: &ActivitySubmissionProps) -> Html {
    let session = use_session();
    let response = use_state(String::new);

    let on_input = {
        let response = response.clone();
        Callback::from(move |e: InputEvent| {
            let input: web_sys::HtmlInputElement = e.target_unchecked_into();
            response.set(input.value());
        })
    };

    let on_submit_form = {
        let response = response.clone();
        let lobby = props.lobby.clone();
        let send_command = session.send_command.clone();
        let participant_id = props.participant_id;

        Callback::from(move |e: SubmitEvent| {
            e.prevent_default();

            if let (Some(lobby), Some(pid)) = (&lobby, participant_id) {
                if let Some(current) = lobby.current_activity() {
                    let response_text = (*response).clone();

                    if let Ok(challenge) = EchoChallenge::from_config(current.config.clone()) {
                        let score = challenge.calculate_score(&response_text);
                        let echo_result = EchoResult::new(response_text, 1000);

                        let result =
                            konnekt_session_core::domain::ActivityResult::new(current.id, pid)
                                .with_data(echo_result.to_json())
                                .with_score(score)
                                .with_time(1000);

                        tracing::info!(
                            "📤 Submitting result for activity {} (participant {})",
                            current.id,
                            pid
                        );

                        // ✅ Just send the command - let P2P + Core handle sync
                        send_command(DomainCommand::SubmitResult {
                            lobby_id: lobby.id(),
                            result,
                        });

                        response.set(String::new());
                        // ✅ NO LOCAL STATE UPDATE - wait for Core to update
                    }
                }
            }
        })
    };

    let on_cancel = {
        let send_command = session.send_command.clone();
        let lobby = props.lobby.clone();

        Callback::from(move |_: MouseEvent| {
            if let Some(lobby) = &lobby {
                if let Some(current) = lobby.current_activity() {
                    send_command(DomainCommand::CancelActivity {
                        lobby_id: lobby.id(),
                        activity_id: current.id,
                    });
                }
            }
        })
    };

    if let Some(lobby) = &props.lobby {
        if let Some(current) = lobby.current_activity() {
            // Parse challenge
            let (prompt, error) = match EchoChallenge::from_config(current.config.clone()) {
                Ok(challenge) => (Some(challenge.prompt.clone()), None),
                Err(e) => (None, Some(format!("Failed to load: {}", e))),
            };

            // ✅ CHECK CORE: Has this participant submitted?
            let results = lobby.get_results(current.id);
            let has_user_submitted = props
                .participant_id
                .map(|id| results.iter().any(|r| r.participant_id == id))
                .unwrap_or(false);

            tracing::debug!(
                "Participant {:?} submitted: {} (results in core: {})",
                props.participant_id,
                has_user_submitted,
                results.len()
            );

            return html! {
                <div class="konnekt-activity-screen">
                    <div class="konnekt-activity-screen__header">
                        <h2 class="konnekt-activity-screen__title">
                            {"🎮 "}{&current.name}
                        </h2>
                        {if props.is_host {
                            html! {
                                <button
                                    class="konnekt-btn konnekt-btn--danger"
                                    onclick={on_cancel}
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
                                <SubmissionStatus
                                    lobby={lobby.clone()}
                                    activity_id={current.id}
                                />

                                // ✅ Check Core state only
                                {if has_user_submitted {
                                    html! {
                                        <div class="konnekt-activity-screen__waiting-message">
                                            <div class="konnekt-waiting-indicator">
                                                <div class="konnekt-spinner-small"></div>
                                                <h3>{"✓ Response Submitted!"}</h3>
                                                <p>{"Waiting for other participants..."}</p>
                                            </div>
                                        </div>
                                    }
                                } else {
                                    html! {
                                        <>
                                            <div class="konnekt-activity-screen__prompt">
                                                <h3>{"Echo this:"}</h3>
                                                <div class="konnekt-activity-screen__prompt-text">
                                                    {prompt_text}
                                                </div>
                                            </div>

                                            <form
                                                class="konnekt-activity-screen__form"
                                                onsubmit={on_submit_form}
                                            >
                                                <label class="konnekt-activity-screen__label">
                                                    {"Your Response:"}
                                                    <input
                                                        class="konnekt-activity-screen__input"
                                                        type="text"
                                                        value={(*response).clone()}
                                                        oninput={on_input}
                                                        placeholder="Type here..."
                                                        autofocus={true}
                                                    />
                                                </label>
                                                <button
                                                    class="konnekt-btn konnekt-btn--primary konnekt-btn--large"
                                                    type="submit"
                                                    disabled={response.is_empty()}
                                                >
                                                    {"Submit Response"}
                                                </button>
                                            </form>
                                        </>
                                    }
                                }}
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
