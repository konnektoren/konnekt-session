use super::{ActivityId, Scorable, Timable};
use crate::model::{Identifiable, PlayerId};
use serde::Serialize;

pub trait ActivityResultTrait: Identifiable + Timable + Scorable + Clone + PartialEq {}

#[derive(Debug, Clone, PartialEq, Hash, Serialize)]
pub struct ActivityResult<T>
where
    T: ActivityResultTrait + Serialize,
{
    pub activity_id: ActivityId,
    pub player_id: PlayerId,
    pub data: T,
}

impl<T> ActivityResult<T>
where
    T: ActivityResultTrait + Serialize,
{
    pub fn new(activity_id: ActivityId, player_id: PlayerId, data: T) -> Self {
        ActivityResult {
            activity_id,
            player_id,
            data,
        }
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TestActivityResult {
        pub id: String,
        pub score: u32,
        pub time: u64,
    }

    impl Identifiable for TestActivityResult {
        fn identifier(&self) -> &str {
            &self.id
        }
    }

    impl Scorable for TestActivityResult {
        fn score(&self) -> u32 {
            self.score
        }
    }

    impl Timable for TestActivityResult {
        fn time_taken(&self) -> u64 {
            self.time
        }
    }

    impl ActivityResultTrait for TestActivityResult {}

    #[test]
    fn test_activity_result() {
        let activity_result = TestActivityResult {
            id: "id".to_string(),
            score: 100,
            time: 1000,
        };

        assert_eq!(activity_result.identifier(), "id");
        assert_eq!(activity_result.score(), 100);
        assert_eq!(activity_result.time_taken(), 1000);
    }
}
