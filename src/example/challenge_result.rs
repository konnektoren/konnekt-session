use serde::{Deserialize, Serialize};

use crate::model::{ActivityResultTrait, Identifiable, Scorable, Timable};

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

impl Timable for ChallengeResult {
    fn time_taken(&self) -> u64 {
        0
    }
}

impl Scorable for ChallengeResult {
    fn score(&self) -> u32 {
        self.performance as u32
    }
}

impl ActivityResultTrait for ChallengeResult {}
