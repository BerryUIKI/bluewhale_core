# Bluewhale Development Plan

## Document Status

- Status: Active development plan
- Date: 2026-11-23
- Scope: `bluewhale_core`, `bluewhale_clients`, `bluewhale_server`
- Basis: approved reviews R1–R6
- Decision owner: project owner
- Execution gate: Phase 0 (Rust + Tauri spike) must pass before feature implementation begins

This plan turns the approved designs into ordered development phases, work items, exit criteria, and GitHub Projects tracking. It does not change any approved product or technical decision. Estimates are indicative sizes, not calendar promises.

## 1. Development principles

- **Design gate first.** A feature starts only after its design is approved. R1–R6 are approved; later features need their own design documents.
- **Spike gate.** The Rust + Tauri spike must provide observed Windows and Android evidence before production feature work.
- **Trunk-based.** `main` stays releasable; work lands in small, focused changes; no long-lived develop or release branches.
- **Vertical slices.** Each slice delivers real behavior, tests, localization, and documentation. No placeholder controls.
- **Evidence, not claims.** A build, test, or spike result is reported only when observed.
- **Boundary discipline.** Public core/clients and the private server are coupled only through the versioned sync protocol.
- **Privacy.** No financial data, credentials, keys, or sensitive device paths in issues, pull requests, logs, screenshots, or project fields.

## 2. Roadmap at a glance

| Phase | Objective | Repositories | Exit gate | Indicative size |
| --- | --- | --- | --- | --- |
| P0 Spike | Validate Rust + Tauri on Windows and Android; lock the client stack | clients, core | `SPIKE_RESULTS.md` recorded; Option B or fallback C chosen | 10 working days |
| P1 Core foundation | Domain, use cases, SQLite schema v1, migrations, export | core | Core tests pass; schema v1 frozen; export round-trips | L |
| P2 Sync and server | E2EE protocol, Go server, two-client convergence | core, server | Multi-device simulation, conflict fixtures, snapshot restore, no server plaintext | L |
| P3 Windows MVP | Tauri 2 Windows client with v1 flows | clients | Windows E2E, accessibility audit, 10k performance, signed installer | L |
| P4 Android MVP | Tauri 2 Android client, or Compose fallback | clients | Device tests, TalkBack, background sync, signed AAB | L |
| P5 Release hardening | Localization, accessibility, performance, security, release | all | R6 release gates; signed v1.0 artifacts | M |
| P6 Post-MVP | Budgets, plans, subscriptions, goals, liabilities, specialist assets | all | Separate approved designs and phases | XL |

Critical path: **P0 → P1 → P2 → P3 → P4 → P5**. P6 is not committed and must not leak into v1 controls.

## 3. Phase 0 — Spike and toolchain

**Goal:** produce observed evidence that the Rust + Tauri architecture works on both targets, or trigger the pre-approved fallback.

### Steps

1. **P0.1 Toolchain**
   - Install the Tauri CLI locally in a throwaway workspace; do not add it to the production repos yet.
   - Windows: confirm Rust, MSVC, WebView2, and SQLCipher builds. These are already present on the current machine except the Tauri CLI.
   - Android: install JDK 17 or 21, Android SDK platform and build-tools, NDK, and an emulator; add the `x86_64-linux-android` Rust target and keep `aarch64-linux-android`.
   - Create the spike outside the production repositories. Spike code is not committed.
2. **P0.2 Windows gates**
   - Tauri 2 window with semantic web UI and keyboard-only flows.
   - NVDA or Narrator reads labels, values, validation errors, and table content.
   - Chinese IME works in transaction fields.
   - SQLCipher database opens, writes, and reopens.
   - Argon2id + AEAD end-to-end encryption round-trip; keys never appear in logs.
   - 10,000-transaction virtualized list remains responsive.
   - MSI or NSIS install and uninstall.
3. **P0.3 Android gates**
   - Tauri 2 app on Android 10 and a current release, on emulator and a physical device.
   - TalkBack reads labels, values, and errors; touch and focus order pass a manual audit.
   - SQLCipher and E2EE round-trip on device.
   - WorkManager background sync through a custom Kotlin plugin; idempotent and survives app backgrounding.
   - BiometricPrompt unlock with a Keystore-protected database key.
   - Signed AAB build.
   - 10,000-transaction list remains responsive.
4. **P0.4 Kotlin baseline (comparison only)**
   - Compose Desktop: screen reader, IME, startup, memory, installer.
   - Compose Android: background sync and biometric integration.
5. **P0.5 Decision record**
   - Record exact devices, builds, commands, and observed outcomes in `docs/clients/SPIKE_RESULTS.md`.
   - Lock Option B if all hard gates pass.
   - If Android fails, lock Option C (Tauri Windows + Compose Android) and keep the Rust core.
   - If Windows fails, evaluate the documented alternatives before locking.

### Exit criteria

- All hard gates have observed results.
- The stack decision is recorded and linked from `docs/architecture/CLIENT_STACK_DECISION.md`.
- No production feature code has been written.

## 4. Phase 1 — Core foundation

**Goal:** a tested Rust core and local database with no UI or network dependency.

### Steps

1. **P1.1 Workspace and CI**
   - Cargo workspace with `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, `core-testkit`.
   - CI: format, clippy, tests, `cargo-deny`, license allowlist.
2. **P1.2 Money and currency**
   - `Money` with `Int64` minor units, currency code, scale.
   - Checked arithmetic, exact decimal parsing, centralized rounding, property tests.
3. **P1.3 Ledger settings and identity**
   - UUIDv7 identifiers, ledger settings, profile, time zone and calendar policy.
   - Device-local registry model, independent of ledger data.
4. **P1.4 Domain entities**
   - Accounts, category groups and leaves, tags, counterparties, transactions, transfers, opening balances, associations, exchange rates.
   - Domain validation and invariants from R3.
5. **P1.5 Application use cases**
   - Ledger setup, account management, category/tag/counterparty management.
   - Transaction create/edit/soft-delete/restore, filters, dashboard, monthly statistics, reports, settings, exchange rates.
6. **P1.6 Persistence**
   - SQLite schema v1, indexes, constraints, migration framework, SQLCipher.
   - Fixtures and migration preservation tests.
7. **P1.7 Export**
   - Versioned JSON export, deterministic ordering, exact minor units.
   - Round-trip validation against schema v1.
8. **P1.8 Quality gates**
   - 100% branch coverage for money, transfer, balance, and month-boundary logic.
   - At least 80% line coverage for core crates.
   - No UI, network, or platform dependency in the domain crates.

### Exit criteria

- Core test suite passes in CI.
- Schema v1 is frozen; migration tests cover the initial backup and open path.
- Export validates against its schema and round-trips.
- No Tauri or UI code exists in the core workspace.

## 5. Phase 2 — Sync and server

**Goal:** end-to-end encrypted sync between two simulated clients and a Go server, with conflict records and restore.

### Steps

1. **P2.1 Protocol contract**
   - JSON Schemas or OpenAPI for the `/v1` API and operation envelope.
   - Protocol versioning and conformance fixtures shared by Rust and Go.
2. **P2.2 Cryptography**
   - Argon2id KEK, random UMK, per-ledger LDK, X25519 wrapping, Ed25519 signatures, recovery key.
   - Test vectors and key-rotation tests.
3. **P2.3 Client sync engine**
   - Outbox, push/pull/ack, cursor, revision checks, `sync_conflict` records, snapshots.
   - Idempotency and offline/reconnect behavior.
4. **P2.4 Go server**
   - Magic-link auth, device registration/revocation, ledger registration, key envelopes, operation log, snapshots.
   - PostgreSQL metadata, S3-compatible encrypted blob storage, rate limits, structured logs.
5. **P2.5 Integration**
   - Two simulated clients with deterministic operations; convergence, conflict, deletion/restore, snapshot restore.
   - Protocol conformance tests against the same fixtures as the server.
6. **P2.6 Security review**
   - Threat-model review, key handling, log redaction, dependency and SBOM checks.

### Exit criteria

- Two clients converge on the same ledger state.
- Financial conflicts produce reviewable records, not silent overwrites.
- Snapshot restore works on a fresh client.
- Server stores no plaintext ledger data, display names, or keys.
- Protocol fixtures pass on both Rust and Go.

## 6. Phase 3 — Windows MVP

**Goal:** a Windows Tauri application implementing the v1 product scope.

### Steps

1. **P3.1 Shell and IPC**
   - Tauri 2 project, React + TypeScript UI, generated IPC types, design tokens, routing, error handling.
2. **P3.2 Ledger setup and selector**
   - First-run setup, multiple ledgers, switching, unavailable-store recovery.
3. **P3.3 Master data**
   - Accounts, two-level categories, tags, counterparties, archive behavior.
4. **P3.4 Transactions**
   - Expense, income, transfers, opening balances, history, search/filter, edit, soft delete/restore, recovery view.
5. **P3.5 Dashboard and reports**
   - Balances, monthly totals, day detail, cash-flow summary, category ranking, missing-rate states.
6. **P3.6 Settings and localization**
   - Language, reporting currency, manual rates, ledger profile, sync settings.
   - English and Simplified Chinese; keyboard shortcuts; full accessibility pass.
7. **P3.7 Sync UI**
   - Sync status, manual sync, device management, conflict review queue.
8. **P3.8 Packaging and validation**
   - MSI/NSIS, signed installer, E2E flows, 10k performance, screen-reader and IME checks.

### Exit criteria

- All v1 flows work offline and with sync enabled.
- Windows accessibility and Chinese IME checks pass.
- 10,000 transactions remain usable.
- Signed installer installs, upgrades, and uninstalls.
- No placeholder controls.

## 7. Phase 4 — Android MVP

**Goal:** an Android client with the same v1 product scope and native platform behavior.

### Steps

1. **P4.1 Android shell**
   - Tauri 2 Android app, or the approved Compose fallback if Phase 0 required it.
   - Adapt navigation, back handling, and layouts.
2. **P4.2 Platform plugins**
   - WorkManager background sync, BiometricPrompt, Keystore, SAF export, notifications.
3. **P4.3 Core flows**
   - Setup/selector, accounts, categories, tags, counterparties, transactions, dashboard, reports, settings.
4. **P4.4 Sync and app lock**
   - Sync status, conflict queue, optional biometric lock, process-death recovery.
5. **P4.5 Validation**
   - TalkBack, localization, 10k performance, signed AAB, device matrix.

### Exit criteria

- MVP flows pass on Android 10 and a current release.
- Background sync is idempotent and honest about last-sync state.
- No data loss on process death or relaunch.
- Signed AAB is ready for internal testing.

## 8. Phase 5 — Release hardening

**Goal:** meet the R6 release gates for v1.0.

### Steps

1. Localization completeness and layout review for English and Simplified Chinese.
2. Accessibility audit with screen readers, keyboard, contrast, and scalable text.
3. Performance budgets: startup, search, 10k lists, memory.
4. Security review: crypto, auth, SQLCipher, logging, dependency audit, SBOM.
5. Migration, backup, restore, and disaster-recovery drills.
6. CI/CD release pipelines, signing, MSI/NSIS, AAB, checksums.
7. Documentation: README, CONTRIBUTING, SECURITY, CODE_OF_CONDUCT, CHANGELOG, user guide.
8. Release candidate validation against all R6 gates.

### Exit criteria

- Every R6 release gate has observed evidence.
- Signed artifacts and SBOM are published.
- No known critical or high vulnerability remains.
- Rollback and incident procedures are documented.

## 9. Phase 6 — Post-MVP

Deferred features, each requiring a separate design and approval:

- Budgets, recurring plans, subscriptions.
- Savings goals, wishes, reconciliation flags, note presets.
- Liabilities, specialist assets, advanced transactions.
- Import, automatic migration, auto-update, deeper platform integrations.
- Collaboration or shared ledgers only after a separate product decision.

## 10. Parallel streams and critical path

After Phase 0, three streams can run with clear boundaries:

- **Core stream:** P1 domain and persistence, then P2 crypto/sync.
- **Server stream:** P2 auth, ledger registration, operation log, storage, and deployment; it can start once the protocol contract is stable.
- **Client stream:** P3 shell, navigation, and UI components once IPC contracts exist; full flows depend on P1/P2.

The critical path is P0 → P1 → P2 → P3 → P4 → P5. The server stream can proceed in parallel with P1 because it depends on the protocol, not the Rust domain. The client shell can start early, but no flow is marked complete before its core and sync dependencies are real.

## 11. GitHub Projects

GitHub Projects is the tracking layer; the repositories remain the source of truth for code, issues, and pull requests.

### 11.1 Required scope

The current `gh` token lacks project access. The owner must run:

```bash
gh auth refresh -s project,read:project
```

This is interactive and must be completed by the account owner.

### 11.2 Project structure

Two projects preserve the public/private boundary:

| Project | Visibility | Linked repositories | Purpose |
| --- | --- | --- | --- |
| **Bluewhale Open Source** | Public | `bluewhale_core`, `bluewhale_clients` | Public roadmap, community-visible progress |
| **Bluewhale Server** | Private | `bluewhale_server` | Private server work, security and operations |

Cross-cutting sync work appears in both only with sanitized, non-sensitive titles. Never put server internals, credentials, or financial data in the public project.

### 11.3 Fields

| Field | Type | Values |
| --- | --- | --- |
| Status | Single select | Backlog, Ready, In Progress, In Review, Blocked, Done |
| Phase | Single select | P0 Spike, P1 Core, P2 Sync, P3 Windows, P4 Android, P5 Release, P6 Post-MVP |
| Area | Single select | Domain, Persistence, Sync, Crypto, UI, Windows, Android, Server, QA, Docs |
| Repository | Single select | bluewhale_core, bluewhale_clients, bluewhale_server |
| Size | Single select | S, M, L, XL |
| Gate | Single select | Design, Spike, Implementation, Release |
| Priority | Single select | P0, P1, P2 |

### 11.4 Views

- **Roadmap:** timeline grouped by Phase.
- **Board:** columns by Status.
- **Current phase:** filter to the active Phase.
- **Blocked:** Status = Blocked.
- **Spike evidence:** Gate = Spike, with links to `SPIKE_RESULTS.md`.
- **My items:** assignee-based.
- **Server-private:** only in the private project.

### 11.5 Labels and milestones

Repository labels:

- `phase:p0-spike` … `phase:p6-post-mvp`
- `area:domain`, `area:persistence`, `area:sync`, `area:crypto`, `area:ui`, `area:windows`, `area:android`, `area:server`, `area:qa`, `area:docs`
- `type:design`, `type:feature`, `type:test`, `type:bug`, `type:chore`, `type:security`
- `gate:spike`, `gate:release`
- `status:blocked`

Milestones are repository-scoped; use them for releases such as `v0.1-core`, `v0.1-clients`, and `v1.0`. Use the project Phase field for cross-repository planning.

### 11.6 Issue and pull-request templates

Add templates to each repository:

- **Design decision:** problem, proposed contract, alternatives, migration impact, approval link.
- **Spike:** gates, environment, commands, observed results, decision.
- **Feature:** user outcome, design reference, acceptance criteria, tests, localization/accessibility impact.
- **Bug:** version, platform, reproduction, expected/actual, no financial data.
- **Release:** gate checklist, artifacts, rollback.

The pull-request template records scope, design reference, tests run, migration impact, accessibility/localization, screenshots without financial data, and DCO sign-off.

### 11.7 Automation

Configure project workflows in the GitHub UI:

- Auto-add new issues and pull requests from linked repositories.
- Item closed → Status Done.
- Pull request linked to an issue → Status In Review.
- Pull request merged → Status Done.
- Label `status:blocked` → Status Blocked.

### 11.8 Seeding order

Seed Phase 0 first, then Phase 1 epics:

1. P0 Windows spike gates.
2. P0 Android toolchain setup.
3. P0 Android spike gates.
4. P0 Kotlin baseline comparison.
5. P0 record results and lock the stack.
6. P1 workspace and CI.
7. P1 money and currency.
8. P1 entities and use cases.
9. P1 SQLite schema and migrations.
10. P1 export.

Each epic links to its design document and contains task checklists. Sub-issues are used where available.

### 11.9 Initialization commands

After the scope refresh:

```bash
gh project create --owner BerryUIKI --title "Bluewhale Open Source"
gh project create --owner BerryUIKI --title "Bluewhale Server"
gh project link <open-source-project-number> --owner BerryUIKI --repo BerryUIKI/bluewhale_core
gh project link <open-source-project-number> --owner BerryUIKI --repo BerryUIKI/bluewhale_clients
gh project link <server-project-number> --owner BerryUIKI --repo BerryUIKI/bluewhale_server
gh project field-create <open-source-project-number> --owner BerryUIKI --name "Phase" --data-type SINGLE_SELECT --single-select-options "P0 Spike,P1 Core,P2 Sync,P3 Windows,P4 Android,P5 Release,P6 Post-MVP"
gh project field-create <open-source-project-number> --owner BerryUIKI --name "Area" --data-type SINGLE_SELECT --single-select-options "Domain,Persistence,Sync,Crypto,UI,Windows,Android,Server,QA,Docs"
gh project field-create <open-source-project-number> --owner BerryUIKI --name "Size" --data-type SINGLE_SELECT --single-select-options "S,M,L,XL"
gh project field-create <open-source-project-number> --owner BerryUIKI --name "Gate" --data-type SINGLE_SELECT --single-select-options "Design,Spike,Implementation,Release"
gh project field-create <open-source-project-number> --owner BerryUIKI --name "Priority" --data-type SINGLE_SELECT --single-select-options "P0,P1,P2"
```

Project visibility is set in the GitHub UI when public visibility is required. Do not make the server project public.

## 12. Definition of Ready and Done

**Ready:**

- Approved design document or an explicit design exemption.
- Clear acceptance criteria and test plan.
- Known repository, dependencies, and migration impact.
- No unresolved blocking decision.

**Done:**

- Acceptance criteria met; behavior is real, not a placeholder.
- Tests and builds observed; results recorded.
- Migration impact, accessibility, and localization addressed.
- Documentation and changelog updated.
- No secrets, financial data, or unrelated changes.
- Follow-up work is explicit.

## 13. Risks and checkpoints

| Checkpoint | Risk | Action |
| --- | --- | --- |
| End of Phase 0 | Tauri Android or Windows gates fail | Apply the pre-approved fallback; record the decision |
| End of Phase 1 | Core schema or money model changes after freeze | Reopen R3; no silent migration |
| End of Phase 2 | Sync convergence or key recovery fails | Block client MVP; fix protocol and fixtures |
| Mid Phase 3 | UI drifts from core rules | Review IPC boundaries; no financial logic in TypeScript |
| Phase 4 | Android background limits | Adjust sync expectations; keep manual sync |
| Phase 5 | Release gates incomplete | Delay release; do not ship unverified |

## 14. Immediate next actions

1. Run `gh auth refresh -s project,read:project` as the account owner.
2. Create the two GitHub Projects and link the repositories.
3. Create the Phase 0 issues and add them to the public project.
4. Install the Tauri CLI and start the Windows spike; install the Android toolchain in parallel.
5. Record spike evidence and lock the client stack.
6. Begin Phase 1 only after Phase 0 exit criteria pass.

This plan is updated as phases complete; material changes require a recorded decision.
