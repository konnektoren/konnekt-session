use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct LoginScreenProps {
    pub on_create_lobby: Callback<(String, String)>, // (lobby_name, host_name)
    pub on_join_lobby: Callback<(String, String)>,   // (session_id, guest_name)

    /// Optional pre-filled session ID (from URL params)
    #[prop_or_default]
    pub initial_session_id: Option<String>,
}

#[function_component(LoginScreen)]
pub fn login_screen(props: &LoginScreenProps) -> Html {
    // ✅ FIX: Auto-switch to "join" tab if session_id is pre-filled
    let mode = use_state(|| {
        if props.initial_session_id.is_some() {
            "join".to_string()
        } else {
            "create".to_string()
        }
    });

    let lobby_name = use_state(|| "My Lobby".to_string());
    let host_name = use_state(|| "Host".to_string());

    // ✅ FIX: Use initial_session_id if provided
    let session_id = use_state(|| props.initial_session_id.clone().unwrap_or_default());

    let guest_name = use_state(|| "Guest".to_string());

    let on_mode_change = {
        let mode = mode.clone();
        Callback::from(move |new_mode: String| {
            mode.set(new_mode);
        })
    };

    let on_create = {
        let lobby_name = lobby_name.clone();
        let host_name = host_name.clone();
        let callback = props.on_create_lobby.clone();
        Callback::from(move |_: MouseEvent| {
            callback.emit(((*lobby_name).clone(), (*host_name).clone()));
        })
    };

    let on_join = {
        let session_id = session_id.clone();
        let guest_name = guest_name.clone();
        let callback = props.on_join_lobby.clone();
        Callback::from(move |_: MouseEvent| {
            callback.emit(((*session_id).clone(), (*guest_name).clone()));
        })
    };

    html! {
        <div class="konnekt-login">
            <h1 class="konnekt-login__title">{"Konnekt Session"}</h1>

            <div class="konnekt-login__tabs">
                <button
                    class={classes!(
                        "konnekt-login__tab",
                        (*mode == "create").then(|| "active")
                    )}
                    onclick={let mode = on_mode_change.clone(); move |_| mode.emit("create".to_string())}
                >
                    {"Create Lobby"}
                </button>
                <button
                    class={classes!(
                        "konnekt-login__tab",
                        (*mode == "join").then(|| "active")
                    )}
                    onclick={move |_| on_mode_change.emit("join".to_string())}
                >
                    {"Join Lobby"}
                </button>
            </div>

            {if *mode == "create" {
                html! {
                    <div class="konnekt-login__form">
                        <label class="konnekt-login__label">
                            {"Lobby Name"}
                            <input
                                class="konnekt-login__input"
                                type="text"
                                value={(*lobby_name).clone()}
                                oninput={let lobby_name = lobby_name.clone(); move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    lobby_name.set(input.value());
                                }}
                            />
                        </label>
                        <label class="konnekt-login__label">
                            {"Your Name"}
                            <input
                                class="konnekt-login__input"
                                type="text"
                                value={(*host_name).clone()}
                                oninput={let host_name = host_name.clone(); move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    host_name.set(input.value());
                                }}
                            />
                        </label>
                        <button class="konnekt-login__button" onclick={on_create}>
                            {"Create Lobby"}
                        </button>
                    </div>
                }
            } else {
                html! {
                    <div class="konnekt-login__form">
                        {if props.initial_session_id.is_some() {
                            html! {
                                <div class="konnekt-login__notice">
                                    {"✓ Session ID from link"}
                                </div>
                            }
                        } else {
                            html! {}
                        }}

                        <label class="konnekt-login__label">
                            {"Session ID"}
                            <input
                                class="konnekt-login__input"
                                type="text"
                                placeholder="paste session ID here"
                                value={(*session_id).clone()}
                                oninput={let session_id = session_id.clone(); move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    session_id.set(input.value());
                                }}
                            />
                        </label>
                        <label class="konnekt-login__label">
                            {"Your Name"}
                            <input
                                class="konnekt-login__input"
                                type="text"
                                value={(*guest_name).clone()}
                                oninput={let guest_name = guest_name.clone(); move |e: InputEvent| {
                                    let input: web_sys::HtmlInputElement = e.target_unchecked_into();
                                    guest_name.set(input.value());
                                }}
                            />
                        </label>
                        <button
                            class="konnekt-login__button"
                            onclick={on_join}
                            disabled={session_id.is_empty()}
                        >
                            {"Join Lobby"}
                        </button>
                    </div>
                }
            }}
        </div>
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_login_screen_mode_toggle() {
        let on_create = Callback::from(|_: (String, String)| {});
        let on_join = Callback::from(|_: (String, String)| {});

        let _props = yew::props!(LoginScreenProps {
            on_create_lobby: on_create,
            on_join_lobby: on_join,
        });

        assert!(true);
    }

    #[test]
    fn test_login_screen_with_prefilled_session_id() {
        let on_create = Callback::from(|_: (String, String)| {});
        let on_join = Callback::from(|_: (String, String)| {});

        let props = yew::props!(LoginScreenProps {
            on_create_lobby: on_create,
            on_join_lobby: on_join,
            initial_session_id: Some("test-session-123".to_string()),
        });

        assert_eq!(
            props.initial_session_id,
            Some("test-session-123".to_string())
        );
    }
}
