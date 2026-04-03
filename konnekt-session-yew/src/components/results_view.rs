use konnekt_session_core::Lobby;
use yew::prelude::*;

#[cfg(feature = "preview")]
use yew_preview::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ResultsViewProps {
    pub lobby: Option<Lobby>,
    pub is_host: bool,
}

#[function_component(ResultsView)]
pub fn results_view(props: &ResultsViewProps) -> Html {
    if let Some(lobby) = &props.lobby {
        let completed: Vec<_> = lobby
            .activities()
            .iter()
            .filter(|a| a.status == konnekt_session_core::domain::ActivityStatus::Completed)
            .collect();

        if completed.is_empty() {
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

                {for completed.iter().map(|activity| {
                    let results = lobby.get_results(activity.id);

                    html! {
                        <div class="konnekt-results-screen__activity">
                            <h3>{&activity.name}</h3>
                            <ul class="konnekt-results-screen__list">
                                {for results.iter().map(|result| {
                                    let name = lobby
                                        .participants()
                                        .get(&result.participant_id)
                                        .map(|p| p.name())
                                        .unwrap_or("Unknown");

                                    html! {
                                        <li class="konnekt-results-screen__item">
                                            <span class="konnekt-results-screen__name">
                                                {name}
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
                        {"Activity completed! "}
                        {if props.is_host {
                            "You can plan a new activity from the lobby."
                        } else {
                            "Waiting for host to plan next activity."
                        }}
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
        let guest2_id = guest2.id();

        lobby.add_guest(guest1).unwrap();
        lobby.add_guest(guest2).unwrap();

        // Create and start an activity
        let activity = ActivityMetadata::new(
            "echo-challenge-v1".to_string(),
            "Echo Challenge".to_string(),
            serde_json::json!({}),
        );
        lobby.plan_activity(activity).unwrap();

        let activity_id = lobby.current_activity().map(|a| a.id);
        if let Some(id) = activity_id {
            lobby.start_activity(id).unwrap();

            // Add results
            let result1 = ActivityResult::new(id, host_id).with_score(95);
            lobby.submit_result(result1).unwrap();

            let result2 = ActivityResult::new(id, guest1_id).with_score(85);
            lobby.submit_result(result2).unwrap();

            let result3 = ActivityResult::new(id, guest2_id).with_score(92);
            lobby.submit_result(result3).unwrap();
        }

        lobby
    }
}

#[cfg(feature = "preview")]
yew_preview::create_preview!(
    ResultsView,
    ResultsViewProps {
        lobby: Some(preview_fixtures::make_sample_lobby()),
        is_host: true,
    },
    (
        "Guest View",
        ResultsViewProps {
            lobby: Some(preview_fixtures::make_sample_lobby()),
            is_host: false,
        }
    ),
    (
        "No Lobby",
        ResultsViewProps {
            lobby: None,
            is_host: false,
        }
    ),
);
