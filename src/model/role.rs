use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, Copy, PartialEq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    #[default]
    Player,
    Observer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "Admin"),
            Role::Player => write!(f, "Player"),
            Role::Observer => write!(f, "Observer"),
        }
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Admin" => Role::Admin,
            "Player" => Role::Player,
            "Observer" => Role::Observer,
            _ => Role::Player,
        }
    }
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        match s {
            "Admin" => Role::Admin,
            "Player" => Role::Player,
            "Observer" => Role::Observer,
            _ => Role::Player,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_role() {
        let role = Role::default();
        assert_eq!(role, Role::Player);
    }

    #[test]
    fn role_to_string() {
        assert_eq!(Role::Admin.to_string(), "Admin");
        assert_eq!(Role::Player.to_string(), "Player");
        assert_eq!(Role::Observer.to_string(), "Observer");
    }

    #[test]
    fn string_to_role() {
        assert_eq!(Role::from("Admin".to_string()), Role::Admin);
        assert_eq!(Role::from("Player".to_string()), Role::Player);
        assert_eq!(Role::from("Observer".to_string()), Role::Observer);
        assert_eq!(Role::from("Unknown".to_string()), Role::Player);
    }

    #[test]
    fn str_to_role() {
        assert_eq!(Role::from("Admin"), Role::Admin);
        assert_eq!(Role::from("Player"), Role::Player);
        assert_eq!(Role::from("Observer"), Role::Observer);
        assert_eq!(Role::from("Unknown"), Role::Player);
    }
}
