use crate::pages::{LoginScreen, SessionScreen};
use crate::providers::SessionProvider;
use konnekt_session_p2p::SessionId;
use yew::prelude::*;

#[derive(Debug, Clone, PartialEq)]
enum AppState {
    Login {
        initial_session_id: Option<String>,
    },
    CreatingSession {
        lobby_name: String,
        host_name: String,
    },
    JoiningSession {
        session_id: String,
        guest_name: String,
    },
    InSession {
        session_id: SessionId,
        name: String,
        is_host: bool,
    },
}

/// Extract session_id from URL query parameters
fn get_session_id_from_url() -> Option<String> {
    if let Some(window) = web_sys::window() {
        if let Ok(url) = window.location().href() {
            if let Ok(parsed) = web_sys::Url::new(&url) {
                let params = parsed.search_params();
                if let Some(session_id) = params.get("session_id") {
                    tracing::info!("Found session_id in URL: {}", session_id);
                    return Some(session_id);
                }
            }
        }
    }
    None
}

#[function_component(App)]
pub fn app() -> Html {
    let state = use_state(|| {
        // ✅ Check URL for session_id parameter
        let initial_session_id = get_session_id_from_url();

        if initial_session_id.is_some() {
            tracing::info!("Auto-switching to Join tab");
        }

        AppState::Login { initial_session_id }
    });

    let on_create_lobby = {
        let state = state.clone();
        Callback::from(move |(lobby_name, host_name): (String, String)| {
            tracing::info!("Creating lobby: {} as {}", lobby_name, host_name);
            state.set(AppState::CreatingSession {
                lobby_name,
                host_name,
            });
        })
    };

    let on_join_lobby = {
        let state = state.clone();
        Callback::from(move |(session_id, guest_name): (String, String)| {
            tracing::info!("Joining session: {} as {}", session_id, guest_name);
            state.set(AppState::JoiningSession {
                session_id,
                guest_name,
            });
        })
    };

    let on_leave = {
        let state = state.clone();
        Callback::from(move |_: ()| {
            tracing::info!("Leaving session");
            state.set(AppState::Login {
                initial_session_id: None,
            });
        })
    };

    html! {
        <div class="app">
            {match &*state {
                AppState::Login { initial_session_id } => {
                    html! {
                        <LoginScreen
                            on_create_lobby={on_create_lobby}
                            on_join_lobby={on_join_lobby}
                            initial_session_id={initial_session_id.clone()}
                        />
                    }
                }

                AppState::CreatingSession { lobby_name, host_name } => {
                    html! {
                        <SessionProvider
                            signalling_server="wss://match.konnektoren.help"
                            name={Some(AttrValue::from(host_name.clone()))}
                        >
                            <SessionScreen on_leave={on_leave.clone()} />
                        </SessionProvider>
                    }
                }

                AppState::JoiningSession { session_id, guest_name } => {
                    html! {
                        <SessionProvider
                            signalling_server="wss://match.konnektoren.help"
                            session_id={Some(AttrValue::from(session_id.clone()))}
                            name={Some(AttrValue::from(guest_name.clone()))}
                        >
                            <SessionScreen on_leave={on_leave.clone()} />
                        </SessionProvider>
                    }
                }

                AppState::InSession { .. } => {
                    html! {
                        <SessionScreen on_leave={on_leave.clone()} />
                    }
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_state_transitions() {
        let state = AppState::Login {
            initial_session_id: None,
        };
        assert!(matches!(state, AppState::Login { .. }));

        let state = AppState::Login {
            initial_session_id: Some("test-123".to_string()),
        };

        if let AppState::Login { initial_session_id } = state {
            assert_eq!(initial_session_id, Some("test-123".to_string()));
        } else {
            panic!("Expected Login state");
        }
    }

    #[test]
    fn test_creating_session_state() {
        let state = AppState::CreatingSession {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
        };
        assert!(matches!(state, AppState::CreatingSession { .. }));
    }
}
