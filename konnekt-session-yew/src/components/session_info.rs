use yew::prelude::*;

#[cfg(feature = "preview")]
use yew_preview::prelude::*;
#[cfg(feature = "preview")]
use yew_preview::test_utils::{exists, has_text};

#[derive(Properties, PartialEq, Clone)]
pub struct SessionInfoProps {
    pub session_id: String,
    #[prop_or_default]
    pub peer_count: usize,
    #[prop_or_default]
    pub is_host: bool,
    #[prop_or(true)]
    pub show_connectivity_warning: bool,
    #[prop_or_default]
    pub host_unreachable: bool,
    #[prop_or_default]
    pub last_host_connection: Option<String>,
}

/// Displays session metadata with shareable URL
#[function_component(SessionInfo)]
pub fn session_info(props: &SessionInfoProps) -> Html {
    let copy_message = use_state(|| None::<String>);

    let copy_session_id = {
        let session_id = props.session_id.clone();
        let copy_message = copy_message.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let session_id = session_id.clone();
                let copy_message = copy_message.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    match wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&session_id))
                        .await
                    {
                        Ok(_) => copy_message.set(Some("✓ Copied!".to_string())),
                        Err(_) => copy_message.set(Some("✗ Failed".to_string())),
                    }
                });
            }
        })
    };

    let copy_shareable_url = {
        let session_id = props.session_id.clone();
        let copy_message = copy_message.clone();
        Callback::from(move |_: MouseEvent| {
            if let Some(window) = web_sys::window() {
                if let Ok(location) = window.location().href() {
                    // Generate shareable URL with session_id parameter
                    let base_url = if let Ok(url) = web_sys::Url::new(&location) {
                        format!("{}://{}{}", url.protocol(), url.host(), url.pathname())
                    } else {
                        location
                    };

                    let shareable_url = format!("{}?session_id={}", base_url, session_id);

                    let clipboard = window.navigator().clipboard();
                    let copy_message = copy_message.clone();
                    wasm_bindgen_futures::spawn_local(async move {
                        match wasm_bindgen_futures::JsFuture::from(
                            clipboard.write_text(&shareable_url),
                        )
                        .await
                        {
                            Ok(_) => copy_message.set(Some("✓ Share link copied!".to_string())),
                            Err(_) => copy_message.set(Some("✗ Failed".to_string())),
                        }
                    });
                }
            }
        })
    };

    html! {
        <div class="konnekt-session-info">
            <div class="konnekt-session-info__row">
                <span class="konnekt-session-info__label">{"Session ID:"}</span>
                <code class="konnekt-session-info__value">{&props.session_id}</code>
                <button
                    class="konnekt-session-info__copy"
                    onclick={copy_session_id}
                    title="Copy Session ID"
                >
                    {"📋"}
                </button>
                <button
                    class="konnekt-session-info__copy"
                    onclick={copy_shareable_url}
                    title="Copy Shareable Link"
                >
                    {"🔗"}
                </button>
            </div>

            {if let Some(msg) = &*copy_message {
                html! {
                    <div class="konnekt-session-info__message">
                        {msg}
                    </div>
                }
            } else {
                html! {}
            }}

            <div class="konnekt-session-info__row">
                <span class="konnekt-session-info__label">{"Connected Peers:"}</span>
                <span class="konnekt-session-info__value">{props.peer_count}</span>
            </div>
            <div class="konnekt-session-info__row">
                <span class="konnekt-session-info__label">{"Role:"}</span>
                <span class="konnekt-session-info__value">
                    {if props.is_host { "👑 Host" } else { "👤 Guest" }}
                </span>
            </div>

            {if props.show_connectivity_warning && props.host_unreachable && !props.is_host {
                html! {
                    <div class="konnekt-session-info__warning">
                        <strong>{"Host unreachable."}</strong>
                        {" "}
                        {if let Some(last) = &props.last_host_connection {
                            format!("Last connection: {}", last)
                        } else {
                            "No successful host connection yet.".to_string()
                        }}
                    </div>
                }
            } else {
                html! {}
            }}
        </div>
    }
}

#[cfg(feature = "preview")]
yew_preview::create_preview_with_tests!(
    component: SessionInfo,
    default_props: SessionInfoProps {
        session_id: "a1b2-c3d4-e5f6".to_string(),
        peer_count: 3,
        is_host: true,
    },
    variants: [
        (
            "Guest View",
            SessionInfoProps {
                session_id: "a1b2-c3d4-e5f6".to_string(),
                peer_count: 3,
                is_host: false,
            }
        ),
        (
            "Solo",
            SessionInfoProps {
                session_id: "a1b2-c3d4-e5f6".to_string(),
                peer_count: 1,
                is_host: true,
            }
        )
    ],
    tests: [
        ("Has main container class", exists("konnekt-session-info")),
        ("Has label class", exists("konnekt-session-info__label")),
        ("Has code tag", exists("code")),
        ("Has copy button class", exists("konnekt-session-info__copy")),
        ("Contains Session ID label", has_text("Session ID:")),
        ("Contains Connected Peers label", has_text("Connected Peers:")),
        ("Contains Role label", has_text("Role:")),
        ("Shows peer count", has_text("3")),
    ]
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_info_renders() {
        let props = yew::props!(SessionInfoProps {
            session_id: "test-session-123".to_string(),
            peer_count: 2,
            is_host: true,
        });

        // Just verify we can create the component
        assert_eq!(props.session_id, "test-session-123");
        assert_eq!(props.peer_count, 2);
        assert!(props.is_host);
    }
}
