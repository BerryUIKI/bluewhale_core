//! Reusable transaction note presets.

use std::fmt;

use jiff::Timestamp;

use crate::ids::TransactionNotePresetId;
use crate::naming::{canonical_key, normalize_name};

/// Errors raised while validating a note preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionNotePresetValidationError {
    /// The text is empty after trimming.
    EmptyText,
}

impl fmt::Display for TransactionNotePresetValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyText => formatter.write_str("empty note preset"),
        }
    }
}

impl std::error::Error for TransactionNotePresetValidationError {}

/// A short note that can be attached to transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionNotePreset {
    id: TransactionNotePresetId,
    text: String,
    sort_order: i32,
    is_archived: bool,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl TransactionNotePreset {
    /// Creates a validated note preset.
    ///
    /// # Errors
    ///
    /// Returns [`TransactionNotePresetValidationError::EmptyText`] when the
    /// text is empty after trimming.
    pub fn new(
        id: TransactionNotePresetId,
        text: &str,
        sort_order: i32,
        is_archived: bool,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, TransactionNotePresetValidationError> {
        let normalized_text = normalize_name(text);
        if normalized_text.is_empty() {
            return Err(TransactionNotePresetValidationError::EmptyText);
        }
        Ok(Self {
            id,
            text: normalized_text,
            sort_order,
            is_archived,
            created_at,
            updated_at,
        })
    }

    /// Returns the preset identifier.
    #[must_use]
    pub fn id(&self) -> TransactionNotePresetId {
        self.id
    }

    /// Returns the note text.
    #[must_use]
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Returns the case-folded key used for uniqueness checks.
    #[must_use]
    pub fn canonical_text_key(&self) -> String {
        canonical_key(&self.text)
    }

    /// Returns the manual sort position.
    #[must_use]
    pub fn sort_order(&self) -> i32 {
        self.sort_order
    }

    /// Returns whether the preset is archived.
    #[must_use]
    pub fn is_archived(&self) -> bool {
        self.is_archived
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    #[test]
    fn preset_trims_text_and_rejects_empty() {
        let preset = TransactionNotePreset::new(
            TransactionNotePresetId::from_uuid(Uuid::from_u128(1)),
            "  Groceries  ",
            0,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(preset.text(), "Groceries");
        assert_eq!(preset.canonical_text_key(), "groceries");

        let error = TransactionNotePreset::new(
            TransactionNotePresetId::from_uuid(Uuid::from_u128(1)),
            " \n ",
            0,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap_err();
        assert_eq!(error, TransactionNotePresetValidationError::EmptyText);
    }
}
