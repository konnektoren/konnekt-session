use super::{ActivityId, Scorable, Timable};
use crate::model::{Identifiable, PlayerId};
use serde::Serialize;

pub trait ActivityResultTrait: Identifiable + Timable + Scorable + Clone + PartialEq {}

#[derive(Debug, Clone, PartialEq, Hash, Serialize)]
pub struct ActivityResult<T>
where
    T: ActivityResultTrait + Serialize,
{
    pub activity_id: ActivityId,
    pub player_id: PlayerId,
    pub data: T,
}

impl<T> ActivityResult<T>
where
    T: ActivityResultTrait + Serialize,
{
    pub fn new(activity_id: ActivityId, player_id: PlayerId, data: T) -> Self {
        ActivityResult {
            activity_id,
            player_id,
            data,
        }
    }
}
