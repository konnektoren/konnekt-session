use crate::model::{Activity, ActivityCatalog, ActivityData, Player, PlayerData, Role};
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

    pub fn select_activity(&mut self, id: &str) {
        if let Some(activity) = self.catalog.get_activity(id) {
            self.activities.push(activity.clone());
        }
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
}
