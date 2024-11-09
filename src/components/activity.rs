use crate::model::{Activity, ActivityData, ActivityStatus, LobbyCommand, Named, Role};
use yew::prelude::*;

#[derive(Properties, PartialEq, Clone)]
pub struct ActivityProps<T>
where
    T: ActivityData + 'static,
{
    pub activity: Activity<T>,
    pub role: Role,
    pub on_command: Callback<LobbyCommand>,
}

#[function_component(ActivityComp)]
pub fn activity_comp<T>(props: &ActivityProps<T>) -> Html
where
    T: ActivityData + 'static,
{
    let is_admin = props.role == Role::Admin;
    let activity = &props.activity;

    let on_start = {
        let on_command = props.on_command.clone();
        let activity_id = activity.id.clone();
        Callback::from(move |_| {
            on_command.emit(LobbyCommand::StartActivity {
                activity_id: activity_id.clone(),
            });
        })
    };

    let on_complete = {
        let on_command = props.on_command.clone();
        let activity_id = activity.id.clone();
        Callback::from(move |_| {
            on_command.emit(LobbyCommand::CompleteActivity {
                activity_id: activity_id.clone(),
            });
        })
    };

    let on_restart = {
        let on_command = props.on_command.clone();
        let activity_id = activity.id.clone();
        Callback::from(move |_| {
            on_command.emit(LobbyCommand::UpdateActivityStatus {
                activity_id: activity_id.clone(),
                status: ActivityStatus::NotStarted,
            });
        })
    };

    let status_class = match activity.status {
        ActivityStatus::NotStarted => "not-started",
        ActivityStatus::InProgress => "in-progress",
        ActivityStatus::Done => "done",
    };

    html! {
        <div class={classes!("konnekt-session-activity", status_class)}>
            <div class="konnekt-session-activity__content">
                <span class="konnekt-session-activity__name">{activity.name()}</span>
                <span class="konnekt-session-activity__status">{format!("Status: {:?}", activity.status)}</span>
            </div>
            if is_admin {
                <div class="konnekt-session-activity__controls">
                    if activity.status == ActivityStatus::NotStarted {
                        <button
                            class="konnekt-session-activity__start-button"
                            onclick={on_start}
                        >
                            {"Start Activity"}
                        </button>
                    }
                    if activity.status == ActivityStatus::InProgress {
                        <button
                            class="konnekt-session-activity__complete-button"
                            onclick={on_complete}
                        >
                            {"Complete Activity"}
                        </button>
                    }
                    if activity.status == ActivityStatus::Done {
                        <button
                            class="konnekt-session-activity__restart-button"
                            onclick={on_restart}
                        >
                            {"Restart Activity"}
                        </button>
                    }
                </div>
            }
        </div>
    }
}
