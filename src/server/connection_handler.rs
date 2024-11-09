use crate::model::Role;
use crate::server::Connection;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::RwLock;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::WebSocketStream;
use uuid::Uuid;

pub trait ConnectionHandler {
    fn handle_connect(
        &self,
        player_id: Uuid,
        role: Role,
        data: &str,
        password: &str,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
        ws_sender: WebSocketStream<tokio::net::TcpStream>,
        tx: mpsc::UnboundedSender<Message>,
    ) -> tokio::task::JoinHandle<()>;

    fn handle_close(
        &self,
        player_id: Uuid,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    );

    fn handle_message(
        &self,
        message: &str,
        player_id: Uuid,
        lobby_id: Uuid,
        connections: Arc<RwLock<HashMap<Uuid, HashMap<Uuid, Connection>>>>,
    ) -> Option<Message>;
}
