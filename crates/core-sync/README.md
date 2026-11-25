# core-sync

Sync protocol v1 types and validation for Bluewhale. The crate performs no
I/O and owns the public operation envelope, the entity/operation matrix, and
the unencrypted `/v1` request and response documents.

- `PROTOCOL_VERSION`, `PAYLOAD_VERSION`, and the operation, batch, and size
  limits.
- `OperationEnvelope` with the v1 validation rules and `PushRequest` batch
  validation.
- Typed documents for authentication, device registration, ledger registration
  and key envelopes, push/pull/ack, snapshots, and API errors.

The authoritative contract is [`contracts/sync/v1`](../../contracts/sync/v1):
`sync.schema.json` for the document shapes, `openapi.yaml` for the API, and
`fixtures/` with a SHA-256 manifest. Semantics are recorded in
[`docs/reference/sync-protocol.md`](../../docs/reference/sync-protocol.md).

The encrypted payload schema and the outbox, cursor, and conflict engine land
with the client sync engine; cryptography lands in `core-crypto`.
