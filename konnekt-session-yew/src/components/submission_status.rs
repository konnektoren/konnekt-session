use crate::hooks::ActiveRunSnapshot;
use konnekt_session_core::Lobby;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct SubmissionStatusProps {
    pub lobby: Lobby,
    pub active_run: ActiveRunSnapshot,
}

#[function_component(SubmissionStatus)]
pub fn submission_status(props: &SubmissionStatusProps) -> Html {
    let results = &props.active_run.results;
    let required_submitters = &props.active_run.required_submitters;

    let outstanding: Vec<_> = required_submitters
        .iter()
        .filter(|participant_id| !results.iter().any(|r| r.participant_id == **participant_id))
        .collect();

    html! {
        <div class="konnekt-submission-status">
            <h3>{"Submission Status"}</h3>
            <div class="konnekt-submission-status__stats">
                <span class="konnekt-submission-status__count">
                    {format!("{} / {} submitted", results.len(), required_submitters.len())}
                </span>
            </div>

            {if !results.is_empty() {
                html! {
                    <div class="konnekt-submission-status__list">
                        <h4>{"✓ Submitted:"}</h4>
                        <ul>
                            {for results.iter().map(|result| {
                                let participant_name = props.lobby
                                    .participants()
                                    .get(&result.participant_id)
                                    .map(|p| p.name())
                                    .unwrap_or("Unknown");
                                html! {
                                    <li class="konnekt-submission-status__submitted">
                                        {"✓ "}{participant_name}
                                    </li>
                                }
                            })}
                        </ul>
                    </div>
                }
            } else {
                html! {}
            }}

            {if !outstanding.is_empty() {
                html! {
                    <div class="konnekt-submission-status__list">
                        <h4>{"⏳ Waiting for:"}</h4>
                        <ul>
                            {for outstanding.iter().map(|participant_id| {
                                let participant_name = props.lobby
                                    .participants()
                                    .get(participant_id)
                                    .map(|p| p.name())
                                    .unwrap_or("Unknown");
                                html! {
                                    <li class="konnekt-submission-status__pending">
                                        {"⏳ "}{participant_name}
                                    </li>
                                }
                            })}
                        </ul>
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
