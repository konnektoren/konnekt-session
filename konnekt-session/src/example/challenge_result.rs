use serde::{Deserialize, Serialize};

use crate::model::{ActivityResultTrait, Identifiable, Scorable, Timable};

#[derive(PartialEq, Clone, Debug, Hash, Serialize, Deserialize)]
pub struct ChallengeResult {
    pub id: String,
    pub performance: u8,
    pub selection: String,
}

impl Identifiable for ChallengeResult {
    fn identifier(&self) -> &str {
        &self.id
    }
}

impl Timable for ChallengeResult {
    fn time_taken(&self) -> u64 {
        0
    }
}

impl Scorable for ChallengeResult {
    fn score(&self) -> u32 {
        match self.selection.as_str() {
            "Correct" => 1,
            _ => 0,
        }
    }
}

impl ActivityResultTrait for ChallengeResult {}
