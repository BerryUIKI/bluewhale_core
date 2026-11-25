//! Round-trip tests over a real encrypted SQLite database.

use std::collections::BTreeSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use core_domain::{
    Account, AccountGroup, AccountGroupId, AccountGroupRepository, AccountId, AccountOptions,
    AccountRepository, AssetAccountType, Category, CategoryGroup, CategoryGroupId,
    CategoryGroupRepository, CategoryId, CategoryKind, CategoryOptions, CategoryRepository,
    Counterparty, CounterpartyId, CounterpartyKind, CounterpartyRepository, Currency,
    CurrencyScalePolicy, ExchangeRateId, ExchangeRateRepository, ExchangeRateSource,
    LedgerBootstrapRepository, LedgerCalendar, LedgerExportRepository, LedgerProfile,
    LedgerProfileId, LedgerProfileRepository, LedgerReadSnapshotRepository, LedgerSettings,
    LedgerSettingsId, LedgerSettingsRepository, LedgerTag, Money, ReconciliationFlag,
    ReconciliationFlagId, ReconciliationFlagOptions, ReconciliationFlagRepository,
    ReconciliationFlagSystemKind, RepositoryError, TagId, TagOptions, TagRepository, Transaction,
    TransactionFields, TransactionId, TransactionKind, TransactionNotePreset,
    TransactionNotePresetId, TransactionNotePresetRepository, TransactionRepository,
};
use core_persistence::{SCHEMA_VERSION, SqliteLedger};
use jiff::Timestamp;
use uuid::Uuid;

static NEXT: AtomicU64 = AtomicU64::new(0);

const KEY: &str = "test-database-key";

fn timestamp(milliseconds: i64) -> Timestamp {
    Timestamp::from_millisecond(milliseconds).unwrap()
}

fn currency(code: &str) -> Currency {
    CurrencyScalePolicy::v1().currency(code).unwrap()
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bluewhale-persistence-{name}-{}-{}.db",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ))
}

fn settings() -> LedgerSettings {
    LedgerSettings::new(
        LedgerSettingsId::from_uuid(Uuid::from_u128(1)),
        currency("USD"),
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
        "Household",
        "🏠",
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn account() -> Account {
    Account::new(
        AccountId::from_uuid(Uuid::from_u128(3)),
        "Cash",
        AssetAccountType::Cash,
        currency("USD"),
        AccountOptions {
            tag_ids: [TagId::from_uuid(Uuid::from_u128(4))].into_iter().collect(),
            note: "Everyday cash".to_string(),
            ..AccountOptions::default()
        },
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn tag() -> LedgerTag {
    LedgerTag::new(
        TagId::from_uuid(Uuid::from_u128(4)),
        "Travel",
        "#112233",
        TagOptions::default(),
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn category() -> Category {
    Category::new(
        CategoryId::from_uuid(Uuid::from_u128(5)),
        "Food",
        CategoryKind::Expense,
        CategoryOptions::default(),
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn transaction() -> Transaction {
    Transaction::new(
        TransactionId::from_uuid(Uuid::from_u128(6)),
        TransactionKind::Expense,
        Money::new(12_345, currency("USD")),
        timestamp(2_000),
        account().id(),
        TransactionFields {
            category_id: Some(category().id()),
            tag_ids: [tag().id()].into_iter().collect(),
            note: Some("Lunch".to_string()),
            ..TransactionFields::default()
        },
        timestamp(2_000),
        timestamp(2_000),
    )
    .unwrap()
}

fn account_group() -> AccountGroup {
    AccountGroup::new(
        AccountGroupId::from_uuid(Uuid::from_u128(30)),
        "Wallets",
        2,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn category_group() -> CategoryGroup {
    CategoryGroup::new(
        CategoryGroupId::from_uuid(Uuid::from_u128(31)),
        "Living",
        CategoryKind::Expense,
        1,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn counterparty() -> Counterparty {
    Counterparty::new(
        CounterpartyId::from_uuid(Uuid::from_u128(32)),
        CounterpartyKind::Organization,
        "Grocer",
        3,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn reconciliation_flag() -> ReconciliationFlag {
    ReconciliationFlag::new(
        ReconciliationFlagId::from_uuid(Uuid::from_u128(33)),
        "Cleared",
        "#00AA55",
        ReconciliationFlagOptions {
            system_kind: Some(ReconciliationFlagSystemKind::Cleared),
            locks_transactions: true,
            shortcut_key: Some("C".to_string()),
            sort_order: 1,
            is_archived: false,
        },
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn note_preset() -> TransactionNotePreset {
    TransactionNotePreset::new(
        TransactionNotePresetId::from_uuid(Uuid::from_u128(34)),
        "Weekly shop",
        4,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

#[test]
fn database_is_encrypted_reopens_and_rejects_wrong_keys() {
    let path = temp_path("encryption");
    {
        let ledger = SqliteLedger::open(&path, KEY).unwrap();
        assert_eq!(ledger.database().schema_version().unwrap(), SCHEMA_VERSION);
    }
    let bytes = fs::read(&path).unwrap();
    assert!(!bytes.starts_with(b"SQLite format 3\0"));

    let reopened = SqliteLedger::open(&path, KEY).unwrap();
    assert_eq!(
        reopened.database().schema_version().unwrap(),
        SCHEMA_VERSION
    );
    assert!(SqliteLedger::open(&path, "wrong-key").is_err());
    let _ = fs::remove_file(path);
}

#[test]
fn settings_profile_and_account_round_trip() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    ledger.save_tag(&tag()).unwrap();
    ledger.save_settings(&settings()).unwrap();
    ledger.save_profile(&profile()).unwrap();
    ledger.save_account(&account()).unwrap();

    assert_eq!(ledger.settings().unwrap(), Some(settings()));
    assert_eq!(ledger.profile().unwrap(), Some(profile()));
    let stored = ledger.account(account().id()).unwrap().unwrap();
    assert_eq!(stored, account());
    assert_eq!(ledger.accounts(false).unwrap().len(), 1);

    let archived = Account::new(
        account().id(),
        "Cash",
        AssetAccountType::Cash,
        currency("USD"),
        AccountOptions {
            is_archived: true,
            tag_ids: BTreeSet::new(),
            ..AccountOptions::default()
        },
        timestamp(1_000),
        timestamp(2_000),
    )
    .unwrap();
    ledger.save_account(&archived).unwrap();
    assert!(ledger.accounts(false).unwrap().is_empty());
    assert_eq!(ledger.accounts(true).unwrap().len(), 1);
}

#[test]
fn transaction_round_trip_preserves_tags_and_soft_deletion() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    ledger.save_settings(&settings()).unwrap();
    ledger.save_tag(&tag()).unwrap();
    ledger.save_account(&account()).unwrap();
    ledger.save_category(&category()).unwrap();
    ledger.save_transaction(&transaction()).unwrap();

    let stored = ledger.transaction(transaction().id()).unwrap().unwrap();
    assert_eq!(stored, transaction());
    assert_eq!(ledger.transactions(false).unwrap().len(), 1);

    let deleted = transaction().soft_deleted(timestamp(3_000)).unwrap();
    ledger.save_transaction(&deleted).unwrap();
    assert!(ledger.transactions(false).unwrap().is_empty());
    assert_eq!(ledger.transactions(true).unwrap().len(), 1);
    assert!(
        ledger
            .transactions(true)
            .unwrap()
            .first()
            .unwrap()
            .is_deleted()
    );
}

#[test]
fn bootstrap_initializes_once_and_snapshot_reads_active_data() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    ledger.save_tag(&tag()).unwrap();
    let did_initialize = ledger
        .initialize(
            &settings(),
            &profile(),
            &account(),
            &[category()],
            Some(&transaction()),
        )
        .unwrap();
    assert!(did_initialize);
    let second = ledger
        .initialize(&settings(), &profile(), &account(), &[category()], None)
        .unwrap();
    assert!(!second);

    let snapshot = ledger.snapshot().unwrap();
    assert_eq!(snapshot.settings, settings());
    assert_eq!(snapshot.active_accounts.len(), 1);
    assert!(snapshot.exchange_rates.is_empty());
}

#[test]
fn exchange_rate_round_trip() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let rate = core_domain::ExchangeRate::new(
        ExchangeRateId::from_uuid(Uuid::from_u128(20)),
        currency("CNY"),
        currency("USD"),
        core_domain::ExactRatio::new(3, 20).unwrap(),
        timestamp(1_000),
        ExchangeRateSource::Manual,
        None,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap();
    ledger.save_exchange_rate(&rate).unwrap();

    assert_eq!(ledger.exchange_rates(true).unwrap(), vec![rate.clone()]);
    assert_eq!(ledger.exchange_rates(false).unwrap(), vec![rate]);
}

#[test]
fn master_data_round_trips_and_archives() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    ledger.save_account_group(&account_group()).unwrap();
    ledger.save_category_group(&category_group()).unwrap();
    ledger.save_counterparty(&counterparty()).unwrap();
    ledger
        .save_reconciliation_flag(&reconciliation_flag())
        .unwrap();
    ledger.save_note_preset(&note_preset()).unwrap();

    assert_eq!(
        ledger.account_group(account_group().id()).unwrap().unwrap(),
        account_group()
    );
    assert_eq!(
        ledger
            .category_group(category_group().id())
            .unwrap()
            .unwrap(),
        category_group()
    );
    assert_eq!(
        ledger.counterparty(counterparty().id()).unwrap().unwrap(),
        counterparty()
    );
    assert_eq!(
        ledger
            .reconciliation_flag(reconciliation_flag().id())
            .unwrap()
            .unwrap(),
        reconciliation_flag()
    );
    assert_eq!(
        ledger.note_preset(note_preset().id()).unwrap().unwrap(),
        note_preset()
    );

    let archived = TransactionNotePreset::new(
        note_preset().id(),
        "Weekly shop",
        4,
        true,
        timestamp(1_000),
        timestamp(2_000),
    )
    .unwrap();
    ledger.save_note_preset(&archived).unwrap();
    assert!(ledger.note_presets(false).unwrap().is_empty());
    assert_eq!(ledger.note_presets(true).unwrap(), vec![archived]);
}

#[test]
fn export_snapshot_includes_archived_and_deleted_rows() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    ledger.save_settings(&settings()).unwrap();
    ledger.save_profile(&profile()).unwrap();
    ledger.save_tag(&tag()).unwrap();
    ledger.save_account(&account()).unwrap();
    ledger.save_category(&category()).unwrap();
    ledger.save_transaction(&transaction()).unwrap();
    ledger
        .save_transaction(&transaction().soft_deleted(timestamp(3_000)).unwrap())
        .unwrap();

    let archived = Account::new(
        account().id(),
        "Cash",
        AssetAccountType::Cash,
        currency("USD"),
        AccountOptions {
            is_archived: true,
            ..AccountOptions::default()
        },
        timestamp(1_000),
        timestamp(2_000),
    )
    .unwrap();
    ledger.save_account(&archived).unwrap();

    let snapshot = ledger.export_snapshot().unwrap();
    assert_eq!(snapshot.settings, settings());
    assert_eq!(snapshot.profile, profile());
    assert_eq!(snapshot.accounts.len(), 1);
    assert!(snapshot.accounts[0].is_archived());
    assert_eq!(snapshot.transactions.len(), 1);
    assert!(snapshot.transactions[0].is_deleted());
    assert_eq!(snapshot.tags.len(), 1);
    assert_eq!(snapshot.categories.len(), 1);
}

#[test]
fn repository_errors_are_reported_for_missing_rows() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    assert!(
        ledger
            .account(AccountId::from_uuid(Uuid::from_u128(99)))
            .unwrap()
            .is_none()
    );
    let error: RepositoryError = ledger.snapshot().unwrap_err();
    assert!(error.message().contains("not initialized"));
}

#[test]
fn every_repository_read_covers_filters_and_missing_rows() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let rate = core_domain::ExchangeRate::new(
        ExchangeRateId::from_uuid(Uuid::from_u128(20)),
        currency("CNY"),
        currency("USD"),
        core_domain::ExactRatio::new(3, 20).unwrap(),
        timestamp(1_000),
        ExchangeRateSource::Manual,
        None,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap();
    ledger.save_settings(&settings()).unwrap();
    ledger.save_profile(&profile()).unwrap();
    ledger.save_account_group(&account_group()).unwrap();
    ledger.save_category_group(&category_group()).unwrap();
    ledger.save_tag(&tag()).unwrap();
    ledger.save_counterparty(&counterparty()).unwrap();
    ledger
        .save_reconciliation_flag(&reconciliation_flag())
        .unwrap();
    ledger.save_note_preset(&note_preset()).unwrap();
    ledger.save_account(&account()).unwrap();
    ledger.save_category(&category()).unwrap();
    ledger.save_transaction(&transaction()).unwrap();
    ledger.save_exchange_rate(&rate).unwrap();

    assert_eq!(
        ledger.account_group(account_group().id()).unwrap(),
        Some(account_group())
    );
    assert_eq!(
        ledger.category_group(category_group().id()).unwrap(),
        Some(category_group())
    );
    assert_eq!(ledger.tag(tag().id()).unwrap(), Some(tag()));
    assert_eq!(
        ledger.counterparty(counterparty().id()).unwrap(),
        Some(counterparty())
    );
    assert_eq!(
        ledger
            .reconciliation_flag(reconciliation_flag().id())
            .unwrap(),
        Some(reconciliation_flag())
    );
    assert_eq!(
        ledger.note_preset(note_preset().id()).unwrap(),
        Some(note_preset())
    );
    assert_eq!(ledger.account(account().id()).unwrap(), Some(account()));
    assert_eq!(ledger.category(category().id()).unwrap(), Some(category()));
    assert_eq!(
        ledger.transaction(transaction().id()).unwrap(),
        Some(transaction())
    );

    assert_eq!(ledger.account_groups(false).unwrap(), vec![account_group()]);
    assert_eq!(ledger.account_groups(true).unwrap(), vec![account_group()]);
    assert_eq!(
        ledger.category_groups(false).unwrap(),
        vec![category_group()]
    );
    assert_eq!(
        ledger.category_groups(true).unwrap(),
        vec![category_group()]
    );
    assert_eq!(ledger.tags(false).unwrap(), vec![tag()]);
    assert_eq!(ledger.tags(true).unwrap(), vec![tag()]);
    assert_eq!(ledger.counterparties(false).unwrap(), vec![counterparty()]);
    assert_eq!(ledger.counterparties(true).unwrap(), vec![counterparty()]);
    assert_eq!(
        ledger.reconciliation_flags(false).unwrap(),
        vec![reconciliation_flag()]
    );
    assert_eq!(
        ledger.reconciliation_flags(true).unwrap(),
        vec![reconciliation_flag()]
    );
    assert_eq!(ledger.note_presets(false).unwrap(), vec![note_preset()]);
    assert_eq!(ledger.note_presets(true).unwrap(), vec![note_preset()]);
    assert_eq!(ledger.accounts(false).unwrap(), vec![account()]);
    assert_eq!(ledger.accounts(true).unwrap(), vec![account()]);
    assert_eq!(ledger.categories(false).unwrap(), vec![category()]);
    assert_eq!(ledger.categories(true).unwrap(), vec![category()]);
    assert_eq!(ledger.transactions(false).unwrap(), vec![transaction()]);
    assert_eq!(ledger.transactions(true).unwrap(), vec![transaction()]);
    assert_eq!(ledger.exchange_rates(false).unwrap(), vec![rate.clone()]);
    assert_eq!(ledger.exchange_rates(true).unwrap(), vec![rate]);

    let missing = Uuid::from_u128(999);
    assert!(
        ledger
            .account(AccountId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .account_group(AccountGroupId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .category(CategoryId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .category_group(CategoryGroupId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(ledger.tag(TagId::from_uuid(missing)).unwrap().is_none());
    assert!(
        ledger
            .counterparty(CounterpartyId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .reconciliation_flag(ReconciliationFlagId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .note_preset(TransactionNotePresetId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
    assert!(
        ledger
            .transaction(TransactionId::from_uuid(missing))
            .unwrap()
            .is_none()
    );
}
