use crate::components::websocket_connection::WebSocketConnection;
use crate::handler::{LocalLobbyCommandHandler, NetworkHandler};
use crate::model::{
    ActivityResultTrait, ActivityTrait, ClientId, Lobby, Player, PlayerTrait, Role,
};
use serde::{Deserialize, Serialize};
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;

#[derive(Clone, PartialEq)]
pub struct LobbyProviderContext<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
{
    pub lobby: UseStateHandle<Lobby<P, A, AR>>,
    pub lobby_handler: UseStateHandle<NetworkHandler<P, A, AR>>,
}

#[derive(Clone, PartialEq)]
pub struct LobbyProviderConfig<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
{
    pub websocket_url: String,
    pub player: Player<P>,
    pub lobby: Lobby<P, A, AR>,
    pub role: Role,
    pub debug: bool,
}

#[derive(Properties, Clone, PartialEq)]
pub struct LobbyProviderProps<P, A, AR>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + PartialEq + 'static,
{
    pub children: Children,
    pub config: LobbyProviderConfig<P, A, AR>,
}

#[function_component(LobbyProvider)]
pub fn lobby_provider<P, A, AR>(props: &LobbyProviderProps<P, A, AR>) -> Html
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static + PartialEq,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static + PartialEq,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static + PartialEq,
{
    let lobby = use_state(|| props.config.lobby.clone());

    let lobby_handler = use_state(|| {
        LocalLobbyCommandHandler::new(
            |data: &str| serde_json::from_str(data).expect("Failed to deserialize player data"),
            |data: &str| serde_json::from_str(data).expect("Failed to deserialize activity data"),
            |data: &str| {
                serde_json::from_str(data).expect("Failed to deserialize activity result data")
            },
        )
    });

    let client_id = use_state(ClientId::new_v4);

    let last_message = use_state(|| None::<String>);

    let websocket_connection =
        use_state(|| WebSocketConnection::new(props.config.websocket_url.clone()));

    let network_handler = use_state(|| {
        let lobby_id = (&*lobby).id.clone();
        NetworkHandler::new(
            websocket_connection.sender(),
            (*lobby_handler).clone(),
            *client_id,
            lobby_id,
            props.config.role,
        )
    });

    {
        let is_connected = websocket_connection.is_connected();
        let last_message = last_message.clone();
        let websocket_connection = websocket_connection.clone();

        use_effect_with(is_connected, move |_| {
            if is_connected {
                let conn = (&*websocket_connection).clone();
                conn.handle_messages(move |message| {
                    last_message.set(Some(message));
                });
            }
            || ()
        });
    }

    {
        let last_message = last_message.clone();
        let network_handler = network_handler.clone();
        let lobby = lobby.clone();
        use_effect_with(last_message.clone(), move |_| {
            spawn_local(async move {
                let last_message = (*last_message).clone();
                if let Some(last_message) = last_message {
                    let mut new_lobby = (*lobby).clone();
                    if let Ok(()) = network_handler.handle_message(&mut new_lobby, last_message) {
                        lobby.set(new_lobby);
                    }
                }
            });
        });
    }

    // Connect to server
    {
        let network_handler = network_handler.clone();
        let role = Role::Admin;
        let lobby = lobby.clone();
        let player = props.config.player.clone();
        use_effect_with((), move |_| {
            let lobby = (&*lobby).clone();
            spawn_local(async move {
                let _ = network_handler.connect(&player, &lobby, role);
            });
        });
    }

    {
        let websocket_connection = websocket_connection.clone();
        let last_message = last_message.clone();
        if !websocket_connection.is_connected() {
            let mut conn = (&*websocket_connection).clone();
            if let Ok(_) = conn.connect() {
                let last_message = last_message.clone();
                {
                    let last_message = last_message.clone();
                    conn.handle_messages(move |message| {
                        last_message.set(Some(message));
                    });
                }
            } else {
                return html! {
                    <div>{"Failed to connect to websocket"}</div>
                };
            }
        }
    }

    let context = LobbyProviderContext {
        lobby,
        lobby_handler: network_handler,
    };

    let debug_comp = if props.config.debug {
        html! {
                <div class="konnekt-session-lobby-debug">
                    <div class="konnekt-session-lobby-debug__client_id">{"Client ID: "}{client_id.to_string()}</div>
                    <div class="konnekt-session-lobby-debug__lobby_id">{"Lobby ID: "}{props.config.lobby.id}</div>
                    <div class="konnekt-session-lobby-debug__websocket_url">{"Websocket URL: "}{&props.config.websocket_url}</div>
                    <div class="konnekt-session-lobby-debug__connected">{"Connected: "}{"true"}</div>
                    <div class="konnekt-session-lobby-debug__message">{"Last message: "}{last_message.as_ref().unwrap_or(&"None".to_string())}</div>

                </div>
        }
    } else {
        html! {}
    };

    html! {
        <ContextProvider<LobbyProviderContext<P, A, AR>> context={context.clone()}>
            {debug_comp}
            {props.children.clone()}
        </ContextProvider<LobbyProviderContext<P, A, AR>>>
    }
}

#[hook]
pub fn use_lobby<P, A, AR>() -> UseStateHandle<Lobby<P, A, AR>>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    use_context::<LobbyProviderContext<P, A, AR>>()
        .expect("use_lobby must be used within a LobbyProvider")
        .lobby
}

#[hook]
pub fn use_lobby_handler<P, A, AR>() -> UseStateHandle<NetworkHandler<P, A, AR>>
where
    P: PlayerTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    A: ActivityTrait + Serialize + for<'de> Deserialize<'de> + 'static,
    AR: ActivityResultTrait + Serialize + for<'de> Deserialize<'de> + 'static,
{
    use_context::<LobbyProviderContext<P, A, AR>>()
        .expect("use_lobby_handler must be used within a LobbyProvider")
        .lobby_handler
}
