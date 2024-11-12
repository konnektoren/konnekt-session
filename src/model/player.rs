use crate::model::{Identifiable, Named, Role};
use uuid::Uuid;

pub type PlayerId = Uuid;

pub trait PlayerTrait: Identifiable + Named + Clone + PartialEq {}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Player<T>
where
    T: PlayerTrait,
{
    pub id: PlayerId,
    pub role: Role,
    pub data: T,
}

impl<T> Player<T>
where
    T: PlayerTrait,
{
    pub fn new(role: Role, data: T) -> Self {
        Player {
            id: Uuid::new_v4(),
            role,
            data,
        }
    }
}

impl<T> Identifiable for Player<T>
where
    T: PlayerTrait,
{
    fn identifier(&self) -> &str {
        self.data.identifier()
    }
}

impl<T> Named for Player<T>
where
    T: PlayerTrait,
{
    fn name(&self) -> &str {
        self.data.name()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Role;

    #[derive(PartialEq, Clone)]
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

    impl PlayerTrait for PlayerProfile {}

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
