# ADR-0005: Apple source reuse and provenance

- Status: accepted
- Date: 2026-11-24
- Deciders: Bluewhale maintainers, repository owner
- Approval: the repository owner approved the reuse decision on 2026-11-24

## Context

The private Apple application `bluewhale-ledger` contains `Packages/BerryLedgerCore`, a Foundation-only domain and application layer that implements the same product semantics the public Rust core must provide. BerryUIKI owns both the private and the public repositories. Translating private code into a public Apache-2.0 repository is a licensing act even when one owner controls both, so it requires a recorded decision before implementation.

The P1.0 assessment in [P1.0_APPLE_REUSE_ASSESSMENT.md](../plan/P1.0_APPLE_REUSE_ASSESSMENT.md) inventories the package and classifies each module as port, reimplement, vectors, or exclude.

## Decision

1. BerryUIKI authorizes reuse of the algorithms, invariants, and test vectors in `Packages/BerryLedgerCore` for the public core under the Apache License 2.0.
2. The Apple application stays closed source and unchanged. Its UI, assets, app services, SwiftData models, and schema migrations are never reused.
3. Ported algorithms are re-expressed in idiomatic Rust and covered by Rust tests; the public repository does not contain verbatim Swift files or Apple-specific comments.
4. Each ported module records its provenance in `docs/reference/apple-provenance.md`: the Apple commit, source files, reuse type (algorithm, invariant, vector), and reviewer.
5. Out-of-scope features (budgets, recurring plans, subscriptions, savings goals, wishes) are not ported.
6. If the owner prefers a clean-room boundary, only the expected test values are reused and every algorithm is reimplemented from the R3 contract; the module decisions in the assessment remain valid.

## Consequences

- Money, exchange-rate conversion, and balance rules can preserve exact observed behavior with low divergence risk.
- The public repository remains Apache-2.0 clean, with a written provenance trail for every ported module.
- A reviewer must check each port against its vectors and record the provenance entry in the same change.
- The Apple application does not gain any dependency on the public crates and is not modified by this project.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| Clean-room reimplementation only | Higher risk of subtle divergence in rounding, rates, and month boundaries; allowed as the owner's explicit variant. |
| Copy Swift sources into the public repo | Mixes frameworks, languages, and private app code into an Apache-2.0 repository; rejected. |
| No reuse at all | Discards tested behavioral contracts that the product already relies on. |

## References

- [P1.0 Apple reuse assessment](../plan/P1.0_APPLE_REUSE_ASSESSMENT.md)
- [Architecture](../architecture/ARCHITECTURE.md)
- [Data model](../domain/DATA_MODEL.md)
