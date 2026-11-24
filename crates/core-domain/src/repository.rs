//! Repository ports implemented by persistence adapters and test fakes.
//!
//! The ports are storage-neutral: they speak in domain entities and never
//! expose SQLite or transaction types.

use std::fmt;

use crate::account::{Account, AccountGroup};
use crate::category::{Category, CategoryGroup};
use crate::counterparty::Counterparty;
use crate::exchange_rate::ExchangeRate;
use crate::ids::{
    AccountGroupId, AccountId, CategoryGroupId, CategoryId, CounterpartyId, TransactionId,
};
use crate::note_preset::TransactionNotePreset;
use crate::profile::LedgerProfile;
use crate::reconciliation::ReconciliationFlag;
use crate::settings::LedgerSettings;
use crate::tag::LedgerTag;
use crate::transaction::Transaction;

/// An opaque repository or backend failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositoryError {
    message: String,
}

impl RepositoryError {
    /// Creates a repository error with a human-readable message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for RepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for RepositoryError {}

/// Reads and writes ledger settings.
pub trait LedgerSettingsRepository {
    /// Returns the settings, when the ledger is initialized.
    fn settings(&self) -> Result<Option<LedgerSettings>, RepositoryError>;

    /// Saves the settings.
    fn save_settings(&self, settings: &LedgerSettings) -> Result<(), RepositoryError>;
}

/// Reads and writes the ledger profile.
pub trait LedgerProfileRepository {
    /// Returns the profile, when the ledger is initialized.
    fn profile(&self) -> Result<Option<LedgerProfile>, RepositoryError>;

    /// Saves the profile.
    fn save_profile(&self, profile: &LedgerProfile) -> Result<(), RepositoryError>;
}

/// Reads and writes accounts.
pub trait AccountRepository {
    /// Returns one account.
    fn account(&self, id: AccountId) -> Result<Option<Account>, RepositoryError>;

    /// Returns accounts in sort order.
    fn accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError>;

    /// Saves an account.
    fn save_account(&self, account: &Account) -> Result<(), RepositoryError>;
}

/// Reads and writes custom account groups.
pub trait AccountGroupRepository {
    /// Returns one group.
    fn account_group(&self, id: AccountGroupId) -> Result<Option<AccountGroup>, RepositoryError>;

    /// Returns groups in sort order.
    fn account_groups(&self, include_archived: bool) -> Result<Vec<AccountGroup>, RepositoryError>;

    /// Saves a group.
    fn save_account_group(&self, group: &AccountGroup) -> Result<(), RepositoryError>;
}

/// Reads and writes leaf categories.
pub trait CategoryRepository {
    /// Returns one category.
    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError>;

    /// Returns categories in sort order.
    fn categories(&self, include_archived: bool) -> Result<Vec<Category>, RepositoryError>;

    /// Saves a category.
    fn save_category(&self, category: &Category) -> Result<(), RepositoryError>;
}

/// Reads and writes category groups.
pub trait CategoryGroupRepository {
    /// Returns one group.
    fn category_group(&self, id: CategoryGroupId)
    -> Result<Option<CategoryGroup>, RepositoryError>;

    /// Returns groups in sort order.
    fn category_groups(
        &self,
        include_archived: bool,
    ) -> Result<Vec<CategoryGroup>, RepositoryError>;

    /// Saves a group.
    fn save_category_group(&self, group: &CategoryGroup) -> Result<(), RepositoryError>;
}

/// Reads and writes tags.
pub trait TagRepository {
    /// Returns one tag.
    fn tag(&self, id: crate::ids::TagId) -> Result<Option<LedgerTag>, RepositoryError>;

    /// Returns tags in sort order.
    fn tags(&self, include_archived: bool) -> Result<Vec<LedgerTag>, RepositoryError>;

    /// Saves a tag.
    fn save_tag(&self, tag: &LedgerTag) -> Result<(), RepositoryError>;
}

/// Reads and writes counterparties.
pub trait CounterpartyRepository {
    /// Returns one counterparty.
    fn counterparty(&self, id: CounterpartyId) -> Result<Option<Counterparty>, RepositoryError>;

    /// Returns counterparties in sort order.
    fn counterparties(&self, include_archived: bool) -> Result<Vec<Counterparty>, RepositoryError>;

    /// Saves a counterparty.
    fn save_counterparty(&self, counterparty: &Counterparty) -> Result<(), RepositoryError>;
}

/// Reads and writes reconciliation flags.
pub trait ReconciliationFlagRepository {
    /// Returns one flag.
    fn reconciliation_flag(
        &self,
        id: crate::ids::ReconciliationFlagId,
    ) -> Result<Option<ReconciliationFlag>, RepositoryError>;

    /// Returns flags in sort order.
    fn reconciliation_flags(
        &self,
        include_archived: bool,
    ) -> Result<Vec<ReconciliationFlag>, RepositoryError>;

    /// Saves a flag.
    fn save_reconciliation_flag(&self, flag: &ReconciliationFlag) -> Result<(), RepositoryError>;
}

/// Reads and writes note presets.
pub trait TransactionNotePresetRepository {
    /// Returns one preset.
    fn note_preset(
        &self,
        id: crate::ids::TransactionNotePresetId,
    ) -> Result<Option<TransactionNotePreset>, RepositoryError>;

    /// Returns presets in sort order.
    fn note_presets(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TransactionNotePreset>, RepositoryError>;

    /// Saves a preset.
    fn save_note_preset(&self, preset: &TransactionNotePreset) -> Result<(), RepositoryError>;
}

/// Reads and writes transactions.
pub trait TransactionRepository {
    /// Returns one transaction.
    fn transaction(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError>;

    /// Returns transactions ordered by occurrence.
    fn transactions(&self, include_deleted: bool) -> Result<Vec<Transaction>, RepositoryError>;

    /// Saves a transaction.
    fn save_transaction(&self, transaction: &Transaction) -> Result<(), RepositoryError>;
}

/// Reads and writes exchange-rate revisions.
pub trait ExchangeRateRepository {
    /// Returns rates, optionally including superseded revisions.
    fn exchange_rates(
        &self,
        include_superseded: bool,
    ) -> Result<Vec<ExchangeRate>, RepositoryError>;

    /// Saves a rate revision.
    fn save_exchange_rate(&self, rate: &ExchangeRate) -> Result<(), RepositoryError>;
}

/// Atomically creates the initial ledger content.
pub trait LedgerBootstrapRepository:
    LedgerSettingsRepository
    + LedgerProfileRepository
    + AccountRepository
    + CategoryRepository
    + TransactionRepository
{
    /// Creates the settings, profile, default account, default categories, and
    /// optional opening balance as one unit.
    ///
    /// Returns `true` when this call initialized the ledger and `false` when
    /// another writer initialized it first.
    fn initialize(
        &self,
        settings: &LedgerSettings,
        profile: &LedgerProfile,
        default_account: &Account,
        default_categories: &[Category],
        opening_balance: Option<&Transaction>,
    ) -> Result<bool, RepositoryError>;
}

/// Atomically creates an account with its opening balance.
pub trait AccountSetupRepository: AccountRepository + TransactionRepository {
    /// Saves the account and the optional opening balance as one unit.
    fn create_account_with_opening_balance(
        &self,
        account: &Account,
        opening_balance: Option<&Transaction>,
    ) -> Result<(), RepositoryError>;
}

/// A read-only snapshot used by dashboard and report use cases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerReadSnapshot {
    /// The ledger settings.
    pub settings: LedgerSettings,
    /// Active accounts.
    pub active_accounts: Vec<Account>,
    /// Active (not soft-deleted) transactions.
    pub active_transactions: Vec<Transaction>,
    /// Exchange-rate revisions.
    pub exchange_rates: Vec<ExchangeRate>,
}

/// Reads a consistent snapshot of the ledger.
pub trait LedgerReadSnapshotRepository {
    /// Returns the snapshot.
    fn snapshot(&self) -> Result<LedgerReadSnapshot, RepositoryError>;
}
