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

## Security checks

CI scans the full history with gitleaks, generates a CycloneDX SBOM for every crate (`cargo cyclonedx --format json --spec-version 1.6`) and uploads it as an artifact. `cargo deny check` covers advisories, licenses, bans, and sources on every push and pull request, and dependency review runs on pull requests now that the repository dependency graph is enabled. The P2.6 review, threat model, and residual risks are in [SECURITY_REVIEW.md](../security/SECURITY_REVIEW.md).

## Feature sets

The workspace never runs `--all-features`. Features that change the storage backend are mutually exclusive, so CI selects them explicitly:

- Default: `bundled-sqlcipher` for `core-persistence`.
- `unencrypted-sqlite` is reserved for the approved OS-encryption fallback and docs builds; it is mutually exclusive with SQLCipher and guarded by `compile_error!`.
- `test-support` enables temporary-ledger helpers used by other crates' tests.
- `http` is planned for the blocking client transport; the sync engine itself is transport-neutral and performs no I/O.

The exact feature commands are added with the features themselves. P1.6 shipped SQLCipher as the only storage backend; `unencrypted-sqlite` and `test-support` are not implemented yet. P2.3 added schema v2 and the sync store. Until the remaining features land, the default workspace commands above are the complete set.

## Integration and migration tests

- `crates/core-persistence/tests/roundtrip.rs` covers opening a keyed database, reopening it, rejecting a wrong key, reading the recorded schema version, and round-tripping every persisted entity.
- `crates/core-persistence/tests/sync.rs` creates a schema v1 database fixture, opens it with the current build, verifies the `<database>.backup-v1` copy with `PRAGMA integrity_check`, migrates to schema v2, and checks that the v1 rows survive. It also round-trips the sync store: outbox, revisions, server sequences, cursor monotonicity, applied operations, conflicts, and snapshot-restore entities.
- Export contract tests live in the `core-application` export module: they validate `contracts/export/v1/fixture.json` and exporter output against the JSON Schema and round-trip through the Rust document types.
- The sync payload codec tests in `core-application/src/sync_payload.rs` convert every entity in the export fixture to a domain aggregate and back, and reject malformed payloads.
- Sync protocol contract tests live in `crates/core-sync/tests/contract.rs`: they validate every fixture against `contracts/sync/v1/sync.schema.json`, verify the manifest SHA-256 values, round-trip every valid document through the `core-sync` types, and enforce the envelope rules in Rust.
- Sync engine tests live in `crates/core-sync/tests/engine.rs`: two in-memory clients share a fake server and cover convergence, last-writer-wins for non-financial data, transaction conflicts and resolution, deletion and restore, offline retry, and snapshot restore followed by pull.
- Sync integration tests live in `crates/core-sync/tests/integration.rs`: two simulated clients run a seeded deterministic operation script against an in-memory server that mirrors the server's log behavior, covering convergence of the export and every revision, a lost push response retried without duplication, concurrent-edit and delete-versus-edit conflicts with both resolution choices, and snapshot restore on a fresh device. Every push and pull document is validated against the published schema. Observed results are in [VERIFICATION.md](../quality/VERIFICATION.md).
- `scripts/check_contracts.py` is the language-neutral check: it validates every contract schema, the fixture manifest and expectations, and the OpenAPI document after bundling its relative schema references. It requires `jsonschema`, `PyYAML`, and `openapi-spec-validator`.
- A workspace-level `tests/integration` (planned) exercises use cases over a real temporary SQLCipher database and wires the client sync engine to the server transport.
- Crypto known-answer tests live in `crates/core-crypto/tests/vectors.rs` and cover RFC 8032 Ed25519, RFC 7748 X25519, RFC 8439 ChaCha20-Poly1305, and Argon2id. The version 1 Argon2id answer and timing test is ignored by default; run it in release with `cargo test -p core-crypto --release --test vectors argon2id_version_one -- --ignored --nocapture`. The formats and recorded timings are in [crypto.md](../reference/crypto.md).

## Windows

The vendored SQLCipher/OpenSSL build needs a native Windows Perl and `nmake`; see [setup.md](setup.md). Environment changes do not persist between separate CI steps.

## Coverage

Coverage uses `cargo-llvm-cov`. Branch coverage requires a nightly toolchain.

```text
cargo +nightly llvm-cov --workspace --branch --json --output-path coverage.json
python3 scripts/check_coverage.py coverage.json --branch
```

The script enforces at least 80% line coverage for each of the six public crates (empty crates are skipped) and 100% branch coverage for money, currency-scale, transfer, balance, and month-boundary modules. On Windows, run it from the SQLCipher environment described above. The first run builds the workspace in `target/llvm-cov-target`; later runs are incremental. Measured figures and any residual branch exclusions are recorded in [COVERAGE.md](../quality/COVERAGE.md).

Dependency-review probe: this line exists only to open a pull request after the repository dependency graph was enabled.
