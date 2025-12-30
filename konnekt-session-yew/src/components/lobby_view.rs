use crate::components::{ActivityList, ParticipantList, SessionInfo};
use crate::hooks::use_session;
use yew::prelude::*;

/// Main lobby view component
///
/// Combines session info, participants, and activities into a complete UI.
#[function_component(LobbyView)]
pub fn lobby_view() -> Html {
    let session = use_session();

    html! {
        <div class="konnekt-lobby-view">
            <h1 class="konnekt-lobby-view__title">{"Lobby"}</h1>

            <SessionInfo
                session_id={session.session_id.to_string()}
                peer_count={session.peer_count}
                is_host={session.is_host}
            />

            {if let Some(lobby) = session.lobby.as_ref() {
                html! {
                    <div class="konnekt-lobby-view__content">
                        <div class="konnekt-lobby-view__section">
                            <ParticipantList lobby={lobby.clone()} />
                        </div>
                        <div class="konnekt-lobby-view__section">
                            <ActivityList lobby={lobby.clone()} />
                        </div>
                    </div>
                }
            } else {
                html! {
                    <p class="konnekt-lobby-view__loading">{"Syncing lobby..."}</p>
                }
            }}
        </div>
    }
}
