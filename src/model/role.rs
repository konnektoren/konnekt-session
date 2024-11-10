use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    #[default]
    Participant,
    Observer,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Admin => write!(f, "Admin"),
            Role::Participant => write!(f, "Participant"),
            Role::Observer => write!(f, "Observer"),
        }
    }
}

impl From<String> for Role {
    fn from(s: String) -> Self {
        match s.as_str() {
            "Admin" => Role::Admin,
            "Participant" => Role::Participant,
            "Observer" => Role::Observer,
            _ => Role::Participant,
        }
    }
}

impl From<&str> for Role {
    fn from(s: &str) -> Self {
        match s {
            "Admin" => Role::Admin,
            "Participant" => Role::Participant,
            "Observer" => Role::Observer,
            _ => Role::Participant,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_role() {
        let role = Role::default();
        assert_eq!(role, Role::Participant);
    }

    #[test]
    fn role_to_string() {
        assert_eq!(Role::Admin.to_string(), "Admin");
        assert_eq!(Role::Participant.to_string(), "Participant");
        assert_eq!(Role::Observer.to_string(), "Observer");
    }

    #[test]
    fn string_to_role() {
        assert_eq!(Role::from("Admin".to_string()), Role::Admin);
        assert_eq!(Role::from("Participant".to_string()), Role::Participant);
        assert_eq!(Role::from("Observer".to_string()), Role::Observer);
        assert_eq!(Role::from("Unknown".to_string()), Role::Participant);
    }

    #[test]
    fn str_to_role() {
        assert_eq!(Role::from("Admin"), Role::Admin);
        assert_eq!(Role::from("Participant"), Role::Participant);
        assert_eq!(Role::from("Observer"), Role::Observer);
        assert_eq!(Role::from("Unknown"), Role::Participant);
    }
}
