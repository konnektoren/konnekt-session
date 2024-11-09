use crate::model::{ActivityData, Identifiable, Named};

#[derive(PartialEq, Clone)]
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

impl ActivityData for Challenge {}
