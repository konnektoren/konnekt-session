use konnekt_session_core::{Lobby, domain::ActivityId};
use std::collections::HashSet;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SubmissionStatusProps {
    pub lobby: Lobby,
    pub activity_id: ActivityId,
}

#[function_component(SubmissionStatus)]
pub fn submission_status(props: &SubmissionStatusProps) -> Html {
    let results = props.lobby.get_results(props.activity_id);
    let active_participants = props.lobby.active_participants();

    let submitted_ids: HashSet<_> = results.iter().map(|r| r.participant_id).collect();

    let outstanding: Vec<_> = active_participants
        .iter()
        .filter(|p| !submitted_ids.contains(&p.id()))
        .collect();

    html! {
        <div class="konnekt-submission-status">
            <h3>{"Submission Status"}</h3>
            <div class="konnekt-submission-status__stats">
                <span class="konnekt-submission-status__count">
                    {format!("{} / {} submitted",
                        submitted_ids.len(),
                        active_participants.len()
                    )}
                </span>
            </div>

            {if !submitted_ids.is_empty() {
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
                            {for outstanding.iter().map(|p| {
                                html! {
                                    <li class="konnekt-submission-status__pending">
                                        {"⏳ "}{p.name()}
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
