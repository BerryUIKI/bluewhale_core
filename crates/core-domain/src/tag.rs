//! Tags with a display color and optional description.

use std::fmt;

use jiff::Timestamp;

use crate::ids::TagId;
use crate::naming::{canonical_key, normalize_color, normalize_name, normalize_optional_text};

/// Errors raised while validating a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagValidationError {
    /// The name is empty after trimming.
    EmptyName,
    /// The color is not a `#RRGGBB` value.
    InvalidColorHex,
}

impl fmt::Display for TagValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty tag name"),
            Self::InvalidColorHex => formatter.write_str("invalid tag color"),
        }
    }
}

impl std::error::Error for TagValidationError {}

/// Optional tag fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TagOptions {
    /// An optional free-form description.
    pub description: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the tag is archived.
    pub is_archived: bool,
}

/// A tag that can be attached to accounts and transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerTag {
    id: TagId,
    name: String,
    color_hex: String,
    options: TagOptions,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl LedgerTag {
    /// Creates a validated tag.
    ///
    /// # Errors
    ///
    /// Returns [`TagValidationError::EmptyName`] for an empty name and
    /// [`TagValidationError::InvalidColorHex`] for a malformed color.
    pub fn new(
        id: TagId,
        name: &str,
        color_hex: &str,
        mut options: TagOptions,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, TagValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(TagValidationError::EmptyName);
        }
        let normalized_color =
            normalize_color(color_hex).ok_or(TagValidationError::InvalidColorHex)?;
        options.description = normalize_optional_text(options.description.as_deref());
        Ok(Self {
            id,
            name: normalized_name,
            color_hex: normalized_color,
            options,
            created_at,
            updated_at,
        })
    }

    /// Returns the tag identifier.
    #[must_use]
    pub fn id(&self) -> TagId {
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
    pub fn options(&self) -> &TagOptions {
        &self.options
    }

    /// Returns whether the tag is archived.
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

    fn tag(name: &str, color: &str) -> Result<LedgerTag, TagValidationError> {
        LedgerTag::new(
            TagId::from_uuid(Uuid::from_u128(1)),
            name,
            color,
            TagOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    #[test]
    fn tag_normalizes_name_and_color() {
        let tag = tag("  Travel ", "#a1b2c3").unwrap();
        assert_eq!(tag.name(), "Travel");
        assert_eq!(tag.color_hex(), "#A1B2C3");
    }

    #[test]
    fn tag_rejects_empty_name_and_invalid_color() {
        assert_eq!(
            tag(" ", "#A1B2C3").unwrap_err(),
            TagValidationError::EmptyName
        );
        for invalid in ["A1B2C3", "#A1B2C", "#A1B2C3D", "#G1B2C3", ""] {
            assert_eq!(
                tag("Travel", invalid).unwrap_err(),
                TagValidationError::InvalidColorHex
            );
        }
    }

    #[test]
    fn tag_description_is_trimmed_and_empty_becomes_none() {
        let with_description = LedgerTag::new(
            TagId::from_uuid(Uuid::from_u128(1)),
            "Travel",
            "#A1B2C3",
            TagOptions {
                description: Some("  Trips  ".to_string()),
                ..TagOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(
            with_description.options().description.as_deref(),
            Some("Trips")
        );

        let without_description = LedgerTag::new(
            TagId::from_uuid(Uuid::from_u128(1)),
            "Travel",
            "#A1B2C3",
            TagOptions {
                description: Some("   ".to_string()),
                ..TagOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(without_description.options().description, None);
    }
}
