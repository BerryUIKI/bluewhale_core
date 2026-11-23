# ADR-0003: Cross-repository consumption and versioning

- Status: proposed
- Date: 2026-11-23
- Deciders: Bluewhale maintainers

## Context

The core, clients, and server are separate repositories. The clients need the core crates, but each repository must build independently, including on a fresh CI runner and for an outside contributor. Publishing the core to crates.io immediately would lock in crate names and ownership before the API is stable.

## Decision

**Phase A, now until an owner-approved core publication: git dependency pinned by full revision.**

- The six core crates are declared once in `bluewhale_clients/Cargo.toml` under `[workspace.dependencies]`, each with `git = "https://github.com/BerryUIKI/bluewhale_core"` and a full 40-hex `rev`. The corresponding annotated tag `vX.Y.Z` is recorded in the clients' `docs/development/core-pin.md`.
- `Cargo.lock` is committed and records the revision. Branch pins, submodules, and path dependencies in tracked manifests are prohibited.
- Local co-development uses a repo-local, gitignored `.cargo/local-core.toml` containing `[patch."https://github.com/BerryUIKI/bluewhale_core"]` entries that point at a sibling checkout. The patch key must match the git URL exactly; Cargo silently ignores a patch whose key does not match the dependency source. The file is never written to the global user configuration, and CI fails if it exists or is tracked.
- The six entries are added in the change that first consumes them (P3.1), not before. Until the first core tag is pushed, the client crates do not depend on the core, so every commit stays buildable.

**Phase B, only after a follow-up ADR: registry dependency.**

- That ADR must verify the six crates.io names are available and controlled, record the publication decision, and freeze the local schema, sync protocol, and export contract at 1.0.
- The client entries then change to `version = "=X.Y.Z"` registry pins, the local patch key changes to `[patch.crates-io]`, and the pin scripts change in the same change. No third mechanism is introduced.

Other rules:

- Publishing to crates.io is deferred. Availability was checked on 2026-11-23: `core-crypto` is already taken on crates.io; the other five names are free. Phase B must resolve that name, for example by publishing it under a different name or by renaming all six crates into a `bluewhale-core-*` namespace in one change.
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
