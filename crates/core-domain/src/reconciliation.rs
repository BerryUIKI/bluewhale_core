//! Reconciliation flags that can lock or annotate transactions.

use std::fmt;

use jiff::Timestamp;

use crate::ids::ReconciliationFlagId;
use crate::naming::{canonical_key, normalize_color, normalize_name, normalize_shortcut};

/// The built-in flag kinds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReconciliationFlagSystemKind {
    /// The transaction is cleared.
    Cleared,
    /// The transaction needs reimbursement.
    NeedsReimbursement,
    /// The transaction has an issue.
    Issue,
}

/// Errors raised while validating a reconciliation flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconciliationFlagValidationError {
    /// The name is empty after trimming.
    EmptyName,
    /// The color is not a `#RRGGBB` value.
    InvalidColorHex,
    /// The shortcut is not a single ASCII letter.
    InvalidShortcut,
}

impl fmt::Display for ReconciliationFlagValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty reconciliation flag name"),
            Self::InvalidColorHex => formatter.write_str("invalid reconciliation flag color"),
            Self::InvalidShortcut => formatter.write_str("invalid reconciliation flag shortcut"),
        }
    }
}

impl std::error::Error for ReconciliationFlagValidationError {}

/// Optional reconciliation flag fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReconciliationFlagOptions {
    /// The built-in kind, when this is a system flag.
    pub system_kind: Option<ReconciliationFlagSystemKind>,
    /// Whether the flag prevents editing attached transactions.
    pub locks_transactions: bool,
    /// A single-letter keyboard shortcut.
    pub shortcut_key: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the flag is archived.
    pub is_archived: bool,
}

/// A flag used to reconcile transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconciliationFlag {
    id: ReconciliationFlagId,
    name: String,
    color_hex: String,
    options: ReconciliationFlagOptions,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl ReconciliationFlag {
    /// Creates a validated reconciliation flag.
    ///
    /// # Errors
    ///
    /// Returns a [`ReconciliationFlagValidationError`] for an empty name, a
    /// malformed color, or an invalid shortcut.
    pub fn new(
        id: ReconciliationFlagId,
        name: &str,
        color_hex: &str,
        mut options: ReconciliationFlagOptions,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, ReconciliationFlagValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(ReconciliationFlagValidationError::EmptyName);
        }
        let normalized_color =
            normalize_color(color_hex).ok_or(ReconciliationFlagValidationError::InvalidColorHex)?;
        options.shortcut_key = normalize_shortcut(options.shortcut_key.as_deref())
            .map_err(|()| ReconciliationFlagValidationError::InvalidShortcut)?;
        Ok(Self {
            id,
            name: normalized_name,
            color_hex: normalized_color,
            options,
            created_at,
            updated_at,
        })
    }

    /// Returns the flag identifier.
    #[must_use]
    pub fn id(&self) -> ReconciliationFlagId {
        self.id
    }

    /// Returns the display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the case-folded key used for uniqueness checks.
    #[must_use]
    pub fn canonical_name_key(&self) -> String {
        canonical_key(&self.name)
    }

    /// Returns the normalized `#RRGGBB` color.
    #[must_use]
    pub fn color_hex(&self) -> &str {
        &self.color_hex
    }

    /// Returns the optional fields.
    #[must_use]
    pub fn options(&self) -> &ReconciliationFlagOptions {
        &self.options
    }

    /// Returns whether the flag locks attached transactions.
    #[must_use]
    pub fn locks_transactions(&self) -> bool {
        self.options.locks_transactions
    }

    /// Returns whether the flag is archived.
    #[must_use]
    pub fn is_archived(&self) -> bool {
        self.options.is_archived
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

    fn flag(
        name: &str,
        color: &str,
        options: ReconciliationFlagOptions,
    ) -> Result<ReconciliationFlag, ReconciliationFlagValidationError> {
        ReconciliationFlag::new(
            ReconciliationFlagId::from_uuid(Uuid::from_u128(1)),
            name,
            color,
            options,
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    #[test]
    fn flag_normalizes_shortcut_and_color() {
        let flag = flag(
            "Cleared",
            "#00ff00",
            ReconciliationFlagOptions {
                shortcut_key: Some(" c ".to_string()),
                ..ReconciliationFlagOptions::default()
            },
        )
        .unwrap();

        assert_eq!(flag.color_hex(), "#00FF00");
        assert_eq!(flag.options().shortcut_key.as_deref(), Some("C"));
    }

    #[test]
    fn flag_rejects_invalid_values() {
        assert_eq!(
            flag(" ", "#00FF00", ReconciliationFlagOptions::default()).unwrap_err(),
            ReconciliationFlagValidationError::EmptyName
        );
        assert_eq!(
            flag("Cleared", "00FF00", ReconciliationFlagOptions::default()).unwrap_err(),
            ReconciliationFlagValidationError::InvalidColorHex
        );
        for invalid in ["AB", "1", "-"] {
            assert_eq!(
                flag(
                    "Cleared",
                    "#00FF00",
                    ReconciliationFlagOptions {
                        shortcut_key: Some(invalid.to_string()),
                        ..ReconciliationFlagOptions::default()
                    }
                )
                .unwrap_err(),
                ReconciliationFlagValidationError::InvalidShortcut
            );
        }
    }
}
