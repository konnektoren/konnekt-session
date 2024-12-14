use super::{Transport, TransportType};

#[cfg(feature = "websocket")]
use super::websocket_connection::WebSocketConnection;

#[cfg(feature = "webrtc")]
use super::webrtc_connection::WebRTCConnection;

pub fn create_transport(transport_type: &TransportType) -> Box<dyn Transport> {
    match transport_type {
        #[cfg(feature = "websocket")]
        TransportType::WebSocket(url) => Box::new(WebSocketConnection::new(url.clone())),

        #[cfg(feature = "webrtc")]
        TransportType::WebRTC(url, lobby_id, client_id, role) => Box::new(WebRTCConnection::new(
            url.clone(),
            client_id.clone(),
            lobby_id.clone(),
            role.clone(),
        )),

        #[cfg(not(any(feature = "websocket", feature = "webrtc")))]
        _ => panic!("No transport implementation available"),
    }
}
