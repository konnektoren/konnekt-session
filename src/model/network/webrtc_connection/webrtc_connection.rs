use super::super::{MessageCallback, NetworkError, Transport, TransportType};
use super::WebRTCConnectionManager;
use crate::model::network::NetworkCommand;
use crate::model::{ClientId, LobbyId, Role, SignalingContent, SignalingMessage};
use futures::channel::mpsc::{self, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use std::sync::{Arc, Mutex, RwLock};
use uuid::Uuid;
use wasm_bindgen_futures::{spawn_local, JsFuture};
use web_sys::wasm_bindgen::closure::Closure;
use web_sys::wasm_bindgen::JsCast;
use web_sys::wasm_bindgen::JsValue;
use web_sys::{
    ErrorEvent, MessageEvent, RtcConfiguration, RtcDataChannel, RtcDataChannelEvent,
    RtcDataChannelState, RtcIceCandidate, RtcIceCandidateInit, RtcPeerConnection,
    RtcPeerConnectionIceEvent, RtcPeerConnectionState, RtcSdpType, RtcSessionDescriptionInit,
    WebSocket,
};

#[derive(Clone)]
pub struct WebRTCConnection {
    signaling_url: String,
    client_id: ClientId,
    lobby_id: LobbyId,
    peer_connection: Arc<RwLock<Option<RtcPeerConnection>>>,
    data_channel: Arc<RwLock<Option<RtcDataChannel>>>,
    sender: UnboundedSender<String>,
    receiver: Arc<RwLock<UnboundedReceiver<String>>>,
    connected: Arc<RwLock<bool>>,
    signaling_websocket: Arc<RwLock<Option<WebSocket>>>,
    connection_manager: Arc<WebRTCConnectionManager>,
    is_admin: Arc<RwLock<bool>>,
}

impl WebRTCConnection {
    pub fn new(signaling_url: String, client_id: ClientId, lobby_id: LobbyId, role: Role) -> Self {
        let (sender, receiver) = mpsc::unbounded();
        let connection_manager = WebRTCConnectionManager::new();
        Self {
            signaling_url,
            client_id,
            lobby_id,
            peer_connection: Arc::new(RwLock::new(None)),
            data_channel: Arc::new(RwLock::new(None)),
            sender,
            receiver: Arc::new(RwLock::new(receiver)),
            connected: Arc::new(RwLock::new(false)),
            signaling_websocket: Arc::new(RwLock::new(None)),
            connection_manager: Arc::new(connection_manager),
            is_admin: Arc::new(RwLock::new(role == Role::Admin)),
        }
    }

    pub fn set_admin(&self, is_admin: bool) {
        log::info!(
            "Setting admin status to {} for client {}",
            is_admin,
            self.client_id
        );
        *self.is_admin.write().unwrap() = is_admin;
    }

    fn create_peer_connection(&self) -> Result<RtcPeerConnection, NetworkError> {
        let rtc_config = RtcConfiguration::new();
        let ice_servers = self.get_ice_servers();
        rtc_config.set_ice_servers(&ice_servers);

        RtcPeerConnection::new_with_configuration(&rtc_config).map_err(|e| {
            NetworkError::ConnectionError(format!("Failed to create peer connection: {:?}", e))
        })
    }

    fn get_ice_servers(&self) -> js_sys::Array {
        let ice_servers = js_sys::Array::new();
        let stun_servers = [
            "stun:stun1.l.google.com:19302",
            "stun:stun2.l.google.com:19302",
            "stun:stun3.l.google.com:19302",
            "stun:stun4.l.google.com:19302",
        ];

        for server in stun_servers.iter() {
            let stun_server = js_sys::Object::new();
            js_sys::Reflect::set(
                &stun_server,
                &JsValue::from_str("urls"),
                &JsValue::from_str(server),
            )
            .unwrap();
            ice_servers.push(&stun_server);
        }

        ice_servers
    }

    async fn create_offer(peer_connection: &RtcPeerConnection) -> Result<String, NetworkError> {
        let offer = JsFuture::from(peer_connection.create_offer())
            .await
            .map_err(|e| {
                NetworkError::ConnectionError(format!("Failed to create offer: {:?}", e))
            })?;

        let offer_sdp = offer.unchecked_into::<RtcSessionDescriptionInit>();

        JsFuture::from(peer_connection.set_local_description(&offer_sdp))
            .await
            .map_err(|e| {
                NetworkError::ConnectionError(format!("Failed to set local description: {:?}", e))
            })?;

        let sdp = peer_connection
            .local_description()
            .ok_or_else(|| {
                NetworkError::ConnectionError("No local description available".to_string())
            })?
            .sdp();

        Ok(sdp)
    }

    fn setup_peer_connection_callbacks(&self, peer_connection: &RtcPeerConnection) {
        self.setup_negotiation_needed_callback(peer_connection);
        self.setup_ice_candidate_callback(peer_connection);
        self.setup_connection_state_change_callback(peer_connection);

        // Add ondatachannel handler
        let data_channel = self.data_channel.clone();
        let self_clone = self.clone();
        let ondatachannel_callback = Closure::wrap(Box::new(move |evt: RtcDataChannelEvent| {
            log::info!("Received data channel from peer");
            let channel = evt.channel();
            log::info!("Data channel state: {:?}", channel.ready_state());
            *data_channel.write().unwrap() = Some(channel.clone());
            self_clone.setup_data_channel(channel);
        })
            as Box<dyn FnMut(RtcDataChannelEvent)>);

        peer_connection.set_ondatachannel(Some(ondatachannel_callback.as_ref().unchecked_ref()));
        ondatachannel_callback.forget();
    }

    fn setup_negotiation_needed_callback(&self, peer_connection: &RtcPeerConnection) {
        let pc = peer_connection.clone();
        let self_clone = self.clone();

        let callback = Closure::wrap(Box::new(move || {
            let pc_clone = pc.clone();
            let self_clone = self_clone.clone();

            spawn_local(async move {
                if let Ok(offer) = WebRTCConnection::create_offer(&pc_clone).await {
                    let message = SignalingMessage {
                        from: self_clone.client_id,
                        to: Uuid::nil(),
                        content: SignalingContent::Offer { sdp: offer },
                    };

                    if let Some(ws) = self_clone.signaling_websocket.read().unwrap().as_ref() {
                        if let Ok(msg_str) = serde_json::to_string(&message) {
                            if let Err(e) = ws.send_with_str(&msg_str) {
                                log::error!("Error sending renegotiation offer: {:?}", e);
                            }
                        }
                    }
                }
            });
        }) as Box<dyn FnMut()>);

        peer_connection.set_onnegotiationneeded(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn setup_ice_candidate_callback(&self, peer_connection: &RtcPeerConnection) {
        let signaling_websocket = self.signaling_websocket.clone();
        let client_id = self.client_id;

        let callback = Closure::wrap(Box::new(move |evt: RtcPeerConnectionIceEvent| {
            if let Some(candidate) = evt.candidate() {
                let message = SignalingMessage {
                    from: client_id,
                    to: Uuid::nil(),
                    content: SignalingContent::IceCandidate {
                        candidate: candidate.candidate(),
                        sdp_mid: candidate.sdp_mid(),
                        sdp_mline_index: candidate.sdp_m_line_index(),
                    },
                };
                if let Some(ws) = signaling_websocket.read().unwrap().as_ref() {
                    if let Ok(msg_str) = serde_json::to_string(&message) {
                        if let Err(e) = ws.send_with_str(&msg_str) {
                            log::error!("Error sending ICE candidate: {:?}", e);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(RtcPeerConnectionIceEvent)>);

        peer_connection.set_onicecandidate(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn setup_connection_state_change_callback(&self, peer_connection: &RtcPeerConnection) {
        let connected = self.connected.clone();
        let data_channel = self.data_channel.clone();
        let pc = peer_connection.clone();

        let callback = Closure::wrap(Box::new(move || {
            let state = pc.connection_state();
            log::info!("Connection state changed to: {:?}", state);

            match state {
                RtcPeerConnectionState::Connected => {
                    if let Ok(mut connected_guard) = connected.write() {
                        *connected_guard = true;
                    }
                    log::info!("WebRTC connection established");

                    // Log the data channel state but don't try to send yet
                    if let Ok(dc_guard) = data_channel.read() {
                        if let Some(dc) = dc_guard.as_ref() {
                            log::info!("Data channel state when connected: {:?}", dc.ready_state());
                        }
                    }
                }
                RtcPeerConnectionState::Failed
                | RtcPeerConnectionState::Disconnected
                | RtcPeerConnectionState::Closed => {
                    if let Ok(mut connected_guard) = connected.write() {
                        *connected_guard = false;
                    }
                    if let Ok(mut dc_guard) = data_channel.write() {
                        *dc_guard = None;
                    }
                    log::warn!("WebRTC connection state changed to: {:?}", state);
                }
                _ => {
                    log::debug!("WebRTC connection state changed to: {:?}", state);
                }
            }
        }) as Box<dyn FnMut()>);

        peer_connection.set_onconnectionstatechange(Some(callback.as_ref().unchecked_ref()));
        callback.forget();
    }

    fn setup_data_channel(&self, data_channel: RtcDataChannel) {
        let sender = self.sender.clone();
        let connected = self.connected.clone();
        let connection_manager = self.connection_manager.clone();
        let client_id = self.client_id;
        let lobby_id = self.lobby_id;
        let is_admin = self.is_admin.clone();

        let data_channel_ref = Arc::new(RwLock::new(Some(data_channel.clone())));

        // Store in connection manager
        connection_manager.add_connection(client_id, lobby_id, data_channel.clone());

        let dc_name = data_channel.label();

        // Setup open callback
        let connected_open = connected.clone();
        let data_channel_open = data_channel.clone();
        let onopen_callback = Closure::wrap(Box::new(move || {
            log::info!(
                "Data channel '{}' opened with state: {:?}",
                dc_name,
                data_channel_open.ready_state()
            );

            if let Ok(mut connected_guard) = connected_open.write() {
                *connected_guard = true;
            }

            // Only send if the data channel is actually open
            if data_channel_open.ready_state() == RtcDataChannelState::Open {
                // Send initial connect message
                let connect_message: NetworkCommand<String> = NetworkCommand::Connect {
                    client_id,
                    lobby_id,
                };

                if let Ok(msg_str) = serde_json::to_string(&connect_message) {
                    match data_channel_open.send_with_str(&msg_str) {
                        Ok(_) => log::info!("✅ Successfully sent connect message"),
                        Err(e) => log::error!("❌ Failed to send connect message: {:?}", e),
                    }
                }
            }
        }) as Box<dyn FnMut()>);

        // Setup close callback
        let connected_close = connected.clone();
        let data_channel_close = data_channel_ref.clone();
        let onclose_callback = Closure::wrap(Box::new(move || {
            log::info!("Data channel closed");
            if let Ok(mut connected_guard) = connected_close.write() {
                *connected_guard = false;
            }
            if let Ok(mut dc_guard) = data_channel_close.write() {
                *dc_guard = None;
            }
        }) as Box<dyn FnMut()>);

        // Setup error callback
        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("Data channel error: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);

        // Setup message callback
        let onmessage_callback = Closure::wrap(Box::new(move |evt: MessageEvent| {
            if let Ok(text) = evt.data().dyn_into::<js_sys::JsString>() {
                if let Some(message) = text.as_string() {
                    log::info!("👋 Received message from peer: {}", message);

                    match serde_json::from_str::<NetworkCommand<String>>(&message) {
                        Ok(command) => {
                            log::info!("✅ Parsed network command: {:?}", command.get_type());

                            // Log the message details for debugging
                            match &command {
                                NetworkCommand::Message {
                                    client_id: from_id, ..
                                } => {
                                    log::info!(
                                        "Message from client_id: {}, current client_id: {}",
                                        from_id,
                                        client_id
                                    );
                                }
                                _ => {}
                            }

                            // Forward to local handler first
                            if let Err(e) = sender.unbounded_send(message.clone()) {
                                log::error!("❌ Failed to forward to local handler: {:?}", e);
                                return;
                            }

                            // If admin, broadcast to other peers
                            if *is_admin.read().unwrap() {
                                log::info!("📢 Admin broadcasting message to other peers");
                                connection_manager.broadcast_to_lobby(
                                    &lobby_id,
                                    &message,
                                    Some(client_id),
                                );
                            }
                        }
                        Err(e) => {
                            log::warn!("Failed to parse as NetworkCommand: {:?}", e);
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        // Set callbacks
        data_channel.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        data_channel.set_onclose(Some(onclose_callback.as_ref().unchecked_ref()));
        data_channel.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));
        data_channel.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));

        // Forget callbacks to prevent them from being dropped
        onopen_callback.forget();
        onclose_callback.forget();
        onerror_callback.forget();
        onmessage_callback.forget();
    }

    async fn handle_signaling_messages(&self) -> Result<(), NetworkError> {
        let ws_url = format!(
            "{}/signaling/{}/{}",
            self.signaling_url, self.lobby_id, self.client_id
        );
        log::info!("Connecting to signaling server: {}", ws_url);

        let ws = WebSocket::new(&ws_url)
            .map_err(|e| NetworkError::ConnectionError(format!("WebSocket failed: {:?}", e)))?;

        let (open_sender, open_receiver) = futures::channel::oneshot::channel();
        let open_sender = Arc::new(Mutex::new(Some(open_sender)));

        let onopen_callback = {
            let open_sender = open_sender.clone();
            Closure::wrap(Box::new(move || {
                log::info!("WebSocket connection established");
                if let Some(sender) = open_sender.lock().unwrap().take() {
                    let _ = sender.send(());
                }
            }) as Box<dyn FnMut()>)
        };

        let onerror_callback = Closure::wrap(Box::new(move |e: ErrorEvent| {
            log::error!("WebSocket error: {:?}", e);
        }) as Box<dyn FnMut(ErrorEvent)>);

        ws.set_onopen(Some(onopen_callback.as_ref().unchecked_ref()));
        ws.set_onerror(Some(onerror_callback.as_ref().unchecked_ref()));

        onopen_callback.forget();
        onerror_callback.forget();

        *self.signaling_websocket.write().unwrap() = Some(ws.clone());

        open_receiver
            .await
            .map_err(|_| NetworkError::ConnectionError("WebSocket connection failed".into()))?;

        // Add delay and check WebSocket state
        gloo_timers::future::TimeoutFuture::new(100).await;
        if ws.ready_state() != WebSocket::OPEN {
            return Err(NetworkError::ConnectionError("WebSocket not ready".into()));
        }

        self.setup_signaling_message_handler(&ws);

        Ok(())
    }

    fn setup_signaling_message_handler(&self, ws: &WebSocket) {
        let peer_connection = self.peer_connection.clone();
        let client_id = self.client_id;
        let signaling_websocket = self.signaling_websocket.clone();

        let onmessage_callback = Closure::wrap(Box::new(move |evt: MessageEvent| {
            if let Ok(text) = evt.data().dyn_into::<js_sys::JsString>() {
                if let Ok(message) =
                    serde_json::from_str::<SignalingMessage>(&text.as_string().unwrap())
                {
                    if message.to == client_id {
                        match message.content {
                            SignalingContent::Offer { sdp } => {
                                let peer_connection = peer_connection.clone();
                                let from = message.from;
                                let client_id = client_id;
                                let signaling_websocket = signaling_websocket.clone();

                                spawn_local(async move {
                                    if let Some(pc) = peer_connection.read().unwrap().as_ref() {
                                        let desc =
                                            RtcSessionDescriptionInit::new(RtcSdpType::Offer);
                                        desc.set_sdp(&sdp);

                                        if let Ok(_) =
                                            JsFuture::from(pc.set_remote_description(&desc)).await
                                        {
                                            if let Ok(answer) =
                                                JsFuture::from(pc.create_answer()).await
                                            {
                                                let answer = answer
                                                    .unchecked_into::<RtcSessionDescriptionInit>();
                                                if let Ok(_) = JsFuture::from(
                                                    pc.set_local_description(&answer),
                                                )
                                                .await
                                                {
                                                    let response = SignalingMessage {
                                                        from: client_id,
                                                        to: from,
                                                        content: SignalingContent::Answer {
                                                            sdp: answer
                                                                .get_sdp()
                                                                .unwrap_or_default(),
                                                        },
                                                    };
                                                    if let Some(ws) =
                                                        signaling_websocket.read().unwrap().as_ref()
                                                    {
                                                        if let Ok(msg_str) =
                                                            serde_json::to_string(&response)
                                                        {
                                                            if let Err(e) =
                                                                ws.send_with_str(&msg_str)
                                                            {
                                                                log::error!(
                                                                    "Error sending answer: {:?}",
                                                                    e
                                                                );
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                });
                            }
                            SignalingContent::Answer { sdp } => {
                                let peer_connection = peer_connection.clone();
                                let answer_desc =
                                    RtcSessionDescriptionInit::new(RtcSdpType::Answer);
                                answer_desc.set_sdp(&sdp);

                                spawn_local(async move {
                                    if let Some(pc) = peer_connection.read().unwrap().as_ref() {
                                        if let Err(e) =
                                            JsFuture::from(pc.set_remote_description(&answer_desc))
                                                .await
                                        {
                                            log::error!("setRemoteDescription failed: {:?}", e);
                                        }
                                    }
                                });
                            }
                            SignalingContent::IceCandidate {
                                candidate,
                                sdp_mid,
                                sdp_mline_index,
                            } => {
                                let peer_connection = peer_connection.clone();
                                let init = RtcIceCandidateInit::new(&candidate);
                                init.set_sdp_mid(sdp_mid.as_deref());
                                init.set_sdp_m_line_index(sdp_mline_index);

                                if let Ok(candidate) = RtcIceCandidate::new(&init) {
                                    spawn_local(async move {
                                        if let Some(pc) = peer_connection.read().unwrap().as_ref() {
                                            if let Err(e) = JsFuture::from(
                                                pc.add_ice_candidate_with_opt_rtc_ice_candidate(
                                                    Some(&candidate),
                                                ),
                                            )
                                            .await
                                            {
                                                log::error!("addIceCandidate failed: {:?}", e);
                                            }
                                        }
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }) as Box<dyn FnMut(MessageEvent)>);

        ws.set_onmessage(Some(onmessage_callback.as_ref().unchecked_ref()));
        onmessage_callback.forget();
    }
}

impl Transport for WebRTCConnection {
    fn connect(&mut self) -> Result<(), NetworkError> {
        let pc = self.create_peer_connection()?;
        self.setup_peer_connection_callbacks(&pc);

        let data_channel = pc.create_data_channel("data");
        self.setup_data_channel(data_channel);

        *self.peer_connection.write().unwrap() = Some(pc.clone());

        let self_clone = self.clone();
        spawn_local(async move {
            if let Err(e) = self_clone.handle_signaling_messages().await {
                log::error!("Signaling message handling failed: {:?}", e);
                return;
            }

            // Wait a bit to allow other peers to connect
            gloo_timers::future::TimeoutFuture::new(500).await;

            if let Some(pc) = self_clone.peer_connection.read().unwrap().as_ref() {
                match WebRTCConnection::create_offer(pc).await {
                    Ok(offer) => {
                        let message = SignalingMessage {
                            from: self_clone.client_id,
                            to: Uuid::nil(), // Server will find an available peer
                            content: SignalingContent::Offer { sdp: offer },
                        };

                        if let Some(ws) = self_clone.signaling_websocket.read().unwrap().as_ref() {
                            if let Ok(msg_str) = serde_json::to_string(&message) {
                                if let Err(e) = ws.send_with_str(&msg_str) {
                                    log::error!("Error sending offer: {:?}", e);
                                }
                            }
                        }
                    }
                    Err(e) => log::error!("Failed to create offer: {:?}", e),
                }
            }
        });

        Ok(())
    }

    fn sender(&self) -> UnboundedSender<String> {
        self.sender.clone()
    }

    fn disconnect(&mut self) {
        if let Some(dc) = self.data_channel.write().unwrap().take() {
            if dc.ready_state() == RtcDataChannelState::Open {
                dc.close();
            }
        }

        if let Some(pc) = self.peer_connection.write().unwrap().take() {
            pc.close();
        }

        if let Some(ws) = self.signaling_websocket.write().unwrap().take() {
            let _ = ws.close();
        }

        *self.connected.write().unwrap() = false;
    }

    fn is_connected(&self) -> bool {
        *self.connected.read().unwrap()
    }

    fn handle_messages(&self, callback: MessageCallback) {
        let receiver = self.receiver.clone();
        let callback = Arc::new(callback);

        spawn_local(async move {
            let mut receiver_guard = receiver.write().unwrap();
            while let Some(message) = receiver_guard.next().await {
                log::debug!("Received message through transport: {}", message);
                callback(message);
            }
        });
    }

    fn transport_type(&self) -> TransportType {
        TransportType::WebRTC(
            self.signaling_url.clone(),
            self.lobby_id,
            self.client_id,
            match *self.is_admin.read().unwrap() {
                true => Role::Admin,
                false => Role::Player,
            },
        )
    }

    fn box_clone(&self) -> Box<dyn Transport> {
        Box::new(self.clone())
    }
}

impl PartialEq for WebRTCConnection {
    fn eq(&self, other: &Self) -> bool {
        self.signaling_url == other.signaling_url
    }
}
