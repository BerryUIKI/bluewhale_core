# Repository Layout

## Document Status

- Status: Draft for owner review
- Date: 2026-11-23
- Scope: production layout for `bluewhale_core`, `bluewhale_clients`, and `bluewhale_server`
- Basis: approved decisions R1-R6 and the Phase 0 findings recorded in `docs/clients/SPIKE_RESULTS.md`

This document describes the target layout of the three repositories and what already exists. It is not a claim that the applications are implemented: the Phase 0 spike is still open and the crates are placeholders.

## 1. Principles

- **Release boundary = repository boundary.** Public core, public clients, and the private server are separate repositories with separate licenses and CI.
- **One build system per repository.** Cargo for core and the client Rust code, pnpm for the web UI, Go modules for the server.
- **Financial logic lives in the core.** UI components, Tauri commands, Kotlin plugins, and server handlers never implement money rules.
- **Contracts are versioned and shared.** The local schema, sync protocol, and export format have explicit versions and fixtures in `bluewhale_core/contracts` (planned).
- **Generated code is isolated and drift-checked.** IPC bindings, the Android Gradle project, sqlc output, and OpenAPI models have fixed locations and a CI check that regenerating produces no diff.
- **Every repository is independently buildable.** Cross-repository dependencies are pinned; no repository relies on a sibling checkout being present.
- **Open-source hygiene is part of the layout.** Contribution, security, conduct, changelog, issue and pull-request templates, CODEOWNERS, and dependency updates exist from the first commit.

## 2. Release boundaries

| Repository | Visibility | License | Contents |
| --- | --- | --- | --- |
| `bluewhale_core` | Public | Apache-2.0 + NOTICE | Domain model, use cases, persistence, sync, crypto, testkit, contracts, project-wide design documents |
| `bluewhale_clients` | Public | Apache-2.0 + NOTICE | Windows and Android applications, shared UI, IPC bindings, end-to-end tests |
| `bluewhale_server` | Private | Proprietary, all rights reserved | Account, device, sync, snapshot service; deployment and migrations |
| `bluewhale-ledger` | Private | Proprietary | Apple-native application used only as a behavioral reference; never copied |

## 3. `bluewhale_core`

Implemented today:

```text
bluewhale_core/
├── Cargo.toml                     # workspace members, shared package metadata, workspace dependencies
├── Cargo.lock                     # committed for reproducible CI
├── rust-toolchain.toml            # Rust 1.97.1, rustfmt and clippy
├── rustfmt.toml, clippy.toml      # formatting and MSRV lint configuration
├── deny.toml                      # license allowlist, bans, source policy
├── .editorconfig, .gitattributes, .gitignore
├── README.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md
├── LICENSE, NOTICE, CLAUDE.md
├── .github/
│   ├── CODEOWNERS, dependabot.yml, PULL_REQUEST_TEMPLATE.md
│   ├── ISSUE_TEMPLATE/{bug_report,feature_request,design_decision}.yml, config.yml
│   └── workflows/{core,audit}.yml
├── crates/
│   ├── core-domain/               # money, accounts, categories, transactions, transfers, reports, repository traits
│   ├── core-application/          # use cases and orchestration
│   ├── core-persistence/          # SQLite schema, migrations, repository implementations
│   ├── core-sync/                 # operation log, envelopes, cursors, conflicts
│   ├── core-crypto/               # Argon2id, AEAD, key wrapping, test vectors
│   └── core-testkit/              # fakes, fixtures, deterministic clocks
└── docs/
    ├── README.md                  # documentation index
    ├── adr/                       # ADR process and template
    ├── product/, architecture/, domain/, sync/, clients/, quality/, plan/
```

Planned next, in phase order:

```text
├── contracts/                     # language-neutral sync/export JSON Schemas, OpenAPI, fixtures
├── xtask/                         # repository checks: layering, banned tokens, license copies
├── fuzz/                          # cargo-fuzz targets for money, rates, envelopes, export
├── tests/integration/             # cross-crate integration tests over real SQLite
├── crates/core-persistence/migrations/   # forward-only schema migrations
├── docs/development/              # setup, build-test, release, dependency policy
├── docs/reference/                # public API, feature flags, version compatibility
└── docs/adr/0001-0004             # workspace layout, core consumption, toolchain and release
```

Crate rules:

- `core-domain` depends on no other workspace crate and performs no I/O.
- `core-application` and `core-persistence` depend on `core-domain`; persistence implements repository traits and is never imported by the domain.
- `core-testkit` is never a normal dependency of a production crate.
- `core-application` and `core-persistence` are siblings: application code receives repository traits, not SQLite handles.

## 4. `bluewhale_clients`

Implemented today:

```text
bluewhale_clients/
├── package.json, pnpm-workspace.yaml, .npmrc, .node-version
├── Cargo.toml, Cargo.lock, rust-toolchain.toml, rustfmt.toml, deny.toml
├── .editorconfig, .gitattributes, .gitignore
├── README.md, CONTRIBUTING.md, CODE_OF_CONDUCT.md, SECURITY.md, CHANGELOG.md
├── LICENSE, NOTICE, CLAUDE.md
├── crates/
│   ├── clients-runtime/           # composes core use cases for both clients
│   ├── clients-ipc/               # Tauri commands, DTOs, bindings generation
│   └── clients-testkit/           # fake platform services and IPC harnesses
├── desktop/README.md              # Tauri 2 Windows application (Phase 1)
├── android/README.md              # Tauri 2 Android application (Phase 1)
├── ui/README.md                   # shared React UI (Phase 1)
├── packages/{ipc,config,testkit}/README.md
└── e2e/README.md                  # browser, Windows, Android end-to-end tests (Phase 1)
```

Planned next:

```text
├── pnpm-lock.yaml                 # committed once dependencies are added
├── desktop/
│   ├── package.json, index.html, vite.config.ts, tsconfig.json
│   ├── src/                       # React entry and platform services
│   └── src-tauri/                 # tauri.conf.json, capabilities, icons, Rust commands
├── android/
│   ├── package.json, index.html, vite.config.ts, tsconfig.json
│   └── src-tauri/
│       ├── tauri.conf.json, capabilities, Rust commands
│       ├── platform/              # Kotlin plugins: WorkManager, biometrics, Keystore, SAF
│       └── gen/android/           # committed Gradle project; build outputs ignored
├── ui/
│   └── src/{features,components,lib,locales,styles}/
├── packages/ipc/src/bindings.ts   # generated by specta; drift-checked
├── packages/config/               # shared tsconfig, ESLint, Prettier, Vite, Vitest
├── packages/testkit/              # mock transport, fixtures, axe helpers
├── scripts/                       # core pin, bindings drift, version and layering checks
└── e2e/{web,windows}/
```

Rules:

- `desktop` and `android` are separate Tauri applications that share `ui` and the client crates. The Android fallback (a Compose application) can replace `android/` without touching `desktop/` or `ui/`.
- The shared UI never performs arithmetic on money. The Rust side owns parsing, arithmetic, and formatting strings.
- `android/src-tauri/gen/android` is committed because it contains the signing configuration and manifest customizations; build outputs, `local.properties`, and keystores are ignored.
- Generated TypeScript bindings are committed and CI fails if regenerating them produces a diff.

## 5. `bluewhale_server`

Implemented today:

```text
bluewhale_server/
├── go.mod
├── Makefile                       # build, run, test, fmt, vet, lint, vuln, tidy
├── .editorconfig, .gitattributes, .gitignore
├── README.md, LICENSE, CLAUDE.md
├── docs/README.md
├── cmd/server/main.go             # version entry point; service not implemented
└── internal/
    ├── app/, apperr/, audit/, auth/, config/, device/
    ├── httpapi/, ledger/, mailer/, maintenance/
    └── observability/, storage/, store/, sync/, testkit/, version/
```

Planned next:

```text
├── api/{openapi.yaml,oapi-codegen.yaml,protocol/}   # pinned contract artifacts and provenance
├── migrations/                    # forward-only goose migrations and checksums
├── deploy/{Dockerfile,compose.yaml,.env.example,config/}
├── internal/store/{queries,postgres/sqlcgen}/       # sqlc queries and generated code
├── internal/httpapi/openapi/openapi.gen.go          # generated strict server
├── scripts/                       # migration and OpenAPI checks
└── .github/                       # CI, release, templates, CODEOWNERS, dependabot
```

Rules:

- The server stores opaque encrypted envelopes and routing metadata. It never learns account names, amounts, or categories.
- Protocol artifacts under `api/protocol` are copied from `bluewhale_core/contracts` with provenance (tag, commit, per-file SHA-256) and retain the Apache-2.0 license for those files only.
- Migrations are forward-only; released migrations are checksummed and never edited.

## 6. Cross-repository dependency and versioning

The clients need the core crates, but the repositories are separate and must build independently.

- **Phase A (now): pinned git revision.** The clients declare the six core crates under `[workspace.dependencies]` with `git = "https://github.com/BerryUIKI/bluewhale_core"` and a full 40-hex `rev`; the annotated crate tag `vX.Y.Z` is recorded in the clients' `docs/development/core-pin.md`. `Cargo.lock` is committed.
- **Local side-by-side work uses an untracked override.** A developer with `bluewhale_core` checked out next to `bluewhale_clients` can create a repo-local `.cargo/local-core.toml` (gitignored) with `[patch."https://github.com/BerryUIKI/bluewhale_core"]` entries pointing at the local crates. The key must match the git URL exactly; Cargo silently ignores a mismatched patch. The override is never written to the global user configuration and CI fails if it exists or is tracked.
- **Until the first core tag is pushed, the client crates do not depend on the core.** The dependency is added in P3.1. This keeps the initial commits honest and buildable.
- **Phase B (after an owner-approved ADR): registry pins.** Once the six crates.io names are verified and controlled, the publication decision is recorded, and the schema/protocol/export contracts are frozen at 1.0, the pins change to `version = "=X.Y.Z"` and the local patch key changes to `[patch.crates-io]` in the same change.
- **Publishing to crates.io is deferred.** Availability was checked on 2026-11-23: `core-crypto` is already taken on crates.io; the other five crate names are free. Phase B must resolve that name (keep the names and publish `core-crypto` under a different name, or rename all six into a `bluewhale-core-*` namespace); ADR `0003` records the decision and the migration path.
- **Compatibility is explicit.** The core records the public API version, local schema version, sync protocol version, and export schema version in `docs/reference/versions.md` (planned). A breaking change requires a migration and a release note.

The server pins the protocol version, not a core crate. Protocol fixtures are copied into the server with provenance, so a client and server can be tested against the same contract.

## 7. Generated code and generated projects

| Artifact | Location | Committed | Drift check |
| --- | --- | --- | --- |
| TypeScript IPC bindings | `bluewhale_clients/packages/ipc/src/bindings.ts` | Yes | Regenerate and fail on diff |
| Android Gradle project | `bluewhale_clients/android/src-tauri/gen/android` | Yes | Wrapper validation; build outputs ignored |
| sqlc code | `bluewhale_server/internal/store/postgres/sqlcgen` | Yes | `go generate` and fail on diff |
| OpenAPI server models | `bluewhale_server/internal/httpapi/openapi` | Yes | `go generate` and fail on diff |
| Protocol fixtures | `bluewhale_core/contracts/fixtures` | Yes | Manifest SHA-256 per case |

Signing material is never committed: Windows code-signing certificates, Android keystores, and their passwords live only in release environments and the platform secret store.

## 8. Governance files

| File | Core | Clients | Server |
| --- | --- | --- | --- |
| `README.md` | Yes | Yes | Yes |
| `LICENSE` / `NOTICE` | Apache-2.0 + NOTICE | Apache-2.0 + NOTICE | Proprietary notice, no open-source grant |
| `CONTRIBUTING.md` | Yes | Yes | Yes (internal) |
| `CODE_OF_CONDUCT.md` | Contributor Covenant 2.1 | Contributor Covenant 2.1 | Not required (private) |
| `SECURITY.md` | Yes | Yes | Yes |
| `CHANGELOG.md` | Keep a Changelog | Keep a Changelog | Keep a Changelog |
| `CLAUDE.md` | Yes | Yes | Yes |
| `.github/ISSUE_TEMPLATE`, PR template | Yes | Yes | Yes |
| `CODEOWNERS`, `dependabot.yml` | Yes | Yes | Yes |

## 9. CI matrix

| Repository | Required jobs |
| --- | --- |
| Core | Linux: fmt, clippy, test, docs, cargo-deny. Windows: test. Weekly: RustSec audit |
| Clients | UI: format, lint, typecheck, unit tests, i18n check. Rust: fmt, clippy, test, bindings drift. Windows: Tauri build. Android: debug APK and instrumented tests. Accessibility: axe, keyboard, screen reader |
| Server | gofmt and module hygiene, vet, golangci-lint, race tests with coverage, govulncheck, Docker build, migration check, OpenAPI drift |

Nightly or weekly jobs (fuzzing, benchmarks, dependency review, OpenSSF Scorecard) are added after the first functional release; the layout reserves `fuzz/`, `bench.yml`, and `scorecard.yml` for them.

## 10. What exists now

| Repository | State | Local verification (2026-11-23) |
| --- | --- | --- |
| Core | Workspace skeleton, governance, core CI | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo deny check`, and `actionlint` passed locally; CI has not run yet |
| Clients | Workspace manifests and three placeholder crates; governance | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, and `cargo deny check` passed locally; `pnpm install` has not been run because the applications are not present |
| Server | Go module, package boundaries, version entry point; proprietary license | `go build ./...`, `go vet ./...`, `go test ./...`, and `actionlint` passed locally; `go run ./cmd/server -version` prints the build version |

## 11. Open decisions

1. **Public product name and application identifier (D9).** The layout uses the working name Bluewhale and the provisional identifier `com.berryui.bluewhale`. The identifier must be confirmed before the first signed Play release; it cannot change afterwards.
2. **Windows ARM64.** The layout assumes x64 for v1. If ARM64 is required, MSI/NSIS targets and CI runners must be added before the first release.
3. **UI router.** R5 does not select a router. The UI package will record the choice in an ADR when the first routes land.
4. **Core publication timing and crate names.** Git tags come first; crates.io publication is deferred. `core-crypto` is already taken on crates.io, so the first publish requires either a name resolution or a `bluewhale-core-*` rename of all six crates. The choice and the compatibility SLA are recorded in an ADR before the first client release.

## 12. Verification

Commands used to validate this layout:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo deny check
go build ./...
go vet ./...
go test ./...
actionlint
```
