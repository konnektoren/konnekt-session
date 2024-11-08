use crate::model::{Activity, Identifiable, Named, Player, Role};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Lobby<T> {
    pub id: Uuid,
    pub participants: Vec<Player<T>>,
    pub activities: Vec<Activity>,
    pub password: Option<String>,
}

impl<T> Lobby<T>
where
    T: Identifiable + Named,
{
    pub fn new(admin: Player<T>, password: Option<String>) -> Self {
        Lobby {
            id: Uuid::new_v4(),
            participants: vec![admin],
            activities: Vec::new(),
            password,
        }
    }

    pub fn add_participant(&mut self, participant: Player<T>) {
        self.participants.push(participant);
    }

    pub fn add_activity(&mut self, activity: Activity) {
        self.activities.push(activity);
    }

    pub fn get_admin(&self) -> &Player<T> {
        self.participants
            .iter()
            .find(|player| player.role == Role::Admin)
            .unwrap()
    }

    pub fn get_participants(&self) -> &Vec<Player<T>> {
        &self.participants
    }

    pub fn get_activities(&self) -> &Vec<Activity> {
        &self.activities
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Player, Role};

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

    #[test]
    fn create_lobby() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let lobby = Lobby::new(admin, None);

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

        let mut lobby = Lobby::new(admin, None);

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
