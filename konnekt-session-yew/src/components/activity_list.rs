use konnekt_session_core::{Lobby, domain::ActivityStatus};
use yew::prelude::*;

#[cfg(feature = "preview")]
use yew_preview::prelude::*;

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

#[cfg(feature = "preview")]
mod preview_fixtures {
    use super::*;
    use konnekt_session_core::{Lobby, Participant, domain::ActivityMetadata};

    pub fn make_sample_lobby() -> Lobby {
        let host = Participant::new_host("Alice".to_string()).unwrap();
        let mut lobby = Lobby::new("Preview Lobby".to_string(), host).unwrap();

        let planned = ActivityMetadata::new(
            "echo".to_string(),
            "Planned Activity".to_string(),
            serde_json::json!({}),
        );
        lobby.plan_activity(planned).unwrap();

        let in_progress = ActivityMetadata::new(
            "echo".to_string(),
            "In Progress Activity".to_string(),
            serde_json::json!({}),
        );
        lobby.plan_activity(in_progress).unwrap();

        // Start the second activity to show it in-progress
        let activity_id = lobby.activities().get(1).map(|a| a.id);
        if let Some(id) = activity_id {
            lobby.start_activity(id).unwrap();
        }

        let completed = ActivityMetadata::new(
            "echo".to_string(),
            "Completed Activity".to_string(),
            serde_json::json!({}),
        );
        lobby.plan_activity(completed).unwrap();

        lobby
    }
}

#[cfg(feature = "preview")]
yew_preview::create_preview!(
    ActivityList,
    ActivityListProps {
        lobby: preview_fixtures::make_sample_lobby(),
    },
);
