use serde::{Deserialize, Serialize};

use crate::model::{ActivityTrait, Identifiable, Named};

#[derive(PartialEq, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct Challenge {
    pub id: String,
    pub name: String,
}

impl Named for Challenge {
    fn name(&self) -> &str {
        &self.name
    }
}

impl Identifiable for Challenge {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl ActivityTrait for Challenge {}
