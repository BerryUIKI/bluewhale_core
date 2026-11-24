//! Test support for the Bluewhale core crates.
//!
//! In-memory repository implementations for application use-case tests. This
//! crate is never a dependency of a production crate.

use std::cell::RefCell;
use std::collections::BTreeMap;

use core_domain::{
    Account, AccountGroup, AccountGroupId, AccountGroupRepository, AccountId, AccountOptions,
    AccountRepository, AccountSetupRepository, Category, CategoryGroup, CategoryGroupId,
    CategoryGroupRepository, CategoryId, CategoryRepository, Counterparty, CounterpartyId,
    CounterpartyRepository, ExchangeRate, ExchangeRateRepository, LedgerBootstrapRepository,
    LedgerProfile, LedgerProfileRepository, LedgerReadSnapshot, LedgerReadSnapshotRepository,
    LedgerSettings, LedgerSettingsRepository, ReconciliationFlag, ReconciliationFlagId,
    ReconciliationFlagRepository, RepositoryError, TagId, TagRepository, Transaction,
    TransactionId, TransactionNotePreset, TransactionNotePresetId, TransactionNotePresetRepository,
    TransactionRepository,
};

/// An in-memory ledger implementing every repository port.
#[derive(Debug, Default)]
pub struct InMemoryLedger {
    inner: RefCell<Inner>,
}

#[derive(Debug, Default)]
struct Inner {
    settings: Option<LedgerSettings>,
    profile: Option<LedgerProfile>,
    accounts: BTreeMap<AccountId, Account>,
    account_groups: BTreeMap<AccountGroupId, AccountGroup>,
    categories: BTreeMap<CategoryId, Category>,
    category_groups: BTreeMap<CategoryGroupId, CategoryGroup>,
    tags: BTreeMap<TagId, core_domain::LedgerTag>,
    counterparties: BTreeMap<CounterpartyId, Counterparty>,
    reconciliation_flags: BTreeMap<ReconciliationFlagId, ReconciliationFlag>,
    note_presets: BTreeMap<TransactionNotePresetId, TransactionNotePreset>,
    transactions: BTreeMap<TransactionId, Transaction>,
    exchange_rates: BTreeMap<core_domain::ExchangeRateId, ExchangeRate>,
}

impl InMemoryLedger {
    /// Creates an empty ledger.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Seeds the settings directly, for tests that do not exercise setup.
    pub fn seed_settings(&self, settings: LedgerSettings) {
        self.inner.borrow_mut().settings = Some(settings);
    }

    /// Seeds the profile directly.
    pub fn seed_profile(&self, profile: LedgerProfile) {
        self.inner.borrow_mut().profile = Some(profile);
    }
}

fn repository_error(message: impl Into<String>) -> RepositoryError {
    RepositoryError::new(message)
}

impl LedgerSettingsRepository for InMemoryLedger {
    fn settings(&self) -> Result<Option<LedgerSettings>, RepositoryError> {
        Ok(self.inner.borrow().settings.clone())
    }

    fn save_settings(&self, settings: &LedgerSettings) -> Result<(), RepositoryError> {
        self.inner.borrow_mut().settings = Some(settings.clone());
        Ok(())
    }
}

impl LedgerProfileRepository for InMemoryLedger {
    fn profile(&self) -> Result<Option<LedgerProfile>, RepositoryError> {
        Ok(self.inner.borrow().profile.clone())
    }

    fn save_profile(&self, profile: &LedgerProfile) -> Result<(), RepositoryError> {
        self.inner.borrow_mut().profile = Some(profile.clone());
        Ok(())
    }
}

impl AccountRepository for InMemoryLedger {
    fn account(&self, id: AccountId) -> Result<Option<Account>, RepositoryError> {
        Ok(self.inner.borrow().accounts.get(&id).cloned())
    }

    fn accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut accounts: Vec<Account> = inner
            .accounts
            .values()
            .filter(|account| include_archived || !account.is_archived())
            .cloned()
            .collect();
        accounts.sort_by(|left, right| {
            left.options()
                .sort_order
                .cmp(&right.options().sort_order)
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(accounts)
    }

    fn save_account(&self, account: &Account) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .accounts
            .insert(account.id(), account.clone());
        Ok(())
    }
}

impl AccountGroupRepository for InMemoryLedger {
    fn account_group(&self, id: AccountGroupId) -> Result<Option<AccountGroup>, RepositoryError> {
        Ok(self.inner.borrow().account_groups.get(&id).cloned())
    }

    fn account_groups(&self, include_archived: bool) -> Result<Vec<AccountGroup>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut groups: Vec<AccountGroup> = inner
            .account_groups
            .values()
            .filter(|group| include_archived || !group.is_archived())
            .cloned()
            .collect();
        groups.sort_by(|left, right| {
            left.sort_order()
                .cmp(&right.sort_order())
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(groups)
    }

    fn save_account_group(&self, group: &AccountGroup) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .account_groups
            .insert(group.id(), group.clone());
        Ok(())
    }
}

impl CategoryRepository for InMemoryLedger {
    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError> {
        Ok(self.inner.borrow().categories.get(&id).cloned())
    }

    fn categories(&self, include_archived: bool) -> Result<Vec<Category>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut categories: Vec<Category> = inner
            .categories
            .values()
            .filter(|category| include_archived || !category.is_archived())
            .cloned()
            .collect();
        categories.sort_by(|left, right| {
            left.options()
                .sort_order
                .cmp(&right.options().sort_order)
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(categories)
    }

    fn save_category(&self, category: &Category) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .categories
            .insert(category.id(), category.clone());
        Ok(())
    }
}

impl CategoryGroupRepository for InMemoryLedger {
    fn category_group(
        &self,
        id: CategoryGroupId,
    ) -> Result<Option<CategoryGroup>, RepositoryError> {
        Ok(self.inner.borrow().category_groups.get(&id).cloned())
    }

    fn category_groups(
        &self,
        include_archived: bool,
    ) -> Result<Vec<CategoryGroup>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut groups: Vec<CategoryGroup> = inner
            .category_groups
            .values()
            .filter(|group| include_archived || !group.is_archived())
            .cloned()
            .collect();
        groups.sort_by(|left, right| {
            left.sort_order()
                .cmp(&right.sort_order())
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(groups)
    }

    fn save_category_group(&self, group: &CategoryGroup) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .category_groups
            .insert(group.id(), group.clone());
        Ok(())
    }
}

impl TagRepository for InMemoryLedger {
    fn tag(&self, id: TagId) -> Result<Option<core_domain::LedgerTag>, RepositoryError> {
        Ok(self.inner.borrow().tags.get(&id).cloned())
    }

    fn tags(&self, include_archived: bool) -> Result<Vec<core_domain::LedgerTag>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut tags: Vec<core_domain::LedgerTag> = inner
            .tags
            .values()
            .filter(|tag| include_archived || !tag.is_archived())
            .cloned()
            .collect();
        tags.sort_by(|left, right| {
            left.options()
                .sort_order
                .cmp(&right.options().sort_order)
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(tags)
    }

    fn save_tag(&self, tag: &core_domain::LedgerTag) -> Result<(), RepositoryError> {
        self.inner.borrow_mut().tags.insert(tag.id(), tag.clone());
        Ok(())
    }
}

impl CounterpartyRepository for InMemoryLedger {
    fn counterparty(&self, id: CounterpartyId) -> Result<Option<Counterparty>, RepositoryError> {
        Ok(self.inner.borrow().counterparties.get(&id).cloned())
    }

    fn counterparties(&self, include_archived: bool) -> Result<Vec<Counterparty>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut counterparties: Vec<Counterparty> = inner
            .counterparties
            .values()
            .filter(|counterparty| include_archived || !counterparty.is_archived())
            .cloned()
            .collect();
        counterparties.sort_by(|left, right| {
            left.sort_order()
                .cmp(&right.sort_order())
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(counterparties)
    }

    fn save_counterparty(&self, counterparty: &Counterparty) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .counterparties
            .insert(counterparty.id(), counterparty.clone());
        Ok(())
    }
}

impl ReconciliationFlagRepository for InMemoryLedger {
    fn reconciliation_flag(
        &self,
        id: ReconciliationFlagId,
    ) -> Result<Option<ReconciliationFlag>, RepositoryError> {
        Ok(self.inner.borrow().reconciliation_flags.get(&id).cloned())
    }

    fn reconciliation_flags(
        &self,
        include_archived: bool,
    ) -> Result<Vec<ReconciliationFlag>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut flags: Vec<ReconciliationFlag> = inner
            .reconciliation_flags
            .values()
            .filter(|flag| include_archived || !flag.is_archived())
            .cloned()
            .collect();
        flags.sort_by(|left, right| {
            left.options()
                .sort_order
                .cmp(&right.options().sort_order)
                .then_with(|| left.name().cmp(right.name()))
        });
        Ok(flags)
    }

    fn save_reconciliation_flag(&self, flag: &ReconciliationFlag) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .reconciliation_flags
            .insert(flag.id(), flag.clone());
        Ok(())
    }
}

impl TransactionNotePresetRepository for InMemoryLedger {
    fn note_preset(
        &self,
        id: TransactionNotePresetId,
    ) -> Result<Option<TransactionNotePreset>, RepositoryError> {
        Ok(self.inner.borrow().note_presets.get(&id).cloned())
    }

    fn note_presets(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TransactionNotePreset>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut presets: Vec<TransactionNotePreset> = inner
            .note_presets
            .values()
            .filter(|preset| include_archived || !preset.is_archived())
            .cloned()
            .collect();
        presets.sort_by(|left, right| {
            left.sort_order()
                .cmp(&right.sort_order())
                .then_with(|| left.text().cmp(right.text()))
        });
        Ok(presets)
    }

    fn save_note_preset(&self, preset: &TransactionNotePreset) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .note_presets
            .insert(preset.id(), preset.clone());
        Ok(())
    }
}

impl TransactionRepository for InMemoryLedger {
    fn transaction(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError> {
        Ok(self.inner.borrow().transactions.get(&id).cloned())
    }

    fn transactions(&self, include_deleted: bool) -> Result<Vec<Transaction>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut transactions: Vec<Transaction> = inner
            .transactions
            .values()
            .filter(|transaction| include_deleted || !transaction.is_deleted())
            .cloned()
            .collect();
        transactions.sort_by(|left, right| {
            left.occurred_at()
                .cmp(&right.occurred_at())
                .then_with(|| left.id().to_string().cmp(&right.id().to_string()))
        });
        Ok(transactions)
    }

    fn save_transaction(&self, transaction: &Transaction) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .transactions
            .insert(transaction.id(), transaction.clone());
        Ok(())
    }
}

impl ExchangeRateRepository for InMemoryLedger {
    fn exchange_rates(
        &self,
        include_superseded: bool,
    ) -> Result<Vec<ExchangeRate>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut rates: Vec<ExchangeRate> = inner
            .exchange_rates
            .values()
            .filter(|rate| include_superseded || !rate.is_superseded())
            .cloned()
            .collect();
        rates.sort_by_key(|rate| rate.effective_at());
        Ok(rates)
    }

    fn save_exchange_rate(&self, rate: &ExchangeRate) -> Result<(), RepositoryError> {
        self.inner
            .borrow_mut()
            .exchange_rates
            .insert(rate.id(), rate.clone());
        Ok(())
    }
}

impl LedgerBootstrapRepository for InMemoryLedger {
    fn initialize(
        &self,
        settings: &LedgerSettings,
        profile: &LedgerProfile,
        default_account: &Account,
        default_categories: &[Category],
        opening_balance: Option<&Transaction>,
    ) -> Result<bool, RepositoryError> {
        let mut inner = self.inner.borrow_mut();
        if inner.settings.is_some() {
            return Ok(false);
        }
        inner.settings = Some(settings.clone());
        inner.profile = Some(profile.clone());
        inner
            .accounts
            .insert(default_account.id(), default_account.clone());
        for category in default_categories {
            inner.categories.insert(category.id(), category.clone());
        }
        if let Some(opening_balance) = opening_balance {
            inner
                .transactions
                .insert(opening_balance.id(), opening_balance.clone());
        }
        Ok(true)
    }
}

impl AccountSetupRepository for InMemoryLedger {
    fn create_account_with_opening_balance(
        &self,
        account: &Account,
        opening_balance: Option<&Transaction>,
    ) -> Result<(), RepositoryError> {
        if self.inner.borrow().settings.is_none() {
            return Err(repository_error("ledger is not initialized"));
        }
        let mut inner = self.inner.borrow_mut();
        inner.accounts.insert(account.id(), account.clone());
        if let Some(opening_balance) = opening_balance {
            inner
                .transactions
                .insert(opening_balance.id(), opening_balance.clone());
        }
        Ok(())
    }
}

impl LedgerReadSnapshotRepository for InMemoryLedger {
    fn snapshot(&self) -> Result<LedgerReadSnapshot, RepositoryError> {
        let inner = self.inner.borrow();
        let settings = inner
            .settings
            .clone()
            .ok_or_else(|| repository_error("ledger is not initialized"))?;
        let active_accounts = inner
            .accounts
            .values()
            .filter(|account| !account.is_archived())
            .cloned()
            .collect();
        let active_transactions = inner
            .transactions
            .values()
            .filter(|transaction| !transaction.is_deleted())
            .cloned()
            .collect();
        let exchange_rates = inner.exchange_rates.values().cloned().collect();
        Ok(LedgerReadSnapshot {
            settings,
            active_accounts,
            active_transactions,
            exchange_rates,
        })
    }
}

/// A convenience alias used by tests that build accounts with defaults.
#[must_use]
pub fn default_account_options() -> AccountOptions {
    AccountOptions::default()
}
