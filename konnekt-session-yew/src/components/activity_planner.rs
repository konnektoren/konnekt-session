use crate::hooks::use_session;
use konnekt_session_core::{ActivityConfig, DomainCommand, EchoChallenge};
use uuid::Uuid;
use yew::prelude::*;

const ACTIVITY_TEMPLATES: &[(&str, &str)] = &[
    ("Echo: Hello Rust", "Hello Rust"),
    ("Echo: WebAssembly", "WebAssembly"),
    ("Echo: Konnekt", "Konnekt"),
    ("Echo: P2P Session", "P2P Session"),
    ("Echo: DDD + Hexagonal", "DDD + Hexagonal"),
];

#[derive(Properties, PartialEq)]
pub struct ActivityPlannerProps {
    pub lobby_id: Uuid,
}

#[function_component(ActivityPlanner)]
pub fn activity_planner(props: &ActivityPlannerProps) -> Html {
    let session = use_session();
    let selected = use_state(|| 0usize);

    let on_select = {
        let selected = selected.clone();
        Callback::from(move |idx: usize| {
            selected.set(idx);
        })
    };

    let on_plan = {
        let selected = *selected;
        let send_command = session.send_command.clone();
        let lobby_id = props.lobby_id;

        Callback::from(move |_: MouseEvent| {
            if let Some((name, prompt)) = ACTIVITY_TEMPLATES.get(selected) {
                let challenge = EchoChallenge::new((*prompt).to_string());
                let config = ActivityConfig::new(
                    "echo-challenge-v1".to_string(),
                    (*name).to_string(),
                    challenge.to_config(),
                );

                send_command(DomainCommand::QueueActivity { lobby_id, config });
            }
        })
    };

    let on_start = {
        let send_command = session.send_command.clone();
        let lobby = session.lobby.clone();

        Callback::from(move |_: MouseEvent| {
            if let Some(lobby) = &lobby {
                if !lobby.activity_queue().is_empty() && !lobby.has_active_run() {
                    send_command(DomainCommand::StartNextRun { lobby_id: lobby.id() });
                }
            }
        })
    };

    let has_planned = session
        .lobby
        .as_ref()
        .map(|l| !l.activity_queue().is_empty())
        .unwrap_or(false);

    html! {
        <div class="konnekt-activity-planner">
            <h3>{"Plan Activity"}</h3>
            <ul class="konnekt-activity-templates">
                {for ACTIVITY_TEMPLATES.iter().enumerate().map(|(idx, (name, _))| {
                    let is_selected = idx == *selected;
                    html! {
                        <li
                            class={classes!(
                                "konnekt-activity-template",
                                is_selected.then(|| "selected")
                            )}
                            onclick={let on_select = on_select.clone(); move |_| on_select.emit(idx)}
                        >
                            {*name}
                        </li>
                    }
                })}
            </ul>
            <button
                class="konnekt-btn konnekt-btn--primary"
                onclick={on_plan}
            >
                {"Plan Selected Activity"}
            </button>

            {if has_planned {
                html! {
                    <button
                        class="konnekt-btn konnekt-btn--success"
                        onclick={on_start}
                    >
                        {"Start First Activity"}
                    </button>
                }
            } else {
                html! {}
            }}
        </div>
    }
}
