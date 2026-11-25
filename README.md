# bluewhale_core

Shared, platform-independent core for the Bluewhale product line.

## Status

Pre-alpha. The product, architecture, domain, sync, client, and quality reviews (R1-R6) are approved, and the Phase 0 Rust + Tauri spike is complete with observed evidence in [SPIKE_RESULTS.md](docs/clients/SPIKE_RESULTS.md). The core foundation is implemented: domain rules, use cases, SQLCipher persistence with schema v1, the versioned JSON export, and the sync protocol v1 contract. `core-crypto` and the client sync engine are not implemented yet, and nothing is published.

The Windows and Android applications live in [bluewhale_clients](https://github.com/BerryUIKI/bluewhale_clients). The sync server is a separate private repository.

## Workspace

| Crate | Responsibility |
| --- | --- |
| `core-domain` | Money, accounts, categories, transactions, transfers, reports, policies, and repository traits |
| `core-application` | Use cases, orchestration, transaction boundaries, and validation flow |
| `core-persistence` | SQLite/SQLCipher schema, migrations, and repository implementations |
| `core-sync` | Operation log, sync envelopes, cursors, and conflict application |
| `core-crypto` | Argon2id key derivation, AEAD, key wrapping, and test vectors |
| `core-testkit` | Fakes, fixtures, and deterministic clocks for tests |

The dependency direction is `core-application` and `core-persistence` toward `core-domain`, never the reverse. `core-domain` has no I/O and no client dependencies.

## Contracts

Language-neutral public contracts live in [`contracts`](contracts/README.md): the versioned ledger export and the sync protocol envelope and `/v1` API, with JSON Schemas, an OpenAPI description, and conformance fixtures.

## Documentation

Start at the [documentation index](docs/README.md). The approved product and technical decisions are in [docs/product](docs/product/PRODUCT_SPEC.md), [docs/architecture](docs/architecture/ARCHITECTURE.md), [docs/domain](docs/domain/DATA_MODEL.md), [docs/sync](docs/sync/SYNC_AND_SERVER_DESIGN.md), [docs/clients](docs/clients/CLIENT_DESIGN.md), and [docs/quality](docs/quality/QUALITY_AND_DELIVERY.md).

## Build and test

Rust 1.97.1 is selected by `rust-toolchain.toml`.

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
python -I scripts/check_contracts.py
```

The contract check requires `jsonschema`, `PyYAML`, and `openapi-spec-validator`. See [CONTRIBUTING.md](CONTRIBUTING.md) for the full setup, including the native Perl and `nmake` requirement for the vendored SQLCipher/OpenSSL build on Windows.

## Licensing

Copyright 2026 BerryUIKI.

Licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE) for attribution.

## Contributing

Read [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md). Security issues are reported privately according to [SECURITY.md](SECURITY.md). Contributions are accepted under the Developer Certificate of Origin; sign off every commit with `git commit -s`.
