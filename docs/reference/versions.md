# Version compatibility

This table records the version identity of each public contract. A change to any row requires a migration and a changelog entry.

| Component | Current version | Status | Source of truth |
| --- | --- | --- | --- |
| Public crates | `0.1.0` | Functional APIs, unpublished; tagged `v0.1.0` for client integration | Workspace `Cargo.toml` |
| Local SQLite schema | `2` | Active since P2.3 (v1 since P1.6) | `core-persistence` migrations and `SCHEMA_VERSION` |
| Sync protocol | `1` | Active since P2.1 | `core-sync` `PROTOCOL_VERSION` and `contracts/sync/v1` |
| Sync payload schema | `1` | Active since P2.3 | Export v1 entity objects and `core-application` `sync_payload` |
| Sync snapshot format | `1` | Active since P2.3 | `core-sync` `SNAPSHOT_VERSION` and `docs/reference/crypto.md` |
| Export schema | `1` | Active since P1.7 | `contracts/export/v1` and `EXPORT_VERSION` in `core-application` |
| Crypto key formats | `1` | Active since P2.2 | `core-crypto` format constants and `docs/reference/crypto.md` |
| HTTP API | `/v1` | Contract published in P2.1; server implementation planned for P2.4 | `contracts/sync/v1/openapi.yaml` |
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

## Created tags

| Tag | Commit | Date | Purpose |
| --- | --- | --- | --- |
| `v0.1.0` | `7e9cdef` | 2026-11-26 | First integration pin for `bluewhale_clients`, after the Phase 2 exit. Annotated; the crates are not published to crates.io. |

Annotated tags are created on `main`; a tag must match the workspace version and the versions of all six crates. The release-identity table above defines the naming for future releases.
