use std::fmt;

use serde::{Deserialize, Serialize};

use crate::error::{RatingError, Result};

/// Which rating a match counts for.
///
/// `game` is a stable identifier chosen by the consuming app
/// (e.g. `"music.freestyle-duel.v1"`). `variant` optionally splits a game into
/// separate ratings (e.g. `"piano.right-hand"`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RatingKey {
    game: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    variant: Option<String>,
}

impl RatingKey {
    pub fn new(game: impl Into<String>) -> Result<Self> {
        let game = game.into();
        if game.trim().is_empty() {
            return Err(RatingError::EmptyGame);
        }
        Ok(Self {
            game,
            variant: None,
        })
    }

    /// Returns the key with a variant; an empty variant means no variant.
    pub fn with_variant(mut self, variant: impl Into<String>) -> Self {
        let variant = variant.into();
        self.variant = (!variant.trim().is_empty()).then_some(variant);
        self
    }

    pub fn game(&self) -> &str {
        &self.game
    }

    pub fn variant(&self) -> Option<&str> {
        self.variant.as_deref()
    }
}

impl fmt::Display for RatingKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.variant {
            Some(variant) => write!(f, "{}/{}", self.game, variant),
            None => f.write_str(&self.game),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_empty_game() {
        assert_eq!(RatingKey::new("  "), Err(RatingError::EmptyGame));
    }

    #[test]
    fn displays_game_and_variant() {
        let key = RatingKey::new("music.freestyle-duel.v1").unwrap();
        assert_eq!(key.to_string(), "music.freestyle-duel.v1");
        let key = key.with_variant("piano.right-hand");
        assert_eq!(key.to_string(), "music.freestyle-duel.v1/piano.right-hand");
        assert_eq!(key.variant(), Some("piano.right-hand"));
    }

    #[test]
    fn empty_variant_is_none() {
        let key = RatingKey::new("game").unwrap().with_variant("");
        assert_eq!(key.variant(), None);
    }

    #[test]
    fn serde_roundtrip_omits_missing_variant() {
        let key = RatingKey::new("game").unwrap();
        let json = serde_json::to_string(&key).unwrap();
        assert_eq!(json, r#"{"game":"game"}"#);
        assert_eq!(serde_json::from_str::<RatingKey>(&json).unwrap(), key);
    }
}
