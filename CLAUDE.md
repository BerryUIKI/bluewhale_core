# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Current status

Pre-alpha. The design reviews R1-R6 are approved and the Phase 0 Rust + Tauri spike is complete, with observed Windows and Android evidence in `docs/clients/SPIKE_RESULTS.md`. Phase 1 core foundation is implemented: `core-domain`, `core-application`, `core-testkit`, and `core-persistence` have functional APIs and passing tests, and local schema v1 and the versioned export are implemented. `core-sync` implements the sync protocol v1 contract in `contracts/sync/v1`; `core-crypto` is still a skeleton. Nothing is published.

## Purpose and release boundary

`bluewhale_core` is the shared, platform-independent core for the Bluewhale product line and is intended to be open source. The Windows and Android client applications live in the sibling `bluewhale_clients` repository. `bluewhale_server` is proprietary and closed source, and `bluewhale-ledger` is a proprietary, Apple-native application.

Rules:

- The private Apple application (`bluewhale-ledger`) is the behavioral reference for this core and is not modified by this project. Do not copy its source code, assets, test fixtures, or design documents into this public repository without an explicit, recorded licensing decision.
- Keep this repository self-contained and buildable without closed-source components.
- The project is licensed under the Apache License, Version 2.0 (`LICENSE`), with attribution in `NOTICE` (Copyright 2026 BerryUIKI). Keep both files intact and clear any dependency-license question before adding a dependency.
- Do not introduce dependencies on a specific client UI framework or on closed-source server internals.

## Workspace layout

| Path | Responsibility |
| --- | --- |
| `crates/core-domain` | Money, accounts, categories, transactions, transfers, reports, policies, and repository traits. No I/O. |
| `crates/core-application` | Use cases, orchestration, transaction boundaries, and validation flow. |
| `crates/core-persistence` | SQLite/SQLCipher schema, migrations, and repository implementations. |
| `crates/core-sync` | Operation log, sync envelopes, cursors, and conflict application. |
| `crates/core-crypto` | Argon2id key derivation, AEAD, key wrapping, and test vectors. |
| `crates/core-testkit` | Fakes, fixtures, and deterministic clocks for tests. |

Layering rules:

- `core-domain` must not depend on any other workspace crate, on persistence, on networking, or on client code.
- `core-application` and `core-persistence` depend on `core-domain`, never the reverse.
- `core-persistence` implements repository traits defined in `core-domain`; it is never imported by domain or application code.
- `core-testkit` is test-only support and must never be a normal dependency of a production crate.
- Money is checked `Int64` minor units plus currency and scale. Never introduce floating point for money.

## Commands

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p core-domain <test_name>          # single test by name
cargo doc --workspace --no-deps
cargo deny check
```

Vendored SQLCipher/OpenSSL builds on Windows need a native Windows Perl (for example Strawberry Perl) and `nmake` on `PATH`; Git's Cygwin Perl is rejected. Run persistence or workspace commands from a shell that has both. The Phase 0 evidence in `docs/clients/SPIKE_RESULTS.md` and the procedure in `docs/development/setup.md` record the exact setup.

## Development workflow

- Trunk-based development on `main`. Keep `main` releasable and commit small focused changes directly to `main`; use a short-lived branch only when a change genuinely needs isolation.
- Conventional Commits (`feat:`, `fix:`, `docs:`, `refactor:`, `test:`, `build:`, `ci:`, `chore:`).
- Sign off every commit with `git commit -s` (Developer Certificate of Origin).
- A behavior, schema, or protocol change updates its design document and `CHANGELOG.md` in the same change.
- New architecture or product decisions use an ADR under `docs/adr/` after the R1-R6 review phase.
- Never include real financial data, credentials, encryption keys, or ciphertext in commits, logs, issues, screenshots, or tests. Use synthetic fixtures.

## Documentation

Start at `docs/README.md`. The approved decisions are in `docs/product/PRODUCT_SPEC.md`, `docs/architecture/ARCHITECTURE.md`, `docs/domain/DATA_MODEL.md`, `docs/sync/SYNC_AND_SERVER_DESIGN.md`, `docs/clients/CLIENT_DESIGN.md`, and `docs/quality/QUALITY_AND_DELIVERY.md`. The Phase 0 evidence is in `docs/clients/SPIKE_RESULTS.md`.

Do not present planned or unverified behavior as implemented. Record observed results with the exact command, environment, and date.
