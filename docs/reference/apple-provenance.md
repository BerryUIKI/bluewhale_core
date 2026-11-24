# Apple source provenance

This file records every reuse of algorithms, invariants, or test vectors from the private Apple package `Packages/BerryLedgerCore`. The reuse decision is recorded in [ADR 0005](../adr/0005-apple-source-reuse-and-provenance.md) and the module inventory in [P1.0_APPLE_REUSE_ASSESSMENT.md](../plan/P1.0_APPLE_REUSE_ASSESSMENT.md).

## Rules

- Record one entry per ported module or vector set, in the same change that introduces it.
- Name the Apple commit, the source files, and the reuse type.
- Do not copy Swift files, comments, assets, or app services into this repository.
- The reviewer confirms the Rust implementation matches the recorded vectors.

## Entries

| Date | Module | Apple commit | Source files | Reuse type | Rust location | Reviewer |
| --- | --- | --- | --- | --- | --- | --- |
| 2026-11-24 | Assessment only; no code ported yet | — | — | Inventory and decision | — | — |
