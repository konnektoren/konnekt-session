use crate::model::{ActivityId, ActivityTrait};

use super::Activity;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ActivityCatalog<T>
where
    T: ActivityTrait,
{
    pub activities: Vec<Activity<T>>,
}

impl<T> Default for ActivityCatalog<T>
where
    T: ActivityTrait,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<T> ActivityCatalog<T>
where
    T: ActivityTrait,
{
    pub fn new() -> Self {
        ActivityCatalog {
            activities: Vec::new(),
        }
    }

    pub fn get_activities(&self) -> &Vec<Activity<T>> {
        &self.activities
    }

    pub fn add_activity(&mut self, activity: Activity<T>) {
        if self.activities.iter().any(|a| a.id == activity.id) {
            return;
        }
        self.activities.push(activity);
    }

    pub fn get_activity(&self, id: &ActivityId) -> Option<&Activity<T>> {
        self.activities.iter().find(|activity| activity.id.eq(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Identifiable, Named};

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct Challenge {
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

    #[test]
    fn create_activity_catalog() {
        let challenge1 = Challenge {
            id: "123".to_string(),
            name: "Challenge 1".to_string(),
        };

        let challenge2 = Challenge {
            id: "456".to_string(),
            name: "Challenge 2".to_string(),
        };

        let mut catalog: ActivityCatalog<Challenge> = ActivityCatalog::new();
        catalog.add_activity(Activity::new(challenge1.clone()));
        catalog.add_activity(Activity::new(challenge2.clone()));

        let activities = catalog.get_activities();
        assert_eq!(activities.len(), 2);
        assert_eq!(activities[0].data, challenge1);
        assert_eq!(activities[1].data, challenge2);

        assert_eq!(
            catalog.get_activity(&"123".to_string()).unwrap().data,
            challenge1
        );
        assert_eq!(
            catalog.get_activity(&"456".to_string()).unwrap().data,
            challenge2
        );
    }
}
