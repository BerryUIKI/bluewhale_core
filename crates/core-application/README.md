# core-application

Application use cases, orchestration, transaction boundaries, and validation
flow for Bluewhale.

The crate also owns the versioned JSON export and the sync payload codec. A sync
payload v1 is the export v1 entity object for the envelope's entity type;
`sync_payload` converts every domain aggregate to and from that shape and
validates it through the domain constructors.

See the [documentation index](../../docs/README.md) and
[sync-protocol.md](../../docs/reference/sync-protocol.md).
