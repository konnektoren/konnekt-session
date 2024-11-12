use crate::model::{Identifiable, PlayerId};

pub trait ActivityResultData: Identifiable + Clone + PartialEq {}

pub struct ActivityResult<T>
where
    T: ActivityResultData,
{
    pub id: String,
    pub player_id: PlayerId,
    pub data: T,
}

impl<T> ActivityResult<T>
where
    T: ActivityResultData,
{
    pub fn new(player_id: PlayerId, data: T) -> Self {
        ActivityResult {
            id: data.identifier().to_string(),
            player_id,
            data,
        }
    }
}
