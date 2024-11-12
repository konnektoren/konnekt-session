use crate::model::{Identifiable, PlayerId};

use super::{ActivityId, Scorable, Timable};

pub trait ActivityResultTrait: Identifiable + Timable + Scorable + Clone + PartialEq {}

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct ActivityResult<T>
where
    T: ActivityResultTrait,
{
    pub activity_id: ActivityId,
    pub player_id: PlayerId,
    pub data: T,
}

impl<T> ActivityResult<T>
where
    T: ActivityResultTrait,
{
    pub fn new(activity_id: ActivityId, player_id: PlayerId, data: T) -> Self {
        ActivityResult {
            activity_id,
            player_id,
            data,
        }
    }
}
