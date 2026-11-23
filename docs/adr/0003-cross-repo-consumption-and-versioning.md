# ADR-0003: Cross-repository consumption and versioning

- Status: proposed
- Date: 2026-11-23
- Deciders: Bluewhale maintainers

## Context

The core, clients, and server are separate repositories. The clients need the core crates, but each repository must build independently, including on a fresh CI runner and for an outside contributor. Publishing the core to crates.io immediately would lock in crate names and ownership before the API is stable.

## Decision

- The clients consume the core crates as git dependencies pinned to a release tag (`core-vX.Y.Z`). The pin is updated by `scripts/set-core-rev` and verified by `scripts/check-core-pin` in CI.
- Local side-by-side development may use an untracked `.cargo/config.toml` `[patch]` override under the exact git source key (`[patch."https://github.com/BerryUIKI/bluewhale_core"]`) that points at a sibling checkout. The override is repo-local and never written to the global user configuration; CI never uses it.
- Until the first core tag is pushed, the client crates do not depend on the core, so every commit remains buildable. The dependency is added in P1.2.
- Publishing to crates.io is deferred until the crate names, ownership, and compatibility policy are confirmed by a follow-up ADR. Availability was checked on 2026-11-23: `core-crypto` is already taken on crates.io; the other five names are free. Before any publish, either resolve the `core-crypto` name (for example publish it under a different name) or rename all six crates into a `bluewhale-core-*` namespace and update the manifests, docs, and client pins in one change. The migration path is: keep the same crate names, add a registry dependency, and retain the git pin only for unreleased canary builds.
- The server pins the sync protocol version and copies protocol fixtures with provenance; it does not depend on a core crate.

## Consequences

- Core releases follow a tag and changelog procedure; clients update the pin deliberately.
- A core change that breaks clients is visible as a failing pin update, not as a silent build break.
- Contributors can work on both repositories without vendoring or submodules.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| git submodule | Adds a second checkout model and a confusing contributor experience. |
| vendoring core into clients | Duplicates the source of truth and hides version drift. |
| Publish to crates.io immediately | Names and API are not stable; a mistaken publish is hard to undo. |

## References

- [REPOSITORY_LAYOUT.md](../architecture/REPOSITORY_LAYOUT.md)
- [CLIENT_DESIGN.md](../clients/CLIENT_DESIGN.md)
