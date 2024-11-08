use crate::model::{Identifiable, Named, Role};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player<T> {
    pub id: Uuid,
    pub role: Role,
    pub data: T,
}

impl<T> Player<T>
where
    T: Identifiable + Named,
{
    pub fn new(role: Role, data: T) -> Self {
        Player {
            id: Uuid::new_v4(),
            role,
            data,
        }
    }

    pub fn identifier(&self) -> &str {
        self.data.identifier()
    }

    pub fn name(&self) -> &str {
        self.data.name()
    }
}

// Example Usage
#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Role;

    #[derive(PartialEq)]
    struct PlayerProfile {
        id: String,
        name: String,
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
    fn create_player() {
        let player_profile = PlayerProfile {
            id: "123".to_string(),
            name: "Test Player".to_string(),
        };

        let player: Player<PlayerProfile> = Player::new(Role::Participant, player_profile);

        assert_eq!(player.identifier(), "123");
        assert_eq!(player.name(), "Test Player");
    }
}
