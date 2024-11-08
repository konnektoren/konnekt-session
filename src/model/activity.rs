#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum ActivityStatus {
    #[default]
    NotStarted,
    InProgress,
    Done,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Activity {
    pub id: String,
    pub name: String,
    pub description: String,
    pub status: ActivityStatus,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_activity() {
        let activity = Activity {
            id: "123".to_string(),
            name: "Test Activity".to_string(),
            description: "This is a test activity".to_string(),
            status: ActivityStatus::NotStarted,
        };

        assert_eq!(activity.id, "123");
        assert_eq!(activity.name, "Test Activity");
        assert_eq!(activity.description, "This is a test activity");
        assert_eq!(activity.status, ActivityStatus::NotStarted);
    }
}
