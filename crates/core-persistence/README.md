# core-persistence

SQLite/SQLCipher schema, forward-only migrations, and repository
implementations for Bluewhale.

- Local schema v2: ledger entities, exchange rates, and the sync store.
- `0002_sync.sql` adds the outbox, entity revisions and server sequences,
  applied operations, cursor, conflicts, and reserved sync state.
- Existing schema v1 databases are copied to `<database>.backup-v1`, verified
  with `PRAGMA integrity_check`, and then migrated.
- `SqliteLedger` implements the domain repository traits and the sync store
  port; local changes, outbox rows, and revisions commit in one transaction.

See the [documentation index](../../docs/README.md) and
[build-test.md](../../docs/development/build-test.md) for the migration tests.
