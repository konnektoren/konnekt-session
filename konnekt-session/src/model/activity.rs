use crate::model::Identifiable;
use crate::model::Named;
use serde::{Deserialize, Serialize};

pub type ActivityId = String;

pub trait ActivityTrait: Named + Identifiable + Clone + PartialEq {}

#[derive(Default, Debug, Clone, PartialEq, Hash, Serialize, Deserialize)]
pub enum ActivityStatus {
    #[default]
    NotStarted,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Activity<T>
where
    T: ActivityTrait,
{
    pub id: ActivityId,
    pub data: T,
    pub status: ActivityStatus,
}

impl<T> Activity<T>
where
    T: ActivityTrait,
{
    pub fn new(data: T) -> Self {
        Activity {
            id: data.identifier().to_string(),
            data,
            status: ActivityStatus::NotStarted,
        }
    }
}

impl<T> Named for Activity<T>
where
    T: ActivityTrait,
{
    fn name(&self) -> &str {
        self.data.name()
    }
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TestActivity {
        pub id: String,
        pub name: String,
    }

    impl Named for TestActivity {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Identifiable for TestActivity {
        fn identifier(&self) -> &str {
            &self.id
        }
    }

    impl ActivityTrait for TestActivity {}

    #[test]
    fn create_activity() {
        let challenge = TestActivity {
            id: "123".to_string(),
            name: "Test Activity".to_string(),
        };

        let activity = Activity::<TestActivity> {
            id: "123".to_string(),
            data: challenge,
            status: ActivityStatus::NotStarted,
        };

        assert_eq!(activity.id, "123");
        assert_eq!(activity.status, ActivityStatus::NotStarted);
        assert_eq!(activity.data.id, "123");
        assert_eq!(activity.data.name, "Test Activity");
    }
}
