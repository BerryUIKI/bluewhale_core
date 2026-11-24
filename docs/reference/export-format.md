# Export format v1

The ledger export is a single JSON document that contains every ledger record needed to reconstruct the ledger: settings, profile, master data, all transactions including soft-deleted ones, and every exchange-rate revision. It excludes device-local registry descriptors and any encryption material.

| Artifact | Location |
| --- | --- |
| JSON Schema 2020-12 | [`contracts/export/v1/export.schema.json`](../../contracts/export/v1/export.schema.json) |
| Synthetic fixture | [`contracts/export/v1/fixture.json`](../../contracts/export/v1/fixture.json) |
| Rust document types | `core-application` `export` module, `EXPORT_VERSION = 1` |

## Encoding

- **Identifiers** are canonical lowercase hyphenated UUID strings.
- **Money** is `{ "minor_units": <integer>, "currency": "USD", "scale": 2 }`. `minor_units` is the exact integer amount; no floating point appears anywhere in the document.
- **Timestamps** are UTC RFC 3339 strings with exactly three fractional digits, for example `2026-11-24T12:00:00.000Z`.
- **Enums** are lowercase snake case (`opening_balance`, `household_member`, `european_central_bank`).
- **Nullable fields** are present and set to `null`; consumers must not treat a missing key as `null`.

## Ordering

Every array is sorted by `id` in ascending UUID order. Object keys are emitted in the order declared by the schema and the Rust types. Given the same ledger and the same `exported_at`, the exporter produces identical bytes.

## Scope

| Included | Excluded |
| --- | --- |
| Settings and profile | Device-local ledger registry |
| Account groups and accounts, archived included | Encryption keys and ciphertext |
| Category groups and categories, archived included | Sync outbox, cursors, and server state |
| Tags, counterparties, reconciliation flags, note presets, archived included | Application preferences unrelated to the ledger |
| Transactions, soft-deleted included | |
| Every exchange-rate revision, superseded included | |

## Compatibility

- A consumer rejects a document whose `export_version` it does not support.
- The document types use `deny_unknown_fields`, so an unrecognized key is an error rather than silently ignored data.
- A breaking change creates `contracts/export/v2/` and increments `EXPORT_VERSION`; v1 stays published.
- Import is not implemented in v1.0 of the product. It arrives as v1.1 with all-or-nothing validation, as recorded in [DATA_MODEL.md](../domain/DATA_MODEL.md) section 9.

## Handling

An export contains the complete financial history of a ledger. Clients treat the file as sensitive user data: write it only where the user asked, never log its contents, and never commit a real export to a repository. The fixture in this repository is synthetic.
