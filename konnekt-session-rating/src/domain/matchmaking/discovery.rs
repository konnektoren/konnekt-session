use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::super::RatingKey;

/// Discovery information advertised again whenever the search window widens.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hello {
    pub version: u16,
    pub search: Uuid,
    pub rating_key: RatingKey,
    pub rating: f64,
    pub window: f64,
}

impl Hello {
    /// Both searches must accept the actual rating gap.
    pub fn accepts(&self, other: &Self) -> bool {
        self.search != other.search
            && self.version == other.version
            && self.rating_key == other.rating_key
            && self.rating.is_finite()
            && other.rating.is_finite()
            && self.window.is_finite()
            && other.window.is_finite()
            && self.window >= 0.0
            && other.window >= 0.0
            && (self.rating - other.rating).abs() <= self.window.min(other.window)
    }
}

/// Suggested search window in rating points, widened every five seconds.
pub fn search_window(elapsed_seconds: u64) -> u32 {
    100 + (elapsed_seconds / 5).min(6) as u32 * 50
}
