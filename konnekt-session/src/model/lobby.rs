use super::PlayerId;
use crate::model::{
    Activity, ActivityCatalog, ActivityId, ActivityResult, ActivityResultTrait, ActivityStatus,
    ActivityTrait, Player, PlayerTrait, Role,
};
use serde::Serialize;
use uuid::Uuid;

pub type LobbyId = Uuid;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Lobby<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
{
    pub id: LobbyId,
    pub player_id: PlayerId,
    pub participants: Vec<Player<P>>,
    pub catalog: ActivityCatalog<A>,
    pub activities: Vec<Activity<A>>,
    pub password: Option<String>,
    pub results: Vec<ActivityResult<AR>>,
}

impl<P, A, AR> Lobby<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
{
    pub fn new_with_id(lobby_id: LobbyId, admin: Player<P>, password: Option<String>) -> Self {
        Lobby {
            id: lobby_id,
            player_id: admin.id,
            participants: vec![admin],
            catalog: ActivityCatalog::new(),
            activities: Vec::new(),
            password,
            results: Vec::new(),
        }
    }
}

impl<P, A, AR> Lobby<P, A, AR>
where
    P: PlayerTrait,
    A: ActivityTrait,
    AR: ActivityResultTrait + Serialize,
{
    pub fn new(admin: Player<P>, password: Option<String>) -> Self {
        Self::new_with_id(LobbyId::new_v4(), admin, password)
    }

    pub fn join(&mut self, player: Player<P>, password: Option<String>) -> Result<(), String> {
        if let Some(ref lobby_password) = self.password {
            if password != Some(lobby_password.to_string()) {
                return Err("Invalid password".to_string());
            }
        }
        self.add_participant(player);
        Ok(())
    }

    pub fn update_player_id(&mut self, player_id: &Uuid) {
        if let Some(player) = self
            .participants
            .iter_mut()
            .find(|p| p.id == self.player_id)
        {
            player.id = *player_id;
        }

        self.player_id = *player_id;
    }

    pub fn add_participant(&mut self, participant: Player<P>) {
        if self.participants.iter().any(|p| p.id == participant.id) {
            return;
        }

        self.participants.push(participant);
    }

    pub fn add_activity(&mut self, activity: Activity<A>) {
        self.catalog.add_activity(activity);
    }

    pub fn get_admin(&self) -> Option<&Player<P>> {
        self.participants
            .iter()
            .find(|player| player.role == Role::Admin)
    }

    pub fn is_admin(&self) -> bool {
        self.get_admin()
            .map_or(false, |admin| admin.id == self.player_id)
    }

    pub fn get_participants(&self) -> &Vec<Player<P>> {
        &self.participants
    }

    pub fn get_activities(&self) -> &Vec<Activity<A>> {
        &self.activities
    }

    pub fn select_activity(&mut self, activity_id: &ActivityId) -> Option<&Activity<A>> {
        if self.activities.iter().any(|a| a.id.eq(activity_id)) {
            return self.activities.iter().find(|a| a.id.eq(activity_id));
        }

        if let Some(activity) = self.catalog.get_activity(activity_id) {
            let activity = activity.clone();
            self.activities.push(activity);
            Some(self.activities.last().unwrap())
        } else {
            None
        }
    }

    pub fn remove_participant(&mut self, participant_id: Uuid) -> Option<Player<P>> {
        if let Some(pos) = self
            .participants
            .iter()
            .position(|p| p.id == participant_id)
        {
            Some(self.participants.remove(pos))
        } else {
            None
        }
    }

    pub fn start_activity(&mut self, activity_id: &str) -> Option<&Activity<A>> {
        self.results.clear();
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            activity.status = ActivityStatus::InProgress;
            Some(activity)
        } else {
            None
        }
    }

    pub fn complete_activity(&mut self, activity_id: &str) -> Option<&Activity<A>> {
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            activity.status = ActivityStatus::Done;
            Some(activity)
        } else {
            None
        }
    }

    pub fn update_activity_info(
        &mut self,
        activity_id: &ActivityId,
        _data: A,
    ) -> Option<&Activity<A>> {
        self.select_activity(activity_id)
    }

    pub fn add_activity_result(&mut self, result: ActivityResult<AR>) {
        self.results.push(result);
    }

    pub fn update_activity_status(
        &mut self,
        activity_id: &str,
        status: ActivityStatus,
    ) -> Option<&Activity<A>> {
        if let Some(activity) = self.activities.iter_mut().find(|a| a.id == activity_id) {
            if status == ActivityStatus::NotStarted {
                self.results.clear();
            }
            activity.status = status;
            Some(activity)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Identifiable, Named, Player, PlayerTrait, Role, Scorable, Timable};

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

    impl PlayerTrait for PlayerProfile {}

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

    impl ActivityTrait for Challenge {}

    #[derive(PartialEq, Clone, Serialize)]
    struct ChallengeResult {}

    impl Identifiable for ChallengeResult {
        fn identifier(&self) -> &str {
            "result"
        }
    }

    impl Timable for ChallengeResult {}

    impl Scorable for ChallengeResult {}

    impl ActivityResultTrait for ChallengeResult {}

    #[test]
    fn create_lobby() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let lobby: Lobby<PlayerProfile, Challenge, ChallengeResult> = Lobby::new(admin, None);

        assert_eq!(lobby.get_admin().unwrap().role, Role::Admin);
        assert_eq!(lobby.get_admin().unwrap().data.identifier(), "123");
        assert_eq!(lobby.get_admin().unwrap().data.name(), "Test Admin");
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

        let mut lobby: Lobby<PlayerProfile, Challenge, ChallengeResult> = Lobby::new(admin, None);

        let participant = Player::new(
            Role::Player,
            PlayerProfile {
                id: "456".to_string(),
                name: "Test Participant".to_string(),
            },
        );

        lobby.add_participant(participant);

        assert_eq!(lobby.participants.len(), 2);
        assert_eq!(lobby.participants[1].role, Role::Player);
        assert_eq!(lobby.participants[1].data.identifier(), "456");
        assert_eq!(lobby.participants[1].data.name(), "Test Participant");
    }

    #[test]
    fn test_select_activity() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge, ChallengeResult> = Lobby::new(admin, None);

        let activity = Activity {
            id: "789".to_string(),
            status: ActivityStatus::NotStarted,
            data: Challenge {
                id: "789".to_string(),
                name: "Test Challenge".to_string(),
            },
        };

        // Add activity to catalog
        lobby.add_activity(activity.clone());

        // First selection should work
        assert!(lobby.select_activity(&activity.id).is_some());
        assert_eq!(lobby.activities.len(), 1);

        // Second selection should not add duplicate
        assert!(lobby.select_activity(&activity.id).is_some());
        assert_eq!(lobby.activities.len(), 1);

        // Selecting non-existent activity should fail
        assert!(lobby.select_activity(&"nonexistent".to_string()).is_none());
        assert_eq!(lobby.activities.len(), 1);
    }

    #[test]
    fn test_activity_workflow() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge, ChallengeResult> = Lobby::new(admin, None);

        let activity = Activity {
            id: "789".to_string(),
            status: ActivityStatus::NotStarted,
            data: Challenge {
                id: "789".to_string(),
                name: "Test Challenge".to_string(),
            },
        };

        // Add to catalog and select
        lobby.add_activity(activity.clone());
        lobby.select_activity(&activity.id);

        // Start activity
        let started = lobby.start_activity(&activity.id);
        assert!(started.is_some());
        assert_eq!(started.unwrap().status, ActivityStatus::InProgress);

        // Complete activity
        let completed = lobby.complete_activity(&activity.id);
        assert!(completed.is_some());
        assert_eq!(completed.unwrap().status, ActivityStatus::Done);
    }

    #[test]
    fn test_update_player_id() {
        let admin = Player::new(
            Role::Admin,
            PlayerProfile {
                id: "123".to_string(),
                name: "Test Admin".to_string(),
            },
        );

        let mut lobby: Lobby<PlayerProfile, Challenge, ChallengeResult> = Lobby::new(admin, None);

        let new_id = Uuid::new_v4();
        lobby.update_player_id(&new_id);

        assert_eq!(lobby.player_id, new_id);
        assert_eq!(lobby.get_admin().unwrap().id, new_id);
    }
}
