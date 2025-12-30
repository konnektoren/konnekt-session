use uuid::Uuid;

/// Commands that can be executed on the lobby domain
#[derive(Debug, Clone, PartialEq)]
pub enum DomainCommand {
    /// Create a new lobby
    CreateLobby {
        lobby_name: String,
        host_name: String,
    },

    /// Join an existing lobby as guest
    JoinLobby { lobby_id: Uuid, guest_name: String },

    /// Leave the lobby
    LeaveLobby {
        lobby_id: Uuid,
        participant_id: Uuid,
    },

    /// Kick a guest (host only)
    KickGuest {
        lobby_id: Uuid,
        host_id: Uuid,
        guest_id: Uuid,
    },

    /// Toggle participation mode (Active ↔ Spectating)
    ToggleParticipationMode {
        lobby_id: Uuid,
        participant_id: Uuid,
        requester_id: Uuid,
        activity_in_progress: bool,
    },

    /// Delegate host role to another participant
    DelegateHost {
        lobby_id: Uuid,
        current_host_id: Uuid,
        new_host_id: Uuid,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_command_clone() {
        let cmd = DomainCommand::CreateLobby {
            lobby_name: "Test".to_string(),
            host_name: "Alice".to_string(),
        };

        let cloned = cmd.clone();
        assert_eq!(cmd, cloned);
    }

    #[test]
    fn test_command_debug() {
        let cmd = DomainCommand::JoinLobby {
            lobby_id: Uuid::new_v4(),
            guest_name: "Bob".to_string(),
        };

        let debug = format!("{:?}", cmd);
        assert!(debug.contains("JoinLobby"));
        assert!(debug.contains("Bob"));
    }
}
