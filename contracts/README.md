# Contracts

Language-neutral public contracts shared by the core, the clients, and the server.

| Directory | Contract | Status |
| --- | --- | --- |
| `export/v1` | Ledger export document | Version 1, implemented in P1.7 |
| `sync/v1` | Sync protocol envelope and `/v1` API | Version 1, implemented in P2.1 |

## Rules

- A contract directory is immutable once published. A breaking change creates a new version directory and updates `docs/reference/versions.md`. Within a published version, changes are additive.
- The JSON Schema is the source of truth for the document shape. Rust types in `core-application` (export) and `core-sync` (sync) mirror it and are tested against it.
- `fixtures/` holds hand-authored, synthetic conformance cases. A `manifest.json` lists each case with its schema definition, expected validity, and SHA-256.
- `openapi.yaml`, when present, describes the HTTP API and references the JSON Schema by relative path. It is not a second source of shape truth.
- Never include real financial data, credentials, encryption keys, or ciphertext. Fixtures use deterministic synthetic values.

## Export v1

- `export/v1/export.schema.json`: JSON Schema 2020-12 for the export document.
- `export/v1/fixture.json`: one valid document containing every entity type.

Ordering is deterministic: every array is sorted by `id` in ascending UUID order. Money is exact integer minor units. Timestamps are UTC RFC 3339 with millisecond precision. A consumer must reject a document whose `export_version` it does not support.

## Sync v1

- `sync/v1/sync.schema.json`: JSON Schema 2020-12 for the operation envelope and every unencrypted HTTP request and response document.
- `sync/v1/openapi.yaml`: OpenAPI 3.1 description of the `/v1` API. It references `sync.schema.json`; a consumer that needs one self-contained document can bundle those references without changing their meaning.
- `sync/v1/fixtures/`: valid and invalid conformance cases plus `manifest.json` with a SHA-256 per case.

Semantics, versioning, limits, the entity/operation matrix, and the signature canonicalization are in `docs/reference/sync-protocol.md`.

The contract is checked two ways:

```text
cargo test -p core-sync
python -I scripts/check_contracts.py
```

The Rust tests validate and round-trip every fixture through the `core-sync` types. The Python check is language-neutral and also validates the schemas, manifest hashes, fixture expectations, and the OpenAPI document.

The encrypted payload schema identified by `payload_version` is not part of P2.1; it lands with the client sync engine. The server never reads payloads.
