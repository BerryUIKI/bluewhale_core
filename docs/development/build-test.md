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

The exact feature commands are added with the features themselves in P1.6 and P2.1. Until then, the default workspace commands above are the complete set.

## Integration and migration tests

- `crates/core-persistence/tests/migrations.rs` opens a fixture for every prior schema version, migrates it to the current version, and asserts that active, archived, and soft-deleted rows are preserved.
- `tests/integration` (planned) exercises use cases over a real temporary SQLCipher database and runs the sync convergence and export contract tests.
- Crypto known-answer tests live in `crates/core-crypto/tests/vectors.rs` once the vectors are recorded in P1.0.

## Windows

The vendored SQLCipher/OpenSSL build needs a native Windows Perl and `nmake`; see [setup.md](setup.md). Environment changes do not persist between separate CI steps.

## Coverage

- 100% branch coverage is required only for money, currency-scale, transfer, balance, rounding, and month-boundary modules.
- The six public crates carry an 80% line gate.
- Coverage thresholds are enabled as the corresponding modules land (P1.2 and P1.8), not while the crates are empty.
