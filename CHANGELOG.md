# Changelog

All notable changes to the Bluewhale core are documented in this file. The project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html) and [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [Unreleased]

### Added

- Money and currency primitives in `core-domain`: validated currency codes, the version 1 currency-scale policy, checked `Money` arithmetic, exact magnitude parsing, and minor-unit rounding, with unit and property tests.
- Typed identifiers, ledger settings, ledger profile validation, and ledger-time-zone calendar intervals in `core-domain`, with jiff-backed UTC instants.
- Domain entities in `core-domain`: accounts and account groups, category groups and categories, tags, counterparties, transactions and transfers, reconciliation flags, and note presets, with validation rules.
- Exchange rates in `core-domain`: exact rational quotes, scale-aware conversion with ties away from zero, and deterministic rate selection.
- Repository traits in `core-domain`, account balances, duplicate-opening-balance validation, and monthly statistics in the ledger time zone.
- Use cases in `core-application`: ledger initialization, account creation with an opening balance, transaction create/update/soft-delete/list, and transaction filtering with search and deterministic ordering.
- Management use cases for category groups, categories, tags, counterparties, account groups, and account updates.
- Exchange-rate use cases with per-day manual revisions, historical conversion, and a dashboard that reports balances, converted net balance, monthly statistics, and daily cash flow.
- SQLCipher-backed persistence in `core-persistence`: local schema v1 with its indexes and constraints, forward-only migrations, the `LedgerDatabase` open path (key, WAL, foreign keys, `application_id`, schema version), and `SqliteLedger` implementations of the domain repository traits.
- Round-trip persistence tests covering encryption and wrong-key rejection, settings, profile, accounts, tags, categories, transactions and soft deletion, master data (account and category groups, counterparties, reconciliation flags, note presets), bootstrap idempotence, snapshots, exchange rates, and missing-row errors.
- Coverage gates: `cargo-llvm-cov` with per-crate line floors and per-module branch floors, enforced by `scripts/check_coverage.py` in a nightly CI job, plus recorded evidence in `docs/quality/COVERAGE.md`.
- Direct tests for the in-memory `core-testkit` repositories, covering archived, deleted, and superseded filtering and the bootstrap and snapshot error paths.
- Settings and profile update use cases: reporting-currency changes, a ledger time-zone change that requires explicit confirmation and never rewrites stored instants, and ledger name/emoji updates.
- Reconciliation-flag and transaction-note-preset management use cases, with system-flag kinds preserved across updates.
- Versioned JSON export in `core-application`: a complete document with settings, profile, all master data, every transaction including soft-deleted records, and every exchange-rate revision, serialized deterministically with exact integer minor units and UTC millisecond timestamps.
- Export contract v1 under `contracts/export/v1`: a JSON Schema, a synthetic fixture, and tests that validate both the fixture and exporter output against the schema and round-trip through the Rust document types.
- `LedgerExportRepository` port with SQLCipher and in-memory implementations that read the whole ledger in one consistent transaction.
- In-memory repository fakes in `core-testkit` for use-case tests.
- Rust workspace with the `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, and `core-testkit` crates.
- Repository governance files: contribution guide, code of conduct, security policy, issue and pull-request templates, dependency update configuration, and editor configuration.
- Documentation index and architecture decision record process.
- Design review documents R1-R6 and the Phase 0 spike evidence.
- Sync protocol v1 under `contracts/sync/v1`: a JSON Schema for the signed operation envelope and every unencrypted `/v1` request and response document, an OpenAPI 3.1 description of the HTTP API, and synthetic conformance fixtures with a SHA-256 manifest.
- Protocol types in `core-sync`: `PROTOCOL_VERSION`, `PAYLOAD_VERSION`, batch and size limits, `OperationEnvelope` with v1 validation, the entity/operation matrix, and typed HTTP documents for authentication, devices, ledgers, push/pull/ack, snapshots, and errors.
- A language-neutral contract checker, `scripts/check_contracts.py`, and a CI job that validate the contract schemas, fixture hashes and expectations, and the OpenAPI document after bundling its relative schema references.
- Sync protocol reference in `docs/reference/sync-protocol.md`, covering versioning, encodings, envelope semantics, limits, the API surface, signature canonicalization, and conformance.
- Cryptographic primitives in `core-crypto`: Argon2id passphrase key derivation and key records, a random UMK, per-ledger data-key wrapping with X25519, HKDF-SHA256, and ChaCha20-Poly1305, Ed25519 device signing keys, and a Crockford-encoded recovery key with its envelope.
- Known-answer tests for Ed25519 (RFC 8032), X25519 (RFC 7748), ChaCha20-Poly1305 (RFC 8439), and Argon2id, plus key-record, recovery-code, tamper, and key-rotation tests and a measured version 1 Argon2id timing.
- `OperationEnvelope::signing_bytes()` in `core-sync`, producing the RFC 8785 canonical signing input, with an end-to-end sign/verify test against `core-crypto`.
- Crypto reference in `docs/reference/crypto.md`, covering algorithms, parameters, binary key formats, recovery-key encoding, signature procedure, vectors, and residual risks.
- The client sync engine in `core-sync`: atomic local outbox capture, encrypted and signed operation payloads, push/pull/ack, per-entity revisions and server sequences, idempotent replay, revision-gap detection, last-writer-wins for non-financial data, transaction conflict records and resolution, offline retry, and encrypted snapshot creation and restore.
- Sync payload v1 in `core-application`: the export v1 entity object for each entity type, with conversions from every domain aggregate and validation through the domain constructors.
- Local schema v2 in `core-persistence`: `outbox_operations`, `sync_entity_state`, `sync_applied_operations`, `sync_cursor`, `sync_conflicts`, and `sync_state`, with a verified `<database>.backup-v1` copy before migrating an existing schema v1 database.
- Two-client sync engine tests covering convergence, last-writer-wins without conflicts, financial conflict preservation and resolution, deletion and restore, offline retry, and snapshot restore followed by pull; SQLCipher sync-store and v1-to-v2 migration tests.
- P2.5 integration suite in `core-sync/tests/integration.rs`: two simulated clients driven by a seeded deterministic operation script over an in-memory server that validates every envelope, assigns per-ledger sequences, deduplicates by operation and idempotency key, and serves ordered pages. It covers six rounds of interleaved local edits with convergence of the export and every entity revision, a lost push response retried without duplication, concurrent-edit and delete-versus-edit conflicts preserved for review and resolved both ways, and snapshot restore on a fresh device followed by catch-up. Every push and pull document the clients exchange is validated against `contracts/sync/v1/sync.schema.json`.
- Verification record in `docs/quality/VERIFICATION.md`, starting with the P2.5 environment, commands, observed results, and what this host could not run.

### Changed

- The coverage gate now counts each instrumented source line once across generic and duplicate build instantiations. The 80% per-crate line threshold and 100% critical-module branch threshold are unchanged.
- Repository status is pre-alpha. The Phase 0 spike is complete; no crate is published yet.
