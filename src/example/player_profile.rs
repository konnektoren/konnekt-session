use crate::model::{Identifiable, Named, PlayerTrait};
use serde::{Deserialize, Serialize};

#[derive(Debug, PartialEq, Clone, Hash, Serialize, Deserialize)]
pub struct PlayerProfile {
    pub id: String,
    pub name: String,
}

impl Identifiable for PlayerProfile {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl Named for PlayerProfile {
    fn name(&self) -> &str {
        &self.name
    }
}

impl PlayerTrait for PlayerProfile {}
