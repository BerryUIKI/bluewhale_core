# Bluewhale Quality and Delivery Plan

## Document Status

- Status: Approved (R6 review)
- Review: R6 — quality and delivery
- Date: 2026-11-23
- Scope: `bluewhale_core`, `bluewhale_clients`, `bluewhale_server`
- Depends on: approved R1 product specification; approved R2 architecture; approved R3 data model; approved R4 sync and server design; approved R5 client design
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: none

This document defines the approved test strategy, required CI checks, release gates, contribution and license policy, security policy, and documentation standards. Decisions **Q1–Q10** are accepted as recommended. Approval does not authorize implementation; the Rust + Tauri spike and the release gates remain prerequisites.

## 1. Goals

- Keep `main` releasable on every supported repository.
- Make every behavioral change carry proportionate automated validation and observed evidence.
- Keep financial data, credentials, and keys out of logs, telemetry, issues, and pull requests.
- Produce signed, reproducible, auditable release artifacts.
- Define a clear open-source contribution path for `bluewhale_core` and `bluewhale_clients`, and a separate private process for `bluewhale_server`.
- Make release, rollback, and incident response explicit.

## 2. Quality model

### 2.1 Test levels

| Level | Scope | Tooling |
| --- | --- | --- |
| Domain unit and property tests | Money, currency, validation, balances, transfers, calendar, reports | Rust `cargo test`, property-based tests |
| Application use-case tests | Multi-record orchestration, atomicity, ordering | Rust with in-memory fakes |
| Persistence tests | SQLite schema, migrations, soft deletion, restore | Temporary and fixture databases |
| Sync and crypto tests | Envelope, idempotency, conflicts, snapshots, key hierarchy | Rust integration tests and deterministic vectors |
| Tauri command tests | IPC mapping, error translation, active-ledger isolation | Rust integration tests |
| UI component tests | Forms, tables, dialogs, localization, accessibility | Vitest, Testing Library, axe-core |
| End-to-end tests | Critical user journeys on Windows and Android | Tauri driver/Playwright, Android instrumented tests |
| Manual validation | Screen readers, Chinese IME, platform packaging, device behavior | Recorded checklists |

### 2.2 Coverage expectations

- 100% branch coverage for money, currency-scale validation, transfer construction, balance calculation, and month-boundary calculations.
- At least 80% line coverage for the public core crates, measured on the host target.
- UI coverage is informational; critical flows require explicit end-to-end tests.
- Every schema version has a migration fixture and a preservation test.
- Coverage thresholds never justify assertion-free or implementation-coupled tests.

### 2.3 Required test scenarios

- Expense, income, and transfer effects; cross-currency transfer exactness.
- Edit, soft delete, restore, and relaunch persistence.
- Month and year boundaries under the ledger time zone, including DST transitions.
- Money parsing, scale rejection, checked overflow, and centralized rounding.
- Missing exchange rates and incomplete-report states.
- Duplicate actions, retries, save failures, and rollback.
- Migration from every supported schema version, including archived and soft-deleted records.
- Sync idempotency, multi-device convergence, conflict records, snapshot restore, and device revocation.
- Localization completeness and locale-invariant stored values.
- Keyboard, focus order, screen-reader labels, and chart alternatives.

## 3. Required checks per change

- **Rust:** `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`, `cargo deny check`, migration and crypto tests.
- **TypeScript:** ESLint/Prettier, `tsc --noEmit`, Vitest, axe checks, production build.
- **Tauri:** command integration tests and a Windows end-to-end smoke test.
- **Android:** Gradle build, unit tests, instrumented tests on an emulator, AAB assembly.
- **Go server (private):** `gofmt`, `go vet`, `staticcheck`, `go test`, `govulncheck`, protocol conformance.
- **Protocol:** JSON Schema validation and client/server conformance tests against the same versioned fixtures.
- **Security:** secret scanning, dependency review, SBOM generation, license allowlist check.
- **Documentation:** Markdown lint, link checks, LF line endings, design-doc references.
- **Localization:** key completeness for English and Simplified Chinese; no hardcoded user-facing strings.
- **Accessibility:** automated axe checks plus a manual checklist for changed UI surfaces.
- **Performance:** core benchmarks and 10,000-transaction UI fixtures for changes that affect listing, search, or aggregation.

A pull request must not merge with unobserved, skipped, or failing relevant checks. Flaky tests are fixed or quarantined with an issue; they are never silently retried into green.

## 4. CI pipelines

GitHub Actions, scoped by repository:

| Workflow | Runner | Checks |
| --- | --- | --- |
| `core.yml` | Linux | Format, clippy, tests, property tests, migrations, cargo-deny, SBOM |
| `ui.yml` | Linux | Lint, typecheck, unit tests, axe, production build |
| `clients-windows.yml` | Windows | Desktop build, command tests, E2E smoke, MSI/NSIS packaging |
| `clients-android.yml` | Linux with Android SDK/NDK | Build, unit tests, emulator instrumented tests, AAB |
| `server.yml` | Linux (private repo) | Go checks, vulnerability scan, protocol conformance |
| `release.yml` | Tag-triggered | Signed artifacts, checksums, SBOM, release notes |

- Caches: Cargo registry and target, Node package manager, Gradle.
- Concurrency: cancel superseded runs on branches; never cancel a release run.
- Branch protection on `main`: required checks, linear history, no force push, no direct code pushes by external contributors.
- Artifacts: test reports, coverage, installers, AAB, SBOM, checksums; bounded retention.
- The private server repository runs its own pipeline; it consumes the public protocol contracts and publishes no client code.

## 5. Branching, review, and commits

- **Trunk-based development:** `main` is the trunk and must remain releasable.
- Code changes use short-lived branches and focused pull requests. Long-lived `develop` or release branches are not used.
- Maintainers may land documentation and trivial maintenance directly on `main`; external contributions always use pull requests.
- Merge strategy: squash or rebase for linear history; no merge commits on `main`.
- Required review: at least one maintainer approval for code; design changes require the relevant design document to be approved first.
- Conventional commit messages (`feat:`, `fix:`, `docs:`, `test:`, `chore:`) and focused diffs.
- Release tags use `vMAJOR.MINOR.PATCH` from `main`. Hotfixes use a short-lived branch, then a new patch tag.
- No force-push to `main`; no rewriting shared history.
- Pull requests must state scope, design reference, tests run, migration impact, and known limitations.

## 6. Licensing and contribution policy

- `bluewhale_core` and `bluewhale_clients`: Apache License 2.0, with `LICENSE` and `NOTICE` (Copyright 2026 BerryUIKI).
- `bluewhale_server`: proprietary, closed source, no external contributions.
- Inbound contributions are accepted under Apache 2.0 via Developer Certificate of Origin sign-off (`git commit -s`). A separate CLA is not required.
- Per-file license headers are not required; the repository-level `LICENSE` and `NOTICE` govern.
- `CONTRIBUTING.md` covers setup, build and test commands, the design-document gate, commit and pull-request rules, DCO, and code style.
- `CODE_OF_CONDUCT.md` uses the Contributor Covenant.
- `SECURITY.md` defines private vulnerability reporting, supported versions, and response expectations. Security reports are never filed as public issues.
- Third-party dependency licenses use an allowlist (Apache-2.0, MIT, BSD, ISC, and similarly permissive licenses). Copyleft licenses require an explicit recorded exception.
- Every release includes an SBOM and dependency-license report.

## 7. Release gates

A release is allowed only when:

- All required CI checks are green on the release commit.
- The relevant design documents are approved and the Rust + Tauri spike evidence is recorded.
- Migration tests pass, and a backup/restore path is verified for the release.
- Accessibility and localization checks are complete for changed surfaces.
- Performance budgets are met for startup, search, and 10,000-transaction lists.
- Security review is complete for changes touching authentication, cryptography, sync, or file access.
- No known critical or high vulnerability remains unresolved.
- Artifacts are signed; checksums and an SBOM are published.
- Release notes list behavior changes, known limitations, and rollback instructions.
- No financial data, credentials, keys, or tokens appear in artifacts, logs, or release notes.

Client releases:

- Windows: signed MSI or NSIS installer; upgrade and uninstall tested; minimum Windows version verified.
- Android: signed AAB with a monotonic version code; internal testing track before production; minimum API verified.
- Server: deployed privately through staging; database migrations rehearsed; observability and rollback plan in place; protocol version compatibility verified.

## 8. Definition of Done

A task is complete only when:

- Acceptance criteria are met and the change stays in scope.
- Domain and architecture rules are respected.
- Tests are added and observed; builds are actually run.
- Migration impact is documented, including `none` when applicable.
- Accessibility and localization implications are addressed for user-facing work.
- Documentation and the changelog are updated.
- No secrets, financial data, generated artifacts, or unrelated changes are included.
- Known limitations and follow-up work are explicit.

## 9. Documentation standards

- Repository root: `README.md`, `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `CHANGELOG.md`, `LICENSE`, `NOTICE`, and `CLAUDE.md`.
- Decision documents: R1–R6 and feature-specific design documents under `docs/`.
- Future architecture or product decisions use short ADRs under `docs/adr/` once the review phase is complete.
- Documentation is English; user-facing product text is English and Simplified Chinese.
- A behavior or schema change updates its design document and the changelog in the same change.
- Markdown files use LF line endings and pass link and lint checks.

## 10. Observability, privacy, and incident response

- Client logs are structured and redacted; crash reporting is opt-in and excludes financial content.
- Server telemetry uses OpenTelemetry metrics and traces without financial payloads; audit logs cover authentication, device, and sync metadata only.
- Telemetry is minimal, aggregate where possible, and never includes ledger content.
- Incident response includes device revocation, key rotation, snapshot restore, server rollback, and a written postmortem.
- Vulnerability disclosure follows `SECURITY.md`; affected releases and mitigations are documented after a fix is available.

## 11. Decisions accepted at R6

Approved by the project owner on 2026-11-23. The recommendations below are accepted.

| ID | Decision | Recommendation |
| --- | --- | --- |
| Q1 | Contribution sign-off | Developer Certificate of Origin; no CLA |
| Q2 | Merge policy | Trunk-based, short-lived PRs, squash/rebase, docs direct by maintainers |
| Q3 | CI provider | GitHub Actions |
| Q4 | Versioning | Semantic Versioning with `vX.Y.Z` tags |
| Q5 | Changelog | Keep a Changelog format plus release notes |
| Q6 | Security reporting | `SECURITY.md` with private advisories |
| Q7 | Dependency policy | License allowlist plus `cargo-deny`, npm audit, and `govulncheck` |
| Q8 | Coverage | 100% branch for critical money/transfer logic; 80% line for core; UI informational |
| Q9 | Performance budgets | Startup, search, and 10,000-transaction list budgets validated in the spike |
| Q10 | Release cadence | Milestone-based, no fixed calendar promises |

## 12. Review closure

R6 was approved on 2026-11-23; the R1–R6 review sequence is complete. The next gates are:

1. Run the 10-day Rust + Tauri spike and record observed evidence in `docs/clients/SPIKE_RESULTS.md`.
2. Resolve any spike failure through the pre-approved Android Compose fallback or the documented alternatives.
3. Produce the implementation plan and milestone roadmap from the approved designs.

R6 approval does not authorize implementation before the spike gates pass.
