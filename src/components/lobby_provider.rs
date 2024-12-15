use crate::handler::{LocalLobbyCommandHandler, NetworkHandler};
use crate::model::network::{create_transport, TransportType};
use crate::model::{
    ActivityResultTrait, ActivityTrait, ClientId, Lobby, Player, PlayerTrait, Role,
};
use gloo::timers::callback::Interval;
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
    pub transport: TransportType,
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
    let ping = use_state(|| None::<u32>);
    let last_message = use_state(|| None::<String>);
    let transport = use_state(|| create_transport(&props.config.transport));
    let connection_established = use_state(|| false);

    let network_handler = use_state(|| {
        let lobby_id = lobby.id.clone();
        NetworkHandler::new(
            (*transport).clone(),
            (*lobby_handler).clone(),
            *client_id,
            lobby_id,
            props.config.role,
        )
    });

    // Initialize transport
    {
        let transport = transport.clone();
        let last_message = last_message.clone();
        let connection_established = connection_established.clone();

        use_effect_with((), move |_| {
            let mut transport = (*transport).clone();
            if !transport.is_connected() {
                if let Ok(()) = transport.connect() {
                    log::debug!("Transport connected, setting up message handler");
                    transport.handle_messages(Box::new(move |message| {
                        log::debug!("Received message through transport: {}", message);
                        last_message.set(Some(message));
                    }));

                    connection_established.set(true);
                }
            }
            || ()
        });
    }

    // Connect to server when transport is ready
    {
        let network_handler = network_handler.clone();
        let role = props.config.role;
        let lobby = lobby.clone();
        let player = props.config.player.clone();
        let connection_established = connection_established.clone();

        use_effect_with((*connection_established, ()), move |_| {
            if *connection_established {
                let lobby = (*lobby).clone();
                spawn_local(async move {
                    log::info!("Connecting to server with role: {:?}", role);
                    if let Err(e) = network_handler.connect(&player, &lobby, role) {
                        log::error!("Failed to connect to server: {:?}", e);
                    }
                });
            }
            || ()
        });
    }

    // Handle messages
    {
        let last_message = last_message.clone();
        let network_handler = network_handler.clone();
        let lobby = lobby.clone();
        let ping = ping.clone();

        use_effect_with(last_message.clone(), move |_| {
            if let Some(message) = (*last_message).clone() {
                log::debug!("Processing message: {}", message);
                spawn_local(async move {
                    let mut new_lobby = (*lobby).clone();
                    let mut new_ping = (*ping).clone();
                    if let Ok(()) =
                        network_handler.handle_message(&mut new_lobby, &mut new_ping, message)
                    {
                        lobby.set(new_lobby);
                        ping.set(new_ping);
                    }
                });
            }
            || ()
        });
    }

    // Send periodic pings
    {
        let network_handler = network_handler.clone();
        let connection_established = connection_established.clone();

        use_effect_with((*connection_established, ()), move |_| {
            let cleanup = if *connection_established {
                log::debug!("Starting ping interval");
                let handler = (*network_handler).clone();
                let interval = Interval::new(10000, move || {
                    handler.send_ping();
                });
                Box::new(move || {
                    log::debug!("Cleaning up ping interval");
                    drop(interval);
                }) as Box<dyn FnOnce()>
            } else {
                Box::new(|| {}) as Box<dyn FnOnce()>
            };
            cleanup
        });
    }

    let context = LobbyProviderContext {
        lobby: lobby.clone(),
        lobby_handler: network_handler.clone(),
    };

    let debug_comp = if props.config.debug {
        let ping_display = match *ping {
            Some(p) => p.to_string(),
            None => "None".to_string(),
        };

        html! {
            <div class="konnekt-session-lobby-debug">
                <div class="konnekt-session-lobby-debug__client_id">
                    {"Client ID: "}{client_id.to_string()}
                </div>
                <div class="konnekt-session-lobby-debug__lobby_id">
                    {"Lobby ID: "}{props.config.lobby.id}
                </div>
                <div class="konnekt-session-lobby-debug__websocket_url">
                    {"URL: "}{format!("{:?}", props.config.transport)}
                </div>
                <div class="konnekt-session-lobby-debug__connected">
                    {"Connected: "}{*connection_established}
                </div>
                <div class="konnekt-session-lobby-debug__ping">
                    {"Ping: "}{ping_display}
                </div>
                <div class="konnekt-session-lobby-debug__message">
                    {"Last message: "}{last_message.as_ref().unwrap_or(&"None".to_string())}
                </div>
            </div>
        }
    } else {
        html! {}
    };

    html! {
        <ContextProvider<LobbyProviderContext<P, A, AR>> context={context}>
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
