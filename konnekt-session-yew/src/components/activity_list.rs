use konnekt_session_core::{Lobby, domain::ActivityStatus};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct ActivityListProps {
    pub lobby: Lobby,
}

/// Displays planned and in-progress activities
#[function_component(ActivityList)]
pub fn activity_list(props: &ActivityListProps) -> Html {
    let activities = props.lobby.activities();

    html! {
        <div class="konnekt-activity-list">
            <h3 class="konnekt-activity-list__title">{"Activities"}</h3>
            {if activities.is_empty() {
                html! {
                    <p class="konnekt-activity-list__empty">{"No activities yet"}</p>
                }
            } else {
                html! {
                    <ul class="konnekt-activity-list__items">
                        {for activities.iter().map(|activity| {
                            let status_class = match activity.status {
                                ActivityStatus::Planned => "planned",
                                ActivityStatus::InProgress => "in-progress",
                                ActivityStatus::Completed => "completed",
                                ActivityStatus::Cancelled => "cancelled",
                            };

                            let status_icon = match activity.status {
                                ActivityStatus::Planned => "📋",
                                ActivityStatus::InProgress => "▶️",
                                ActivityStatus::Completed => "✅",
                                ActivityStatus::Cancelled => "❌",
                            };

                            html! {
                                <li class={classes!("konnekt-activity-list__item", status_class)}>
                                    <span class="konnekt-activity-list__icon">{status_icon}</span>
                                    <span class="konnekt-activity-list__name">{&activity.name}</span>
                                    <span class="konnekt-activity-list__status">
                                        {format!("{:?}", activity.status)}
                                    </span>
                                </li>
                            }
                        })}
                    </ul>
                }
            }}
        </div>
    }
}
