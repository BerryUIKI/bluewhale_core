//! Test support for the Bluewhale core crates.
//!
//! In-memory repository implementations for application use-case tests. This
//! crate is never a dependency of a production crate.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

use core_domain::{
    Account, AccountGroup, AccountGroupId, AccountGroupRepository, AccountId, AccountOptions,
    AccountRepository, AccountSetupRepository, Category, CategoryGroup, CategoryGroupId,
    CategoryGroupRepository, CategoryId, CategoryRepository, Counterparty, CounterpartyId,
    CounterpartyRepository, ExchangeRate, ExchangeRateRepository, LedgerBootstrapRepository,
    LedgerExportRepository, LedgerExportSnapshot, LedgerProfile, LedgerProfileRepository,
    LedgerReadSnapshot, LedgerReadSnapshotRepository, LedgerSettings, LedgerSettingsRepository,
    ReconciliationFlag, ReconciliationFlagId, ReconciliationFlagRepository, RepositoryError,
    SyncConflict, SyncEntity, SyncEntityType, SyncOperation, TagId, TagRepository, Transaction,
    TransactionId, TransactionNotePreset, TransactionNotePresetId, TransactionNotePresetRepository,
    TransactionRepository,
};
use jiff::Timestamp;
use uuid::Uuid;

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
    sync_operations: BTreeMap<Uuid, SyncOperation>,
    applied_operations: BTreeSet<Uuid>,
    sync_revisions: BTreeMap<(SyncEntityType, Uuid), u64>,
    sync_server_seqs: BTreeMap<(SyncEntityType, Uuid), u64>,
    sync_cursor: u64,
    conflicts: BTreeMap<Uuid, SyncConflict>,
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
        let categories = inner.categories.values().cloned().collect();
        let category_groups = inner.category_groups.values().cloned().collect();
        Ok(LedgerReadSnapshot {
            settings,
            active_accounts,
            active_transactions,
            exchange_rates,
            categories,
            category_groups,
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

impl core_domain::SyncRepository for InMemoryLedger {
    fn cursor(&self) -> Result<u64, RepositoryError> {
        Ok(self.inner.borrow().sync_cursor)
    }

    fn save_cursor(&self, seq: u64) -> Result<(), RepositoryError> {
        let mut inner = self.inner.borrow_mut();
        inner.sync_cursor = inner.sync_cursor.max(seq);
        Ok(())
    }

    fn pending_operations(&self, limit: usize) -> Result<Vec<SyncOperation>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut operations: Vec<SyncOperation> = inner
            .sync_operations
            .values()
            .filter(|operation| operation.server_seq.is_none())
            .cloned()
            .collect();
        operations.sort_by_key(|operation| (operation.client_timestamp, operation.op_id));
        operations.truncate(limit);
        Ok(operations)
    }

    fn mark_operations_sent(&self, accepted: &[(Uuid, u64)]) -> Result<(), RepositoryError> {
        let mut inner = self.inner.borrow_mut();
        for (op_id, seq) in accepted {
            let key = {
                let operation = inner
                    .sync_operations
                    .get_mut(op_id)
                    .ok_or_else(|| repository_error("outbox operation not found"))?;
                if operation
                    .server_seq
                    .is_some_and(|existing| existing != *seq)
                {
                    return Err(repository_error("outbox sequence conflict"));
                }
                operation.server_seq = Some(*seq);
                (operation.entity_type, operation.entity_id)
            };
            let current = inner.sync_server_seqs.get(&key).copied().unwrap_or(0);
            inner.sync_server_seqs.insert(key, current.max(*seq));
        }
        Ok(())
    }

    fn is_operation_known(&self, op_id: Uuid) -> Result<bool, RepositoryError> {
        let inner = self.inner.borrow();
        Ok(inner.sync_operations.contains_key(&op_id) || inner.applied_operations.contains(&op_id))
    }

    fn entity_revision(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError> {
        Ok(self
            .inner
            .borrow()
            .sync_revisions
            .get(&(entity_type, entity_id))
            .copied()
            .unwrap_or(0))
    }

    fn entity_server_seq(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError> {
        Ok(self
            .inner
            .borrow()
            .sync_server_seqs
            .get(&(entity_type, entity_id))
            .copied()
            .unwrap_or(0))
    }

    fn load_entity(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<Option<SyncEntity>, RepositoryError> {
        Ok(load_sync_entity(
            &self.inner.borrow(),
            entity_type,
            entity_id,
        ))
    }

    fn record_local_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
    ) -> Result<(), RepositoryError> {
        require_sync_match(operation, entity)?;
        if operation.server_seq.is_some() {
            return Err(repository_error(
                "a local change cannot already have a server sequence",
            ));
        }
        let mut inner = self.inner.borrow_mut();
        save_sync_entity(&mut inner, entity);
        inner
            .sync_operations
            .insert(operation.op_id, operation.clone());
        inner
            .sync_revisions
            .insert((operation.entity_type, operation.entity_id), revision);
        Ok(())
    }

    fn apply_remote_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
        conflict: Option<&SyncConflict>,
    ) -> Result<(), RepositoryError> {
        require_sync_match(operation, entity)?;
        if operation.server_seq.is_none() {
            return Err(repository_error(
                "a remote operation must have a server sequence",
            ));
        }
        if let Some(conflict) = conflict
            && (conflict.entity_type != operation.entity_type
                || conflict.entity_id != operation.entity_id
                || conflict.remote_op_id != operation.op_id)
        {
            return Err(repository_error(
                "conflict does not match the remote operation",
            ));
        }
        let mut inner = self.inner.borrow_mut();
        save_sync_entity(&mut inner, entity);
        inner
            .sync_revisions
            .insert((operation.entity_type, operation.entity_id), revision);
        inner.sync_server_seqs.insert(
            (operation.entity_type, operation.entity_id),
            operation.server_seq.unwrap_or(0),
        );
        inner.applied_operations.insert(operation.op_id);
        if let Some(conflict) = conflict {
            inner.conflicts.insert(conflict.id, conflict.clone());
        }
        Ok(())
    }

    fn restore_entity(
        &self,
        entity: &SyncEntity,
        revision: u64,
        server_seq: u64,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.borrow_mut();
        save_sync_entity(&mut inner, entity);
        inner
            .sync_revisions
            .insert((entity.entity_type(), entity.entity_id()), revision);
        inner
            .sync_server_seqs
            .insert((entity.entity_type(), entity.entity_id()), server_seq);
        Ok(())
    }

    fn entity_revisions(&self) -> Result<Vec<(SyncEntityType, Uuid, u64)>, RepositoryError> {
        Ok(self
            .inner
            .borrow()
            .sync_revisions
            .iter()
            .map(|((entity_type, entity_id), revision)| (*entity_type, *entity_id, *revision))
            .collect())
    }

    fn conflicts(&self) -> Result<Vec<SyncConflict>, RepositoryError> {
        let inner = self.inner.borrow();
        let mut conflicts: Vec<SyncConflict> = inner.conflicts.values().cloned().collect();
        conflicts.sort_by_key(|conflict| (conflict.detected_at, conflict.id));
        Ok(conflicts)
    }

    fn resolve_conflict(
        &self,
        conflict_id: Uuid,
        resolution: core_domain::SyncConflictResolution,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        let mut inner = self.inner.borrow_mut();
        let conflict = inner
            .conflicts
            .get_mut(&conflict_id)
            .ok_or_else(|| repository_error("conflict not found"))?;
        if conflict.resolution.is_some() {
            return Err(repository_error("conflict already resolved"));
        }
        conflict.resolution = Some(resolution);
        conflict.resolved_at = Some(at);
        Ok(())
    }
}

fn save_sync_entity(inner: &mut Inner, entity: &SyncEntity) {
    match entity {
        SyncEntity::Account(entity) => {
            inner.accounts.insert(entity.id(), entity.clone());
        }
        SyncEntity::AccountGroup(entity) => {
            inner.account_groups.insert(entity.id(), entity.clone());
        }
        SyncEntity::Category(entity) => {
            inner.categories.insert(entity.id(), entity.clone());
        }
        SyncEntity::CategoryGroup(entity) => {
            inner.category_groups.insert(entity.id(), entity.clone());
        }
        SyncEntity::Counterparty(entity) => {
            inner.counterparties.insert(entity.id(), entity.clone());
        }
        SyncEntity::ExchangeRate(entity) => {
            inner.exchange_rates.insert(entity.id(), entity.clone());
        }
        SyncEntity::LedgerProfile(entity) => {
            inner.profile = Some(entity.clone());
        }
        SyncEntity::LedgerSettings(entity) => {
            inner.settings = Some(entity.clone());
        }
        SyncEntity::NotePreset(entity) => {
            inner.note_presets.insert(entity.id(), entity.clone());
        }
        SyncEntity::ReconciliationFlag(entity) => {
            inner
                .reconciliation_flags
                .insert(entity.id(), entity.clone());
        }
        SyncEntity::Tag(entity) => {
            inner.tags.insert(entity.id(), entity.clone());
        }
        SyncEntity::Transaction(entity) => {
            inner.transactions.insert(entity.id(), entity.clone());
        }
    }
}

fn load_sync_entity(
    inner: &Inner,
    entity_type: SyncEntityType,
    entity_id: Uuid,
) -> Option<SyncEntity> {
    match entity_type {
        SyncEntityType::Account => inner
            .accounts
            .get(&core_domain::AccountId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::Account),
        SyncEntityType::AccountGroup => inner
            .account_groups
            .get(&core_domain::AccountGroupId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::AccountGroup),
        SyncEntityType::Category => inner
            .categories
            .get(&core_domain::CategoryId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::Category),
        SyncEntityType::CategoryGroup => inner
            .category_groups
            .get(&core_domain::CategoryGroupId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::CategoryGroup),
        SyncEntityType::Counterparty => inner
            .counterparties
            .get(&core_domain::CounterpartyId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::Counterparty),
        SyncEntityType::ExchangeRate => inner
            .exchange_rates
            .get(&core_domain::ExchangeRateId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::ExchangeRate),
        SyncEntityType::LedgerProfile => inner
            .profile
            .clone()
            .filter(|profile| profile.id().as_uuid() == entity_id)
            .map(SyncEntity::LedgerProfile),
        SyncEntityType::LedgerSettings => inner
            .settings
            .clone()
            .filter(|settings| settings.id().as_uuid() == entity_id)
            .map(SyncEntity::LedgerSettings),
        SyncEntityType::NotePreset => inner
            .note_presets
            .get(&core_domain::TransactionNotePresetId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::NotePreset),
        SyncEntityType::ReconciliationFlag => inner
            .reconciliation_flags
            .get(&core_domain::ReconciliationFlagId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::ReconciliationFlag),
        SyncEntityType::Tag => inner
            .tags
            .get(&core_domain::TagId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::Tag),
        SyncEntityType::Transaction => inner
            .transactions
            .get(&core_domain::TransactionId::from_uuid(entity_id))
            .cloned()
            .map(SyncEntity::Transaction),
    }
}

fn require_sync_match(
    operation: &SyncOperation,
    entity: &SyncEntity,
) -> Result<(), RepositoryError> {
    if operation.entity_type != entity.entity_type() || operation.entity_id != entity.entity_id() {
        return Err(repository_error(
            "operation does not match its entity aggregate",
        ));
    }
    Ok(())
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

    #[test]
    fn sync_store_round_trips_every_entity_type() {
        use core_domain::{SyncConflictResolution, SyncOperationType, SyncRepository};

        let ledger = InMemoryLedger::new();
        let entities = [
            SyncEntity::LedgerSettings(settings()),
            SyncEntity::LedgerProfile(profile()),
            SyncEntity::Account(account(false)),
            SyncEntity::AccountGroup(account_group(false)),
            SyncEntity::Category(category(false)),
            SyncEntity::CategoryGroup(category_group(false)),
            SyncEntity::Tag(tag(false)),
            SyncEntity::Counterparty(counterparty(false)),
            SyncEntity::ReconciliationFlag(flag(false)),
            SyncEntity::NotePreset(preset(false)),
            SyncEntity::Transaction(transaction()),
            SyncEntity::ExchangeRate(exchange_rate(90, false)),
        ];
        let mut accepted = Vec::new();
        for (index, entity) in entities.iter().enumerate() {
            let operation = SyncOperation {
                op_id: Uuid::from_u128(1_000 + index as u128),
                ledger_id: Uuid::from_u128(1),
                device_id: Uuid::from_u128(2),
                entity_type: entity.entity_type(),
                entity_id: entity.entity_id(),
                op_type: match entity {
                    SyncEntity::LedgerSettings(_) => SyncOperationType::SettingsReplace,
                    SyncEntity::LedgerProfile(_) => SyncOperationType::ProfileReplace,
                    SyncEntity::ExchangeRate(_) => SyncOperationType::RateUpsert,
                    _ => SyncOperationType::Upsert,
                },
                base_revision: 0,
                client_timestamp: 1_000,
                idempotency_key: Uuid::from_u128(2_000 + index as u128),
                key_version: 1,
                nonce: "nonce".to_string(),
                ciphertext: "ciphertext".to_string(),
                payload_version: 1,
                signature: "signature".to_string(),
                server_seq: None,
            };
            ledger.record_local_change(entity, &operation, 1).unwrap();
            assert_eq!(
                ledger
                    .entity_revision(entity.entity_type(), entity.entity_id())
                    .unwrap(),
                1
            );
            assert_eq!(
                ledger
                    .entity_server_seq(entity.entity_type(), entity.entity_id())
                    .unwrap(),
                0
            );
            assert_eq!(
                ledger
                    .load_entity(entity.entity_type(), entity.entity_id())
                    .unwrap(),
                Some(entity.clone())
            );
            assert!(ledger.is_operation_known(operation.op_id).unwrap());
            assert_eq!(ledger.pending_operations(100).unwrap().len(), index + 1);
            accepted.push((operation.op_id, index as u64 + 1));
        }
        ledger.mark_operations_sent(&accepted).unwrap();
        assert!(ledger.pending_operations(100).unwrap().is_empty());
        for (index, entity) in entities.iter().enumerate() {
            assert_eq!(
                ledger
                    .entity_server_seq(entity.entity_type(), entity.entity_id())
                    .unwrap(),
                index as u64 + 1
            );
        }
        assert_eq!(ledger.entity_revisions().unwrap().len(), entities.len());
        assert!(
            ledger
                .load_entity(SyncEntityType::Account, Uuid::from_u128(999))
                .unwrap()
                .is_none()
        );

        let remote = SyncEntity::Tag(tag(true));
        let remote_operation = SyncOperation {
            op_id: Uuid::from_u128(3_000),
            ledger_id: Uuid::from_u128(1),
            device_id: Uuid::from_u128(3),
            entity_type: SyncEntityType::Tag,
            entity_id: remote.entity_id(),
            op_type: SyncOperationType::Upsert,
            base_revision: 0,
            client_timestamp: 1_100,
            idempotency_key: Uuid::from_u128(3_001),
            key_version: 1,
            nonce: "nonce".to_string(),
            ciphertext: "ciphertext".to_string(),
            payload_version: 1,
            signature: "signature".to_string(),
            server_seq: Some(100),
        };
        let conflict = SyncConflict {
            id: Uuid::from_u128(4_000),
            entity_type: SyncEntityType::Tag,
            entity_id: remote.entity_id(),
            local_payload: b"local".to_vec(),
            remote_op_id: remote_operation.op_id,
            local_revision: 1,
            remote_base_revision: 0,
            detected_at: timestamp(1_000),
            resolution: None,
            resolved_at: None,
        };
        ledger
            .apply_remote_change(&remote, &remote_operation, 1, Some(&conflict))
            .unwrap();
        assert!(ledger.is_operation_known(remote_operation.op_id).unwrap());
        assert_eq!(
            ledger
                .entity_server_seq(SyncEntityType::Tag, remote.entity_id())
                .unwrap(),
            100
        );
        assert_eq!(ledger.conflicts().unwrap(), vec![conflict.clone()]);
        ledger
            .resolve_conflict(
                conflict.id,
                SyncConflictResolution::Remote,
                timestamp(2_000),
            )
            .unwrap();
        assert_eq!(
            ledger.conflicts().unwrap()[0].resolution,
            Some(SyncConflictResolution::Remote)
        );
        assert!(
            ledger
                .resolve_conflict(conflict.id, SyncConflictResolution::Local, timestamp(3_000))
                .is_err()
        );

        let restored = SyncEntity::Counterparty(counterparty(true));
        ledger.restore_entity(&restored, 5, 101).unwrap();
        assert_eq!(
            ledger
                .entity_revision(restored.entity_type(), restored.entity_id())
                .unwrap(),
            5
        );
        assert_eq!(
            ledger
                .entity_server_seq(restored.entity_type(), restored.entity_id())
                .unwrap(),
            101
        );
        assert_eq!(ledger.cursor().unwrap(), 0);
        ledger.save_cursor(10).unwrap();
        ledger.save_cursor(5).unwrap();
        assert_eq!(ledger.cursor().unwrap(), 10);
    }
}
