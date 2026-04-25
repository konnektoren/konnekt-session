use crate::hooks::ActiveRunSnapshot;
use konnekt_session_core::Lobby;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivityListProps {
    pub lobby: Lobby,
    pub active_run: Option<ActiveRunSnapshot>,
}

/// Displays queued activities and the currently running activity (if any).
#[function_component(ActivityList)]
pub fn activity_list(props: &ActivityListProps) -> Html {
    let queue = props.lobby.activity_queue();

    html! {
        <div class="konnekt-activity-list">
            <h3 class="konnekt-activity-list__title">{"Activities"}</h3>

            {if let Some(run) = &props.active_run {
                html! {
                    <div class="konnekt-activity-list__item in-progress">
                        <span class="konnekt-activity-list__icon">{"▶️"}</span>
                        <span class="konnekt-activity-list__name">{run.name.clone()}</span>
                        <span class="konnekt-activity-list__status">{"InProgress"}</span>
                    </div>
                }
            } else {
                html! {}
            }}

            {if queue.is_empty() {
                html! {
                    <p class="konnekt-activity-list__empty">{"No queued activities"}</p>
                }
            } else {
                html! {
                    <ul class="konnekt-activity-list__items">
                        {for queue.iter().map(|activity| {
                            html! {
                                <li class="konnekt-activity-list__item planned">
                                    <span class="konnekt-activity-list__icon">{"📋"}</span>
                                    <span class="konnekt-activity-list__name">{activity.name.clone()}</span>
                                    <span class="konnekt-activity-list__status">{"Queued"}</span>
                                </li>
                            }
                        })}
                    </ul>
                }
            }}
        </div>
    }
}
