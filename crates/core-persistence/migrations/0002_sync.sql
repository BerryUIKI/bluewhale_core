-- Bluewhale local schema v2: the client sync engine's outbox, revisions,
-- applied-operation ledger, cursor, and conflict records.
--
-- Timestamps are Unix milliseconds in UTC. Identifiers are lowercase
-- hyphenated UUIDs. One database holds one ledger.

CREATE TABLE outbox_operations (
    op_id TEXT PRIMARY KEY,
    ledger_id TEXT NOT NULL,
    device_id TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    op_type TEXT NOT NULL,
    base_revision INTEGER NOT NULL CHECK (base_revision >= 0),
    client_timestamp INTEGER NOT NULL,
    idempotency_key TEXT NOT NULL UNIQUE,
    key_version INTEGER NOT NULL CHECK (key_version >= 1),
    nonce TEXT NOT NULL,
    ciphertext TEXT NOT NULL,
    payload_version INTEGER NOT NULL,
    signature TEXT NOT NULL,
    server_seq INTEGER UNIQUE,
    created_at INTEGER NOT NULL
);

CREATE INDEX outbox_pending ON outbox_operations (created_at)
    WHERE server_seq IS NULL;

CREATE TABLE sync_entity_state (
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    revision INTEGER NOT NULL CHECK (revision >= 0),
    last_server_seq INTEGER NOT NULL DEFAULT 0 CHECK (last_server_seq >= 0),
    PRIMARY KEY (entity_type, entity_id)
);

CREATE TABLE sync_applied_operations (
    op_id TEXT PRIMARY KEY,
    server_seq INTEGER NOT NULL CHECK (server_seq >= 1)
);

CREATE TABLE sync_cursor (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    applied_seq INTEGER NOT NULL CHECK (applied_seq >= 0)
);

CREATE TABLE sync_conflicts (
    id TEXT PRIMARY KEY,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    local_payload BLOB NOT NULL,
    remote_op_id TEXT NOT NULL,
    local_revision INTEGER NOT NULL CHECK (local_revision >= 0),
    remote_base_revision INTEGER NOT NULL CHECK (remote_base_revision >= 0),
    detected_at INTEGER NOT NULL,
    resolution TEXT,
    resolved_at INTEGER,
    CHECK (
        (resolution IS NULL AND resolved_at IS NULL)
        OR (resolution IS NOT NULL AND resolved_at IS NOT NULL)
    )
);

CREATE INDEX sync_conflicts_detected ON sync_conflicts (detected_at, id);

CREATE TABLE sync_state (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
