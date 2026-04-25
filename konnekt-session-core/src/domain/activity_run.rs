use crate::domain::{ActivityConfig, ActivityResult};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

pub type ActivityRunId = Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStatus {
    InProgress,
    Completed,
    Cancelled,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum ActivityRunError {
    #[error("Participant not in required submitters: {0}")]
    NotARequiredSubmitter(Uuid),

    #[error("Participant already submitted: {0}")]
    DuplicateSubmission(Uuid),

    #[error("Run is not in progress")]
    NotInProgress,
}

/// Aggregate root for one game in progress.
///
/// `required_submitters` is snapshotted at creation — never grows.
/// Completes when all required submitters have submitted or been removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActivityRun {
    id: ActivityRunId,
    lobby_id: Uuid,
    config: ActivityConfig,
    required_submitters: HashSet<Uuid>,
    results: HashMap<Uuid, ActivityResult>,
    status: RunStatus,
}

impl ActivityRun {
    pub fn new(
        id: ActivityRunId,
        lobby_id: Uuid,
        config: ActivityConfig,
        active_participants: HashSet<Uuid>,
    ) -> Self {
        Self {
            id,
            lobby_id,
            config,
            required_submitters: active_participants,
            results: HashMap::new(),
            status: RunStatus::InProgress,
        }
    }

    pub fn id(&self) -> ActivityRunId {
        self.id
    }

    pub fn lobby_id(&self) -> Uuid {
        self.lobby_id
    }

    pub fn config(&self) -> &ActivityConfig {
        &self.config
    }

    pub fn status(&self) -> RunStatus {
        self.status
    }

    pub fn results(&self) -> &HashMap<Uuid, ActivityResult> {
        &self.results
    }

    pub fn required_submitters(&self) -> &HashSet<Uuid> {
        &self.required_submitters
    }

    pub fn is_complete(&self) -> bool {
        self.status == RunStatus::Completed
    }

    /// Submit a result. Returns true if this submission completed the run.
    pub fn submit_result(&mut self, result: ActivityResult) -> Result<bool, ActivityRunError> {
        if self.status != RunStatus::InProgress {
            return Err(ActivityRunError::NotInProgress);
        }

        let participant_id = result.participant_id;

        if !self.required_submitters.contains(&participant_id) {
            return Err(ActivityRunError::NotARequiredSubmitter(participant_id));
        }

        if self.results.contains_key(&participant_id) {
            return Err(ActivityRunError::DuplicateSubmission(participant_id));
        }

        self.results.insert(participant_id, result);

        if self.all_submitted() {
            self.status = RunStatus::Completed;
            return Ok(true);
        }

        Ok(false)
    }

    /// Remove a participant from required submitters (on disconnect).
    /// Returns true if this removal completed the run.
    pub fn remove_submitter(&mut self, participant_id: Uuid) -> Result<bool, ActivityRunError> {
        if self.status != RunStatus::InProgress {
            return Err(ActivityRunError::NotInProgress);
        }

        self.required_submitters.remove(&participant_id);

        if self.required_submitters.is_empty() {
            self.status = RunStatus::Cancelled;
            return Ok(true);
        }

        if self.all_submitted() {
            self.status = RunStatus::Completed;
            return Ok(true);
        }

        Ok(false)
    }

    pub fn cancel(&mut self) -> Result<(), ActivityRunError> {
        if self.status != RunStatus::InProgress {
            return Err(ActivityRunError::NotInProgress);
        }
        self.status = RunStatus::Cancelled;
        Ok(())
    }

    fn all_submitted(&self) -> bool {
        self.required_submitters
            .iter()
            .all(|id| self.results.contains_key(id))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ActivityConfig;

    fn make_run(participants: Vec<Uuid>) -> ActivityRun {
        let config = ActivityConfig::new(
            "quiz".to_string(),
            "Test Quiz".to_string(),
            serde_json::json!({}),
        );
        ActivityRun::new(
            Uuid::new_v4(),
            Uuid::new_v4(),
            config,
            participants.into_iter().collect(),
        )
    }

    #[test]
    fn test_submit_completes_when_all_submitted() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let mut run = make_run(vec![p1, p2]);

        let completed = run
            .submit_result(ActivityResult::new(Uuid::new_v4(), p1))
            .unwrap();
        assert!(!completed);
        assert_eq!(run.status(), RunStatus::InProgress);

        let completed = run
            .submit_result(ActivityResult::new(Uuid::new_v4(), p2))
            .unwrap();
        assert!(completed);
        assert_eq!(run.status(), RunStatus::Completed);
    }

    #[test]
    fn test_remove_submitter_completes_run() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let mut run = make_run(vec![p1, p2]);

        run.submit_result(ActivityResult::new(Uuid::new_v4(), p1))
            .unwrap();

        // p2 disconnects — run should complete
        let completed = run.remove_submitter(p2).unwrap();
        assert!(completed);
        assert_eq!(run.status(), RunStatus::Completed);
    }

    #[test]
    fn test_all_disconnect_cancels_run() {
        let p1 = Uuid::new_v4();
        let mut run = make_run(vec![p1]);

        let completed = run.remove_submitter(p1).unwrap();
        assert!(completed);
        assert_eq!(run.status(), RunStatus::Cancelled);
    }

    #[test]
    fn test_duplicate_submission_rejected() {
        let p1 = Uuid::new_v4();
        let p2 = Uuid::new_v4();
        let mut run = make_run(vec![p1, p2]);

        run.submit_result(ActivityResult::new(Uuid::new_v4(), p1))
            .unwrap();

        // p1 tries to submit again — should be rejected before p2 submits
        let err = run
            .submit_result(ActivityResult::new(Uuid::new_v4(), p1))
            .unwrap_err();
        assert_eq!(err, ActivityRunError::DuplicateSubmission(p1));
    }

    #[test]
    fn test_non_submitter_rejected() {
        let p1 = Uuid::new_v4();
        let outsider = Uuid::new_v4();
        let mut run = make_run(vec![p1]);

        let err = run
            .submit_result(ActivityResult::new(Uuid::new_v4(), outsider))
            .unwrap_err();
        assert_eq!(err, ActivityRunError::NotARequiredSubmitter(outsider));
    }

    #[test]
    fn test_snapshot_not_affected_by_late_joiners() {
        // Snapshot taken at creation — late joiner cannot submit
        let p1 = Uuid::new_v4();
        let late_joiner = Uuid::new_v4();
        let mut run = make_run(vec![p1]);

        let err = run
            .submit_result(ActivityResult::new(Uuid::new_v4(), late_joiner))
            .unwrap_err();
        assert_eq!(err, ActivityRunError::NotARequiredSubmitter(late_joiner));
    }
}
