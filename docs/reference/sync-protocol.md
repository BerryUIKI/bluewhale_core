# Sync protocol v1

Status: active since P2.1 (2026-11-25). The authoritative artifacts are
`contracts/sync/v1/sync.schema.json` (document shapes),
`contracts/sync/v1/openapi.yaml` (HTTP API), and
`contracts/sync/v1/fixtures/` (conformance cases with a SHA-256 manifest).
This document records the semantics that the schemas cannot express. The
approved design is [SYNC_AND_SERVER_DESIGN.md](../sync/SYNC_AND_SERVER_DESIGN.md)
(R4).

## Scope

Protocol v1 covers the signed operation envelope, the unencrypted HTTP
documents, and the decrypted payload shape. The client sync engine in
`core-sync::engine` implements the outbox, push/pull, revisions, conflicts,
cursors, and snapshots against a transport port. It does not define:

- the cryptographic algorithms and binary key formats; they are implemented in
  `core-crypto` and documented in [crypto.md](crypto.md);
- server implementation and deployment, which land in P2.4;
- the HTTP transport adapter and client command wiring, which follow with the
  client shell (P3.1). The P2.5 integration suite exercises two simulated
  clients over an in-memory server and validates every document they exchange
  against the published schema.

The server never needs the payload shape: it validates the envelope, verifies
the signature, assigns a sequence, and stores ciphertext and routing metadata.

## Versioning

| Value | Current | Source |
| --- | --- | --- |
| Sync protocol | `1` | `core-sync` `PROTOCOL_VERSION`, `protocol_version` in every envelope |
| Decrypted payload schema | `1` | `core-sync` `PAYLOAD_VERSION`, `payload_version` in every envelope |
| HTTP API | `/v1` | `contracts/sync/v1/openapi.yaml` |

- Within protocol version 1, the `/v1` base path stays stable across additive
  HTTP changes such as new endpoints. Version 1 documents are closed: every
  object rejects unknown fields (`additionalProperties: false`), so a new field
  or enum value in an existing document is a shape change, not an in-place
  extension.
- A shape change publishes a new contract directory and increments
  `PROTOCOL_VERSION`. The server can serve multiple protocol versions at the
  same `/v1` path and rejects an unsupported envelope with
  `unsupported_protocol_version`. A change that cannot be negotiated in place
  moves the base path to `/v2`.
- The server rejects an envelope whose `protocol_version` it does not support
  with the `unsupported_protocol_version` error code.
- The protocol version is independent of the crate version, the local schema
  version, and the export schema version.

## Encoding

- **Identifiers.** Lowercase canonical UUID text. `op_id`, `ledger_id`, and
  `entity_id` are UUIDv7 (RFC 9562), with version nibble `7` and variant `8`,
  `9`, `a`, or `b`. `device_id` and `idempotency_key` are UUIDs of any version;
  clients generate them as UUIDv7 where practical.
- **Binary fields.** Unpadded base64url (`A-Z`, `a-z`, `0-9`, `-`, `_`). The
  nonce is 12 bytes (16 characters); the Ed25519 signature is 64 bytes (86
  characters); public keys are raw 32 bytes (43 characters). The ciphertext is
  at most 1 MiB decoded.
- **Server timestamps.** UTC RFC 3339 with exactly three fractional digits and
  a `Z` suffix, for example `2026-11-24T12:00:00.000Z`.
- **Client timestamp.** `client_timestamp` is informational UTC epoch
  milliseconds. The server never orders by it.
- **Numbers.** Sequence numbers, revisions, sizes, and timestamps are
  non-negative integers no greater than `9007199254740991` (`2^53 - 1`), the
  exact integer range of IEEE 754 doubles and RFC 8785 canonical JSON. This
  keeps the signature input exact and every consumer's number handling safe.
- **Email.** Annotated with `format: email`; the server performs the actual
  syntax and normalization checks.

## Operation envelope

The envelope is `$defs.operationEnvelope`. Every field is required and unknown
fields are rejected.

| Field | Semantics |
| --- | --- |
| `protocol_version` | Protocol version; `1` in this contract. |
| `op_id` | UUIDv7 unique within the ledger. A conflicting duplicate is rejected; an identical retry returns the existing sequence. |
| `ledger_id` | UUIDv7 ledger identity, equal to the settings record identifier. |
| `device_id` | The registered device that produced the operation. |
| `entity_type` | Entity kind; see the operation matrix. |
| `entity_id` | UUIDv7 identity of the mutated entity. |
| `op_type` | Mutation kind; see the operation matrix. |
| `base_revision` | Entity revision the edit was based on; `0` when creating the entity. |
| `client_timestamp` | Informational client time as UTC epoch milliseconds. |
| `idempotency_key` | Client-generated UUID reused on every retry. The server deduplicates by it. |
| `key_version` | Ledger data-key version used to encrypt the payload. |
| `nonce` | 12-byte AEAD nonce. |
| `ciphertext` | Encrypted payload. |
| `payload_version` | Decrypted payload schema version; `1` in this contract. |
| `signature` | Ed25519 signature over the canonical envelope. |

### Signature

The signature is computed over the RFC 8785 (JSON Canonicalization Scheme)
canonical JSON encoding of the envelope object with the `signature` field
omitted. The signed object is the one on the wire, including `ciphertext`,
`nonce`, `idempotency_key`, and `client_timestamp`; no fields are normalized or
re-serialized from decoded values. The device signs with its Ed25519 private
key, and the server verifies with the registered `signing_public_key`.

P2.2 implements this procedure: `OperationEnvelope::signing_bytes()` produces
the canonical input, `core-crypto`'s `DeviceSigningKey` signs it, and
`DeviceVerifyingKey` verifies it. The formats and test vectors are in
[crypto.md](crypto.md), and `crates/core-sync/tests/signing.rs` signs and
verifies an envelope end-to-end.

### Entity and operation matrix

Every permitted operation carries the full entity aggregate in its payload.

| Entity type | Permitted operations |
| --- | --- |
| `account`, `account_group`, `category`, `category_group`, `counterparty`, `note_preset`, `reconciliation_flag`, `tag` | `upsert` |
| `transaction` | `upsert`, `delete`, `restore` |
| `exchange_rate` | `rate_upsert` |
| `ledger_settings` | `settings_replace` |
| `ledger_profile` | `profile_replace` |

The schema encodes this matrix with conditionals, and `core-sync` enforces the
same matrix in `OperationEnvelope::validate` and
`EntityType::allowed_operations`.

## Payload v1

The decrypted payload is the export v1 entity object for the envelope's
`entity_type`, serialized as compact JSON. It does not include the export
document wrapper. The authoritative entity shapes are the `$defs` in
`contracts/export/v1/export.schema.json`:

| `entity_type` | Export schema definition |
| --- | --- |
| `account` | `$defs.account` |
| `account_group` | `$defs.accountGroup` |
| `category` | `$defs.category` |
| `category_group` | `$defs.categoryGroup` |
| `counterparty` | `$defs.counterparty` |
| `exchange_rate` | `$defs.exchangeRate` |
| `ledger_profile` | `$defs.profile` |
| `ledger_settings` | `$defs.settings` |
| `note_preset` | `$defs.notePreset` |
| `reconciliation_flag` | `$defs.reconciliationFlag` |
| `tag` | `$defs.tag` |
| `transaction` | `$defs.transaction` |

`core_application::sync_payload` converts between domain aggregates and these
objects. A payload that is malformed, fails domain validation, or describes a
different entity than the envelope is rejected before the store is touched.
Payload v1 accepts only currency-scale policy version 1; unknown currencies and
mismatched currency scales are rejected.

The payload is encrypted with the ledger data key and bound to the envelope's
ledger, entity, operation, base revision, and key version; the binary details
are in [crypto.md](crypto.md).

## Limits

| Limit | Value | Enforcement |
| --- | --- | --- |
| Operations per push batch | 500 | Schema `maxItems`, `core-sync` `MAX_OPERATIONS_PER_BATCH` |
| Serialized operation size | 1 MiB | Server and transport, `MAX_OPERATION_BYTES` |
| Serialized batch size | 8 MiB | Server and transport, `MAX_BATCH_BYTES` |
| Pull page size | 1–500, default 100 | OpenAPI parameter |

Limits are configurable server-side but not below these protocol maxima.

## HTTP API

Base path `/v1`, JSON over TLS 1.3. See `openapi.yaml` for request and response
schemas and status codes.

| Method and path | Purpose |
| --- | --- |
| `POST /v1/auth/magic-link` | Request an email magic link; always `202`. |
| `POST /v1/auth/verify` | Exchange a magic-link token for a session. |
| `POST /v1/auth/refresh` | Rotate the access and refresh tokens. |
| `POST /v1/auth/logout` | Revoke the current session. |
| `POST /v1/devices` | Register a device and its public keys. |
| `GET /v1/devices` | List device metadata, including revoked devices. |
| `DELETE /v1/devices/{device_id}` | Revoke a device. |
| `POST /v1/ledgers` | Register a client-generated ledger and its key envelopes. |
| `GET /v1/ledgers` | List ledger IDs, key versions, and latest sequences. |
| `PUT /v1/ledgers/{ledger_id}/keys` | Replace the envelope set for a key version. |
| `POST /v1/ledgers/{ledger_id}/ops` | Push a batch of operations. |
| `GET /v1/ledgers/{ledger_id}/ops` | Pull operations after a sequence. |
| `POST /v1/ledgers/{ledger_id}/ack` | Store the device cursor. |
| `POST /v1/ledgers/{ledger_id}/snapshots` | Reserve an encrypted snapshot upload. |
| `GET /v1/ledgers/{ledger_id}/snapshots/latest` | Get the latest snapshot metadata. |
| `GET /v1/ledgers/{ledger_id}/snapshots/{snapshot_id}` | Get snapshot metadata. |

The server returns no ledger display names, amounts, notes, categories, or tags,
and never logs tokens or ciphertext bodies.

### Ledger registration and key envelopes

- A ledger is registered with a non-empty set of initial key envelopes. Every
  envelope in the set carries the same initial `key_version`.
- `PUT /v1/ledgers/{ledger_id}/keys` replaces the complete envelope set for one
  key version. Every envelope must carry the request's `key_version`. A version
  greater than the current latest appends a new key version, the same version
  replaces its envelope set, and a lower version is a `conflict`.
- The envelope `body` is opaque to the server; the server stores and returns it
  without interpretation. Its structure is defined by P2.2.

### Push, pull, and acknowledgement

- A push batch is atomic: either every operation is appended in request order
  or the request fails. Operations receive consecutive per-ledger sequences.
- A duplicate `op_id` is not appended again. The response returns its existing
  sequence. A duplicate key with different envelope content is a `conflict`.
- Pull returns operations in ascending `seq` order. Delivery is at-least-once;
  applying an operation is idempotent by `op_id` and revision.
- `ack` stores the highest applied sequence per device. The stored cursor never
  moves backwards, and one device's cursor does not affect another's.
- Retries reuse the same `op_id` and `idempotency_key` with exponential backoff
  and jitter.

### Local changes

A local mutation is recorded through `SyncEngine::record_local_change`, which
saves the entity, advances its revision, encrypts and signs the payload, and
inserts the outbox row in one store transaction. A direct repository write
bypasses sync: the client command layer must use the engine boundary so the
local state and its operation commit together.

### Errors and retries

Errors use `$defs.error`: `{ code, message, retryable, details }`. `code` is a
stable machine-readable value; `message` is diagnostic English and is never
shown as localized UI text. A `429` response also carries a standard
`Retry-After` header. The full code set is in the schema (for example
`invalid_request`, `unauthorized`, `conflict`, `rate_limited`,
`unsupported_protocol_version`, `device_revoked`).

### Snapshots

The client encrypts a consistent ledger export at a known sequence with the
ledger data key, hashes it with SHA-256, and reserves an upload. The server
stores metadata and returns short-lived object-storage URLs; it never receives
the plaintext or a decryption key. A restore downloads and verifies the hash,
decrypts locally, imports into a fresh database, sets the cursor to the snapshot
sequence, and pulls later operations.

The encrypted snapshot payload is version 1. It contains the export document,
the ledger identity, the local schema version, the covered server sequence, and
every tracked entity revision. The blob format and associated data are in
[crypto.md](crypto.md). `SyncEngine::create_snapshot` and
`SyncEngine::restore_snapshot` implement creation and restore; the server-facing
upload API is part of `/v1`. Restore requires an empty ledger: a zero cursor,
an empty outbox, and no tracked entity revisions. Any other state is refused,
because a non-empty store would keep its higher cursor and silently skip the
operations in between. The engine restores entities one by one, so a failed
restore must discard and recreate the database rather than reuse a partial
import. Snapshot creation requires a drained outbox, and the client must not
run local writes while the export and revision set are captured.

## Conformance

The contract ships language-neutral fixtures so Rust and Go consumers test
against the same bytes:

- `sync.schema.json` is the source of truth for every document shape.
- `fixtures/valid/` holds documents that must validate and deserialize.
- `fixtures/invalid/` holds documents that must be rejected, one rule each.
- `fixtures/manifest.json` lists every case with its schema definition,
  expected validity, and SHA-256. The hash detects accidental edits and lets
  another repository copy the contract with provenance.

Two checks run in CI:

```text
cargo test -p core-sync
python -I scripts/check_contracts.py
```

The Rust tests validate every fixture against the schema, verify the manifest
hashes, and round-trip every valid document through the `core-sync` types. The
P2.5 integration suite applies the same schema to the push and pull documents
the client actually emits and consumes during a two-client run, so conformance
covers live traffic as well as fixtures. The Python check is language-neutral:
it validates the schemas, the manifest, the fixtures, and the OpenAPI document,
and confirms that every object schema referenced by the API has a valid
fixture. It needs `jsonschema`, `PyYAML`, and `openapi-spec-validator`.

`openapi.yaml` references `sync.schema.json` by relative path. A consumer that
needs one self-contained file can bundle those references without changing
their meaning; `check_contracts.py` performs that bundle before validating the
OpenAPI document.

Fixtures are synthetic and contain no real financial data, credentials, keys,
or ciphertext. The ciphertext and signature values are deterministic byte
patterns, not outputs of a production key.

## Compatibility with the rest of the core

- `core-sync` owns the protocol types and the client sync engine. The engine is
  I/O-free; `SyncTransport` is implemented by the clients' HTTP adapter and by
  test servers.
- `core-application` owns the payload codec that maps domain aggregates to the
  export v1 entity objects.
- `core-crypto` (P2.2) produces and verifies signatures and encrypts payloads
  and snapshots; it uses the sizes and canonicalization above.
- `core-persistence` (P2.3) adds the outbox, entity revision, applied
  operation, cursor, conflict, and sync-state tables as schema v2. A schema v1
  database is backed up to `<database>.backup-v1`, verified, and then migrated.
- `bluewhale_server` (P2.4) copies `contracts/sync/v1` with provenance and
  pins `PROTOCOL_VERSION`; it never consumes a core crate.
