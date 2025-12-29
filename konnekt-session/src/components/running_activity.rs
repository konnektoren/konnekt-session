use super::ActivityProps;
use crate::model::{Activity, ActivityStatus, ActivityTrait, LobbyCommand, PlayerId, Role};
use std::marker::PhantomData;
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct RunningActivityProps<T, C>
where
    T: ActivityTrait + 'static,
    C: Component<Properties = ActivityProps<T>> + PartialEq + 'static,
{
    pub player_id: PlayerId,
    pub activities: Vec<Activity<T>>,
    pub role: Role,
    pub on_command: Callback<LobbyCommand>,
    #[prop_or_default]
    pub _phantom: PhantomData<C>,
}

#[function_component(RunningActivityComp)]
pub fn running_activity<T, C>(props: &RunningActivityProps<T, C>) -> Html
where
    T: ActivityTrait + 'static,
    C: Component<Properties = ActivityProps<T>> + PartialEq + 'static,
{
    let current_activity = props
        .activities
        .iter()
        .find(|activity| activity.status == ActivityStatus::InProgress);

    html! {
        <div class="konnekt-session-running-activity">
            if let Some(activity) = current_activity {
                <div class="konnekt-session-running-activity__content">
                    <C
                        player_id={props.player_id}
                        activity={activity.clone()}
                        role={props.role}
                        on_command={props.on_command.clone()}
                    />
                </div>
            } else {
                <div class="konnekt-session-running-activity__empty">
                    <p>{"No activity currently running"}</p>
                </div>
            }
        </div>
    }
}
