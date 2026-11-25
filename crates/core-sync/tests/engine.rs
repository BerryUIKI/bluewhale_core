//! End-to-end sync engine tests with two in-memory clients and a fake server.

use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;

use core_application::{Clock, IdSource, decode_entity};
use core_crypto::{DeviceSigningKey, DeviceVerifyingKey, KeyVersion, SymmetricKey};
use core_domain::{
    AccountId, CategoryId, Currency, LedgerCalendar, LedgerProfile, LedgerProfileId,
    LedgerProfileRepository, LedgerSettings, LedgerSettingsId, LedgerSettingsRepository, LedgerTag,
    Money, SyncConflictResolution, SyncEntity, SyncEntityType, SyncOperationType, SyncRepository,
    TagId, TagOptions, TagRepository, Transaction, TransactionFields, TransactionId,
    TransactionKind, TransactionRepository,
};
use core_sync::{
    AcceptedOperation, PullResponse, PushRequest, PushResponse, StoredOperation, SyncEngine,
    SyncError, SyncKeys, SyncTransport, SyncTransportError,
};
use core_testkit::InMemoryLedger;
use jiff::Timestamp;
use uuid::Uuid;

fn v7(value: u128) -> Uuid {
    Uuid::parse_str(&format!("0199c0f0-0000-7000-8000-{value:012x}")).unwrap()
}

fn timestamp(milliseconds: i64) -> Timestamp {
    Timestamp::from_millisecond(milliseconds).unwrap()
}

struct TestClock(Cell<i64>);

impl TestClock {
    fn new(milliseconds: i64) -> Self {
        Self(Cell::new(milliseconds))
    }
}

impl Clock for TestClock {
    fn now(&self) -> Timestamp {
        timestamp(self.0.get())
    }
}

struct TestIds(Cell<u128>);

impl TestIds {
    fn new(start: u128) -> Self {
        Self(Cell::new(start))
    }
}

impl IdSource for TestIds {
    fn next_uuid(&self) -> Uuid {
        let value = self.0.get();
        self.0.set(value + 1);
        v7(value)
    }
}

#[derive(Default)]
struct FakeServer {
    operations: RefCell<Vec<StoredOperation>>,
    latest_seq: Cell<u64>,
    acknowledged: Cell<u64>,
    fail_push: Cell<bool>,
    fail_pull: Cell<bool>,
}

impl FakeServer {
    fn new() -> Self {
        Self::default()
    }
}

impl SyncTransport for FakeServer {
    fn push(
        &self,
        _ledger_id: Uuid,
        request: &PushRequest,
    ) -> Result<PushResponse, SyncTransportError> {
        if self.fail_push.get() {
            return Err(SyncTransportError::new("offline", true));
        }
        let mut operations = self.operations.borrow_mut();
        let mut accepted = Vec::with_capacity(request.ops.len());
        for envelope in &request.ops {
            if let Some(existing) = operations
                .iter()
                .find(|stored| stored.envelope.op_id == envelope.op_id)
            {
                accepted.push(AcceptedOperation {
                    op_id: envelope.op_id,
                    seq: existing.seq,
                });
                continue;
            }
            let seq = self.latest_seq.get() + 1;
            self.latest_seq.set(seq);
            operations.push(StoredOperation {
                seq,
                received_at: "2026-11-25T00:00:00.000Z".to_string(),
                envelope: envelope.clone(),
            });
            accepted.push(AcceptedOperation {
                op_id: envelope.op_id,
                seq,
            });
        }
        Ok(PushResponse {
            accepted,
            latest_seq: self.latest_seq.get(),
        })
    }

    fn pull(
        &self,
        _ledger_id: Uuid,
        after_seq: u64,
        limit: u32,
    ) -> Result<PullResponse, SyncTransportError> {
        if self.fail_pull.get() {
            return Err(SyncTransportError::new("offline", true));
        }
        let operations = self.operations.borrow();
        let selected: Vec<StoredOperation> = operations
            .iter()
            .filter(|stored| stored.seq > after_seq)
            .take(limit as usize)
            .cloned()
            .collect();
        let last = selected
            .last()
            .map(|stored| stored.seq)
            .unwrap_or(after_seq);
        let has_more = operations.iter().any(|stored| stored.seq > last);
        Ok(PullResponse {
            ops: selected,
            latest_seq: self.latest_seq.get(),
            has_more,
        })
    }

    fn acknowledge(&self, _ledger_id: Uuid, seq: u64) -> Result<(), SyncTransportError> {
        self.acknowledged.set(seq);
        Ok(())
    }
}

fn settings() -> LedgerSettings {
    LedgerSettings::new(
        LedgerSettingsId::from_uuid(v7(1)),
        Currency::new("USD", 2).unwrap(),
        1,
        LedgerCalendar::new("UTC").unwrap(),
        timestamp(1_000),
        timestamp(1_000),
    )
}

fn profile() -> LedgerProfile {
    LedgerProfile::new(
        LedgerProfileId::from_uuid(v7(2)),
        settings().id(),
        "Household",
        "🏠",
        timestamp(1_000),
        timestamp(1_000),
    )
    .unwrap()
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

fn transaction(amount: i64, updated_at: i64) -> Transaction {
    Transaction::new(
        TransactionId::from_uuid(v7(20)),
        TransactionKind::Expense,
        Money::new(amount, Currency::new("USD", 2).unwrap()),
        timestamp(2_000),
        AccountId::from_uuid(v7(21)),
        TransactionFields {
            category_id: Some(CategoryId::from_uuid(v7(22))),
            ..TransactionFields::default()
        },
        timestamp(2_000),
        timestamp(updated_at),
    )
    .unwrap()
}

fn keys_for(
    device_id: Uuid,
    seed: u8,
    other_device: Uuid,
    other_key: &DeviceVerifyingKey,
) -> SyncKeys {
    let signing_key = DeviceSigningKey::from_seed([seed; 32]);
    let mut verifying_keys = BTreeMap::new();
    verifying_keys.insert(device_id, signing_key.public_key());
    verifying_keys.insert(other_device, other_key.clone());
    SyncKeys {
        ledger_id: v7(1),
        device_id,
        signing_key,
        verifying_keys,
        ledger_keys: BTreeMap::from([(1, SymmetricKey::from_bytes([0x42; 32]))]),
        current_key_version: KeyVersion::INITIAL,
    }
}

macro_rules! with_clients {
    ($ledger_a:ident, $ledger_b:ident, $engine_a:ident, $engine_b:ident, $body:block) => {{
        let device_a = v7(3);
        let device_b = v7(4);
        let signing_a = DeviceSigningKey::from_seed([1; 32]);
        let verifying_a = signing_a.public_key();
        let verifying_b = DeviceSigningKey::from_seed([2; 32]).public_key();
        let $ledger_a = InMemoryLedger::new();
        let $ledger_b = InMemoryLedger::new();
        let clock_a = TestClock::new(1_795_000_000_000);
        let clock_b = TestClock::new(1_795_000_000_000);
        let ids_a = TestIds::new(100);
        let ids_b = TestIds::new(10_000);
        let keys_a = SyncKeys {
            ledger_id: v7(1),
            device_id: device_a,
            signing_key: signing_a,
            verifying_keys: BTreeMap::from([
                (device_a, verifying_a.clone()),
                (device_b, verifying_b.clone()),
            ]),
            ledger_keys: BTreeMap::from([(1, SymmetricKey::from_bytes([0x42; 32]))]),
            current_key_version: KeyVersion::INITIAL,
        };
        let keys_b = keys_for(device_b, 2, device_a, &verifying_a);
        let $engine_a = SyncEngine::new(&$ledger_a, keys_a, &clock_a, &ids_a).unwrap();
        let $engine_b = SyncEngine::new(&$ledger_b, keys_b, &clock_b, &ids_b).unwrap();
        $body
    }};
}

#[test]
fn a_tag_converges_to_both_devices() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let server = FakeServer::new();
        let food = tag(10, "Food");
        engine_a
            .record_local_change(&SyncEntity::Tag(food.clone()), SyncOperationType::Upsert)
            .unwrap();
        let pushed = engine_a.sync(&server).unwrap();
        assert_eq!(pushed.push.pushed, 1);
        assert_eq!(pushed.cursor, 1);
        assert_eq!(server.acknowledged.get(), 1);

        engine_b.sync(&server).unwrap();
        assert_eq!(ledger_b.tag(food.id()).unwrap().unwrap(), food);
        assert_eq!(
            ledger_a
                .entity_revision(SyncEntityType::Tag, food.id().as_uuid())
                .unwrap(),
            1
        );
        assert_eq!(
            ledger_b
                .entity_revision(SyncEntityType::Tag, food.id().as_uuid())
                .unwrap(),
            1
        );
        assert!(engine_a.conflicts().unwrap().is_empty());
        assert!(engine_b.conflicts().unwrap().is_empty());
    });
}

#[test]
fn concurrent_tags_converge_to_server_order_without_conflicts() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let server = FakeServer::new();
        let original = tag(10, "Food");
        engine_a
            .record_local_change(
                &SyncEntity::Tag(original.clone()),
                SyncOperationType::Upsert,
            )
            .unwrap();
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();

        let from_a = tag(10, "A food");
        let from_b = tag(10, "B food");
        engine_a
            .record_local_change(&SyncEntity::Tag(from_a), SyncOperationType::Upsert)
            .unwrap();
        engine_b
            .record_local_change(&SyncEntity::Tag(from_b), SyncOperationType::Upsert)
            .unwrap();

        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        engine_a.sync(&server).unwrap();

        let a = ledger_a.tag(original.id()).unwrap().unwrap();
        let b = ledger_b.tag(original.id()).unwrap().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.name(), "B food");
        assert!(engine_a.conflicts().unwrap().is_empty());
        assert!(engine_b.conflicts().unwrap().is_empty());
        assert_eq!(
            ledger_a
                .entity_revision(SyncEntityType::Tag, original.id().as_uuid())
                .unwrap(),
            ledger_b
                .entity_revision(SyncEntityType::Tag, original.id().as_uuid())
                .unwrap()
        );
    });
}

#[test]
fn concurrent_transactions_converge_and_preserve_conflicts() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let server = FakeServer::new();
        let original = transaction(100, 2_000);
        engine_a
            .record_local_change(
                &SyncEntity::Transaction(original.clone()),
                SyncOperationType::Upsert,
            )
            .unwrap();
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();

        let edited_a = transaction(200, 3_000);
        let edited_b = transaction(300, 3_000);
        engine_a
            .record_local_change(
                &SyncEntity::Transaction(edited_a.clone()),
                SyncOperationType::Upsert,
            )
            .unwrap();
        engine_b
            .record_local_change(
                &SyncEntity::Transaction(edited_b),
                SyncOperationType::Upsert,
            )
            .unwrap();

        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        engine_a.sync(&server).unwrap();

        let a = ledger_a.transaction(original.id()).unwrap().unwrap();
        let b = ledger_b.transaction(original.id()).unwrap().unwrap();
        assert_eq!(a, b);
        assert_eq!(a.amount().minor_units(), 300);

        let conflicts_a = engine_a.conflicts().unwrap();
        assert_eq!(conflicts_a.len(), 1);
        assert!(conflicts_a[0].resolution.is_none());
        let losing_local =
            decode_entity(conflicts_a[0].entity_type, &conflicts_a[0].local_payload).unwrap();
        assert_eq!(losing_local, SyncEntity::Transaction(edited_a));

        let conflicts_b = engine_b.conflicts().unwrap();
        assert!(
            conflicts_b.is_empty(),
            "the later server-order edit is not a losing local version"
        );

        let resolved = engine_a
            .resolve_conflict(conflicts_a[0].id, SyncConflictResolution::Local)
            .unwrap()
            .expect("choosing the local side records an operation");
        assert!(resolved.server_seq.is_none());
        assert_eq!(
            engine_a.conflicts().unwrap()[0].resolution,
            Some(SyncConflictResolution::Local)
        );

        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        assert_eq!(
            ledger_a
                .transaction(original.id())
                .unwrap()
                .unwrap()
                .amount()
                .minor_units(),
            200
        );
        assert_eq!(
            ledger_b
                .transaction(original.id())
                .unwrap()
                .unwrap()
                .amount()
                .minor_units(),
            200
        );
    });
}

#[test]
fn deletion_and_restore_converge() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let server = FakeServer::new();
        let original = transaction(100, 2_000);
        engine_a
            .record_local_change(
                &SyncEntity::Transaction(original.clone()),
                SyncOperationType::Upsert,
            )
            .unwrap();
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();

        let deleted = original.soft_deleted(timestamp(4_000)).unwrap();
        engine_a
            .record_local_change(
                &SyncEntity::Transaction(deleted.clone()),
                SyncOperationType::Delete,
            )
            .unwrap();
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        assert!(
            ledger_b
                .transaction(original.id())
                .unwrap()
                .unwrap()
                .is_deleted()
        );

        let restored = deleted.restored(timestamp(5_000)).unwrap();
        engine_a
            .record_local_change(
                &SyncEntity::Transaction(restored),
                SyncOperationType::Restore,
            )
            .unwrap();
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        assert!(
            !ledger_b
                .transaction(original.id())
                .unwrap()
                .unwrap()
                .is_deleted()
        );
    });
}

#[test]
fn an_offline_push_keeps_the_outbox_and_retries() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let server = FakeServer::new();
        server.fail_push.set(true);
        let food = tag(10, "Food");
        engine_a
            .record_local_change(&SyncEntity::Tag(food.clone()), SyncOperationType::Upsert)
            .unwrap();
        let error = engine_a.sync(&server).unwrap_err();
        assert!(matches!(error, SyncError::Transport(_)));
        assert_eq!(engine_a.cursor().unwrap(), 0);
        assert_eq!(ledger_a.pending_operations(10).unwrap().len(), 1);

        server.fail_push.set(false);
        engine_a.sync(&server).unwrap();
        engine_b.sync(&server).unwrap();
        assert_eq!(ledger_b.tag(food.id()).unwrap().unwrap(), food);
    });
}

#[test]
fn a_snapshot_requires_a_drained_outbox() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        let _ = (&ledger_b, &engine_b);
        ledger_a.seed_settings(settings());
        ledger_a.seed_profile(profile());
        let pending = tag(12, "Pending");
        engine_a
            .record_local_change(&SyncEntity::Tag(pending), SyncOperationType::Upsert)
            .unwrap();
        assert!(engine_a.create_snapshot(&ledger_a, "1.0.0", 2).is_err());
    });
}

#[test]
fn a_snapshot_restores_and_pulls_later_operations() {
    with_clients!(ledger_a, ledger_b, engine_a, engine_b, {
        ledger_a.seed_settings(settings());
        ledger_a.seed_profile(profile());
        let server = FakeServer::new();

        let first = tag(10, "One");
        engine_a
            .record_local_change(&SyncEntity::Tag(first.clone()), SyncOperationType::Upsert)
            .unwrap();
        engine_a.sync(&server).unwrap();
        let snapshot = engine_a.create_snapshot(&ledger_a, "1.0.0", 2).unwrap();
        assert_eq!(snapshot.metadata.server_seq, 1);
        assert_eq!(
            snapshot.metadata.sha256_hex(),
            core_sync::sha256_hex(&snapshot.bytes)
        );

        let second = tag(11, "Two");
        engine_a
            .record_local_change(&SyncEntity::Tag(second.clone()), SyncOperationType::Upsert)
            .unwrap();
        engine_a.sync(&server).unwrap();

        let restored = engine_b
            .restore_snapshot(&snapshot.metadata, &snapshot.bytes, 2)
            .unwrap();
        assert_eq!(restored, 1);
        assert_eq!(ledger_b.settings().unwrap(), Some(settings()));
        assert_eq!(ledger_b.profile().unwrap(), Some(profile()));
        assert!(ledger_b.tag(first.id()).unwrap().is_some());
        assert!(ledger_b.tag(second.id()).unwrap().is_none());
        assert_eq!(engine_b.cursor().unwrap(), 1);

        engine_b.sync(&server).unwrap();
        assert_eq!(ledger_b.tag(second.id()).unwrap().unwrap(), second);
        assert_eq!(engine_b.cursor().unwrap(), 2);

        let mut tampered = snapshot.bytes.clone();
        let last = tampered.len() - 1;
        tampered[last] ^= 1;
        assert!(
            engine_b
                .restore_snapshot(&snapshot.metadata, &tampered, 2)
                .is_err()
        );
        assert!(
            engine_b
                .restore_snapshot(&snapshot.metadata, &snapshot.bytes, 3)
                .is_err()
        );
    });
}
