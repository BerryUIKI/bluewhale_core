# Bluewhale Product Specification

## Document Status

- Status: Approved (R1 requirements review)
- Review: R1 — product requirements
- Date: 2026-11-23
- Scope: `bluewhale_core`, `bluewhale_clients`, `bluewhale_server`
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: none

This document defines the approved product requirements for the Bluewhale cross-platform line (working name). Decisions **D1–D9** are accepted as recommended. Approval authorizes the R2 technical review, not implementation.

## 1. Product boundary

Bluewhale is a local-first personal finance product line for Windows and Android.

| Repository | Visibility | License | Role |
| --- | --- | --- | --- |
| `bluewhale_core` | public | Apache-2.0 | Shared domain and application logic |
| `bluewhale_clients` | public | Apache-2.0 | Windows and Android applications |
| `bluewhale_server` | private | proprietary | Identity, synchronization, backup |
| `bluewhale-ledger` | private | proprietary | Apple-native reference product; outside the open-source line |

The private Apple application is a reference for domain behavior only. The proposed boundary is an independent implementation: no source code, tests, assets, or design text are copied into the public repositories unless a separate recorded decision authorizes it (**D1**). This document is independently written for that reason.

## 2. Product positioning

Bluewhale records personal expenses, income, and transfers. It favors manual entry, exact multi-currency accounting, understandable history, offline availability, and user ownership of data over bank-feed automation or aggregation-heavy finance suites.

Primary user: an individual who manages personal cash flow on Windows and/or Android, values privacy and offline access, and enters transactions manually.

Secondary user: a user with multiple devices who wants private synchronization, or who keeps multiple ledgers (personal, household, side business).

Not targeted in v1: bank feeds, investment or tax accounting, shared household collaboration, enterprise accounting, or financial advice.

## 3. Product principles

Proposed as non-negotiable for v1:

- **Local-first.** Core ledger operations work without a network. Network failure never blocks entry or corrupts local data.
- **Exact money.** Monetary values use checked integer minor units with explicit currency and scale. No floating-point financial arithmetic.
- **Derived truth.** Balances and statistics are derived from ledger events, not stored as editable totals.
- **Recoverable history.** Financial records are soft-deleted and restorable; hard deletion is not the default.
- **Explicit time.** Month and day boundaries use the persisted ledger calendar and time zone, not the device time zone.
- **Privacy.** No third-party analytics of financial content. Server-side access to plaintext ledger data is a decision, not a default (**D4**).
- **Portability.** Users can export their data in a documented, versioned, open format.
- **Localization.** English and Simplified Chinese at first release; stored values and machine-readable keys remain locale-independent.

## 4. MVP scope (proposed v1)

### 4.1 Ledger setup and identity

- First-run setup with reporting currency, ledger time zone, ledger name, and emoji.
- Multiple local ledgers with isolated stores, a selector, create/switch, and non-destructive recovery when a store cannot be opened.
- No import or migration from the private Apple application in v1 unless separately approved (**D5**).

### 4.2 Accounts

- Asset accounts only: cash, checking, savings, stored value.
- One explicit currency and scale per account; opening balances recorded as ledger events.
- Archive without destroying history; optional tags and owner.
- Credit, loan, mortgage, receivable, and payable accounts are out of v1.

### 4.3 Categories

- Two-level expense and income hierarchy: group plus leaf category; only leaves are transaction-assignable.
- Archive and restore with historical references preserved.
- No arbitrary-depth hierarchy in v1.

### 4.4 Transactions

- Expense and income against one account and one leaf category.
- Same-currency and cross-currency transfers as one atomic event with exact source and destination amounts.
- Notes, multiple tags, and one optional counterparty.
- Deterministic history order; search and filter by text, kind, account, category, tag, counterparty, and ledger-local date range.
- Edit, soft delete, and restore with duplicate-safe and atomic persistence.
- Reconciliation flags and note presets are deferred to v1.1 unless the review promotes them.

### 4.5 Dashboard and reports

- Per-account native-currency balances and a net balance.
- Monthly income and expense in the reporting currency, excluding transfers and soft-deleted events.
- A basic cash-flow summary and category ranking report.
- Explicit incomplete-report state when a required exchange rate is missing; no silent conversion, omission, or partial total presented as complete.

### 4.6 Settings

- Application language: system default, English, Simplified Chinese.
- Reporting currency and manual historical exchange rates.
- Ledger profile (name and emoji).
- Device-local preferences remain separate from ledger data.

### 4.7 Server (proposed v1)

- User identity and device registration.
- Optional synchronization and backup across a user's Windows and Android devices.
- No bank connections, advertising, data sale, or third-party analytics of financial content.
- Offline-first behavior remains complete when the server is unavailable.
- Encryption and trust model are defined in R2/R4; end-to-end encryption is proposed (**D4**).

### 4.8 Data portability

- A documented, versioned export containing all ledger data, produced deterministically.
- Import is proposed for v1.1 after round-trip and recovery validation (**D5**).

### 4.9 Out of scope for v1

- Budgets, recurring plans, subscriptions, savings goals, and wishes.
- Liabilities, investments, insurance, commodities, futures, and other specialist assets.
- Bank feeds, automatic categorization, receipt OCR, and attachments.
- Shared or collaborative ledgers.
- Web, iOS, iPadOS, and macOS clients.
- A command-line interface.
- Import from other finance applications and automatic migration from the private Apple application.

These are candidates for later milestones; they are not implied by any v1 control.

## 5. Non-functional requirements

- **Platforms (proposed):** Windows 10 and 11; Android 10+ (API 29) phones and tablets. Architecture support is confirmed in R2 (**D7**).
- **Offline:** every core ledger operation works offline; synchronization is optional and resumable.
- **Performance (proposed):** 10,000 transactions per ledger remain usable without blocking the UI; startup and search targets are validated in R2.
- **Accessibility:** full keyboard operation on Windows; screen-reader labels and focus order on both platforms; scalable text and contrast.
- **Localization:** complete English and Simplified Chinese for user-facing text; no localized identifiers or stored values.
- **Security:** encryption in transit; platform-appropriate encryption at rest; no financial values, notes, credentials, or tokens in diagnostic logs.
- **Privacy:** minimal telemetry, opt-in where required, and no financial content in telemetry.
- **Reliability:** atomic multi-record operations, crash-safe local persistence, and recoverable backup.
- **Portability:** the export format has a version and a documented schema.

## 6. v1 success criteria

- A new user can set up a ledger and record, edit, soft-delete, and restore expense, income, and transfer events entirely offline.
- Per-account balances, net balance, and monthly totals reconcile exactly with ledger events under the persisted ledger time zone.
- Multi-currency accounts and cross-currency transfers preserve exact amounts; reports fail visibly when a required rate is missing.
- Two devices synchronize one ledger without duplicate or lost events; offline edits converge under a documented conflict policy.
- Export produces a complete, deterministic, schema-versioned dataset; import (if in scope) round-trips it without loss.
- English and Simplified Chinese UI is complete, with no untranslated user-facing text.
- No financial content appears in logs, telemetry, or server plaintext if end-to-end encryption is approved.
- Required automated tests and Windows/Android builds pass with observed results; no placeholder control is presented as functional.

## 7. Decisions required for R1 approval

| ID | Decision | Recommendation |
| --- | --- | --- |
| D1 | Relationship to the private Apple product | Independent implementation sharing domain semantics; no code or design-text copying |
| D2 | v1 feature set | Lean scope in section 4; defer planning, liabilities, specialist assets, and collaboration |
| D3 | Server v1 role | Identity, optional sync, backup; no web access or mandatory online account |
| D4 | Server trust model | End-to-end encryption for ledger payloads; server stores ciphertext and sync metadata |
| D5 | Data movement | Export in v1; import in v1.1 after round-trip validation; no automatic Apple-app migration |
| D6 | First-release languages | English and Simplified Chinese |
| D7 | Minimum platforms | Windows 10/11; Android 10+ |
| D8 | Cross-product sync | None in v1 |
| D9 | Product name | Confirm the public product name before release branding |

## 8. Review sequence and gates

| Review | Subject | Output |
| --- | --- | --- |
| R1 | Product requirements (this document) | Approved product spec |
| R2 | Technical architecture and stack | Repository boundaries, core runtime, client frameworks, server stack, sync and encryption approach |
| R3 | Domain model and data contracts | Money, ledger, account, transaction, category, tag, soft deletion, identifiers, schema versioning |
| R4 | Sync and server technical design | Identity, device model, operation log, conflicts, backup, recovery, threat model |
| R5 | Client technical design | Windows and Android UI architecture, local store, platform integration, accessibility |
| R6 | Quality and delivery | Test strategy, CI, release, contribution and license policy |

Each review ends with a recorded decision. Implementation of a feature begins only after its requirements and technical design are approved. A post-approval change requires a revision note and re-approval.

## 9. Approval

R1 was approved on 2026-11-23. D1–D9 are accepted as recommended. This document is the approved product specification for the Bluewhale line; R2 technical review is the next gate. Approval does not authorize implementation before the relevant technical design is approved.
