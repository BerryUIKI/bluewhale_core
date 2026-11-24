# ADR-0006: Time and calendar library

- Status: accepted
- Date: 2026-11-24
- Deciders: Bluewhale maintainers

## Context

The core must store UTC instants and compute ledger calendar boundaries (day, month, year) in an IANA time zone. Ledger month membership decides reports and balances, so DST transitions, month ends, and non-UTC zones must be correct. The implementation must build on Windows, Linux, and Android with Rust 1.97.1, keep dependencies license-clean, and produce deterministic tests. The Apple implementation used Foundation's Gregorian `Calendar` with a `TimeZone`; that behavior is the reference, not the framework.

## Decision

Use [jiff](https://docs.rs/jiff) 1.0.2 with its default features (`std`, system time zone, and bundled tzdb fallbacks):

- `jiff::Timestamp` is the UTC instant type for entity timestamps and intervals.
- `jiff::tz::TimeZone` validates IANA identifiers and resolves the ledger time zone.
- `jiff::civil::Date` computes first-of-month, first-of-year, and next-day boundaries; midnight in the ledger zone converts back to a `Timestamp`.
- The core wraps these in domain types (`LedgerCalendar`, `InstantInterval`) so the public API stays in Bluewhale vocabulary.

Intervals are half-open `[start, end_exclusive)`. Day inclusion follows the ledger time zone: a filter day range is converted to start-of-day through the day after the end day.

## Consequences

- Month and day boundaries follow jiff's DST disambiguation and match the observed Apple vectors (for example, 2026-01-31T16:30Z is February in `Asia/Singapore`).
- The public crates depend on jiff for instants; export and sync formats must still define their own serialization (RFC 3339 with an explicit offset) rather than serializing jiff internals.
- jiff requires Rust 1.85; the workspace MSRV 1.97.1 is satisfied.
- The license is `Unlicense OR MIT`, which the `deny.toml` allowlist accepts through MIT.
- Android uses the platform tzdata when present and the bundled fallback otherwise; tests pin common zones and boundary instants.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| `time` plus `time-tz` | Needs an external tzdb source and more glue; less explicit DST disambiguation. |
| `chrono` plus `chrono-tz` | Mature but larger data and a `LocalResult` ambiguity model that is easy to misuse for ledger boundaries. |
| Hand-rolled civil date math | Reinvents DST, leap years, and zone data; unacceptable for financial month boundaries. |
| Store only UTC offsets | Cannot answer "which month is this in the ledger time zone" across DST. |

## References

- [DATA_MODEL.md](../domain/DATA_MODEL.md)
- [P1.0 Apple reuse assessment](../plan/P1.0_APPLE_REUSE_ASSESSMENT.md)
- [Apple provenance](../reference/apple-provenance.md)
