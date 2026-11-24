# Contracts

Language-neutral public contracts shared by the core, the clients, and the server.

| Directory | Contract | Status |
| --- | --- | --- |
| `export/v1` | Ledger export document | Version 1, implemented in P1.7 |
| `sync/v1` | Sync protocol envelope and API | Planned for P2.1 |

## Rules

- A contract directory is immutable once published. A breaking change creates a new version directory and updates `docs/reference/versions.md`.
- The JSON Schema is the source of truth for the document shape. Rust types in `core-application` mirror it and are tested against it.
- `fixture.json` is a hand-authored, valid document used by tests and by other repositories as a conformance example. It contains only synthetic data.

## Export v1

- `export.schema.json`: JSON Schema 2020-12 for the export document.
- `fixture.json`: one valid document containing every entity type.

Ordering is deterministic: every array is sorted by `id` in ascending UUID order. Money is exact integer minor units. Timestamps are UTC RFC 3339 with millisecond precision. A consumer must reject a document whose `export_version` it does not support.
