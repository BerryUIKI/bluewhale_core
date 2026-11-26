# Coverage

Gate evidence for the core workspace. P1.8 recorded the initial floors; P2.1 added the sync protocol types, P2.2 the cryptographic primitives, and P2.3 the client sync engine and schema v2. P2.5 added integration tests only and changed no production source, so the measured line and branch coverage is unchanged. Thresholds are unchanged: at least 80% line coverage for each public crate and 100% branch coverage for the critical domain modules.

Line coverage is counted once per instrumented source line. LLVM reports generic monomorphizations and duplicate build instantiations separately; summing the raw per-file summaries counts one source line several times and understates real coverage. `scripts/check_coverage.py` computes the unique-line union from the export segments. The thresholds and the branch gate are unchanged.

## Environment

| Item | Value |
| --- | --- |
| Date | 2026-11-24 (P1.8); 2026-11-25 (P2.1-P2.3); 2026-11-26 (P2.5, tests only) |
| Host | Windows 10 Pro 10.0.19045, `x86_64-pc-windows-msvc` |
| Line-coverage toolchain | Rust 1.97.1 (the workspace toolchain) |
| Branch-coverage toolchain | `rustc 1.102.0-nightly (20fe935d5 2026-11-23)` |
| Coverage tool | `cargo-llvm-cov 0.9.2` with `llvm-tools-preview` |
| Native build | Vendored SQLCipher/OpenSSL with Strawberry Perl 5.40.5 and MSVC 14.44 `nmake` |

The first run builds the workspace in `target/llvm-cov-target`; later runs are incremental. CI repeats the combined gate on a clean Linux runner with nightly Rust.

## Commands

```text
cargo llvm-cov --workspace --summary-only
cargo llvm-cov report --json --output-path coverage-stable.json
python -I scripts/check_coverage.py coverage-stable.json

cargo +nightly llvm-cov --workspace --branch --json --output-path coverage-branch.json
python -I scripts/check_coverage.py coverage-branch.json --branch
```

## Line coverage

Measured on 2026-11-25 with the stable toolchain, unique source lines per crate. The gate is 80% per public crate.

| Crate | Lines covered | Lines | Coverage |
| --- | ---: | ---: | ---: |
| `core-domain` | 2,241 | 2,599 | 86.2% |
| `core-application` | 2,667 | 3,114 | 85.6% |
| `core-persistence` | 1,539 | 1,887 | 81.6% |
| `core-sync` | 893 | 1,047 | 85.3% |
| `core-crypto` | 974 | 982 | 99.2% |
| `core-testkit` | 952 | 1,125 | 84.6% |

## Branch coverage

Measured with nightly branch instrumentation. The gate is 100% per critical module.

| Module | Branches covered | Branches | Coverage |
| --- | ---: | ---: | ---: |
| `money.rs` (money and rounding) | 22 | 22 | 100% |
| `currency.rs` (currency-scale validation) | 8 | 8 | 100% |
| `transaction.rs` (transfer construction) | 26 | 26 | 100% |
| `balance.rs` (balance calculation) | 10 | 10 | 100% |
| `calendar.rs` (month boundaries) | 6 | 6 | 100% |
| `statistics.rs` (monthly statistics) | 4 | 4 | 100% |

The branch tests added for this gate cover the rejected transaction-field combinations, malformed money input, overflow and currency-mismatch paths, unrelated-account balance cases, December and DST month intervals, and statistics validation and currency failures. P2.1-P2.3 did not change the critical domain modules.

## Dependency isolation

- `cargo tree -p core-domain --edges normal` resolves only `jiff`, `unicode-normalization`, `unicode-segmentation`, and `uuid`.
- `cargo tree -p core-application --edges normal` adds `jiff`, `serde`, `serde_json`, `unicode-normalization`, and `uuid` on top of `core-domain`.
- `cargo tree -p core-sync --edges normal` adds `base64`, `core-application`, `core-crypto`, `core-domain`, `jiff`, `serde`, `serde_json`, `sha2`, and `uuid`; there is no HTTP client, server, or UI dependency.
- `cargo tree -p core-crypto --edges normal` adds `argon2`, `chacha20poly1305`, `ed25519-dalek`, `getrandom`, `hkdf`, `sha2`, `unicode-normalization`, `uuid`, `x25519-dalek`, and `zeroize`, plus their RustCrypto and `dalek` support crates; there is no I/O, network, or UI dependency.
- No `std::net`, `tauri`, `reqwest`, `hyper`, or `tokio` import exists in the core crates; the core workspace contains no Tauri or UI code.

## Residual notes

- All six crates now contain code. `core-sync` covers the protocol types, engine, and snapshots; `core-crypto` covers the key hierarchy; `core-persistence` covers schema v2, migrations, repositories, and the sync store.
- `core-testkit` is test-only support but is included in the line gate because it is a published crate. Its figures include its own tests and the core-sync engine tests that use the in-memory ledger.
- The unique-line union corrects duplicate generic and build instantiations. The raw LLVM per-file summaries for `core-persistence` and `core-testkit` are lower because the same source line appears in several instrumented copies; the threshold is applied to actual source lines.
- The version 1 Argon2id known-answer and timing test is ignored by default and does not contribute to these figures.
- Defensive branches that protect invariants already enforced by construction are kept when removing them would weaken the API; each critical module still reports 100% of the branches the compiler instruments.
