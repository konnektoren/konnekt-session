use crate::model::Identifiable;
use crate::model::Named;
use serde::{Deserialize, Serialize};

pub trait ActivityData: Named + Identifiable + Clone + PartialEq {}

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
    T: ActivityData,
{
    pub id: String,
    pub data: T,
    pub status: ActivityStatus,
}

impl<T> Activity<T>
where
    T: ActivityData,
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
    T: ActivityData,
{
    fn name(&self) -> &str {
        self.data.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_activity() {
        #[derive(Clone, PartialEq)]
        struct Challenge {
            pub id: String,
            pub name: String,
            pub description: String,
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

        let challenge = Challenge {
            id: "123".to_string(),
            name: "Test Activity".to_string(),
            description: "This is a test activity".to_string(),
        };

        let activity = Activity::<Challenge> {
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
