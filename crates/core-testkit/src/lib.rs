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
    LedgerExportRepository, LedgerExportSnapshot, LedgerProfile, LedgerProfileRepository,
    LedgerReadSnapshot, LedgerReadSnapshotRepository, LedgerSettings, LedgerSettingsRepository,
    ReconciliationFlag, ReconciliationFlagId, ReconciliationFlagRepository, RepositoryError, TagId,
    TagRepository, Transaction, TransactionId, TransactionNotePreset, TransactionNotePresetId,
    TransactionNotePresetRepository, TransactionRepository,
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

impl LedgerExportRepository for InMemoryLedger {
    fn export_snapshot(&self) -> Result<LedgerExportSnapshot, RepositoryError> {
        let inner = self.inner.borrow();
        let settings = inner
            .settings
            .clone()
            .ok_or_else(|| repository_error("ledger is not initialized"))?;
        let profile = inner
            .profile
            .clone()
            .ok_or_else(|| repository_error("ledger profile is not initialized"))?;
        Ok(LedgerExportSnapshot {
            settings,
            profile,
            account_groups: inner.account_groups.values().cloned().collect(),
            accounts: inner.accounts.values().cloned().collect(),
            category_groups: inner.category_groups.values().cloned().collect(),
            categories: inner.categories.values().cloned().collect(),
            tags: inner.tags.values().cloned().collect(),
            counterparties: inner.counterparties.values().cloned().collect(),
            reconciliation_flags: inner.reconciliation_flags.values().cloned().collect(),
            note_presets: inner.note_presets.values().cloned().collect(),
            transactions: inner.transactions.values().cloned().collect(),
            exchange_rates: inner.exchange_rates.values().cloned().collect(),
        })
    }
}

/// A convenience alias used by tests that build accounts with defaults.
#[must_use]
pub fn default_account_options() -> AccountOptions {
    AccountOptions::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        AccountGroup, AccountOptions, AssetAccountType, CategoryGroup, CategoryKind,
        CategoryOptions, Currency, CurrencyScalePolicy, ExactRatio, ExchangeRateId,
        ExchangeRateSource, LedgerCalendar, LedgerProfile, LedgerProfileId, LedgerSettings,
        LedgerSettingsId, LedgerTag, Money, ReconciliationFlagOptions,
        ReconciliationFlagSystemKind, TagOptions, TransactionFields, TransactionKind,
        TransactionNotePreset,
    };
    use jiff::Timestamp;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    fn settings() -> LedgerSettings {
        LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(1)),
            usd(),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    fn profile() -> LedgerProfile {
        LedgerProfile::new(
            LedgerProfileId::from_uuid(Uuid::from_u128(2)),
            settings().id(),
            "Home",
            "🏠",
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn account(archived: bool) -> Account {
        Account::new(
            AccountId::from_uuid(Uuid::from_u128(10)),
            "Cash",
            AssetAccountType::Cash,
            usd(),
            AccountOptions {
                is_archived: archived,
                ..AccountOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn account_group(archived: bool) -> AccountGroup {
        AccountGroup::new(
            AccountGroupId::from_uuid(Uuid::from_u128(20)),
            "Wallets",
            0,
            archived,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn category(archived: bool) -> Category {
        Category::new(
            CategoryId::from_uuid(Uuid::from_u128(30)),
            "Food",
            CategoryKind::Expense,
            CategoryOptions {
                is_archived: archived,
                ..CategoryOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn category_group(archived: bool) -> CategoryGroup {
        CategoryGroup::new(
            CategoryGroupId::from_uuid(Uuid::from_u128(31)),
            "Living",
            CategoryKind::Expense,
            0,
            archived,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn tag(archived: bool) -> LedgerTag {
        LedgerTag::new(
            TagId::from_uuid(Uuid::from_u128(40)),
            "Travel",
            "#112233",
            TagOptions {
                is_archived: archived,
                ..TagOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn counterparty(archived: bool) -> Counterparty {
        Counterparty::new(
            CounterpartyId::from_uuid(Uuid::from_u128(50)),
            core_domain::CounterpartyKind::Organization,
            "Grocer",
            0,
            archived,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn flag(archived: bool) -> ReconciliationFlag {
        ReconciliationFlag::new(
            ReconciliationFlagId::from_uuid(Uuid::from_u128(60)),
            "Cleared",
            "#00AA55",
            ReconciliationFlagOptions {
                system_kind: Some(ReconciliationFlagSystemKind::Cleared),
                is_archived: archived,
                ..ReconciliationFlagOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn preset(archived: bool) -> TransactionNotePreset {
        TransactionNotePreset::new(
            TransactionNotePresetId::from_uuid(Uuid::from_u128(70)),
            "Weekly shop",
            0,
            archived,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn transaction() -> Transaction {
        Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(80)),
            TransactionKind::Expense,
            Money::new(1_000, usd()),
            timestamp(2_000),
            account(false).id(),
            TransactionFields {
                category_id: Some(category(false).id()),
                ..TransactionFields::default()
            },
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap()
    }

    fn exchange_rate(id: u128, superseded: bool) -> ExchangeRate {
        ExchangeRate::new(
            ExchangeRateId::from_uuid(Uuid::from_u128(id)),
            CurrencyScalePolicy::v1().currency("CNY").unwrap(),
            usd(),
            ExactRatio::new(3, 20).unwrap(),
            timestamp(1_000),
            ExchangeRateSource::Manual,
            superseded.then(|| timestamp(2_000)),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    #[test]
    fn settings_and_profile_round_trip() {
        let ledger = InMemoryLedger::new();
        assert_eq!(ledger.settings().unwrap(), None);
        assert_eq!(ledger.profile().unwrap(), None);

        ledger.save_settings(&settings()).unwrap();
        ledger.save_profile(&profile()).unwrap();

        assert_eq!(ledger.settings().unwrap(), Some(settings()));
        assert_eq!(ledger.profile().unwrap(), Some(profile()));
    }

    #[test]
    fn master_data_round_trips_and_filters_archived_rows() {
        let ledger = InMemoryLedger::new();
        ledger.save_account(&account(false)).unwrap();
        ledger.save_account_group(&account_group(false)).unwrap();
        ledger.save_category(&category(false)).unwrap();
        ledger.save_category_group(&category_group(false)).unwrap();
        ledger.save_tag(&tag(false)).unwrap();
        ledger.save_counterparty(&counterparty(false)).unwrap();
        ledger.save_reconciliation_flag(&flag(false)).unwrap();
        ledger.save_note_preset(&preset(false)).unwrap();

        assert_eq!(
            ledger.account(account(false).id()).unwrap(),
            Some(account(false))
        );
        assert_eq!(
            ledger.account_group(account_group(false).id()).unwrap(),
            Some(account_group(false))
        );
        assert_eq!(
            ledger.category(category(false).id()).unwrap(),
            Some(category(false))
        );
        assert_eq!(
            ledger.category_group(category_group(false).id()).unwrap(),
            Some(category_group(false))
        );
        assert_eq!(ledger.tag(tag(false).id()).unwrap(), Some(tag(false)));
        assert_eq!(
            ledger.counterparty(counterparty(false).id()).unwrap(),
            Some(counterparty(false))
        );
        assert_eq!(
            ledger.reconciliation_flag(flag(false).id()).unwrap(),
            Some(flag(false))
        );
        assert_eq!(
            ledger.note_preset(preset(false).id()).unwrap(),
            Some(preset(false))
        );
        assert_eq!(ledger.accounts(false).unwrap(), vec![account(false)]);
        assert_eq!(
            ledger.account_groups(false).unwrap(),
            vec![account_group(false)]
        );
        assert_eq!(ledger.categories(false).unwrap(), vec![category(false)]);
        assert_eq!(
            ledger.category_groups(false).unwrap(),
            vec![category_group(false)]
        );
        assert_eq!(ledger.tags(false).unwrap(), vec![tag(false)]);
        assert_eq!(
            ledger.counterparties(false).unwrap(),
            vec![counterparty(false)]
        );
        assert_eq!(
            ledger.reconciliation_flags(false).unwrap(),
            vec![flag(false)]
        );
        assert_eq!(ledger.note_presets(false).unwrap(), vec![preset(false)]);

        ledger.save_account(&account(true)).unwrap();
        ledger.save_account_group(&account_group(true)).unwrap();
        ledger.save_category(&category(true)).unwrap();
        ledger.save_category_group(&category_group(true)).unwrap();
        ledger.save_tag(&tag(true)).unwrap();
        ledger.save_counterparty(&counterparty(true)).unwrap();
        ledger.save_reconciliation_flag(&flag(true)).unwrap();
        ledger.save_note_preset(&preset(true)).unwrap();

        assert!(ledger.accounts(false).unwrap().is_empty());
        assert!(ledger.account_groups(false).unwrap().is_empty());
        assert!(ledger.categories(false).unwrap().is_empty());
        assert!(ledger.category_groups(false).unwrap().is_empty());
        assert!(ledger.tags(false).unwrap().is_empty());
        assert!(ledger.counterparties(false).unwrap().is_empty());
        assert!(ledger.reconciliation_flags(false).unwrap().is_empty());
        assert!(ledger.note_presets(false).unwrap().is_empty());
        assert_eq!(ledger.accounts(true).unwrap(), vec![account(true)]);
        assert_eq!(
            ledger.account_groups(true).unwrap(),
            vec![account_group(true)]
        );
        assert_eq!(ledger.categories(true).unwrap(), vec![category(true)]);
        assert_eq!(
            ledger.category_groups(true).unwrap(),
            vec![category_group(true)]
        );
        assert_eq!(ledger.tags(true).unwrap(), vec![tag(true)]);
        assert_eq!(
            ledger.counterparties(true).unwrap(),
            vec![counterparty(true)]
        );
        assert_eq!(ledger.reconciliation_flags(true).unwrap(), vec![flag(true)]);
        assert_eq!(ledger.note_presets(true).unwrap(), vec![preset(true)]);
    }

    #[test]
    fn transactions_and_exchange_rates_filter_their_tombstones() {
        let ledger = InMemoryLedger::new();
        ledger.save_transaction(&transaction()).unwrap();
        ledger
            .save_transaction(&transaction().soft_deleted(timestamp(3_000)).unwrap())
            .unwrap();
        assert_eq!(
            ledger.transaction(transaction().id()).unwrap(),
            Some(transaction().soft_deleted(timestamp(3_000)).unwrap())
        );
        assert_eq!(ledger.transactions(false).unwrap(), Vec::new());
        assert_eq!(
            ledger.transactions(true).unwrap(),
            vec![transaction().soft_deleted(timestamp(3_000)).unwrap()]
        );

        ledger
            .save_exchange_rate(&exchange_rate(90, false))
            .unwrap();
        ledger.save_exchange_rate(&exchange_rate(91, true)).unwrap();
        assert_eq!(
            ledger.exchange_rates(false).unwrap(),
            vec![exchange_rate(90, false)]
        );
        assert_eq!(
            ledger.exchange_rates(true).unwrap(),
            vec![exchange_rate(90, false), exchange_rate(91, true)]
        );
    }

    #[test]
    fn initialize_is_idempotent_and_account_setup_requires_settings() {
        let ledger = InMemoryLedger::new();
        assert!(
            ledger
                .create_account_with_opening_balance(&account(false), Some(&transaction()))
                .is_err()
        );

        let initialized = ledger
            .initialize(
                &settings(),
                &profile(),
                &account(false),
                &[category(false)],
                Some(&transaction()),
            )
            .unwrap();
        assert!(initialized);
        let duplicate = ledger
            .initialize(&settings(), &profile(), &account(false), &[], None)
            .unwrap();
        assert!(!duplicate);
        assert_eq!(ledger.accounts(true).unwrap(), vec![account(false)]);
        assert_eq!(ledger.categories(true).unwrap(), vec![category(false)]);
        assert_eq!(ledger.transactions(true).unwrap(), vec![transaction()]);

        assert!(
            ledger
                .create_account_with_opening_balance(&account(false), Some(&transaction()))
                .is_ok()
        );
    }

    #[test]
    fn snapshots_report_missing_settings_and_profile() {
        let ledger = InMemoryLedger::new();
        assert!(ledger.snapshot().is_err());
        assert!(ledger.export_snapshot().is_err());

        ledger.save_settings(&settings()).unwrap();
        assert!(ledger.export_snapshot().is_err());
        ledger.save_profile(&profile()).unwrap();

        let snapshot = ledger.snapshot().unwrap();
        assert_eq!(snapshot.settings, settings());
        assert!(snapshot.active_accounts.is_empty());
        assert!(snapshot.active_transactions.is_empty());
        assert!(snapshot.exchange_rates.is_empty());

        let export = ledger.export_snapshot().unwrap();
        assert_eq!(export.settings, settings());
        assert_eq!(export.profile, profile());
    }
}
