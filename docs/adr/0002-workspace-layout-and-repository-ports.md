# ADR-0002: Workspace layout and repository ports

- Status: proposed
- Date: 2026-11-23
- Deciders: Bluewhale maintainers

## Context

R2 defines six core crates and a strict layering rule: the domain owns repository traits, persistence implements them, and the application layer orchestrates use cases. The skeleton must make that direction obvious and mechanically checkable, without forcing every crate into one binary or leaking SQLite types into the domain.

## Decision

- The workspace has one crate per responsibility: `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, and `core-testkit`.
- Repository traits live in `core-domain`. `core-persistence` depends on `core-domain`; the domain never depends on persistence.
- `core-application` depends on `core-domain` and receives repository traits, clocks, and identifier generators as injected ports. It has no database or network dependency.
- `core-testkit` provides in-memory fakes and deterministic clocks and is never a normal dependency of a production crate.
- Dependency direction is enforced in CI by a workspace check (planned `xtask deps-check`) rather than by convention alone.

## Consequences

- Crate boundaries match the approved architecture and can be tested independently.
- Adding a new persistence backend means a new adapter crate, not a change to the domain.
- The workspace has more crates than a single-crate project; the extra manifests are a deliberate cost.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| One crate with modules | The compiler cannot enforce the layering, and the public API would expose persistence types. |
| Traits in a separate `core-ports` crate | Adds a crate without adding clarity; the domain is the natural owner of its ports. |

## References

- [ARCHITECTURE.md](../architecture/ARCHITECTURE.md)
- [REPOSITORY_LAYOUT.md](../architecture/REPOSITORY_LAYOUT.md)
