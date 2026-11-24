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
- In-memory repository fakes in `core-testkit` for use-case tests.
- Rust workspace with the `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, and `core-testkit` crates.
- Repository governance files: contribution guide, code of conduct, security policy, issue and pull-request templates, dependency update configuration, and editor configuration.
- Documentation index and architecture decision record process.
- Design review documents R1-R6 and the Phase 0 spike evidence.

### Changed

- Repository status is pre-alpha. The Phase 0 spike is not complete and no crate is published yet.
