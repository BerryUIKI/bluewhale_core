//! P2.5 integration: two simulated clients, deterministic operations, and
//! wire-level conformance against the published protocol contract.
//!
//! Each client is a real [`SyncEngine`] over an in-memory ledger with its own
//! device key, clock, and identifier source. [`SimulatedServer`] behaves like
//! the Go server: it validates every envelope, assigns one per-ledger
//! sequence, deduplicates by operation and idempotency key, and serves ordered
//! pages. Every request and response document in this file is validated
//! against `contracts/sync/v1/sync.schema.json`, the same contract the server
//! validates against, so these tests exercise the shared fixtures' schema and
//! the documents the client actually emits.

use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use core_application::{Clock, ExportDocument, ExportLedgerUseCase, IdSource, decode_entity};
use core_crypto::{DeviceSigningKey, DeviceVerifyingKey, KeyVersion, SymmetricKey};
use core_domain::{
    AccountId, CategoryId, Currency, LedgerCalendar, LedgerProfile, LedgerProfileId,
    LedgerSettings, LedgerSettingsId, LedgerTag, Money, SyncConflict, SyncConflictResolution,
    SyncEntity, SyncEntityType, SyncOperation, SyncOperationType, SyncRepository, TagId,
    TagOptions, TagRepository, Transaction, TransactionFields, TransactionId, TransactionKind,
    TransactionRepository,
};
use core_sync::{
    AcceptedOperation, OperationType, PAYLOAD_VERSION, PROTOCOL_VERSION, PullResponse, PushRequest,
    PushResponse, StoredOperation, SyncEngine, SyncError, SyncKeys, SyncOutcome, SyncTransport,
    SyncTransportError,
};
use core_testkit::InMemoryLedger;
use jiff::Timestamp;
use serde_json::Value;
use uuid::Uuid;

/// The fixed server receipt time used in stored operations.
const RECEIVED_AT: &str = "2026-11-25T12:00:00.000Z";

/// The frozen instant every simulated clock reports.
const NOW_MILLIS: i64 = 1_795_000_000_000;

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

/// A seeded SplitMix64 generator; the operation script is reproducible.
struct SplitMix64(u64);

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self(seed)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.0;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

fn schema() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../contracts/sync/v1/sync.schema.json");
        let text = fs::read_to_string(path).expect("read contracts/sync/v1/sync.schema.json");
        serde_json::from_str(&text).expect("parse the published sync schema")
    })
}

/// Validates one document against the published contract.
fn assert_valid(document: &str, value: &Value) {
    let definitions = schema()
        .get("$defs")
        .expect("the published schema has $defs");
    assert!(
        definitions.get(document).is_some(),
        "the published schema has no $defs.{document}"
    );
    let wrapper = serde_json::json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$ref": format!("#/$defs/{document}"),
        "$defs": definitions,
    });
    let validator = jsonschema::validator_for(&wrapper).expect("compile the published schema");
    if let Err(error) = validator.validate(value) {
        panic!("{document} violates the published contract: {error}\n{value}");
    }
}

/// An in-memory server with the operation-log behavior the client relies on.
#[derive(Default)]
struct SimulatedServer {
    operations: RefCell<Vec<StoredOperation>>,
    sequences: RefCell<BTreeMap<Uuid, u64>>,
    idempotency: RefCell<BTreeMap<Uuid, Uuid>>,
    latest_seq: Cell<u64>,
    acknowledged: Cell<u64>,
    lose_next_push_response: Cell<bool>,
    validated_documents: Cell<usize>,
}

impl SimulatedServer {
    fn new() -> Self {
        Self::default()
    }

    fn len(&self) -> usize {
        self.operations.borrow().len()
    }

    fn latest_seq(&self) -> u64 {
        self.latest_seq.get()
    }

    fn acknowledged(&self) -> u64 {
        self.acknowledged.get()
    }

    fn validated_documents(&self) -> usize {
        self.validated_documents.get()
    }

    fn devices(&self) -> BTreeSet<Uuid> {
        self.operations
            .borrow()
            .iter()
            .map(|stored| stored.envelope.device_id)
            .collect()
    }

    fn sequences(&self) -> Vec<u64> {
        self.operations
            .borrow()
            .iter()
            .map(|stored| stored.seq)
            .collect()
    }

    fn operation_types(&self) -> BTreeSet<OperationType> {
        self.operations
            .borrow()
            .iter()
            .map(|stored| stored.envelope.op_type)
            .collect()
    }

    fn validate(&self, document: &str, value: &Value) {
        assert_valid(document, value);
        self.validated_documents
            .set(self.validated_documents.get() + 1);
    }
}

impl SyncTransport for SimulatedServer {
    fn push(
        &self,
        ledger_id: Uuid,
        request: &PushRequest,
    ) -> Result<PushResponse, SyncTransportError> {
        assert_eq!(ledger_id, v7(1), "push addressed an unexpected ledger");
        request
            .validate()
            .expect("the engine produced an invalid batch");
        self.validate("pushRequest", &serde_json::to_value(request).unwrap());

        let mut accepted = Vec::with_capacity(request.ops.len());
        for envelope in &request.ops {
            assert_eq!(envelope.ledger_id, ledger_id);
            assert_eq!(envelope.protocol_version, PROTOCOL_VERSION);
            assert_eq!(envelope.payload_version, PAYLOAD_VERSION);

            if let Some(existing) = self.sequences.borrow().get(&envelope.op_id).copied() {
                accepted.push(AcceptedOperation {
                    op_id: envelope.op_id,
                    seq: existing,
                });
                continue;
            }
            let known = self
                .idempotency
                .borrow()
                .get(&envelope.idempotency_key)
                .copied();
            if let Some(known) = known {
                assert_eq!(
                    known, envelope.op_id,
                    "an idempotency key was reused for another operation"
                );
            }

            let seq = self.latest_seq.get() + 1;
            self.latest_seq.set(seq);
            self.sequences.borrow_mut().insert(envelope.op_id, seq);
            self.idempotency
                .borrow_mut()
                .insert(envelope.idempotency_key, envelope.op_id);
            self.operations.borrow_mut().push(StoredOperation {
                seq,
                received_at: RECEIVED_AT.to_string(),
                envelope: envelope.clone(),
            });
            accepted.push(AcceptedOperation {
                op_id: envelope.op_id,
                seq,
            });
        }

        let response = PushResponse {
            accepted,
            latest_seq: self.latest_seq.get(),
        };
        self.validate("pushResponse", &serde_json::to_value(&response).unwrap());
        if self.lose_next_push_response.replace(false) {
            return Err(SyncTransportError::new(
                "connection reset after the server committed the batch",
                true,
            ));
        }
        Ok(response)
    }

    fn pull(
        &self,
        ledger_id: Uuid,
        after_seq: u64,
        limit: u32,
    ) -> Result<PullResponse, SyncTransportError> {
        assert_eq!(ledger_id, v7(1), "pull addressed an unexpected ledger");
        let selected: Vec<StoredOperation> = self
            .operations
            .borrow()
            .iter()
            .filter(|stored| stored.seq > after_seq)
            .take(limit as usize)
            .cloned()
            .collect();
        let last = selected
            .last()
            .map(|stored| stored.seq)
            .unwrap_or(after_seq);
        let has_more = self
            .operations
            .borrow()
            .iter()
            .any(|stored| stored.seq > last);
        let response = PullResponse {
            ops: selected,
            latest_seq: self.latest_seq.get(),
            has_more,
        };
        self.validate("pullResponse", &serde_json::to_value(&response).unwrap());
        Ok(response)
    }

    fn acknowledge(&self, ledger_id: Uuid, seq: u64) -> Result<(), SyncTransportError> {
        assert_eq!(
            ledger_id,
            v7(1),
            "acknowledgement addressed an unexpected ledger"
        );
        self.acknowledged.set(self.acknowledged.get().max(seq));
        Ok(())
    }
}

/// One simulated device: its own ledger store, keys, clock, and identifiers.
struct ClientState {
    device_id: Uuid,
    seed: u8,
    ledger: InMemoryLedger,
    clock: TestClock,
    ids: TestIds,
}

impl ClientState {
    fn new(seed: u8, first_id: u128) -> Self {
        Self {
            device_id: v7(2 + u128::from(seed)),
            seed,
            ledger: InMemoryLedger::new(),
            clock: TestClock::new(NOW_MILLIS),
            ids: TestIds::new(first_id),
        }
    }

    fn keys(&self, verifying_keys: &BTreeMap<Uuid, DeviceVerifyingKey>) -> SyncKeys {
        SyncKeys {
            ledger_id: v7(1),
            device_id: self.device_id,
            signing_key: DeviceSigningKey::from_seed([self.seed; 32]),
            verifying_keys: verifying_keys.clone(),
            ledger_keys: BTreeMap::from([(1u32, SymmetricKey::from_bytes([0x42; 32]))]),
            current_key_version: KeyVersion::INITIAL,
        }
    }

    fn engine<'a>(&'a self, verifying_keys: &BTreeMap<Uuid, DeviceVerifyingKey>) -> SyncEngine<'a> {
        SyncEngine::new(
            &self.ledger,
            self.keys(verifying_keys),
            &self.clock,
            &self.ids,
        )
        .expect("construct the sync engine")
    }

    fn export(&self) -> ExportDocument {
        ExportLedgerUseCase::new(&self.ledger, &self.clock)
            .export("0.0.0-p2.5")
            .expect("export the ledger")
    }

    fn revisions(&self) -> Vec<(SyncEntityType, Uuid, u64)> {
        let mut revisions = self.ledger.entity_revisions().expect("read revisions");
        revisions.sort();
        revisions
    }
}

/// Two devices sharing the ledger identity and the data key.
struct World {
    verifying_keys: BTreeMap<Uuid, DeviceVerifyingKey>,
    server: SimulatedServer,
    a: ClientState,
    b: ClientState,
}

impl World {
    fn new() -> Self {
        let verifying_keys = (1u8..=3)
            .map(|seed| {
                (
                    v7(2 + u128::from(seed)),
                    DeviceSigningKey::from_seed([seed; 32]).public_key(),
                )
            })
            .collect();
        let a = ClientState::new(1, 100);
        let b = ClientState::new(2, 10_000);
        for client in [&a, &b] {
            client.ledger.seed_settings(settings());
            client.ledger.seed_profile(profile());
        }
        Self {
            verifying_keys,
            server: SimulatedServer::new(),
            a,
            b,
        }
    }

    fn client(&self, index: usize) -> &ClientState {
        match index {
            0 => &self.a,
            1 => &self.b,
            other => panic!("no simulated client {other}"),
        }
    }

    fn sync(&self, index: usize) -> SyncOutcome {
        self.client(index)
            .engine(&self.verifying_keys)
            .sync(&self.server)
            .expect("synchronize")
    }

    /// Synchronizes both clients until neither has anything left to exchange.
    fn settle(&self) {
        for _ in 0..2 {
            self.sync(0);
            self.sync(1);
        }
    }

    fn record(&self, index: usize, entity: &SyncEntity, op_type: SyncOperationType) {
        self.client(index)
            .engine(&self.verifying_keys)
            .record_local_change(entity, op_type)
            .expect("record a local change");
    }

    /// Applies one deterministic operation to a client's local state.
    fn perform_random_operation(&self, random: &mut SplitMix64, index: usize, counter: &mut u64) {
        *counter += 1;
        let sequence = *counter;
        let slot = random.below(5);
        if slot < 3 {
            let name = format!("tag-{slot}-{sequence}");
            let tag = LedgerTag::new(
                TagId::from_uuid(v7(10 + u128::from(slot))),
                &name,
                "#112233",
                TagOptions::default(),
                timestamp(1_000),
                timestamp(1_000 + sequence as i64),
            )
            .expect("build a tag");
            self.record(index, &SyncEntity::Tag(tag), SyncOperationType::Upsert);
            return;
        }

        let id = v7(20 + u128::from(slot - 3));
        let current = self
            .client(index)
            .ledger
            .transaction(TransactionId::from_uuid(id))
            .expect("read the local transaction");
        match current {
            None => {
                let created = transaction(id, 100 + sequence as i64, 2_000 + sequence as i64);
                self.record(
                    index,
                    &SyncEntity::Transaction(created),
                    SyncOperationType::Upsert,
                );
            }
            Some(existing) if existing.is_deleted() => {
                let restored = existing
                    .restored(timestamp(3_000 + sequence as i64))
                    .expect("restore a transaction");
                self.record(
                    index,
                    &SyncEntity::Transaction(restored),
                    SyncOperationType::Restore,
                );
            }
            Some(existing) => {
                let delete = random.below(4) == 0;
                if delete {
                    let deleted = existing
                        .soft_deleted(timestamp(3_000 + sequence as i64))
                        .expect("delete a transaction");
                    self.record(
                        index,
                        &SyncEntity::Transaction(deleted),
                        SyncOperationType::Delete,
                    );
                } else {
                    let edited = transaction(id, 100 + sequence as i64, 2_000 + sequence as i64);
                    self.record(
                        index,
                        &SyncEntity::Transaction(edited),
                        SyncOperationType::Upsert,
                    );
                }
            }
        }
    }

    fn exported(&self, index: usize) -> ExportDocument {
        self.client(index).export()
    }

    fn revisions(&self, index: usize) -> Vec<(SyncEntityType, Uuid, u64)> {
        self.client(index).revisions()
    }

    fn conflicts(&self, index: usize) -> Vec<SyncConflict> {
        self.client(index)
            .engine(&self.verifying_keys)
            .conflicts()
            .expect("read conflicts")
    }

    fn resolve(
        &self,
        index: usize,
        conflict_id: Uuid,
        resolution: SyncConflictResolution,
    ) -> Option<SyncOperation> {
        self.client(index)
            .engine(&self.verifying_keys)
            .resolve_conflict(conflict_id, resolution)
            .expect("resolve a conflict")
    }

    /// Accepts the server-order version for every unresolved conflict.
    ///
    /// This records no operation, so it cannot create further conflicts.
    fn accept_server_order(&self) -> usize {
        let mut resolved = 0;
        for index in 0..2 {
            for conflict in self
                .conflicts(index)
                .into_iter()
                .filter(|conflict| conflict.resolution.is_none())
            {
                let operation = self.resolve(index, conflict.id, SyncConflictResolution::Remote);
                assert!(operation.is_none());
                resolved += 1;
            }
        }
        resolved
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

fn transaction(id: Uuid, amount: i64, updated_at: i64) -> Transaction {
    Transaction::new(
        TransactionId::from_uuid(id),
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

#[test]
fn deterministic_operation_rounds_converge() {
    let world = World::new();
    let mut random = SplitMix64::new(0x5EED_2026_1126);
    let mut counter = 0;

    // A deterministic prologue puts every operation kind in the log and shows
    // that a deletion and its restoration reach both devices.
    let seeded = transaction(v7(30), 100, 2_000);
    world.record(
        0,
        &SyncEntity::Transaction(seeded.clone()),
        SyncOperationType::Upsert,
    );
    world.settle();
    let deleted = seeded.soft_deleted(timestamp(3_000)).unwrap();
    world.record(
        0,
        &SyncEntity::Transaction(deleted.clone()),
        SyncOperationType::Delete,
    );
    world.settle();
    let restored = deleted.restored(timestamp(3_500)).unwrap();
    world.record(
        0,
        &SyncEntity::Transaction(restored),
        SyncOperationType::Restore,
    );
    world.settle();

    for round in 0..6 {
        let first = usize::from(round % 2 == 1);
        let second = 1 - first;

        world.perform_random_operation(&mut random, first, &mut counter);
        world.sync(first);
        world.perform_random_operation(&mut random, second, &mut counter);
        world.perform_random_operation(&mut random, second, &mut counter);
        world.sync(second);
        world.perform_random_operation(&mut random, first, &mut counter);
        world.sync(first);
        world.sync(second);
    }
    world.settle();

    // Both clients applied the same server order and hold the same state.
    assert_eq!(world.exported(0), world.exported(1));
    assert_eq!(world.revisions(0), world.revisions(1));
    assert!(
        world
            .client(0)
            .ledger
            .pending_operations(1)
            .unwrap()
            .is_empty()
    );
    assert!(
        world
            .client(1)
            .ledger
            .pending_operations(1)
            .unwrap()
            .is_empty()
    );

    // The server saw one contiguous sequence and both devices.
    let sequences = world.server.sequences();
    let expected: Vec<u64> = (1..=sequences.len() as u64).collect();
    assert_eq!(sequences, expected);
    assert_eq!(world.server.devices().len(), 2);
    assert_eq!(world.server.latest_seq(), sequences.len() as u64);
    assert_eq!(world.server.acknowledged(), sequences.len() as u64);

    // The script exercised creation, editing, deletion, and restoration.
    let operation_types = world.server.operation_types();
    assert!(operation_types.contains(&OperationType::Upsert));
    assert!(operation_types.contains(&OperationType::Delete));
    assert!(operation_types.contains(&OperationType::Restore));

    // Every request and response was exercised against the published contract.
    assert!(world.server.validated_documents() > 0);

    // Financial divergence is reviewable; no other entity type conflicts.
    let mut conflicts = 0;
    for index in 0..2 {
        for conflict in world.conflicts(index) {
            conflicts += 1;
            assert_eq!(
                conflict.entity_type,
                SyncEntityType::Transaction,
                "only financial entities produce conflict records"
            );
            assert_ne!(
                conflict.local_revision, conflict.remote_base_revision,
                "a conflict must preserve a divergent local version"
            );
        }
    }
    assert!(
        conflicts > 0,
        "the scenario must exercise a financial conflict"
    );

    // Accepting the server-order version converges without new operations.
    let before = world.server.len();
    let accepted = world.accept_server_order();
    assert!(
        accepted > 0,
        "the scenario must leave a conflict to resolve"
    );
    world.settle();
    assert_eq!(world.server.len(), before);
    assert!(world.conflicts(0).iter().all(|c| c.resolution.is_some()));
    assert!(world.conflicts(1).iter().all(|c| c.resolution.is_some()));
    assert_eq!(world.exported(0), world.exported(1));
}

#[test]
fn a_lost_push_response_retries_without_duplicating_operations() {
    let world = World::new();
    let food = tag(10, "Food");
    world.record(0, &SyncEntity::Tag(food.clone()), SyncOperationType::Upsert);

    world.server.lose_next_push_response.set(true);
    let error = world
        .client(0)
        .engine(&world.verifying_keys)
        .sync(&world.server)
        .expect_err("the lost response fails the push");
    assert!(matches!(error, SyncError::Transport(_)));
    assert_eq!(world.server.len(), 1, "the server committed the batch");
    assert_eq!(world.server.acknowledged(), 0);
    assert_eq!(
        world.client(0).ledger.pending_operations(10).unwrap().len(),
        1,
        "the outbox keeps the operation until the server acknowledges it"
    );

    let outcome = world.sync(0);
    assert_eq!(outcome.push.pushed, 1);
    assert_eq!(world.server.len(), 1, "the retry is deduplicated");
    assert_eq!(world.server.latest_seq(), 1);
    assert!(
        world
            .client(0)
            .ledger
            .pending_operations(10)
            .unwrap()
            .is_empty()
    );

    world.sync(1);
    assert_eq!(
        world.client(1).ledger.tag(food.id()).unwrap().unwrap(),
        food
    );
}

#[test]
fn a_concurrent_edit_is_reviewable_and_resolves_to_the_local_side() {
    let world = World::new();
    let original = transaction(v7(20), 100, 2_000);
    world.record(
        0,
        &SyncEntity::Transaction(original.clone()),
        SyncOperationType::Upsert,
    );
    world.sync(0);
    world.sync(1);
    assert_eq!(world.revisions(0), world.revisions(1));

    let edit_a = transaction(v7(20), 200, 3_000);
    let edit_b = transaction(v7(20), 300, 3_000);
    world.record(
        0,
        &SyncEntity::Transaction(edit_a.clone()),
        SyncOperationType::Upsert,
    );
    world.record(
        1,
        &SyncEntity::Transaction(edit_b),
        SyncOperationType::Upsert,
    );

    world.sync(0);
    world.sync(1);
    world.sync(0);

    // Server order decides the live version on both devices.
    let live_a = world
        .client(0)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    let live_b = world
        .client(1)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    assert_eq!(live_a, live_b);
    assert_eq!(live_a.amount().minor_units(), 300);

    // The losing version is preserved for review, not silently overwritten.
    let conflicts = world.conflicts(0);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].resolution, None);
    assert_eq!(conflicts[0].local_revision, 2);
    assert_eq!(conflicts[0].remote_base_revision, 1);
    assert_eq!(
        decode_entity(conflicts[0].entity_type, &conflicts[0].local_payload).unwrap(),
        SyncEntity::Transaction(edit_a)
    );
    // The winning device already holds its own later version, so it skips the
    // earlier concurrent one and has nothing to review.
    assert!(world.conflicts(1).is_empty());

    // Choosing the local side records a new operation on top of the revision.
    let operation = world
        .resolve(0, conflicts[0].id, SyncConflictResolution::Local)
        .expect("choosing the local side records an operation");
    assert!(operation.server_seq.is_none());

    world.settle();
    let live_a = world
        .client(0)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    let live_b = world
        .client(1)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    assert_eq!(live_a, live_b);
    assert_eq!(live_a.amount().minor_units(), 200);
    assert!(world.conflicts(0).iter().all(|c| c.resolution.is_some()));
    assert!(world.conflicts(1).iter().all(|c| c.resolution.is_some()));
}

#[test]
fn a_concurrent_delete_and_edit_keeps_both_versions_reviewable() {
    let world = World::new();
    let original = transaction(v7(20), 100, 2_000);
    world.record(
        0,
        &SyncEntity::Transaction(original.clone()),
        SyncOperationType::Upsert,
    );
    world.sync(0);
    world.sync(1);

    let deleted = original.soft_deleted(timestamp(3_000)).unwrap();
    world.record(
        0,
        &SyncEntity::Transaction(deleted.clone()),
        SyncOperationType::Delete,
    );
    world.record(
        1,
        &SyncEntity::Transaction(transaction(v7(20), 300, 3_000)),
        SyncOperationType::Upsert,
    );

    world.sync(0);
    world.sync(1);
    world.sync(0);

    // The later server-order edit is live; the deletion stays in the record.
    let live_a = world
        .client(0)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    let live_b = world
        .client(1)
        .ledger
        .transaction(original.id())
        .unwrap()
        .unwrap();
    assert_eq!(live_a, live_b);
    assert!(!live_a.is_deleted());
    assert_eq!(live_a.amount().minor_units(), 300);

    let conflicts = world.conflicts(0);
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].resolution, None);
    assert_eq!(
        decode_entity(conflicts[0].entity_type, &conflicts[0].local_payload).unwrap(),
        SyncEntity::Transaction(deleted)
    );

    // Accepting the server order discards the deletion without an operation.
    let operation = world.resolve(0, conflicts[0].id, SyncConflictResolution::Remote);
    assert!(operation.is_none());
    world.settle();
    assert!(world.conflicts(0).iter().all(|c| c.resolution.is_some()));
    assert_eq!(world.exported(0), world.exported(1));
}

#[test]
fn a_snapshot_restores_a_fresh_client_and_catches_up() {
    let world = World::new();

    let first = tag(10, "One");
    world.record(
        0,
        &SyncEntity::Tag(first.clone()),
        SyncOperationType::Upsert,
    );
    let recorded = transaction(v7(20), 100, 2_000);
    world.record(
        0,
        &SyncEntity::Transaction(recorded.clone()),
        SyncOperationType::Upsert,
    );
    world.sync(0);
    assert_eq!(
        world
            .client(0)
            .engine(&world.verifying_keys)
            .cursor()
            .unwrap(),
        2
    );

    let snapshot = world
        .client(0)
        .engine(&world.verifying_keys)
        .create_snapshot(&world.client(0).ledger, "0.0.0-p2.5", 2)
        .expect("create a snapshot");
    assert_eq!(snapshot.metadata.server_seq, 2);
    assert_eq!(
        snapshot.metadata.sha256_hex(),
        core_sync::sha256_hex(&snapshot.bytes)
    );

    // A fresh device restores the snapshot instead of replaying the log.
    let fresh = ClientState::new(3, 20_000);
    let restored = fresh
        .engine(&world.verifying_keys)
        .restore_snapshot(&snapshot.metadata, &snapshot.bytes, 2)
        .expect("restore the snapshot");
    assert_eq!(restored, 2);
    assert_eq!(fresh.engine(&world.verifying_keys).cursor().unwrap(), 2);
    assert_eq!(fresh.export(), world.exported(0));
    // The snapshot carries the revision of every synced entity; the settings
    // and profile, which were seeded rather than synced, are restored at
    // revision zero.
    let tracked: Vec<(SyncEntityType, Uuid, u64)> = fresh
        .revisions()
        .into_iter()
        .filter(|(_, _, revision)| *revision > 0)
        .collect();
    assert_eq!(tracked, world.revisions(0));
    assert_eq!(fresh.ledger.tag(first.id()).unwrap().unwrap(), first);
    assert_eq!(
        fresh.ledger.transaction(recorded.id()).unwrap().unwrap(),
        recorded
    );

    // Later operations arrive through the normal pull path.
    let second = tag(11, "Two");
    world.record(
        0,
        &SyncEntity::Tag(second.clone()),
        SyncOperationType::Upsert,
    );
    world.sync(0);
    fresh
        .engine(&world.verifying_keys)
        .sync(&world.server)
        .unwrap();
    assert_eq!(fresh.engine(&world.verifying_keys).cursor().unwrap(), 3);
    assert_eq!(fresh.ledger.tag(second.id()).unwrap().unwrap(), second);
    assert_eq!(fresh.export(), world.exported(0));

    // The snapshot is authenticated and schema-versioned. A separate empty
    // device exercises the rejected restores.
    let target = ClientState::new(3, 30_000);
    let mut tampered = snapshot.bytes.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;
    assert!(
        target
            .engine(&world.verifying_keys)
            .restore_snapshot(&snapshot.metadata, &tampered, 2)
            .is_err()
    );
    assert!(
        target
            .engine(&world.verifying_keys)
            .restore_snapshot(&snapshot.metadata, &snapshot.bytes, 3)
            .is_err()
    );

    // A ledger that already holds state refuses the restore instead of
    // rolling back entities and keeping its higher cursor.
    let error = fresh
        .engine(&world.verifying_keys)
        .restore_snapshot(&snapshot.metadata, &snapshot.bytes, 2)
        .expect_err("restoring into a non-empty ledger must fail");
    assert!(matches!(
        error,
        SyncError::Snapshot("restore requires an empty ledger")
    ));
}
