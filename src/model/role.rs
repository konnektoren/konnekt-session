use serde::{Deserialize, Serialize};

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Role {
    Admin,
    #[default]
    Participant,
    Observer,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_role() {
        let role = Role::default();
        assert_eq!(role, Role::Participant);
    }
}
