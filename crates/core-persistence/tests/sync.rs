//! Sync store and schema-v1 migration tests over SQLCipher.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use core_domain::{
    Currency, LedgerCalendar, LedgerSettings, LedgerSettingsId, LedgerSettingsRepository,
    LedgerTag, SyncConflict, SyncConflictResolution, SyncEntity, SyncEntityType, SyncOperation,
    SyncOperationType, SyncRepository, TagId, TagOptions, TagRepository,
};
use core_persistence::{SCHEMA_VERSION, SqliteLedger};
use jiff::Timestamp;
use rusqlite::Connection;
use uuid::Uuid;

static NEXT: AtomicU64 = AtomicU64::new(0);

const KEY: &str = "sync-test-database-key";

fn v7(value: u128) -> Uuid {
    Uuid::parse_str(&format!("0199c0f0-0000-7000-8000-{value:012x}")).unwrap()
}

fn timestamp(milliseconds: i64) -> Timestamp {
    Timestamp::from_millisecond(milliseconds).unwrap()
}

fn temp_path(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "bluewhale-sync-{name}-{}-{}.db",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::SeqCst)
    ))
}

fn tag(id: u128, name: &str) -> LedgerTag {
    LedgerTag::new(
        TagId::from_uuid(v7(id)),
        name,
        "#112233",
        TagOptions::default(),
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn operation(op_id: u128, entity_id: Uuid, server_seq: Option<u64>) -> SyncOperation {
    SyncOperation {
        op_id: v7(op_id),
        ledger_id: v7(1),
        device_id: v7(3),
        entity_type: SyncEntityType::Tag,
        entity_id,
        op_type: SyncOperationType::Upsert,
        base_revision: 0,
        client_timestamp: 1_000,
        idempotency_key: v7(op_id + 1),
        key_version: 1,
        nonce: "AAECAwQFBgcICQoL".to_string(),
        ciphertext: "YmFzZTY0".to_string(),
        payload_version: 1,
        signature: "AA".to_string(),
        server_seq,
    }
}

#[test]
fn the_sync_store_round_trips_outbox_revisions_and_conflicts() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let first = tag(10, "Food");
    let first_operation = operation(100, first.id().as_uuid(), None);
    ledger
        .record_local_change(&SyncEntity::Tag(first.clone()), &first_operation, 1)
        .unwrap();

    assert_eq!(
        ledger
            .entity_revision(SyncEntityType::Tag, first.id().as_uuid())
            .unwrap(),
        1
    );
    assert_eq!(
        ledger
            .load_entity(SyncEntityType::Tag, first.id().as_uuid())
            .unwrap(),
        Some(SyncEntity::Tag(first.clone()))
    );
    assert_eq!(ledger.pending_operations(10).unwrap().len(), 1);
    assert!(ledger.is_operation_known(first_operation.op_id).unwrap());

    ledger
        .mark_operations_sent(&[(first_operation.op_id, 5)])
        .unwrap();
    assert!(ledger.pending_operations(10).unwrap().is_empty());
    assert_eq!(
        ledger
            .entity_server_seq(SyncEntityType::Tag, first.id().as_uuid())
            .unwrap(),
        5
    );
    ledger.save_cursor(5).unwrap();
    assert_eq!(ledger.cursor().unwrap(), 5);
    ledger.save_cursor(4).unwrap();
    assert_eq!(ledger.cursor().unwrap(), 5);

    let renamed = tag(10, "Groceries");
    let remote_operation = operation(102, renamed.id().as_uuid(), Some(6));
    let conflict = SyncConflict {
        id: v7(200),
        entity_type: SyncEntityType::Tag,
        entity_id: renamed.id().as_uuid(),
        local_payload: b"local".to_vec(),
        remote_op_id: remote_operation.op_id,
        local_revision: 1,
        remote_base_revision: 0,
        detected_at: timestamp(2_000),
        resolution: None,
        resolved_at: None,
    };
    ledger
        .apply_remote_change(
            &SyncEntity::Tag(renamed.clone()),
            &remote_operation,
            1,
            Some(&conflict),
        )
        .unwrap();
    assert_eq!(
        ledger
            .load_entity(SyncEntityType::Tag, renamed.id().as_uuid())
            .unwrap(),
        Some(SyncEntity::Tag(renamed.clone()))
    );
    assert!(ledger.is_operation_known(remote_operation.op_id).unwrap());
    assert_eq!(
        ledger
            .entity_server_seq(SyncEntityType::Tag, renamed.id().as_uuid())
            .unwrap(),
        6
    );

    let conflicts = ledger.conflicts().unwrap();
    assert_eq!(conflicts, vec![conflict.clone()]);
    ledger
        .resolve_conflict(conflict.id, SyncConflictResolution::Local, timestamp(3_000))
        .unwrap();
    let resolved = ledger.conflicts().unwrap();
    assert_eq!(resolved[0].resolution, Some(SyncConflictResolution::Local));
    assert_eq!(resolved[0].resolved_at, Some(timestamp(3_000)));
    assert!(
        ledger
            .resolve_conflict(
                conflict.id,
                SyncConflictResolution::Remote,
                timestamp(4_000)
            )
            .is_err()
    );

    let restored = tag(11, "Travel");
    ledger
        .restore_entity(&SyncEntity::Tag(restored.clone()), 2, 7)
        .unwrap();
    assert_eq!(
        ledger
            .load_entity(SyncEntityType::Tag, restored.id().as_uuid())
            .unwrap(),
        Some(SyncEntity::Tag(restored.clone()))
    );
    assert_eq!(
        ledger
            .entity_revision(SyncEntityType::Tag, restored.id().as_uuid())
            .unwrap(),
        2
    );
    assert_eq!(
        ledger
            .entity_server_seq(SyncEntityType::Tag, restored.id().as_uuid())
            .unwrap(),
        7
    );
    assert_eq!(ledger.entity_revisions().unwrap().len(), 2);
}

#[test]
fn a_schema_v1_database_is_backed_up_and_migrated() {
    let path = temp_path("migration");
    let backup_path = PathBuf::from(format!("{}.backup-v1", path.display()));
    let first = tag(10, "Food");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(&format!("PRAGMA key = '{KEY}';"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version INTEGER PRIMARY KEY,
                     applied_at INTEGER NOT NULL
                 );
                 INSERT INTO schema_migration (version, applied_at) VALUES (1, 1000);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tags (id, name, color_hex, description, sort_order, is_archived,
                                   created_at, updated_at)
                 VALUES (?1, 'Food', '#112233', NULL, 0, 0, 1000, 1000)",
                [first.id().to_string()],
            )
            .unwrap();
    }

    {
        let ledger = SqliteLedger::open(&path, KEY).unwrap();
        assert_eq!(ledger.database().schema_version().unwrap(), SCHEMA_VERSION);
        assert_eq!(ledger.tag(first.id()).unwrap().unwrap().name(), "Food");
        assert!(ledger.pending_operations(10).unwrap().is_empty());
    }

    assert!(backup_path.exists());
    let backup = Connection::open(&backup_path).unwrap();
    backup
        .execute_batch(&format!("PRAGMA key = '{KEY}';"))
        .unwrap();
    let integrity: String = backup
        .query_row("PRAGMA integrity_check", [], |row| row.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    let count: i64 = backup
        .query_row("SELECT COUNT(*) FROM tags", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1);

    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(&backup_path);
}

#[test]
fn settings_round_trip_through_the_sync_load_path() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let settings = LedgerSettings::new(
        LedgerSettingsId::from_uuid(v7(1)),
        Currency::new("USD", 2).unwrap(),
        1,
        LedgerCalendar::new("UTC").unwrap(),
        timestamp(1_000),
        timestamp(1_000),
    );
    ledger.save_settings(&settings).unwrap();
    assert_eq!(
        ledger
            .load_entity(SyncEntityType::LedgerSettings, settings.id().as_uuid())
            .unwrap(),
        Some(SyncEntity::LedgerSettings(settings))
    );
}

fn settings_entity() -> LedgerSettings {
    LedgerSettings::new(
        LedgerSettingsId::from_uuid(v7(1)),
        Currency::new("USD", 2).unwrap(),
        1,
        LedgerCalendar::new("UTC").unwrap(),
        timestamp(1_000),
        timestamp(1_000),
    )
}

fn profile_entity() -> core_domain::LedgerProfile {
    core_domain::LedgerProfile::new(
        core_domain::LedgerProfileId::from_uuid(v7(2)),
        settings_entity().id(),
        "Household",
        "🏠",
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn account_group_entity() -> core_domain::AccountGroup {
    core_domain::AccountGroup::new(
        core_domain::AccountGroupId::from_uuid(v7(30)),
        "Everyday",
        0,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn category_group_entity() -> core_domain::CategoryGroup {
    core_domain::CategoryGroup::new(
        core_domain::CategoryGroupId::from_uuid(v7(31)),
        "Living",
        core_domain::CategoryKind::Expense,
        0,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn category_entity() -> core_domain::Category {
    core_domain::Category::new(
        core_domain::CategoryId::from_uuid(v7(32)),
        "Food",
        core_domain::CategoryKind::Expense,
        core_domain::CategoryOptions {
            group_id: Some(category_group_entity().id()),
            ..core_domain::CategoryOptions::default()
        },
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn counterparty_entity() -> core_domain::Counterparty {
    core_domain::Counterparty::new(
        core_domain::CounterpartyId::from_uuid(v7(33)),
        core_domain::CounterpartyKind::Organization,
        "Grocer",
        0,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn flag_entity() -> core_domain::ReconciliationFlag {
    core_domain::ReconciliationFlag::new(
        core_domain::ReconciliationFlagId::from_uuid(v7(34)),
        "Cleared",
        "#00AA55",
        core_domain::ReconciliationFlagOptions::default(),
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn preset_entity() -> core_domain::TransactionNotePreset {
    core_domain::TransactionNotePreset::new(
        core_domain::TransactionNotePresetId::from_uuid(v7(35)),
        "Weekly shop",
        0,
        false,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn account_entity() -> core_domain::Account {
    core_domain::Account::new(
        core_domain::AccountId::from_uuid(v7(36)),
        "Cash",
        core_domain::AssetAccountType::Cash,
        Currency::new("USD", 2).unwrap(),
        core_domain::AccountOptions {
            tag_ids: [tag(10, "Food").id()].into_iter().collect(),
            owner_id: Some(counterparty_entity().id()),
            custom_group_id: Some(account_group_entity().id()),
            ..core_domain::AccountOptions::default()
        },
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

fn transaction_entity() -> core_domain::Transaction {
    core_domain::Transaction::new(
        core_domain::TransactionId::from_uuid(v7(37)),
        core_domain::TransactionKind::Expense,
        core_domain::Money::new(1_000, Currency::new("USD", 2).unwrap()),
        timestamp(2_000),
        account_entity().id(),
        core_domain::TransactionFields {
            category_id: Some(category_entity().id()),
            reconciliation_flag_id: Some(flag_entity().id()),
            tag_ids: [tag(10, "Food").id()].into_iter().collect(),
            ..core_domain::TransactionFields::default()
        },
        timestamp(2_000),
        timestamp(2_000),
    )
    .unwrap()
}

fn exchange_rate_entity() -> core_domain::ExchangeRate {
    core_domain::ExchangeRate::new(
        core_domain::ExchangeRateId::from_uuid(v7(38)),
        Currency::new("CNY", 2).unwrap(),
        Currency::new("USD", 2).unwrap(),
        core_domain::ExactRatio::new(3, 20).unwrap(),
        timestamp(1_000),
        core_domain::ExchangeRateSource::Manual,
        None,
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
}

#[test]
fn every_entity_type_loads_through_the_sync_store() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let entities = [
        SyncEntity::LedgerSettings(settings_entity()),
        SyncEntity::LedgerProfile(profile_entity()),
        SyncEntity::AccountGroup(account_group_entity()),
        SyncEntity::Tag(tag(10, "Food")),
        SyncEntity::Counterparty(counterparty_entity()),
        SyncEntity::CategoryGroup(category_group_entity()),
        SyncEntity::Category(category_entity()),
        SyncEntity::ReconciliationFlag(flag_entity()),
        SyncEntity::NotePreset(preset_entity()),
        SyncEntity::Account(account_entity()),
        SyncEntity::Transaction(transaction_entity()),
        SyncEntity::ExchangeRate(exchange_rate_entity()),
    ];
    for entity in &entities {
        ledger.restore_entity(entity, 1, 10).unwrap();
        assert_eq!(
            ledger
                .load_entity(entity.entity_type(), entity.entity_id())
                .unwrap(),
            Some(entity.clone())
        );
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
            10
        );
    }
    assert_eq!(ledger.entity_revisions().unwrap().len(), entities.len());
    assert!(
        ledger
            .load_entity(SyncEntityType::Account, v7(999))
            .unwrap()
            .is_none()
    );
}

#[test]
fn sync_store_rejects_inconsistent_operations() {
    let ledger = SqliteLedger::open_in_memory(KEY).unwrap();
    let food = tag(10, "Food");
    let local = operation(100, food.id().as_uuid(), None);

    let already_sent = SyncOperation {
        server_seq: Some(1),
        ..local.clone()
    };
    assert!(
        ledger
            .record_local_change(&SyncEntity::Tag(food.clone()), &already_sent, 1)
            .is_err()
    );

    let wrong_entity = operation(101, v7(999), None);
    assert!(
        ledger
            .record_local_change(&SyncEntity::Tag(food.clone()), &wrong_entity, 1)
            .is_err()
    );

    assert!(
        ledger
            .apply_remote_change(&SyncEntity::Tag(food.clone()), &local, 1, None)
            .is_err()
    );

    let remote = operation(102, food.id().as_uuid(), Some(5));
    let mismatched_conflict = SyncConflict {
        id: v7(200),
        entity_type: SyncEntityType::Tag,
        entity_id: v7(999),
        local_payload: b"local".to_vec(),
        remote_op_id: remote.op_id,
        local_revision: 1,
        remote_base_revision: 0,
        detected_at: timestamp(1_000),
        resolution: None,
        resolved_at: None,
    };
    assert!(
        ledger
            .apply_remote_change(
                &SyncEntity::Tag(food.clone()),
                &remote,
                1,
                Some(&mismatched_conflict)
            )
            .is_err()
    );

    assert!(ledger.mark_operations_sent(&[(v7(999), 1)]).is_err());
    assert!(
        ledger
            .resolve_conflict(v7(999), SyncConflictResolution::Remote, timestamp(1_000))
            .is_err()
    );

    ledger
        .record_local_change(&SyncEntity::Tag(food), &local, 1)
        .unwrap();
    ledger.mark_operations_sent(&[(local.op_id, 1)]).unwrap();
    assert!(ledger.mark_operations_sent(&[(local.op_id, 2)]).is_err());
}

#[test]
fn an_empty_database_key_is_rejected() {
    assert!(SqliteLedger::open_in_memory("").is_err());
}

#[test]
fn an_existing_valid_backup_is_reused_and_a_corrupt_one_is_rejected() {
    let path = temp_path("backup-reuse");
    let backup_path = PathBuf::from(format!("{}.backup-v1", path.display()));
    let first = tag(10, "Food");
    {
        let connection = Connection::open(&path).unwrap();
        connection
            .execute_batch(&format!("PRAGMA key = '{KEY}';"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version INTEGER PRIMARY KEY,
                     applied_at INTEGER NOT NULL
                 );
                 INSERT INTO schema_migration (version, applied_at) VALUES (1, 1000);",
            )
            .unwrap();
        connection
            .execute(
                "INSERT INTO tags (id, name, color_hex, description, sort_order, is_archived,
                                   created_at, updated_at)
                 VALUES (?1, 'Food', '#112233', NULL, 0, 0, 1000, 1000)",
                [first.id().to_string()],
            )
            .unwrap();
    }
    fs::copy(&path, &backup_path).unwrap();
    {
        let ledger = SqliteLedger::open(&path, KEY).unwrap();
        assert_eq!(ledger.database().schema_version().unwrap(), SCHEMA_VERSION);
    }
    let _ = fs::remove_file(&path);
    let _ = fs::remove_file(&backup_path);

    let corrupt_path = temp_path("backup-corrupt");
    let corrupt_backup = PathBuf::from(format!("{}.backup-v1", corrupt_path.display()));
    {
        let connection = Connection::open(&corrupt_path).unwrap();
        connection
            .execute_batch(&format!("PRAGMA key = '{KEY}';"))
            .unwrap();
        connection
            .execute_batch(include_str!("../migrations/0001_initial.sql"))
            .unwrap();
        connection
            .execute_batch(
                "CREATE TABLE schema_migration (
                     version INTEGER PRIMARY KEY,
                     applied_at INTEGER NOT NULL
                 );
                 INSERT INTO schema_migration (version, applied_at) VALUES (1, 1000);",
            )
            .unwrap();
    }
    fs::write(&corrupt_backup, b"not a database").unwrap();
    assert!(SqliteLedger::open(&corrupt_path, KEY).is_err());
    let _ = fs::remove_file(&corrupt_path);
    let _ = fs::remove_file(&corrupt_backup);
}
