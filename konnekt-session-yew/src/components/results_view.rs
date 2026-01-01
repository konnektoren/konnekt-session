use konnekt_session_core::Lobby;
use yew::prelude::*;

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
