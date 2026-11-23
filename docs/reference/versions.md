# Version compatibility

This table records the version identity of each public contract. A change to any row requires a migration and a changelog entry.

| Component | Current version | Status | Source of truth |
| --- | --- | --- | --- |
| Public crates | `0.1.0` | Skeleton, unpublished | Workspace `Cargo.toml` |
| Local SQLite schema | `1` | Planned for P1.6 | `core-persistence` migrations and `SCHEMA_VERSION` |
| Sync protocol | `1` | Planned for P2.1 | `core-sync` `PROTOCOL_VERSION` and `contracts/sync/v1` |
| Export schema | `1` | Planned for P1.7 | `contracts/export/v1` |
| HTTP API | `/v1` | Planned for P2.4 | Server OpenAPI contract |
| Rust | `1.97.1` | Active | `rust-toolchain.toml`; MSRV `1.97.1` |
| Node / pnpm | `24.15.0` / `11.18.0` | Active for clients | `.node-version`, `package.json` |
| Go | `1.25.5` | Active for the server | `go.mod` |

## Compatibility rules

- Pre-1.0 public crates: a minor release may break the API and must carry a migration note; a patch release is compatible.
- The local schema is forward-only. A database is upgraded with a verified backup; downgrades are not supported.
- The sync protocol and export schema have independent version constants. A breaking change increments the version and adds fixtures for both the old and new shape.
- The server pins a protocol version and its contract fixtures; it never consumes a core crate.
- A release is identified by an annotated tag on `main`. The release workflow verifies that the tag, the workspace version, and all six crate versions agree.

## Release identity

| Artifact | Tag | When |
| --- | --- | --- |
| Core crates | `vMAJOR.MINOR.PATCH` | Owner-approved release, after Phase 0 and the relevant phase gate |
| Protocol contract | `protocol-vMAJOR.MINOR.PATCH` | P2.1 exit, before the server pins it |
| Clients | `vMAJOR.MINOR.PATCH` | After the core pin and application gates |
| Server | `vMAJOR.MINOR.PATCH` | After the protocol pin and deployment gate |

No tag has been created yet. The table is a design target, not a record of published artifacts.
