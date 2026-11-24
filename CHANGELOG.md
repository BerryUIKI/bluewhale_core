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
- Versioned JSON export in `core-application`: a complete document with settings, profile, all master data, every transaction including soft-deleted records, and every exchange-rate revision, serialized deterministically with exact integer minor units and UTC millisecond timestamps.
- Export contract v1 under `contracts/export/v1`: a JSON Schema, a synthetic fixture, and tests that validate both the fixture and exporter output against the schema and round-trip through the Rust document types.
- `LedgerExportRepository` port with SQLCipher and in-memory implementations that read the whole ledger in one consistent transaction.
- In-memory repository fakes in `core-testkit` for use-case tests.
- Rust workspace with the `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, and `core-testkit` crates.
- Repository governance files: contribution guide, code of conduct, security policy, issue and pull-request templates, dependency update configuration, and editor configuration.
- Documentation index and architecture decision record process.
- Design review documents R1-R6 and the Phase 0 spike evidence.

### Changed

- Repository status is pre-alpha. The Phase 0 spike is not complete and no crate is published yet.
