use konnekt_session_core::Lobby;
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ResultsViewProps {
    pub lobby: Option<Lobby>,
    pub is_host: bool,
}

#[function_component(ResultsView)]
pub fn results_view(props: &ResultsViewProps) -> Html {
    if props.lobby.is_none() {
        return html! {
            <div class="konnekt-results-screen">
                <p>{"Loading..."}</p>
            </div>
        };
    }

    html! {
        <div class="konnekt-results-screen">
            <div class="konnekt-results-screen__header">
                <h2>{"🏆 Results"}</h2>
            </div>
            <p>{"Run history is not yet exposed in the current snapshot model."}</p>
            <div class="konnekt-results-screen__footer">
                <p class="konnekt-results-screen__note">
                    {if props.is_host {
                        "You can plan a new activity from the lobby."
                    } else {
                        "Waiting for host to plan next activity."
                    }}
                </p>
            </div>
        </div>
    }
}
