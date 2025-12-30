use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct SessionInfoProps {
    pub session_id: String,
    #[prop_or_default]
    pub peer_count: usize,
    #[prop_or_default]
    pub is_host: bool,
}

/// Displays session metadata
#[function_component(SessionInfo)]
pub fn session_info(props: &SessionInfoProps) -> Html {
    let copy_session_id = {
        let session_id = props.session_id.clone();
        Callback::from(move |_: MouseEvent| {
            // Copy to clipboard (web-sys)
            if let Some(window) = web_sys::window() {
                let clipboard = window.navigator().clipboard();
                let session_id = session_id.clone();
                wasm_bindgen_futures::spawn_local(async move {
                    let _ = wasm_bindgen_futures::JsFuture::from(clipboard.write_text(&session_id))
                        .await;
                });
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
                    title="Copy to clipboard"
                >
                    {"📋"}
                </button>
            </div>
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
        </div>
    }
}
