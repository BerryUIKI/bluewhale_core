# Build and test

## Levels

| Level | Command | What it proves |
| --- | --- | --- |
| Format | `cargo fmt --all --check` | Style is stable and CI-clean |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | No lint warnings in any target |
| Unit and doc tests | `cargo test --workspace` | Crate behavior and documentation examples |
| Single test | `cargo test -p core-domain <test_name>` | One focused test by name |
| Documentation | `cargo doc --workspace --no-deps` | Public items are documented and links resolve |
| Dependency policy | `cargo deny check` | Licenses, advisories, bans, and sources |
| Packaging | `cargo package -p <crate> --no-verify --allow-dirty` | A crate can be packaged with its README, LICENSE, and NOTICE |

## Feature sets

The workspace never runs `--all-features`. Features that change the storage backend are mutually exclusive, so CI selects them explicitly:

- Default: `bundled-sqlcipher` for `core-persistence`.
- `unencrypted-sqlite` is reserved for the approved OS-encryption fallback and docs builds; it is mutually exclusive with SQLCipher and guarded by `compile_error!`.
- `test-support` enables temporary-ledger helpers used by other crates' tests.
- `http` enables the blocking sync transport in `core-sync`.

The exact feature commands are added with the features themselves. P1.6 ships SQLCipher as the only storage backend; `unencrypted-sqlite` and `test-support` are not implemented yet. Until they are, the default workspace commands above are the complete set.

## Integration and migration tests

- Schema v1 has no predecessor, so there is no prior-version fixture yet. `crates/core-persistence/tests/roundtrip.rs` covers opening a keyed database, reopening it, rejecting a wrong key, reading the recorded schema version, and round-tripping every persisted entity.
- A migration fixture for each prior schema version is added with the first post-v1 migration, and it must assert that active, archived, and soft-deleted rows are preserved.
- Export contract tests live in the `core-application` export module: they validate `contracts/export/v1/fixture.json` and exporter output against the JSON Schema and round-trip through the Rust document types.
- `tests/integration` (planned) exercises use cases over a real temporary SQLCipher database and runs the sync convergence tests.
- Crypto known-answer tests live in `crates/core-crypto/tests/vectors.rs` once the vectors are recorded in P1.0.

## Windows

The vendored SQLCipher/OpenSSL build needs a native Windows Perl and `nmake`; see [setup.md](setup.md). Environment changes do not persist between separate CI steps.

## Coverage

- 100% branch coverage is required only for money, currency-scale, transfer, balance, rounding, and month-boundary modules.
- The six public crates carry an 80% line gate.
- Coverage thresholds are enabled as the corresponding modules land (P1.2 and P1.8), not while the crates are empty.
