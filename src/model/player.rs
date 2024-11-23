use crate::model::{Identifiable, Named, Role};
use uuid::Uuid;

pub type PlayerId = Uuid;

pub trait PlayerTrait: Identifiable + Named + Clone + PartialEq {}

#[derive(Debug, Clone, PartialEq, Hash, Default)]
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
pub mod tests {
    use super::*;
    use crate::model::Role;

    #[derive(Debug, Clone, PartialEq)]
    pub struct TestPlayer {
        pub id: String,
        pub name: String,
    }

    impl Named for TestPlayer {
        fn name(&self) -> &str {
            &self.name
        }
    }

    impl Identifiable for TestPlayer {
        fn identifier(&self) -> &str {
            &self.id
        }
    }

    impl PlayerTrait for TestPlayer {}

    #[test]
    fn create_player() {
        let player_profile = TestPlayer {
            id: "123".to_string(),
            name: "Test Player".to_string(),
        };

        let player: Player<TestPlayer> = Player::new(Role::Player, player_profile);

        assert_eq!(player.identifier(), "123");
        assert_eq!(player.name(), "Test Player");
    }
}
