use uuid::Uuid;

pub type ClientId = Uuid;

#[derive(Debug, Clone)]
pub struct Client {
    pub id: ClientId,
    pub ping: u32,
}
