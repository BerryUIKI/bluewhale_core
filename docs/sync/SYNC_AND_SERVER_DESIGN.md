# Bluewhale Sync and Server Design

## Document Status

- Status: Approved (R4 review)
- Review: R4 — sync and server technical design
- Date: 2026-11-23
- Scope: client sync engine, public protocol, and server contract
- Depends on: approved R1 product specification; approved R2 architecture; approved R3 data model
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: none

This document defines the approved synchronization protocol, key hierarchy, conflict policy, device lifecycle, backup/restore flow, threat model, and HTTP contract. Server implementation details remain in the private `bluewhale_server` repository; this document records the public contract and the cross-cutting design.

Decisions **D4.1–D4.10** are accepted as recommended. Approval authorizes R5 design, not implementation. The private Apple implementation is the behavioral reference for the domain model; it does not currently implement cross-platform synchronization, so this protocol is new.

Revision note (2026-11-25): P2.1 publishes the version 1 contract in `contracts/sync/v1`. The envelope table below listed nine entity types; version 1 also carries `account_group`, `reconciliation_flag`, and `note_preset`, which are part of the approved R3 data model and must synchronize. Version 1 documents are closed objects, so a new field or enum value in an existing document requires a new protocol version; new endpoints remain additive within `/v1`. `docs/reference/sync-protocol.md` records the encodings, operation-type matrix, limits, signature canonicalization, and conformance procedure.

Revision note (2026-11-25, P2.2): `core-crypto` implements the key hierarchy with Argon2id (64 MiB, 3 iterations, one lane), ChaCha20-Poly1305, X25519 with HKDF-SHA256, and Ed25519. The key record, wrapped LDK, and recovery envelope are 89, 93, and 61 bytes; the recovery key uses Crockford Base32 with a 20-bit checksum. `docs/reference/crypto.md` records the formats, test vectors, and host timing.

Revision note (2026-11-25, P2.3): the client sync engine is implemented in `core-sync`: local changes commit atomically with their outbox operation and entity revision; push, pull, acknowledgement, idempotent retries, conflicts, and snapshots run over a transport port. The decrypted payload version 1 is the export v1 entity object. Each entity tracks the server sequence of its latest known version, so an operation at or before that sequence is skipped; an applied operation sets the entity revision to `base_revision + 1`, and a later own operation that supersedes an earlier remote one is reapplied without a conflict. Financial conflicts are recorded on the device whose local version loses in server order. Schema v2 adds the sync tables. The blocking HTTP transport and the client command wiring follow in P3.1.

Revision note (2026-11-26, P2.5): the integration suite in `core-sync/tests/integration.rs` runs two simulated clients with a seeded, deterministic operation script over an in-memory server that mirrors the Go server's log behavior: envelope validation, one sequence per accepted operation, deduplication by operation identifier and idempotency key, ordered pull pages, and cursor acknowledgement. The observed results are recorded in `docs/quality/VERIFICATION.md`: both clients converge on the same export and on every entity revision; concurrent edits and a delete racing an edit leave the losing local version in a conflict record rather than silently overwriting it, and both resolution choices converge; a fresh device restored from an encrypted snapshot catches up through the normal pull path; and every push and pull document exchanged in the suite validates against `contracts/sync/v1/sync.schema.json`. The contract directory remains unchanged from P2.1; at P2.5 its 39 files were byte-identical to the copy the server pins, so both implementations run the same 36 conformance fixtures.

## 1. Goals and non-goals

Goals:

- Optional multi-device synchronization for one account; complete offline operation when sync is unavailable.
- End-to-end encryption: the server stores ciphertext and minimal routing metadata, never plaintext ledger data.
- Deterministic convergence: every device applies the same operation order.
- No silent financial conflict resolution; losing edits remain reviewable.
- One independent key and operation log per ledger.
- Encrypted snapshots for fast restore and backup.
- Go server and Rust client, coupled only through a versioned JSON protocol.

Non-goals for v1:

- Shared, collaborative, or household ledgers.
- Real-time push delivery; sync is pull-based with optional background scheduling.
- Web, iOS, or macOS clients.
- Bank feeds, server-side analytics, or server-side financial validation.
- Server-side key escrow or recovery without the user's recovery key.
- Automatic conflict-free merge of financial edits.
- Import from other applications.

## 2. Sync model

```text
Client A: local transaction -> local commit -> outbox -> push
                                                |
Server:                                   append to ledger log
                                          assign server_seq
                                                |
Client B: pull -> verify signature -> decrypt -> check revision -> apply -> ack
```

- Every ledger has an append-only operation log on the server, ordered by a monotonic `server_seq`.
- A local mutation commits to SQLite first and enqueues an outbox operation in the same transaction. Sync never blocks local entry.
- Push is idempotent. A duplicate `op_id` returns its existing sequence instead of appending again.
- Pull is ordered by `server_seq`. The client applies operations in order and acknowledges the applied sequence.
- Delivery is at-least-once. Applying an operation is idempotent by `op_id` and revision.
- Sync runs when the app opens, when connectivity returns, and on an optional background schedule. It never claims real-time delivery.
- Each device has its own cursor; one device's progress does not imply another's.

## 3. Identity and authentication

- **Account identity:** email magic link as the v1 primary method. Passkeys are a v1.1 candidate. No password is required for account access.
- **Encryption passphrase:** separate from account authentication. It is never sent to the server and is used to derive a key-encryption key locally.
- **Tokens:** short-lived access token plus a rotating refresh token, bound to a registered device. Refresh-token reuse or signature failure revokes the session.
- **Account recovery:** email recovers account access only. Data recovery requires the user-held recovery key. The server cannot reset or reconstruct the encryption key.
- **Device enrollment:** a new device authenticates, generates its key pair, and registers its public keys. It receives ledger key envelopes only after an existing unlocked device or the recovery key authorizes enrollment.
- **Device revocation:** the server marks the device revoked and rejects its tokens and operations. Key rotation for future data is described in section 9.

## 4. Key hierarchy

```text
passphrase --Argon2id--> KEK
KEK --wraps--> UMK (random 256-bit)
UMK --wraps--> LDK_1, LDK_2, ... (random 256-bit per ledger)
LDK --encrypts--> operation payloads and snapshots
recovery key --wraps--> UMK
device X25519 key --receives--> wrapped LDK envelopes
device Ed25519 key --signs--> operation envelopes
```

- Argon2id parameters are stored with the account key record and tuned for the target devices; exact parameters are set in R5 with measured timings.
- The user master key (UMK) is random, not directly derived from the passphrase. A passphrase change rewraps the UMK and does not re-encrypt ledger data.
- Each ledger has an independent data key (LDK). The LDK is wrapped for each authorized device's X25519 public key using an ephemeral X25519 exchange, HKDF, and an AEAD cipher. The server stores only opaque envelopes.
- Each device has an Ed25519 signing key. Every operation envelope is signed; the server verifies the signature against the registered device key.
- The recovery key is a 256-bit random value encoded for human transcription. It wraps the UMK and is stored by the user. The server may store a recovery envelope, but never the recovery key or UMK.
- Key versions are monotonic. Each operation and snapshot records the key version used.
- New-device enrollment requires an existing unlocked device or the recovery key. A server-only compromise cannot enroll a device that can read ledger data.

## 5. Operation envelope and payload

Every operation is a signed JSON envelope:

| Field | Semantics |
| --- | --- |
| `protocol_version` | Protocol version, starting at 1 |
| `op_id` | UUIDv7, globally unique within the ledger |
| `ledger_id` | UUIDv7 ledger identity |
| `device_id` | Registered device identity |
| `entity_type` | `account`, `account_group`, `category_group`, `category`, `tag`, `counterparty`, `reconciliation_flag`, `note_preset`, `transaction`, `exchange_rate`, `ledger_settings`, `ledger_profile` |
| `entity_id` | UUIDv7 entity identity |
| `op_type` | `upsert`, `delete`, `restore`, `settings_replace`, `profile_replace`, `rate_upsert` |
| `base_revision` | Entity revision the edit was based on |
| `client_timestamp` | Informational UTC epoch milliseconds |
| `idempotency_key` | Client-generated retry key |
| `key_version` | LDK version used for this payload |
| `nonce` | AEAD nonce |
| `ciphertext` | Encrypted payload |
| `payload_version` | Version of the decrypted payload schema |
| `signature` | Ed25519 signature over the canonical envelope |

- **Aggregate payloads:** a transaction payload includes its tag and counterparty associations so the aggregate commits atomically. Account payloads include tags and owner. Settings, profile, and exchange-rate payloads are single-entity replacements.
- **Full-state upserts:** payloads carry the complete current entity aggregate, not a field patch. This simplifies deterministic application and conflict detection. `base_revision` identifies the state the edit was based on.
- **Server validation:** the server validates envelope shape, byte limits, device/ledger membership, signature, and idempotency. It cannot validate financial semantics because the payload is encrypted; semantic validation happens on the client before push and after pull.
- **Limits (initial):** 1 MiB per operation, 500 operations per batch, 8 MiB per batch. Limits are configurable and versioned.
- **Replay protection:** `op_id` is unique per ledger; the server rejects a conflicting duplicate. `idempotency_key` deduplicates client retries.
- **Machine-readable contract:** `contracts/sync/v1/sync.schema.json` defines the envelope and every unencrypted API document; `contracts/sync/v1/fixtures/` holds conformance cases with a SHA-256 manifest. The operation-type matrix and signature canonicalization are in `docs/reference/sync-protocol.md`.

## 6. HTTP contract

Base path: `/v1`. JSON over TLS 1.3. Errors use `{ "code", "message", "retryable", "details" }` with stable machine-readable codes. The OpenAPI 3.1 description is `contracts/sync/v1/openapi.yaml`; it references the JSON Schema in the same directory.

### Authentication

- `POST /v1/auth/magic-link` `{ email }` -> `202` with an indistinguishable response for known and unknown addresses.
- `POST /v1/auth/verify` `{ token }` -> access token, refresh token, account summary.
- `POST /v1/auth/refresh` -> new access and refresh tokens.
- `POST /v1/auth/logout` -> revoke the current session.

### Devices

- `POST /v1/devices` `{ device_id, name, platform, signing_public_key, agreement_public_key }`.
- `GET /v1/devices` -> device metadata, no financial data.
- `DELETE /v1/devices/{device_id}` -> revoke the device.

### Ledgers

- `POST /v1/ledgers` `{ ledger_id, key_envelopes }` -> register a client-generated ledger ID and its initial key envelopes.
- `GET /v1/ledgers` -> ledger IDs, key versions, latest sequence; no display names or financial data.
- `PUT /v1/ledgers/{ledger_id}/keys` -> replace or append device key envelopes after enrollment or rotation.

### Sync

- `POST /v1/ledgers/{ledger_id}/ops` `{ ops: [...] }` -> `{ accepted: [{ op_id, seq }], latest_seq }`.
- `GET /v1/ledgers/{ledger_id}/ops?after_seq={seq}&limit={n}` -> `{ ops: [...], latest_seq, has_more }`.
- `POST /v1/ledgers/{ledger_id}/ack` `{ seq }` -> `204`. The server stores the per-device cursor.

### Snapshots

- `POST /v1/ledgers/{ledger_id}/snapshots` `{ seq, sha256, size, key_version }` -> upload URL.
- `GET /v1/ledgers/{ledger_id}/snapshots/latest` -> snapshot metadata and download URL.
- `GET /v1/ledgers/{ledger_id}/snapshots/{snapshot_id}` -> download URL.

### Cross-cutting rules

- The server never returns ledger display names, amounts, notes, categories, or tags.
- Requests and responses use stable field names and enum values; localized text is client-side only.
- Retries use exponential backoff with jitter. A retry reuses the same `op_id` and `idempotency_key`.
- Rate limits apply per account, device, and ledger; 429 responses include a retry hint.
- Protocol changes are additive within `/v1`. A breaking change requires `/v2` or an explicitly negotiated version.
- The server logs request metadata without financial content, tokens, or ciphertext bodies.

## 7. Conflict model

The server orders operations; the client determines whether an operation conflicts with its local state.

Application rule:

- If `base_revision` equals the local entity revision, apply the operation.
- If `base_revision` is greater than the local revision, a gap exists; buffer or re-pull before applying.
- If `base_revision` is less than the local revision, the edit is concurrent.

Field classification:

- **Non-financial fields:** tag name, color, description, order; category and group names and order; counterparty name and kind; ledger profile name and emoji. Apply server-order last-writer-wins, merging independent changes where possible.
- **Set associations:** tag associations merge as a set union of active tags; explicit removals require a `delete` operation and are applied in server order.
- **Single-valued associations:** account owner and transaction counterparty use server-order last-writer-wins.
- **Financial fields:** transaction kind, occurrence time, account and destination, category, source and destination money, deletion state, and opening balances. These are never silently overwritten. The server-order version becomes the live entity for convergence, and the losing local version is preserved in a `sync_conflict` record for user review.
- **Deletion versus edit:** the tombstone is the live state; a concurrent edit is preserved in a conflict record.
- **Restore versus delete:** operations apply in server order with revision checks; a concurrent case is recorded as a conflict.

A `sync_conflict` record contains the entity type and ID, the local aggregate snapshot, the remote operation ID, both revisions, detection time, resolution, and resolution time. Resolving a conflict creates a new `upsert` with `revision + 1`. Conflicted losing versions are not included in balances or reports until the user resolves them.

Concurrent creation of the same logical entity on two devices produces two UUIDs and two records. Automatic deduplication is not in v1; the duplicate remains visible and must be deleted manually.

## 8. Snapshots, backup, restore, and retention

- A snapshot is an encrypted, consistent export of one ledger at a known `server_seq`, including soft-deleted records and the current schema version.
- Snapshots are encrypted with the ledger's LDK and key version, hashed with SHA-256, and stored in S3-compatible object storage. The server stores metadata only.
- Clients create a snapshot after a configurable number of operations (initial: 500) or 24 hours, and on explicit user request. The server retains the latest three unless a retention policy says otherwise.
- Restore flow: new device authenticates, obtains key envelopes, downloads the latest snapshot, verifies its hash, decrypts it, imports it into a fresh ledger database, sets the cursor to the snapshot sequence, and pulls subsequent operations.
- Backup uses the same snapshot mechanism; the user can trigger an encrypted backup and download it.
- Operation retention in v1 is indefinite. This avoids cursor gaps and keeps every device able to catch up. Compaction and retention windows are deferred to a later version with explicit device-cursor handling.
- A snapshot restores derived data only after the client rebuilds local indexes; balances and reports are recalculated, never imported as stored truth.

## 9. Device lifecycle

- **Enrollment:** a new device authenticates and registers its public keys. An existing unlocked device or the recovery key wraps the required LDKs to the new device, and the client uploads the new key envelopes.
- **Revocation:** the server marks the device revoked and rejects its tokens and operations. The ledger rotates to a new LDK version, wraps it for remaining devices, and uses it for future operations. Historical operations remain decryptable by the revoked device because it held the old LDK; this limitation is documented and cannot be fully remedied without re-encrypting all history.
- **Lost device:** revoke it, then enroll a replacement with the recovery key or another unlocked device.
- **Device limit:** a configurable maximum (initial proposal: 10) and rate-limited enrollment reduce abuse.
- **Device metadata:** name and platform are stored server-side for management; no ledger names or financial data.

## 10. Threat model

Assets: ledger plaintext, ledger keys, account credentials, device keys, and metadata.

Adversaries and mitigations:

| Adversary | Mitigation | Residual risk |
| --- | --- | --- |
| Network attacker | TLS 1.3, signed envelopes, idempotency, replay protection | Traffic analysis of sizes and timing |
| Compromised server | E2EE; server stores ciphertext and minimal metadata | Metadata, availability, deletion of ciphertext |
| Stolen device | SQLCipher, OS keystore, biometric unlock, remote revoke | Unlocked-device access before revoke; old data on a revoked device |
| Malicious client | Server-side envelope validation, size/rate limits, signature checks | Server cannot validate encrypted financial semantics |
| Account takeover | Separate encryption passphrase and recovery key; device revocation | Attacker can delete or deny ciphertext; cannot decrypt without keys |
| Supply chain | Pinned dependencies, license checks, signed builds, SBOM | Compromised build or dependency |
| Lost credentials | Mandatory recovery key; tested restore flow | Lost passphrase and recovery key means permanent data loss |

The server never receives plaintext ledger data, ledger display names, passphrases, UMK, LDKs, or recovery keys. Metadata minimization is a design requirement; payload padding is deferred. The P2.6 review, including the threat model, findings, and the residual risk register, is in [SECURITY_REVIEW.md](../security/SECURITY_REVIEW.md).

## 11. Decisions accepted at R4

Approved by the project owner on 2026-11-23. The recommendations below are accepted.

| ID | Decision | Recommendation |
| --- | --- | --- |
| D4.1 | Account authentication | Email magic link in v1; passkeys in v1.1 |
| D4.2 | Key hierarchy | Random UMK wrapped by Argon2id-derived KEK; X25519 device agreement key; Ed25519 signing key; per-ledger LDK |
| D4.3 | Recovery | 256-bit user-held recovery key; server stores only an encrypted recovery envelope; no escrow |
| D4.4 | Device enrollment | Requires an existing unlocked device or the recovery key |
| D4.5 | Conflict policy | Server-order convergence; conflict records for financial fields; no silent financial last-writer-wins |
| D4.6 | Retention | Operations retained indefinitely in v1; encrypted snapshots for restore; compaction deferred |
| D4.7 | Device revocation | Revoke and rotate the LDK for future operations; document historical-data limitation |
| D4.8 | API style | REST/JSON `/v1`, batch push/pull/ack, idempotency keys, signed envelopes |
| D4.9 | Metadata | No ledger names or financial content server-side; payload padding deferred |
| D4.10 | Tokens | Short-lived access token plus rotating refresh token, device-bound |

## 12. Next review

R5 — client technical design: Tauri 2 project structure, web UI framework and architecture, local service adapters, background sync, biometrics, secure storage, accessibility, packaging, and the spike results. R5 records Tauri-specific decisions as conditional until the 10-day spike produces observed evidence. R4 approval does not authorize implementation.
