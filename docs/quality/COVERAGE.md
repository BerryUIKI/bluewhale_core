# Coverage

Gate evidence for the core workspace. P1.8 recorded the initial floors; P2.1 adds the sync protocol types, which are covered by their contract tests. Coverage is a lower bound on test quality, not a substitute for assertions: the quality policy forbids assertion-free or implementation-coupled tests written only to move a number.

## Environment

| Item | Value |
| --- | --- |
| Date | 2026-11-24; `core-sync` measured 2026-11-25 |
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

cargo llvm-cov -p core-sync --summary-only
```

## Line coverage

Measured with the stable toolchain. The P1.8 crates were measured on 2026-11-24 and `core-sync` on 2026-11-25. The gate is 80% per public crate.

| Crate | Lines covered | Lines | Coverage |
| --- | ---: | ---: | ---: |
| `core-domain` | 2,567 | 2,965 | 86.6% |
| `core-application` | 2,766 | 3,277 | 84.4% |
| `core-persistence` | 1,139 | 1,399 | 81.4% |
| `core-testkit` | 666 | 740 | 90.0% |
| `core-sync` | 378 | 379 | 99.7% |
| `core-crypto` | no code yet | 0 | n/a |

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

The branch tests added for this gate cover the rejected transaction-field combinations, malformed money input, overflow and currency-mismatch paths, unrelated-account balance cases, December and DST month intervals, and statistics validation and currency failures.

## Dependency isolation

- `cargo tree -p core-domain --edges normal` resolves only `jiff`, `unicode-normalization`, `unicode-segmentation`, and `uuid`.
- `cargo tree -p core-application --edges normal` adds only `serde` and `serde_json`.
- `cargo tree -p core-sync --edges normal` adds only `serde`, `serde_json`, and `uuid` (with their proc-macro and formatting support crates); there is no HTTP client, server, or UI dependency.
- No `std::net`, `tauri`, `reqwest`, `hyper`, or `tokio` import exists in `core-domain` or `core-application`; the core workspace contains no Tauri or UI code.

## Residual notes

- `core-sync` gained code in P2.1. Its line coverage was measured separately on 2026-11-25 with `cargo llvm-cov -p core-sync --summary-only`: 378 of 379 lines (99.7%). The workspace gate now includes it on the next full run.
- `core-crypto` is an empty crate; it has no lines to cover. The same gates apply when its implementation lands in P2.2.
- `core-testkit` is test-only support but is included in the line gate because it is a published crate. Its own tests now cover the repository fakes directly.
- Defensive branches that protect invariants already enforced by construction are kept when removing them would weaken the API; each critical module still reports 100% of the branches the compiler instruments.
