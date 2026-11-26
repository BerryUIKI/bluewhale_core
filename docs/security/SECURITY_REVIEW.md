# Security review and threat model

Status: active. This document records the P2.6 security review (2026-11-26) of
the core crates, the sync protocol, and the client sync engine. The server's own
implementation review, including its findings and fixes, is maintained in the
private `bluewhale_server` repository; this document covers the boundary
properties the protocol relies on.

## 1. Scope and method

Reviewed:

- `core-crypto`: key hierarchy, key records, recovery key, AEAD, Ed25519, X25519,
  randomness, secret lifetime and debug redaction.
- `core-sync`: envelope validation, signature canonicalization, payload
  encryption and associated data, revision and conflict handling, cursors, and
  snapshot creation and restore.
- `core-persistence`: SQLCipher open path, key handling, migrations, and the
  migration backup.
- `core-application`, `core-domain`: input validation on the paths that reach
  sync payloads and the export document.
- `contracts/sync/v1`: the shared schema and fixture set that both
  implementations enforce.
- The client/server trust boundary: what the server can observe, what it must
  never receive, and which properties the client enforces itself.

Not in scope: the Windows and Android applications (Phase 3), deployment
infrastructure, the email provider, and third-party services. The private
server implementation is reviewed in its own repository.

Method: design review against the R4 threat model, source review of every
security-relevant path, key-lifetime and logging walk-throughs, dependency,
license, and vulnerability scans, and SBOM generation. Confirmed findings were
fixed with regression tests; the disposition of each finding is in §5.

## 2. System model and trust boundaries

```text
[device A: ledger DB, UMK, LDK, device keys]
        |  TLS 1.3 (terminated at the edge), signed and encrypted envelopes
        v
[server: PostgreSQL metadata, S3-compatible ciphertext blobs]
        ^
        |  same protocol
[device B: independent copy of the ledger]
```

Trust assumptions:

- The edge terminates TLS with a valid certificate; the server itself speaks
  plain HTTP on a private address.
- The server and its database are operated honestly for availability and
  ordering, but are **not trusted with content**: the server can read metadata
  but never plaintext, ledger display names, passphrases, the UMK, LDKs, device
  private keys, or the recovery key.
- A device is trusted only while it is unlocked; the local database key lives
  in the platform keystore, outside this repository.
- Email is delivered by the configured SMTP endpoint; whoever controls the
  mailbox can redeem a magic link while it is valid.
- The user keeps the recovery key; losing both the passphrase and the recovery
  key makes the ledger unrecoverable by design.

What the server can observe: account email, device names and platforms, device
and ledger identifiers, operation sizes and timing, key-envelope sizes, IP
addresses, and ciphertext.

## 3. Assets

| Asset | Where it lives | Impact if disclosed |
| --- | --- | --- |
| Ledger content (accounts, transactions, notes, balances) | Client, encrypted in transit and at rest | Financial and personal privacy loss |
| UMK, KEK, LDKs, recovery key | Client only; server holds wrapped envelopes it cannot open | Full ledger decryption |
| Device private keys | Client keystore | Operation forgery for that device |
| Magic-link and session tokens | Hashed at the server, raw only in transit and on the client | Account takeover |
| Account email and metadata | Server | Identity and social-graph exposure |
| Operation-log integrity | Server storage, client checks | Wrong balances, silent data loss |
| Availability of ciphertext | Server and object storage | Denial of sync and restore |

## 4. Adversaries and threats

| Adversary | Threats | Mitigations | Residual risk |
| --- | --- | --- | --- |
| Network attacker | Read or modify client-server traffic; intercept email | TLS 1.3 at the edge; signed envelopes; AEAD with associated data; single-use short-lived links; STARTTLS required for SMTP | Traffic analysis of sizes and timing |
| Compromised server | Read content; forge, reorder, replay, or withhold operations; serve stale snapshots | E2EE; Ed25519 signatures verified by clients; revision checks turn replays into reviewable conflicts; snapshot hash and schema checks; cross-ledger envelopes rejected | Server can withhold or reorder data (availability and non-financial rollback); it cannot forge content |
| Stolen or lost device | Read the ledger; impersonate the device | SQLCipher; OS keystore; biometric unlock (client phase); remote revocation; LDK rotation on revocation | Unlocked-device access before revocation; a revoked device keeps historical keys |
| Malicious client | Push oversized, malformed, or forged data; abuse storage or mail | Strict envelope validation; Ed25519 verification bound to the session device; size and rate limits; server never parses content | The server cannot judge the semantics of encrypted content |
| Account takeover (email) | Redeem a link; add a device; delete data | Single-use 15-minute links; device-bound sessions; rotation with reuse detection; revocation; passphrase and recovery key remain required for decryption | An attacker with mailbox access can delete ciphertext or deny access, but cannot read it |
| Supply chain | Compromised or vulnerable dependency or build | `cargo deny` (advisories, bans, licenses, sources); `govulncheck`; pinned toolchains; CycloneDX SBOMs; secret scanning; dependency review on pull requests | A compromised build environment or a vulnerable but unreported dependency |
| Operator error | Weak configuration, plaintext services, leaked logs | Configuration validation; discard mailer by default; STARTTLS required; structured logs without secrets; no-store responses; documented deployment requirements | Misconfiguration outside the reviewed paths |
| Database or backup thief | Read stored metadata and ciphertext | Ciphertext only; token hashes only; no plaintext payloads, display names, or keys | Metadata exposure (account email, device names, timestamps) |

## 5. Findings and dispositions

| ID | Severity | Finding | Disposition |
| --- | --- | --- | --- |
| F1 | Medium | A snapshot could be restored into a non-empty local ledger. The cursor keeps its higher value, so entities are rolled back while the operations in between are silently skipped. | **Fixed** in `core-sync`: restore requires a zero cursor, an empty outbox, and no tracked revisions; regression tests cover a non-empty store, a tampered blob, and a schema mismatch. |
| F2 | Low | The SQLCipher key was interpolated into a `PRAGMA key` statement through two ordinary heap strings that were not wiped after use. | **Fixed** in `core-persistence`: the statement is built once in a `Zeroizing` buffer; a test opens and reopens a database whose key contains quotes. |
| F3 | Low | The client accepted an operation envelope carrying another ledger's identifier; decryption with the wrong key would have failed, but the failure depended on encryption rather than an explicit check. | **Fixed** in `core-sync`: a foreign `ledger_id` is rejected as a protocol error before any key use; a regression test injects a correctly signed foreign envelope. |
| F4 | Low | Panic recovery on the server logs the panic value and full stack. A stack can contain argument values from the panicking call. | **Accepted**: panics are defects, stacks are required for diagnosis, and request data does not reach panic arguments by design. Reviewed as part of the server record. |
| F5 | Low | Refresh-token reuse detection remembers only the immediately previous token; an older stolen token is rejected but does not revoke the session. | **Accepted**: such a token cannot be used; detection of the latest rotation already covers the common theft path. |
| F6 | Low | A client can reserve snapshot metadata without completing the upload, leaving a download that fails. | **Accepted**: it affects only the requester's own ledger, is bounded by retention, and cannot corrupt another ledger. |
| F7 | Low | Rate limits are per process instance. Behind several instances the effective limit multiplies. | **Accepted for v1**, documented; a shared limiter belongs to deployment hardening. |
| F8 | Informational | A malicious server can withhold operations or reorder them. Financial changes then surface as reviewable conflicts; non-financial fields (tags, names) can be rolled back by server order. | **Accepted** under the trust model; authenticated checkpoints or transparency logs are a possible later addition. |
| F9 | Informational | A revoked device keeps the LDK generations it already held, so it can decrypt history it received. | **Accepted** by decision D4.7; revocation rotates the key for future operation only. |
| F10 | Informational | Zeroization is best-effort: the allocator, the OS page file, and runtime copies may retain key bytes. | **Accepted**; the threat model excludes an attacker with kernel-level or live-process memory access. |
| F11 | Informational | Operation sizes and timing are visible to the server; payload padding is deferred. | **Accepted** by decision D4.9. |
| F12 | Informational | The local migration backup `<database>.backup-vN` remains beside the database after a migration. | **Accepted**: it uses the same SQLCipher key and permissions as the database. |

### Server-boundary findings

The server review found two issues that are fixed in the private repository and
summarized here because they affect login security:

- The SMTP mailer could send a magic link over a plaintext connection when the
  submission server offered no STARTTLS and no username was configured. It now
  refuses to send unless STARTTLS is negotiated.
- Magic-link requests were limited only per client IP. A second bucket keyed by
  the normalized email address (default five requests per fifteen minutes)
  bounds mail flooding from many addresses.

## 6. Component review notes

### Cryptography and key handling

- Argon2id v1.3 with 64 MiB, three iterations, one lane, a 16-byte salt, and a
  32-byte output; the key record binds the parameters and is authenticated.
- ChaCha20-Poly1305 with a fresh random 96-bit nonce per message and a 128-bit
  tag. The random-nonce collision bound (about 2^32 messages per key) is far
  above the expected operation volume per LDK version.
- X25519 with HKDF-SHA256 for wrapping LDKs per device; Ed25519 with
  `verify_strict` for operations; recovery keys use a 20-bit checksum and are
  zeroized on drop.
- Every secret type zeroizes on drop and redacts its `Debug` output; errors
  carry no secret material. `SymmetricKey` is not `Clone`, so copies are
  explicit.
- Operation associated data binds the ledger, entity type and identity,
  operation type, base revision, and key version. Snapshot associated data binds
  the ledger, covered sequence, and key version. A payload cannot be moved to a
  different envelope, entity, or key version.

### Sync engine and protocol

- Envelopes are validated before any cryptographic work: protocol and payload
  version, entity/operation matrix, UUID versions, integer ranges, base64url
  lengths, and the ciphertext size limit.
- Remote operations are signature-verified with the device key before
  decryption; the payload must decode to the envelope's entity and identity.
- Revisions, per-entity server sequences, and the cursor make application
  idempotent and turn replays into conflicts rather than silent overwrites.
- Financial conflicts preserve the losing local aggregate for review; resolving
  the local side records a new operation on top of the current revision.
- Restore verifies size and SHA-256 before decryption, then the format version,
  key version, ledger identity, covered sequence, and local schema version, and
  refuses a non-empty ledger (F1).
- A foreign ledger identifier is rejected explicitly (F3).

### Local persistence

- SQLCipher with a caller-supplied key; quotes are escaped and the key copy in
  the PRAGMA statement is zeroized (F2).
- Forward-only migrations run in a transaction after a verified backup; the
  backup is opened with the same key and checked with `PRAGMA integrity_check`.
- The database key is generated on first setup and kept in the platform
  keystore by the client (Phase 3); this repository never stores it.

### Server boundary (verified properties)

The protocol and the server's contract tests establish that the server stores
ciphertext and routing metadata only. The review confirmed: opaque 256-bit
tokens stored as SHA-256 hashes; single-use magic links consumed atomically;
rotating refresh tokens with reuse revocation; device-bound sessions checked on
every authenticated request; revocation enforced for tokens and pushes;
parameterized SQL; strict JSON with duplicate-member, unknown-field, and
trailing-data rejection; request size limits; per-account, per-device,
per-ledger, per-IP, and now per-address rate limits; presigned uploads with a
signed content length; and structured access and audit logs without tokens,
email addresses, bodies, ciphertext, or keys.

## 7. Residual risk register

| Risk | Why accepted | Follow-up |
| --- | --- | --- |
| Malicious server withholding or reordering | Integrity beyond signatures requires client-verifiable checkpoints | Consider signed checkpoints or a transparency log after v1 |
| Metadata exposure (email, device names, sizes, timing) | Needed to route and authenticate | Payload padding and metadata minimization remain deferred (D4.9) |
| Revoked device keeps historical plaintext | Re-encrypting all history would require every device to re-upload | Documented user guidance: revoke and rotate on loss |
| Per-instance rate limits | Simpler v1 deployment | Shared limiter at the edge or in the service |
| Best-effort zeroization | No kernel-level attacker in the model | Platform secure-memory work in the client phase |
| Snapshot reservation without upload | Self-inflicted availability only | Optional upload confirmation in a later protocol version |

## 8. Verification evidence

Environment: Windows 10 Pro 10.0.19045, `x86_64-pc-windows-msvc`,
2026-11-26. Rust 1.97.1; Go 1.26.11; PostgreSQL 17.7; cargo-deny 0.22.0;
golangci-lint 2.15.0; cargo-cyclonedx 0.5.10; cyclonedx-gomod 1.12.0.

| Command | Result |
| --- | --- |
| `cargo test --workspace` | 209 passed, 0 failed, 1 ignored (the measured Argon2id timing test). |
| `cargo clippy --workspace --all-targets -- -D warnings` | Clean. |
| `cargo fmt --all --check` | Clean. |
| `cargo deny check` | advisories ok, bans ok, licenses ok, sources ok. |
| `python -I scripts/check_contracts.py` | 2 schemas, 1 manifest, 36 fixtures ok. |
| `cargo cyclonedx --format json --spec-version 1.6` | CycloneDX SBOM per crate. |
| `go test -count=1 ./...` (server) | All packages pass. |
| `go vet ./...`, `gofmt -l .`, `golangci-lint run` (server) | Clean; 0 issues. |
| `govulncheck ./...` (server) | No vulnerabilities found. |
| `cyclonedx-gomod mod -json` (server) | CycloneDX 1.6 SBOM, 19 components. |

CI additions: a history secret scan (`gitleaks`), a CycloneDX SBOM artifact,
and, for the public repository, dependency review on pull requests. The SBOM
and secret-scan jobs were configured but have not executed yet; this host cannot
run GitHub Actions. Their first run is on the next push.

## 9. Conclusion

No high-severity issue was identified in the reviewed code. Three defense-in-
depth issues in the core and two login-path issues in the server were fixed with
regression tests. The Phase 2 exit criteria hold: two clients converge, financial
conflicts are reviewable, snapshots restore on a fresh device, the server stores
no plaintext ledger data, display names, or keys, and the protocol fixtures pass
on both implementations. The remaining risks are documented above and are
either inherent to the trust model or deferred by an accepted decision.
