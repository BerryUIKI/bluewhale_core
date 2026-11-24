# Bluewhale Data Model and Domain Contracts

## Document Status

- Status: Approved (R3 review)
- Review: R3 — domain model and data contracts
- Date: 2026-11-23
- Scope: `bluewhale_core` domain, application, persistence, and sync contracts
- Depends on: approved R1 product specification; approved R2 architecture
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: none

This document defines the approved conceptual data model, invariants, local schema policy, and export contract for v1. Decisions **D3.1–D3.12** are accepted as recommended. Approval authorizes R4 design, not implementation.

Revision note (2026-11-23): the private Apple implementation is the behavioral reference for the domain model. The Apple application is not modified; the public schema and code are implemented for the Rust stack. Source or design-text copying requires a recorded licensing decision.

## 1. Purpose and principles

- The model uses the private Apple implementation as the behavioral reference; the Apple application is not modified. The public schema and code are implemented for the Rust stack; source or design-text copying requires a recorded licensing decision.
- Money is exact and integer-based; balances and reports are derived, never stored as editable truth.
- Local SQLite is the authoritative client store. The server stores encrypted operations and metadata only.
- Records use stable identifiers, audit timestamps, and explicit versioning so local persistence, sync, and export can evolve without silent data loss.
- The domain core is pure Rust and has no dependency on Tauri, SQLite types, networking, or web UI code.

## 2. Money and currency

### 2.1 Representation

```text
Money {
  minor_units: i64,
  currency_code: ISO-4217 alpha-3,
  scale: u8,            // 0, 2, or 3 for supported currencies
}
```

- Every persisted monetary value stores `minor_units`, `currency_code`, and `scale` together. Storing the scale preserves historical interpretation even if a currency policy table later changes.
- Supported scales are 0, 2, and 3. A currency is valid only when its policy entry matches the declared scale.
- Amounts are stored as positive magnitudes. Transaction kind and account role determine direction. Derived balances may be negative.
- Addition, subtraction, multiplication by an integer ratio, and sign application use checked `i64` or `i128` intermediates. Overflow, underflow, and scale mismatch are domain errors; they never wrap or clamp silently.
- Floating-point types are prohibited for money and financial arithmetic.

### 2.2 Parsing and rounding

- Input is parsed from an exact decimal string. More fractional digits than the declared scale are rejected with a validation error; input is never silently rounded.
- Rounding is required only for derived currency conversion and averages. Convert with an exact rational rate using a wide integer intermediate, then round once to the nearest minor unit with a documented half-up rule at the aggregation boundary.
- Rounding rule versions are recorded so historical reports remain explainable.
- Views, Tauri commands, and Kotlin plugins never perform financial rounding.

## 3. Ledger settings, profile, and registry

### 3.1 Ledger settings

A ledger has one settings record:

| Field | Semantics |
| --- | --- |
| `ledger_id` | UUIDv7, stable ledger identity |
| `reporting_currency_code` | ISO-4217 code for report totals |
| `reporting_currency_scale` | Persisted scale for the reporting currency |
| `calendar_id` | Fixed to the proleptic Gregorian calendar in v1 |
| `time_zone_id` | IANA time-zone identifier chosen by the user |
| `currency_policy_version` | Version of the input/default scale policy |
| `created_at` / `updated_at` | UTC audit timestamps |

### 3.2 Ledger profile

| Field | Semantics |
| --- | --- |
| `ledger_id` | One-to-one with settings |
| `name` | Trimmed, non-empty display name |
| `emoji` | One grapheme used by the ledger selector |
| `created_at` / `updated_at` | UTC audit timestamps |

### 3.3 Device-local registry

The registry is outside any ledger database and contains no financial data:

- `registry_version`, `active_ledger_id`;
- one descriptor per known ledger: `ledger_id`, store location, display-name/emoji snapshot, timestamps.

The registry may be a small versioned JSON document or a device-local SQLite database; the implementation is chosen in R5. A malformed or unavailable registry must never delete, move, or reset a ledger store. A failed store remains selectable for retry.

### 3.4 Time

- `occurred_at`, `created_at`, and `updated_at` are absolute UTC instants stored as epoch milliseconds.
- Ledger-local dates, month boundaries, and day grouping are derived with the persisted `time_zone_id` and the Gregorian calendar, independent of the device time zone.
- Changing the ledger time zone is an explicit, confirmed settings operation. It recomputes grouping but does not rewrite stored instants.

## 4. Entities

### 4.1 Account

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `name` | Trimmed, non-empty display name |
| `asset_type` | `cash`, `checking`, `savings`, or `stored_value` |
| `currency_code` / `currency_scale` | Native account currency |
| `sort_order` | Stable user-facing order |
| `is_archived` | Hides from new entry; history remains |
| `created_at` / `updated_at` | UTC audit timestamps |
| `revision` | Monotonic per-entity sync revision |

Accounts are asset accounts only in v1. Liability accounts are out of scope. Account tags and one optional owner are modeled as association rows (section 4.6). Balances are never stored on the account.

### 4.2 Category group and category

**Category group**

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `name` | Trimmed; canonically unique within its kind |
| `kind` | `expense` or `income` |
| `sort_order` | Stable order within kind |
| `is_archived` | Archive state |
| `created_at` / `updated_at` / `revision` | Audit and sync fields |

**Category**

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `name` | Trimmed display name |
| `kind` | `expense` or `income` |
| `group_id` | Optional membership in one compatible group |
| `sort_order` | Stable order |
| `is_archived` | Prevents new assignment; history resolves |
| `created_at` / `updated_at` / `revision` | Audit and sync fields |

Only leaf categories are assignable to expense or income transactions. A category's kind cannot change while referenced by any transaction. Groups are organizational and never carry amounts.

### 4.3 Tag

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `name` | Trimmed display name |
| `name_normalized` | Case-folded unique key within the ledger |
| `color_hex` | Uppercase `#RRGGBB` |
| `description` | Optional trimmed text |
| `sort_order` | Stable order |
| `is_archived` | Prevents new assignment; history resolves |
| `created_at` / `updated_at` / `revision` | Audit and sync fields |

Tags are descriptive metadata. They never change account balances, transaction direction, category intent, or report totals.

### 4.4 Counterparty

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `kind` | `household_member`, `person`, or `organization` |
| `name` | Trimmed, non-empty display name; duplicates allowed |
| `sort_order` | Stable order |
| `is_archived` | Prevents new assignment; history resolves |
| `created_at` / `updated_at` / `revision` | Audit and sync fields |

At most one counterparty may own an account. At most one counterparty may be attached to an expense or income transaction. Counterparties are financially neutral.

### 4.5 Transaction and opening balance

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `kind` | `expense`, `income`, `transfer`, or `opening_balance` |
| `occurred_at` | Absolute UTC instant |
| `note` | Optional trimmed text |
| `account_id` | Source or single account |
| `destination_account_id` | Transfer destination only |
| `category_id` | Expense or income only |
| `counterparty_id` | Expense or income only |
| `source_minor_units` / `source_currency_code` / `source_currency_scale` | Exact source money |
| `destination_minor_units` / `destination_currency_code` / `destination_currency_scale` | Exact destination money for transfers |
| `created_at` / `updated_at` | UTC audit timestamps |
| `deleted_at` | Soft-deletion tombstone; null while active |
| `revision` | Monotonic per-entity sync revision |

Kind-specific rules:

- **Expense:** one account, one active expense leaf category, one positive source amount. Optional counterparty and tags.
- **Income:** one account, one active income leaf category, one positive source amount. Optional counterparty and tags.
- **Transfer:** two distinct accounts, exact source and destination money, no category or counterparty. Tags are allowed. Same-currency transfers store equal magnitudes; cross-currency transfers store both exact sides and never infer a rate.
- **Opening balance:** one asset account, one positive source amount, no category, no destination, no counterparty. Opening balances are ordinary recoverable events and are excluded from income and expense totals.

Soft deletion applies to transactions. Deleting sets `deleted_at` and increments the revision; restoring clears `deleted_at` and increments again. Hard deletion is not a product operation.

### 4.6 Association records

Tags and counterparties use first-class recoverable association rows:

| Association | Cardinality |
| --- | --- |
| `account_tag` | Many active tags per account |
| `transaction_tag` | Many active tags per transaction |
| `account_owner` | At most one active owner per account |
| `transaction_counterparty` | At most one active counterparty per expense or income transaction |

Each association row carries its own UUIDv7, owner and target identifiers, `created_at`, optional `deleted_at`, and `revision`. Associations are written in the same local transaction as the owning record. Archived targets remain resolvable for history but cannot receive new assignments.

### 4.7 Exchange rate

| Field | Semantics |
| --- | --- |
| `id` | UUIDv7 |
| `base_currency_code` | Source currency |
| `quote_currency_code` | Reporting or target currency |
| `rate_numerator` / `rate_denominator` | Exact positive rational rate |
| `effective_date` | Ledger-local calendar day |
| `source` | `manual` in v1; reference provider is a later decision |
| `created_at` / `updated_at` / `revision` | Audit and sync fields |

Rates are immutable records. Selection for an event uses the newest rate effective on or before the event's ledger-local day; ties are resolved by the newest `created_at`. Only direct rates are used in v1; missing direct rates produce an explicit incomplete-report state. Triangulation, inferred rates, and silent omission are prohibited.

## 5. Invariants

1. Monetary values use checked integers and explicit currency/scale; `Float`/`Double` are never used.
2. Stored amounts are positive magnitudes; transaction kind and account role determine direction.
3. A transfer is one atomic record containing both sides' exact money.
4. Account balances and all report totals are derived from active events; no mutable balance or spent total is persisted.
5. Soft-deleted transactions are excluded from balances, monthly totals, reports, and budget-like calculations.
6. Expense and income require a compatible active leaf category; transfers and opening balances never carry a category.
7. Categorically incompatible edits are rejected rather than reinterpreting history.
8. Archived accounts, categories, tags, and counterparties remain resolvable on historical records but are ineligible for new entry.
9. Month and day membership uses the persisted ledger time zone and Gregorian calendar.
10. Every mutable entity and association carries a stable UUIDv7 and a monotonically increasing `revision`.
11. Created and updated timestamps are UTC; export renders them as ISO 8601.
12. The server never receives plaintext financial data.

## 6. Identifiers, revisions, and audit fields

- Entity, association, operation, device, and ledger identifiers are UUIDv7.
- UUIDs are stored as canonical lowercase text in v1 for portability and debugging. Revisit BLOB storage only if index size becomes material.
- `revision` starts at 1 and increments on every mutation, including soft deletion and restoration.
- `created_at` and `updated_at` are epoch milliseconds in UTC. `updated_at` is never used as the only conflict signal; revision and server sequence are authoritative for sync.
- `deleted_at` is a tombstone for transactions and association rows. Master data uses `is_archived`.

## 7. Local schema (conceptual)

One SQLite database per ledger, opened with:

- `PRAGMA foreign_keys = ON`;
- WAL journal mode;
- a bounded busy timeout;
- application identifier and `schema_migration` version table.

### 7.1 Tables

| Table | Purpose |
| --- | --- |
| `schema_migration` | Applied migration version and timestamp |
| `ledger_settings` | Single ledger settings row |
| `ledger_profile` | Single ledger profile row |
| `account` | Asset accounts |
| `category_group` | Two-level category parents |
| `category` | Leaf categories |
| `tag` | Descriptive tags |
| `counterparty` | People and organizations |
| `account_tag`, `account_owner` | Account associations |
| `transaction` | All transaction kinds, including opening balances |
| `transaction_tag`, `transaction_counterparty` | Transaction associations |
| `exchange_rate` | Exact dated direct rates |
| `outbox_operation` | Pending encrypted sync operations |
| `sync_cursor` | Last applied server sequence per ledger |
| `sync_state` | Device and key metadata required for sync |

**Implementation notes (schema v1, P1.6).** The conceptual names above are singular; the shipped schema uses plural table names (`settings`, `profile`, `accounts`, `account_groups`, `categories`, `category_groups`, `tags`, `counterparties`, `reconciliation_flags`, `note_presets`, `transactions`, `exchange_rates`) plus the association tables `account_tags` and `transaction_tags`. One owner per account and one counterparty per transaction are nullable foreign-key columns (`accounts.owner_id`, `transactions.counterparty_id`), not association tables, matching the approved single-owner and single-counterparty rules. The `transaction` table is named `transactions` because `transaction` is a SQL keyword. The three sync tables (`outbox_operation`, `sync_cursor`, `sync_state`) are not part of local schema v1; they land with the sync engine in P2.3 and are added by a forward migration.

### 7.2 Constraints and indexes

Schema v1 enforces in SQL:

- Association uniqueness: `account_tags` and `transaction_tags` use a composite primary key, and tag associations are hard-deleted with their owner, so there is no soft-deleted duplicate to protect against.
- Single owner and single counterparty: nullable `owner_id` and `counterparty_id` columns make a second active association unrepresentable.
- Composed monetary columns are all present or all absent (`CHECK` on the destination amount columns); currency scales are range-checked on write.
- Identifier, timestamp, and boolean columns use the declared storage types, and booleans are `CHECK`ed to `0`/`1`.

The remaining rules are enforced by domain validation before a repository call, not by SQL: the unique normalized tag name (normalization is computed in Rust, not stored), kind-specific transaction rules (transfer destination, no category on a transfer, no destination amount on expense or income), and value ranges that need normalizing or message-level errors. A future schema version may duplicate the cheap checks as constraints; the domain stays authoritative.

Shipped indexes: `accounts(is_archived)`, `accounts(custom_group_id)`, `account_tags(tag_id)`, `categories(group_id)`, `categories(kind)`, `transactions(occurred_at)`, `transactions(account_id)`, `transactions(destination_account_id)`, `transactions(deleted_at)`, `transaction_tags(tag_id)`, and `exchange_rates(source_currency_code, reporting_currency_code, effective_at)`. Reads currently load rows and sort in Rust, so the composite ordering indexes (`transaction(occurred_at DESC, id)`, `account(is_archived, sort_order)`, `category(group_id, sort_order)`) are added when query pushdown lands, not before.

### 7.3 Migration policy

- The initial schema is version 1. Migrations are forward-only, numbered, and applied transactionally.
- Schema v1 is created by `crates/core-persistence/migrations/0001_initial.sql`, which shares one transaction with the `schema_migration` row. Because there is no earlier version to migrate from, the backup-before-migration step is not exercised yet; it is implemented with the first post-v1 migration, together with its preservation fixtures.
- Before applying a migration, create a consistent backup copy with the SQLite backup API and verify its integrity.
- A failed migration rolls back the transaction and preserves the original database and backup. The app never deletes, resets, or recreates a ledger to recover from migration failure.
- Every migration ships with fixtures built from each supported prior version and tests that verify data preservation, including soft-deleted and archived records.
- Additive changes are preferred. Dropping or renaming a column requires an export-compatibility analysis and a documented recovery plan.
- Each ledger migrates independently when opened. The device registry has its own version and must tolerate unknown ledger schema versions without modifying their files.
- Export format versioning is independent of the local schema version.

## 8. Sync contract (high level)

An operation envelope contains:

- `op_id` (UUIDv7), `ledger_id`, `device_id`, `entity_type`, `entity_id`, `base_revision`;
- `client_timestamp`, `idempotency_key`;
- encrypted payload, nonce, and key version.

The payload contains the domain mutation in a versioned serialization. The server assigns a monotonic per-ledger sequence and stores ciphertext plus routing metadata. Local writes commit first; the outbox drains asynchronously. Entity revision detects conflicts; R4 defines conflict application, tombstones, retention, compaction, and restore.

## 9. Export contract

A versioned JSON document contains:

- `export_version`, `ledger_id`, `exported_at`, `app_version`;
- ledger settings and profile;
- account groups and accounts, category groups and categories, tags, counterparties, reconciliation flags, and note presets, including archived records;
- transactions and their tag associations, including soft-deleted records;
- every exchange-rate revision, including superseded rates;
- registry descriptors are device-local and excluded.

Money is exported as `{ "minor_units": 12345, "currency": "USD", "scale": 2 }`. Every array is sorted by `id` in ascending UUID order and object keys follow the declared order, so the same ledger always produces the same document. Timestamps are UTC RFC 3339 with millisecond precision. `ledger_id` is the ledger's settings record identifier; the device-local registry identifier is not exported. Import (v1.1) validates the version and schema and is all-or-nothing; it never partially mutates a ledger.

The authoritative shape is the JSON Schema at `contracts/export/v1/export.schema.json`, with a synthetic fixture beside it and the encoding rules in `docs/reference/export-format.md`.

## 10. Decisions accepted at R3

Approved by the project owner on 2026-11-23. The recommendations below are accepted.

| ID | Decision | Recommendation |
| --- | --- | --- |
| D3.1 | Deletion scope | Soft-delete transactions only; archive master data; no hard delete |
| D3.2 | Opening balance | A `opening_balance` transaction kind; positive amount only in v1 |
| D3.3 | Currency scale storage | Store currency code and scale with every money-bearing record |
| D3.4 | Exchange rate form | Exact positive rational `numerator`/`denominator`; direct rates only in v1 |
| D3.5 | Rounding | Exact rational intermediate; half-up once at the aggregation boundary; versioned |
| D3.6 | UUID storage | Canonical lowercase text in v1 |
| D3.7 | Associations | First-class recoverable association rows with UUIDs and revisions |
| D3.8 | Tag uniqueness | Case-folded unique normalized name per ledger |
| D3.9 | Time-zone change | Explicit confirmation; recompute grouping; never rewrite instants |
| D3.10 | Migration policy | Forward-only, backup before migration, fixtures and tests, never reset a store |
| D3.11 | Export format | Versioned JSON with exact minor units; deterministic ordering |
| D3.12 | Multi-currency transfer | Store both exact sides; no inferred or stored rate on the transaction |

## 11. Next review

R4 — sync and server technical design: operation envelope, authentication, device lifecycle, key hierarchy, conflict policy, tombstones, retention, snapshots, restore, threat model, and API contract. R4 will keep server implementation details in the private repository while publishing the protocol contract. R3 approval does not authorize implementation.
