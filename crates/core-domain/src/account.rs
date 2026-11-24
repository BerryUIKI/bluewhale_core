//! Asset accounts and custom account groups.

use std::collections::BTreeSet;
use std::fmt;

use jiff::Timestamp;

use crate::currency::Currency;
use crate::ids::{AccountGroupId, AccountId, CounterpartyId, TagId};
use crate::naming::{canonical_key, normalize_name};

/// The kind of asset account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AssetAccountType {
    /// Physical cash.
    Cash,
    /// A checking or current account.
    Checking,
    /// A savings account.
    Savings,
    /// Stored value such as a transit or gift card.
    StoredValue,
}

/// Errors raised while validating an account.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountValidationError {
    /// The name is empty after trimming.
    EmptyName,
}

impl fmt::Display for AccountValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty account name"),
        }
    }
}

impl std::error::Error for AccountValidationError {}

/// Optional account fields with sensible defaults.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountOptions {
    /// Manual sort position among accounts.
    pub sort_order: i32,
    /// Whether the account is archived.
    pub is_archived: bool,
    /// Tags associated with the account.
    pub tag_ids: BTreeSet<TagId>,
    /// The household member who owns the account.
    pub owner_id: Option<CounterpartyId>,
    /// The custom group that contains the account.
    pub custom_group_id: Option<AccountGroupId>,
    /// A free-form note.
    pub note: String,
    /// Whether the account is hidden from the main list.
    pub is_hidden: bool,
    /// Whether the account contributes to net worth.
    pub is_included_in_net_worth: bool,
}

impl Default for AccountOptions {
    fn default() -> Self {
        Self {
            sort_order: 0,
            is_archived: false,
            tag_ids: BTreeSet::new(),
            owner_id: None,
            custom_group_id: None,
            note: String::new(),
            is_hidden: false,
            is_included_in_net_worth: true,
        }
    }
}

/// An asset account denominated in one currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Account {
    id: AccountId,
    name: String,
    asset_type: AssetAccountType,
    currency: Currency,
    options: AccountOptions,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Account {
    /// Creates a validated account.
    ///
    /// # Errors
    ///
    /// Returns [`AccountValidationError::EmptyName`] when the name is empty
    /// after trimming.
    pub fn new(
        id: AccountId,
        name: &str,
        asset_type: AssetAccountType,
        currency: Currency,
        mut options: AccountOptions,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, AccountValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(AccountValidationError::EmptyName);
        }
        options.note = normalize_name(&options.note);
        Ok(Self {
            id,
            name: normalized_name,
            asset_type,
            currency,
            options,
            created_at,
            updated_at,
        })
    }

    /// Returns the account identifier.
    #[must_use]
    pub fn id(&self) -> AccountId {
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

    /// Returns the asset type.
    #[must_use]
    pub fn asset_type(&self) -> AssetAccountType {
        self.asset_type
    }

    /// Returns the account currency.
    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Returns the optional fields.
    #[must_use]
    pub fn options(&self) -> &AccountOptions {
        &self.options
    }

    /// Returns whether the account is archived.
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

/// Errors raised while validating an account group.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccountGroupValidationError {
    /// The name is empty after trimming.
    EmptyName,
}

impl fmt::Display for AccountGroupValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyName => formatter.write_str("empty account group name"),
        }
    }
}

impl std::error::Error for AccountGroupValidationError {}

/// A user-defined group of accounts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountGroup {
    id: AccountGroupId,
    name: String,
    sort_order: i32,
    is_archived: bool,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl AccountGroup {
    /// Creates a validated account group.
    ///
    /// # Errors
    ///
    /// Returns [`AccountGroupValidationError::EmptyName`] when the name is
    /// empty after trimming.
    pub fn new(
        id: AccountGroupId,
        name: &str,
        sort_order: i32,
        is_archived: bool,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, AccountGroupValidationError> {
        let normalized_name = normalize_name(name);
        if normalized_name.is_empty() {
            return Err(AccountGroupValidationError::EmptyName);
        }
        Ok(Self {
            id,
            name: normalized_name,
            sort_order,
            is_archived,
            created_at,
            updated_at,
        })
    }

    /// Returns the group identifier.
    #[must_use]
    pub fn id(&self) -> AccountGroupId {
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
    use crate::currency::CurrencyScalePolicy;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    #[test]
    fn account_trims_name_and_defaults() {
        let account = Account::new(
            AccountId::from_uuid(Uuid::from_u128(1)),
            "  Cash  ",
            AssetAccountType::Cash,
            usd(),
            AccountOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();

        assert_eq!(account.name(), "Cash");
        assert_eq!(account.canonical_name_key(), "cash");
        assert!(account.options().is_included_in_net_worth);
        assert!(!account.is_archived());
    }

    #[test]
    fn account_rejects_empty_name() {
        let result = Account::new(
            AccountId::from_uuid(Uuid::from_u128(1)),
            " \n ",
            AssetAccountType::Cash,
            usd(),
            AccountOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        );
        assert_eq!(result.unwrap_err(), AccountValidationError::EmptyName);
    }

    #[test]
    fn account_group_trims_name_and_rejects_empty() {
        let group = AccountGroup::new(
            AccountGroupId::from_uuid(Uuid::from_u128(2)),
            "  Bank accounts ",
            1,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        assert_eq!(group.name(), "Bank accounts");

        let error = AccountGroup::new(
            AccountGroupId::from_uuid(Uuid::from_u128(2)),
            "  ",
            0,
            false,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap_err();
        assert_eq!(error, AccountGroupValidationError::EmptyName);
    }
}
