//! Ledger profile: user-facing name and emoji.

use std::fmt;

use jiff::Timestamp;
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;

use crate::ids::{LedgerProfileId, LedgerSettingsId};

/// The maximum ledger-name length in grapheme clusters.
pub const MAXIMUM_NAME_CHARACTERS: usize = 60;

/// The maximum number of Unicode scalars in a profile emoji.
pub const MAXIMUM_EMOJI_SCALARS: usize = 16;

/// Errors raised while validating a ledger profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerProfileValidationError {
    /// The name is empty after trimming.
    EmptyName,
    /// The name is longer than [`MAXIMUM_NAME_CHARACTERS`] graphemes.
    NameTooLong {
        /// The maximum number of grapheme clusters.
        maximum_characters: usize,
    },
    /// The name contains a newline or control character.
    InvalidNameCharacters,
    /// The emoji is not exactly one emoji grapheme.
    InvalidEmoji,
    /// The update instant precedes the creation instant.
    UpdatedBeforeCreation,
}

impl fmt::Display for LedgerProfileValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty ledger name"),
            Self::NameTooLong { maximum_characters } => write!(
                formatter,
                "ledger name exceeds {maximum_characters} characters"
            ),
            Self::InvalidNameCharacters => formatter.write_str("invalid characters in ledger name"),
            Self::InvalidEmoji => formatter.write_str("invalid ledger emoji"),
            Self::UpdatedBeforeCreation => formatter.write_str("update instant precedes creation"),
        }
    }
}

impl std::error::Error for LedgerProfileValidationError {}

/// The user-facing identity of one ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerProfile {
    id: LedgerProfileId,
    settings_id: LedgerSettingsId,
    name: String,
    emoji: String,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl LedgerProfile {
    /// Creates a validated ledger profile.
    ///
    /// # Errors
    ///
    /// Returns a [`LedgerProfileValidationError`] when the name or emoji is
    /// invalid, or when the update instant precedes creation.
    pub fn new(
        id: LedgerProfileId,
        settings_id: LedgerSettingsId,
        name: &str,
        emoji: &str,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, LedgerProfileValidationError> {
        let normalized_name = normalize(name);
        if normalized_name.is_empty() {
            return Err(LedgerProfileValidationError::EmptyName);
        }
        if normalized_name.graphemes(true).count() > MAXIMUM_NAME_CHARACTERS {
            return Err(LedgerProfileValidationError::NameTooLong {
                maximum_characters: MAXIMUM_NAME_CHARACTERS,
            });
        }
        if normalized_name.chars().any(is_forbidden_name_character) {
            return Err(LedgerProfileValidationError::InvalidNameCharacters);
        }

        let normalized_emoji = normalize(emoji);
        validate_emoji(&normalized_emoji)?;

        if updated_at < created_at {
            return Err(LedgerProfileValidationError::UpdatedBeforeCreation);
        }

        Ok(Self {
            id,
            settings_id,
            name: normalized_name,
            emoji: normalized_emoji,
            created_at,
            updated_at,
        })
    }

    /// Returns the profile identifier.
    #[must_use]
    pub fn id(&self) -> LedgerProfileId {
        self.id
    }

    /// Returns the associated settings identifier.
    #[must_use]
    pub fn settings_id(&self) -> LedgerSettingsId {
        self.settings_id
    }

    /// Returns the normalized name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the normalized emoji.
    #[must_use]
    pub fn emoji(&self) -> &str {
        &self.emoji
    }

    /// Returns the creation instant.
    #[must_use]
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the last update instant.
    #[must_use]
    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// Returns a profile with a new name and emoji.
    ///
    /// # Errors
    ///
    /// Returns a [`LedgerProfileValidationError`] when the new values are
    /// invalid or the update instant precedes creation.
    pub fn updating(
        &self,
        name: &str,
        emoji: &str,
        at: Timestamp,
    ) -> Result<Self, LedgerProfileValidationError> {
        Self::new(self.id, self.settings_id, name, emoji, self.created_at, at)
    }
}

fn normalize(value: &str) -> String {
    value.trim().nfc().collect()
}

fn is_forbidden_name_character(character: char) -> bool {
    character.is_control() || matches!(character, '\u{2028}' | '\u{2029}')
}

fn validate_emoji(emoji: &str) -> Result<(), LedgerProfileValidationError> {
    let graphemes = emoji.graphemes(true).count();
    let scalars = emoji.chars().count();
    if graphemes != 1
        || scalars == 0
        || scalars > MAXIMUM_EMOJI_SCALARS
        || emoji.chars().any(char::is_whitespace)
        || !is_emoji_like(emoji)
    {
        return Err(LedgerProfileValidationError::InvalidEmoji);
    }
    Ok(())
}

/// Approximates the platform's emoji-presentation check without a Unicode
/// property table: a scalar in a common emoji range, or a keycap sequence with
/// a digit base and the variation selector.
fn is_emoji_like(value: &str) -> bool {
    let mut has_emoji_presentation = false;
    let mut has_variation_selector = false;
    let mut has_keycap = false;
    for character in value.chars() {
        let scalar = u32::from(character);
        if EMOJI_RANGES
            .iter()
            .any(|(start, end)| (*start..=*end).contains(&scalar))
        {
            has_emoji_presentation = true;
        }
        if character == '\u{FE0F}' {
            has_variation_selector = true;
        }
        if character == '\u{20E3}' {
            has_keycap = true;
        }
    }
    if has_emoji_presentation {
        return true;
    }
    has_variation_selector
        && has_keycap
        && value
            .chars()
            .next()
            .is_some_and(|character| character.is_ascii_digit())
}

const EMOJI_RANGES: &[(u32, u32)] = &[(0x1F000, 0x1FAFF), (0x2600, 0x27BF), (0x2B00, 0x2BFF)];

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn settings_id() -> LedgerSettingsId {
        LedgerSettingsId::from_uuid(Uuid::from_u128(1))
    }

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn profile(name: &str, emoji: &str) -> Result<LedgerProfile, LedgerProfileValidationError> {
        LedgerProfile::new(
            LedgerProfileId::from_uuid(Uuid::from_u128(2)),
            settings_id(),
            name,
            emoji,
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    #[test]
    fn profile_normalizes_name_and_accepts_composed_emoji() {
        let family = profile("  Family Ledger  ", "👨‍👩‍👧‍👦").unwrap();
        let flag = profile("Travel", "🇸🇬").unwrap();
        let keycap = profile("Numbered Ledger", "1️⃣").unwrap();

        assert_eq!(family.name(), "Family Ledger");
        assert_eq!(family.emoji(), "👨‍👩‍👧‍👦");
        assert_eq!(flag.emoji(), "🇸🇬");
        assert_eq!(keycap.emoji(), "1️⃣");
    }

    #[test]
    fn profile_rejects_invalid_names() {
        assert_eq!(
            profile(" \n ", "🍓").unwrap_err(),
            LedgerProfileValidationError::EmptyName
        );
        assert_eq!(
            profile(&"a".repeat(61), "🍓").unwrap_err(),
            LedgerProfileValidationError::NameTooLong {
                maximum_characters: 60
            }
        );
        assert_eq!(
            profile("Private\u{0000}Ledger", "🍓").unwrap_err(),
            LedgerProfileValidationError::InvalidNameCharacters
        );
    }

    #[test]
    fn profile_rejects_plain_text_and_multiple_emoji() {
        for invalid in ["", "A", "🍓🍒", "  "] {
            assert_eq!(
                profile("Ledger", invalid).unwrap_err(),
                LedgerProfileValidationError::InvalidEmoji
            );
        }
    }

    #[test]
    fn updating_preserves_identity_and_creation() {
        let original = profile("Ledger", "🍓").unwrap();
        let updated = original
            .updating("Household", "🏠", timestamp(2_000))
            .unwrap();

        assert_eq!(updated.id(), original.id());
        assert_eq!(updated.settings_id(), original.settings_id());
        assert_eq!(updated.created_at(), original.created_at());
        assert_eq!(updated.updated_at(), timestamp(2_000));
        assert_eq!(updated.name(), "Household");
        assert_eq!(updated.emoji(), "🏠");
    }

    #[test]
    fn profile_rejects_update_before_creation() {
        let result = LedgerProfile::new(
            LedgerProfileId::from_uuid(Uuid::from_u128(3)),
            settings_id(),
            "Ledger",
            "🍓",
            timestamp(1_000),
            timestamp(999),
        );
        assert_eq!(
            result.unwrap_err(),
            LedgerProfileValidationError::UpdatedBeforeCreation
        );
    }
}
