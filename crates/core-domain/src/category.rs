//! Category groups and leaf categories.

use std::fmt;

use jiff::Timestamp;

use crate::ids::{CategoryGroupId, CategoryId};
use crate::naming::{canonical_key, normalize_name};

/// Whether a category classifies spending or income.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CategoryKind {
    /// Spending categories.
    Expense,
    /// Income categories.
    Income,
}

/// Errors raised while validating a category.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryValidationError {
    /// The name is empty after trimming.
    EmptyName,
}

impl fmt::Display for CategoryValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty category name"),
        }
    }
}

impl std::error::Error for CategoryValidationError {}

/// Optional category fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct CategoryOptions {
    /// The containing category group, when grouped.
    pub group_id: Option<CategoryGroupId>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the category is archived.
    pub is_archived: bool,
}

/// A leaf category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Category {
    id: CategoryId,
    name: String,
    kind: CategoryKind,
    options: CategoryOptions,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Category {
    /// Creates a validated category.
    ///
    /// # Errors
    ///
    /// Returns [`CategoryValidationError::EmptyName`] when the name is empty
    /// after trimming.
    pub fn new(
        id: CategoryId,
        name: &str,
        kind: CategoryKind,
        options: CategoryOptions,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, CategoryValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(CategoryValidationError::EmptyName);
        }
        Ok(Self {
            id,
            name: normalized_name,
            kind,
            options,
            created_at,
            updated_at,
        })
    }

    /// Returns the category identifier.
    #[must_use]
    pub fn id(&self) -> CategoryId {
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

    /// Returns the category kind.
    #[must_use]
    pub fn kind(&self) -> CategoryKind {
        self.kind
    }

    /// Returns the optional fields.
    #[must_use]
    pub fn options(&self) -> CategoryOptions {
        self.options
    }

    /// Returns whether the category is archived.
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

/// Errors raised while validating a category group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CategoryGroupValidationError {
    /// The name is empty after trimming.
    EmptyName,
}

impl fmt::Display for CategoryGroupValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty category group name"),
        }
    }
}

impl std::error::Error for CategoryGroupValidationError {}

/// A two-level category group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryGroup {
    id: CategoryGroupId,
    name: String,
    kind: CategoryKind,
    sort_order: i32,
    is_archived: bool,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl CategoryGroup {
    /// Creates a validated category group.
    ///
    /// # Errors
    ///
    /// Returns [`CategoryGroupValidationError::EmptyName`] when the name is
    /// empty after trimming.
    pub fn new(
        id: CategoryGroupId,
        name: &str,
        kind: CategoryKind,
        sort_order: i32,
        is_archived: bool,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, CategoryGroupValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(CategoryGroupValidationError::EmptyName);
        }
        Ok(Self {
            id,
            name: normalized_name,
            kind,
            sort_order,
            is_archived,
            created_at,
            updated_at,
        })
    }

    /// Returns the group identifier.
    #[must_use]
    pub fn id(&self) -> CategoryGroupId {
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

    /// Returns the category kind.
    #[must_use]
    pub fn kind(&self) -> CategoryKind {
        self.kind
    }

    /// Returns the manual sort position.
    #[must_use]
    pub fn sort_order(&self) -> i32 {
        self.sort_order
    }

    /// Returns whether the group is archived.
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
    fn category_trims_name_and_rejects_empty() {
        let category = Category::new(
            CategoryId::from_uuid(Uuid::from_u128(1)),
            "  Food  ",
            CategoryKind::Expense,
            CategoryOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(category.name(), "Food");
        assert_eq!(category.kind(), CategoryKind::Expense);

        let error = Category::new(
            CategoryId::from_uuid(Uuid::from_u128(1)),
            "\t",
            CategoryKind::Expense,
            CategoryOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap_err();
        assert_eq!(error, CategoryValidationError::EmptyName);
    }

    #[test]
    fn category_group_carries_kind_and_sort_order() {
        let group = CategoryGroup::new(
            CategoryGroupId::from_uuid(Uuid::from_u128(2)),
            "Essentials",
            CategoryKind::Expense,
            3,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(group.name(), "Essentials");
        assert_eq!(group.kind(), CategoryKind::Expense);
        assert_eq!(group.sort_order(), 3);
    }
}
