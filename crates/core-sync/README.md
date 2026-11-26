# core-sync

Sync protocol v1 types and the client sync engine for Bluewhale.

- Protocol types: `PROTOCOL_VERSION`, `PAYLOAD_VERSION`, limits,
  `OperationEnvelope` with validation and RFC 8785 `signing_bytes`, and the
  typed `/v1` request and response documents.
- Client engine: `SyncEngine` records local changes atomically with their
  outbox operation and revision, pushes pending operations, pulls and applies
  remote operations with signature and decryption checks, detects revision
  gaps, preserves financial conflicts, advances the cursor, and retries safely
  after offline failures.
- Transport: `SyncTransport` is implemented by the clients' HTTP adapter and by
  test servers; this crate performs no I/O.
- Snapshots: `create_snapshot` encrypts the export document, entity revisions,
  schema version, and covered sequence; `restore_snapshot` verifies the hash and
  restores into a fresh store.

The authoritative contract is [`contracts/sync/v1`](../../contracts/sync/v1):
`sync.schema.json` for the document shapes, `openapi.yaml` for the API, and
`fixtures/` with a SHA-256 manifest. Semantics are recorded in
[`docs/reference/sync-protocol.md`](../../docs/reference/sync-protocol.md) and
[`docs/reference/crypto.md`](../../docs/reference/crypto.md).

The blocking HTTP transport and the client command wiring follow with the client
shell; the domain payload codec lives in `core-application`. The integration
suite in `tests/integration.rs` runs two simulated clients over an in-memory
server and validates every wire document they exchange against the published
schema.
