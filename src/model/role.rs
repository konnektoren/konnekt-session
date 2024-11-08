#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub enum Role {
    Admin,
    #[default]
    Participant,
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
