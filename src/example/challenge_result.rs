use serde::{Deserialize, Serialize};

use crate::model::{ActivityResultData, Identifiable};

#[derive(PartialEq, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ChallengeResult {
    pub id: String,
    pub performance: u8,
}

impl Identifiable for ChallengeResult {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl ActivityResultData for ChallengeResult {}
