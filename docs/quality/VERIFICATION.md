# Verification record

Observed results for integration work, with the command, environment, and date
for each run. Planned or unverified behavior is listed separately. Coverage
evidence lives in [COVERAGE.md](COVERAGE.md); design decisions and their
revision notes live in [SYNC_AND_SERVER_DESIGN.md](../sync/SYNC_AND_SERVER_DESIGN.md).

## P2.5 integration (2026-11-26)

### Environment

| Item | Value |
| --- | --- |
| Date | 2026-11-26 |
| Host | Windows 10 Pro 10.0.19045, `x86_64-pc-windows-msvc` |
| Rust | 1.97.1 (the workspace toolchain) |
| Contract | `contracts/sync/v1`, unchanged since P2.1 |

### Commands and results

| Command | Result |
| --- | --- |
| `cargo test -p core-sync --test integration` | 5 passed, 0 failed. |
| `cargo test -p core-domain -p core-application -p core-crypto -p core-sync -p core-testkit --all-targets` | 191 passed, 0 failed, 1 ignored (the measured Argon2id timing test). |
| `cargo fmt --all --check` | Clean. |
| `cargo clippy -p core-domain -p core-application -p core-crypto -p core-sync -p core-testkit --all-targets -- -D warnings` | Clean. |

`core-persistence` is excluded from the local run because its vendored
OpenSSL/SQLCipher build needs a native Windows Perl and `nmake`; this shell only
has Git's Cygwin Perl, which OpenSSL's `VC-WIN64A` configuration rejects. CI
runs the full workspace on Linux. The P2.5 change adds a test file and
documentation only and does not touch `core-persistence`.

### What the P2.5 suite covers

`crates/core-sync/tests/integration.rs` runs real sync engines over in-memory
ledgers against a server model that validates every envelope, assigns one
per-ledger sequence, deduplicates by operation identifier and idempotency key,
serves ordered pull pages, and records the acknowledged cursor.

- **Deterministic convergence:** a seeded SplitMix64 script drives six rounds of
  interleaved creates, edits, deletes, and restores across two devices and five
  entities, with pushes between rounds. Both devices end with the same export
  document and the same revision for every entity; the served sequences are
  contiguous and the final cursor is acknowledged.
- **Lost push response:** the server commits a batch and then fails the
  response. The outbox keeps the operation, the retry is deduplicated by
  operation identifier and idempotency key, and the peer receives exactly one
  copy.
- **Concurrent edit:** two edits of the same transaction keep server order as
  the live version and preserve the losing local version in a conflict record.
  Choosing the local side records a new operation and both devices converge to
  it; the winning device carries no unresolved conflict.
- **Delete versus edit:** a concurrent soft delete and edit keep the edit live
  and the deleted version reviewable; accepting the server order resolves the
  record without an operation and both devices converge.
- **Snapshot restore:** a fresh device restores an encrypted snapshot, matches
  the source export and entity revisions, pulls later operations through the
  normal path, and rejects a flipped bit and a mismatched local schema version.
- **Wire conformance:** every `pushRequest`, `pushResponse`, and `pullResponse`
  exchanged during these scenarios is validated against
  `contracts/sync/v1/sync.schema.json`, the same schema the Go server validates
  against. The suite fails if a document drifts from the contract.

### Cross-implementation contract check

Both implementations test the same conformance fixtures. On 2026-11-26 the 39
files under `contracts/sync/v1` were compared by SHA-256 with the copy pinned in
`bluewhale_server` `main` at `9cd3f71` (copied from core `e9dc083`, the P2.3
merge): all 39 shared files were byte-identical, and the server copy adds only
`PROVENANCE.md` and `SHA256SUMS`. The server's own contract test verifies those
digests and all 36 fixtures; the Rust tests do the same from this repository.

### Not verified in this environment

- **Two clients against the running server over HTTP:** `core-sync` has no HTTP
  transport yet; the blocking adapter and client wiring are P3.1. The server's
  own test suite covers its side of the API, including PostgreSQL persistence,
  presigned object storage, rate limits, and log redaction. No end-to-end HTTP
  exchange between a real client and a deployed server has been observed.
- **Full workspace test run:** see the `core-persistence` note above.
- **Performance:** the suite is functional, not a benchmark; throughput and
  large-batch behavior are not measured.

## P2.6 security review (2026-11-26)

The review and its findings are in
[docs/security/SECURITY_REVIEW.md](../security/SECURITY_REVIEW.md). This section
records the commands behind its evidence.

A portable Strawberry Perl 5.42.3 with the Visual Studio 2022 (MSVC 14.44)
build environment was installed for this run, so the full workspace, including
the vendored SQLCipher/OpenSSL build in `core-persistence`, builds and tests on
this host.

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 209 passed, 0 failed, 1 ignored (the measured Argon2id timing test). |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean. |
| `cargo fmt --all --check` | Clean. |
| `cargo deny check` | `advisories ok, bans ok, licenses ok, sources ok`. |
| `python -I scripts/check_contracts.py` | `contracts ok: 2 schema(s), 1 manifest(s), 36 fixture(s)`. |
| `cargo cyclonedx --format json --spec-version 1.6` | One CycloneDX 1.6 SBOM per crate; generated for the check and not committed. |
| `cargo test -p core-persistence --test roundtrip` | 10 passed, including the quoted-key test. |
| `cargo test -p core-sync` | 17 + 6 + 8 + 5 + 2 passed, including the empty-ledger restore and foreign-ledger envelope tests. |
| `gitleaks git .` (local one-off, gitleaks 8) | 57 commits scanned, no leaks found. |
| Core CI on `main` (run 45744950406) | All jobs passed: Linux checks, Windows tests, dependency policy, contract checks, coverage gates, secret scan (10 s), SBOM (1 min 33 s). |
| Dependency review (probe pull request #10) | Passed in 5 s after the repository dependency graph was enabled; every other check on the probe also passed. |

### Not verified in this environment

- **Dependency advisories:** `cargo deny check` used the advisory database
  available on 2026-11-26; a later disclosure is not covered by this record.

