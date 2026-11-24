//! Counterparties and account owners.

use std::fmt;

use jiff::Timestamp;

use crate::ids::CounterpartyId;
use crate::naming::{canonical_key, normalize_name};

/// The kind of counterparty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CounterpartyKind {
    /// A member of the household.
    HouseholdMember,
    /// A person outside the household.
    Person,
    /// An organization.
    Organization,
}

/// Errors raised while validating a counterparty.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CounterpartyValidationError {
    /// The name is empty after trimming.
    EmptyName,
}

impl fmt::Display for CounterpartyValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty counterparty name"),
        }
    }
}

impl std::error::Error for CounterpartyValidationError {}

/// A person or organization that owns accounts or appears in transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Counterparty {
    id: CounterpartyId,
    kind: CounterpartyKind,
    name: String,
    sort_order: i32,
    is_archived: bool,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Counterparty {
    /// Creates a validated counterparty.
    ///
    /// # Errors
    ///
    /// Returns [`CounterpartyValidationError::EmptyName`] when the name is
    /// empty after trimming.
    pub fn new(
        id: CounterpartyId,
        kind: CounterpartyKind,
        name: &str,
        sort_order: i32,
        is_archived: bool,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, CounterpartyValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(CounterpartyValidationError::EmptyName);
        }
        Ok(Self {
            id,
            kind,
            name: normalized_name,
            sort_order,
            is_archived,
            created_at,
            updated_at,
        })
    }

    /// Returns the counterparty identifier.
    #[must_use]
    pub fn id(&self) -> CounterpartyId {
        self.id
    }

    /// Returns the counterparty kind.
    #[must_use]
    pub fn kind(&self) -> CounterpartyKind {
        self.kind
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

    /// Returns the manual sort position.
    #[must_use]
    pub fn sort_order(&self) -> i32 {
        self.sort_order
    }

    /// Returns whether the counterparty is archived.
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
    fn counterparty_normalizes_name_and_rejects_empty() {
        let counterparty = Counterparty::new(
            CounterpartyId::from_uuid(Uuid::from_u128(1)),
            CounterpartyKind::Person,
            "  Alex  ",
            0,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(counterparty.name(), "Alex");
        assert_eq!(counterparty.kind(), CounterpartyKind::Person);

        let error = Counterparty::new(
            CounterpartyId::from_uuid(Uuid::from_u128(1)),
            CounterpartyKind::Person,
            " ",
            0,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap_err();
        assert_eq!(error, CounterpartyValidationError::EmptyName);
    }
}
