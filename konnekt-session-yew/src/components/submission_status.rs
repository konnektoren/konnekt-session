use konnekt_session_core::{Lobby, domain::ActivityId};
use std::collections::HashSet;
use yew::prelude::*;

#[cfg(feature = "preview")]
use yew_preview::prelude::*;

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

#[cfg(feature = "preview")]
mod preview_fixtures {
    use super::*;
    use konnekt_session_core::{
        Lobby, Participant, domain::ActivityMetadata, domain::ActivityResult,
    };

    pub fn make_sample_lobby() -> Lobby {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let host_id = host.id();
        let mut lobby = Lobby::new("Preview Lobby".to_string(), host).unwrap();

        let guest1 = Participant::new_guest("Bob".to_string()).unwrap();
        let guest1_id = guest1.id();
        let guest2 = Participant::new_guest("Charlie".to_string()).unwrap();
        lobby.add_guest(guest1).unwrap();
        lobby.add_guest(guest2).unwrap();

        let activity = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Echo Challenge".to_string(),
            serde_json::json!({}),
        );
        lobby.plan_activity(activity).unwrap();

        let activity_id = lobby.activities().first().map(|a| a.id).unwrap();
        lobby.start_activity(activity_id).unwrap();

        // Add some results
        let result1 = ActivityResult::new(activity_id, host_id).with_score(95);
        lobby.submit_result(result1).unwrap();

        let result2 = ActivityResult::new(activity_id, guest1_id).with_score(85);
        lobby.submit_result(result2).unwrap();

        lobby
    }
}

#[cfg(feature = "preview")]
yew_preview::create_preview!(SubmissionStatus, {
    let lobby = preview_fixtures::make_sample_lobby();
    let activity_id = lobby.current_activity().map(|a| a.id).unwrap_or_default();
    SubmissionStatusProps { lobby, activity_id }
},);
