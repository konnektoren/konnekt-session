use crate::model::{
    Activity, ActivityCatalog, ActivityData, ActivityStatus, Player, PlayerData, Role,
};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lobby<P, A>
where
    P: PlayerData,
    A: ActivityData,
{
    pub id: Uuid,
    pub participants: Vec<Player<P>>,
    pub catalog: ActivityCatalog<A>,
    pub activities: Vec<Activity<A>>,
    pub password: Option<String>,
}

impl<P, A> Lobby<P, A>
where
    P: PlayerData,
    A: ActivityData,
{
    pub fn new(admin: Player<P>, password: Option<String>) -> Self {
        Lobby {
            id: Uuid::new_v4(),
            participants: vec![admin],
            catalog: ActivityCatalog::new(),
            activities: Vec::new(),
            password,
        }
    }

    pub fn add_participant(&mut self, participant: Player<P>) {
        self.participants.push(participant);
    }

    pub fn add_activity(&mut self, activity: Activity<A>) {
        self.catalog.add_activity(activity);
    }

    pub fn get_admin(&self) -> &Player<P> {
        self.participants
            .iter()
            .find(|player| player.role == Role::Admin)
            .unwrap()
    }

    pub fn get_participants(&self) -> &Vec<Player<P>> {
        &self.participants
    }

    pub fn get_activities(&self) -> &Vec<Activity<A>> {
        &self.activities
    }

    pub fn select_activity(&mut self, activity_id: &str) -> Option<&Activity<A>> {
        // Check if activity is already selected
        if self.activities.iter().any(|a| a.id == activity_id) {
            return self.activities.iter().find(|a| a.id == activity_id);
        }

        // If not already selected, get from catalog and add
        if let Some(activity) = self.catalog.get_activity(activity_id) {
            let activity = activity.clone();
            self.activities.push(activity);
            Some(self.activities.last().unwrap())
        } else {
            None
        }
    }

    pub fn remove_participant(&mut self, participant_id: Uuid) -> Option<Player<P>> {
        if let Some(pos) = self
            .participants
            .iter()
            .position(|p| p.id == participant_id)
        {
            Some(self.participants.remove(pos))
        } else {
            None
        }
    }

    pub fn start_activity(&mut self, activity_id: &str) -> Option<&Activity<A>> {
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            activity.status = ActivityStatus::InProgress;
            Some(activity)
        } else {
            None
        }
    }

    pub fn complete_activity(&mut self, activity_id: &str) -> Option<&Activity<A>> {
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            activity.status = ActivityStatus::Done;
            Some(activity)
        } else {
            None
        }
    }

    pub fn update_activity_status(
        &mut self,
        activity_id: &str,
        status: ActivityStatus,
    ) -> Option<&Activity<A>> {
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            activity.status = status;
            Some(activity)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Identifiable, Named, Player, PlayerData, Role};

    #[derive(PartialEq, Clone)]
    struct PlayerProfile {
        pub id: String,
        pub name: String,
    }

    impl Identifiable for PlayerProfile {
        fn identifier(&self) -> &str {
            &self.id
        }
    }

    impl Named for PlayerProfile {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl PlayerData for PlayerProfile {}

    #[derive(PartialEq, Clone)]
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

    impl ActivityData for Challenge {}

    #[test]
    fn create_lobby() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let lobby: Lobby<PlayerProfile, Challenge> = Lobby::new(admin, None);

        assert_eq!(lobby.get_admin().role, Role::Admin);
        assert_eq!(lobby.get_admin().data.identifier(), "123");
        assert_eq!(lobby.get_admin().data.name(), "Test Admin");
        assert_eq!(lobby.participants.len(), 1);
        assert_eq!(lobby.activities.len(), 0);
        assert_eq!(lobby.password, None);
    }

    #[test]
    fn add_participant() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge> = Lobby::new(admin, None);

        let participant = Player::new(
            Role::Participant,
            PlayerProfile {
                id: "456".to_string(),
                name: "Test Participant".to_string(),
            },
        );

        lobby.add_participant(participant);

        assert_eq!(lobby.participants.len(), 2);
        assert_eq!(lobby.participants[1].role, Role::Participant);
        assert_eq!(lobby.participants[1].data.identifier(), "456");
        assert_eq!(lobby.participants[1].data.name(), "Test Participant");
    }

    #[test]
    fn test_select_activity() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge> = Lobby::new(admin, None);

        let activity = Activity {
            id: "789".to_string(),
            status: ActivityStatus::NotStarted,
            data: Challenge {
                id: "789".to_string(),
                name: "Test Challenge".to_string(),
            },
        };

        // Add activity to catalog
        lobby.add_activity(activity.clone());

        // First selection should work
        assert!(lobby.select_activity(&activity.id).is_some());
        assert_eq!(lobby.activities.len(), 1);

        // Second selection should not add duplicate
        assert!(lobby.select_activity(&activity.id).is_some());
        assert_eq!(lobby.activities.len(), 1);

        // Selecting non-existent activity should fail
        assert!(lobby.select_activity("nonexistent").is_none());
        assert_eq!(lobby.activities.len(), 1);
    }

    #[test]
    fn test_activity_workflow() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge> = Lobby::new(admin, None);

        let activity = Activity {
            id: "789".to_string(),
            status: ActivityStatus::NotStarted,
            data: Challenge {
                id: "789".to_string(),
                name: "Test Challenge".to_string(),
            },
        };

        // Add to catalog and select
        lobby.add_activity(activity.clone());
        lobby.select_activity(&activity.id);

        // Start activity
        let started = lobby.start_activity(&activity.id);
        assert!(started.is_some());
        assert_eq!(started.unwrap().status, ActivityStatus::InProgress);

        // Complete activity
        let completed = lobby.complete_activity(&activity.id);
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, ActivityStatus::Done);
    }
}
