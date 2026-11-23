# Bluewhale Technical Architecture

## Document Status

- Status: Approved (R2 technical review)
- Review: R2 — technical architecture and stack
- Date: 2026-11-23
- Scope: `bluewhale_core`, `bluewhale_clients`, `bluewhale_server`
- Depends on: approved R1 product specification (`docs/product/PRODUCT_SPEC.md`)
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: revision 1
- Implementation gate: the Rust + Tauri spike in `CLIENT_STACK_DECISION.md` must pass its hard gates before feature implementation begins.

## 1. Goals and constraints

From R1:

- Windows 10/11 and Android 10+ clients; English and Simplified Chinese.
- Local-first, offline-complete core; optional private synchronization.
- Exact integer money, derived balances, soft deletion, ledger-local time.
- Open-source core and clients (Apache-2.0); private server.
- End-to-end encryption for ledger payloads.
- Lean v1: accounts, categories, transactions and transfers, dashboard and reports, settings, multiple ledgers, export.
- No code or design-text copying from the private Apple product.

Technical goals:

- One authoritative domain implementation for both clients.
- Domain and use cases testable without UI, network, or platform storage.
- A clear public/private boundary: the server must not become a second financial domain.
- Versioned public contracts for the core API, local schema, sync protocol, and export format.
- Small-team delivery with a small trusted core and thin platform adapters.

## 2. Architecture at a glance

```text
bluewhale_clients (public)
  TypeScript web UI (shared)
    -> Tauri commands
    -> Rust application use cases
         |
bluewhale_core (public)  v
  domain models and policies <- repository traits
       ^
       |-- rusqlite + SQLite/SQLCipher (one database per ledger)
       |-- sync engine (outbox, crypto, cursor)

bluewhale_server (private)
  Go API (auth, devices, sync, backup)
    -> PostgreSQL metadata
    -> encrypted object storage
```

The clients own presentation, window integration, and device services. The core owns domain rules, use cases, local persistence, sync, and cryptography. The server owns identity, device registration, encrypted log storage, and backup; it never owns financial rules or plaintext ledger data.

## 3. Repository and module boundaries

| Module | Repository | Visibility | Responsibility |
| --- | --- | --- | --- |
| `core-domain` | `bluewhale_core` | public | Rust crate: money, accounts, categories, transactions, transfers, reports, policies, repository traits |
| `core-application` | `bluewhale_core` | public | Rust crate: use cases, orchestration, transaction boundaries, validation flow |
| `core-persistence` | `bluewhale_core` | public | Rust crate: SQLite schema, migrations, `rusqlite` repository implementations |
| `core-sync` | `bluewhale_core` | public | Rust crate: operation log, sync DTOs, cursor, conflict application |
| `core-crypto` | `bluewhale_core` | public | Rust crate: Argon2id, AEAD, key wrapping, test vectors |
| `core-testkit` | `bluewhale_core` | public | Rust crate: fakes, fixtures, deterministic clocks |
| `clients-shared-ui` | `bluewhale_clients` | public | TypeScript web UI, view state, navigation |
| `clients-windows` | `bluewhale_clients` | public | Tauri 2 Windows shell, WebView2 integration, MSI/NSIS packaging |
| `clients-android` | `bluewhale_clients` | public | Tauri 2 Android shell, Kotlin plugins for WorkManager, biometrics, Keystore, notifications |
| `server-api` | `bluewhale_server` | private | Go: auth, device, sync, and backup endpoints |
| `server-storage` | `bluewhale_server` | private | Go: PostgreSQL metadata, encrypted object storage, operation index |

Dependency rules:

- `core-domain` depends only on the Rust standard library and time primitives. No UI, network, storage, or crypto implementation.
- `core-application` depends on `core-domain`. It owns cross-entity use cases and transaction boundaries.
- `core-persistence` implements repository traits and may depend on `core-domain`; it must not be imported by `core-domain`.
- `core-sync` depends on domain and application interfaces plus `core-crypto`.
- `clients-shared-ui` invokes Rust through Tauri commands. It never opens SQLite, constructs money, calculates balances, or calls the sync API directly.
- Tauri commands and Kotlin plugins are thin adapters over application use cases. Financial behavior never lives in TypeScript or platform plugin code.
- `server-*` depends only on the published sync protocol contract (JSON Schema or OpenAPI). It must not import core or client code, so the server cannot become a parallel source of financial truth.
- The public sync protocol is versioned; a change requires a compatibility decision and a server-side migration plan.

## 4. Technology decisions

### 4.1 Core and client stack (T1)

**Accepted: Rust core + Tauri 2.**

Rationale:

- The Rust core is the long-lived, language-neutral asset: memory-safe, fast, testable without UI, and reusable by future CLI, native, or web clients through Tauri or UniFFI.
- Tauri uses WebView2 on Windows, which has mature accessibility, IME, and text rendering compared with a JVM/Compose desktop shell.
- One TypeScript UI serves Windows and Android, keeping the lean v1 scope manageable.
- The Go server already makes the project polyglot; a language-neutral core and JSON protocol are the right boundary.

**Pre-approved fallback:** if the Android spike fails background sync, biometric unlock, secure storage, accessibility, or Play packaging, switch the Android client to native Jetpack Compose and keep the Rust core. This is Option C in `CLIENT_STACK_DECISION.md`.

**Implementation gate:** the 10-day spike in `CLIENT_STACK_DECISION.md` section 11 must pass before production feature work begins. Spike code is throwaway.

### 4.2 UI strategy (T2)

**Accepted: shared TypeScript web UI plus platform-specific shell and integrations.**

- Windows: Tauri window, native menus, file dialogs, keyboard shortcuts, WebView2 accessibility.
- Android: Tauri mobile shell, Android WebView, Kotlin plugins for WorkManager background sync, BiometricPrompt, Keystore-backed key storage, notifications, and share intents.
- The web UI must use semantic HTML, ARIA where needed, keyboard focus management, and scalable text.
- No financial logic in TypeScript; the UI calls typed Tauri commands backed by application use cases.

### 4.3 Server (T3)

**Accepted: Go + PostgreSQL + S3-compatible object storage.**

- HTTP: standard library `net/http` router, or `chi` where routing composition helps.
- Data: `pgx` driver and `sqlc`-generated queries; versioned SQL migrations.
- Storage: PostgreSQL for identity, device, ledger routing, and operation metadata; S3-compatible object storage for encrypted snapshots and large blobs.
- The server implements only the public sync protocol and never imports core or client code.
- Go's small binaries, simple deployment, and concurrency fit a private sync service.

### 4.4 Local persistence (T4)

**Accepted: SQLite, one database file per ledger, explicit versioned migrations.**

- Driver: `rusqlite` with bundled SQLCipher.
- Each ledger is an isolated database with its own encryption key.
- Migrations are forward-only, numbered, transactional, and tested against fixtures from every prior schema version.
- A backup copy is made before a schema migration; a failed migration must not destroy the original database.
- The physical schema and migration policy are R3 decisions.

### 4.5 Sync protocol (T6)

**Accepted: versioned JSON over HTTPS with an append-only operation log and server-assigned sequence numbers.**

- Encrypted payloads are base64 fields inside the JSON envelope.
- The detailed envelope, conflict rules, retention, device revocation, and recovery flow are R4.
- The protocol contract is language-neutral so the Rust clients and Go server implement the same schema.

### 4.6 Local at-rest encryption (T5)

**Accepted: SQLCipher for the local database.**

- The database key is protected by Android Keystore on Android and DPAPI/Credential Manager on Windows.
- This is a spike validation item; if SQLCipher is not viable on a target, fall back to OS-level disk encryption plus app sandbox and record the gap.
- E2EE still protects server-side data either way.

### 4.7 Identifiers (T8)

**Accepted: UUIDv7** for entity identifiers, association identifiers, operation identifiers, and device identifiers.

- Time-ordered identifiers reduce index fragmentation and simplify cursors and debugging.
- Entities carry a monotonically increasing per-entity revision for sync.

### 4.8 Windows packaging (T9)

**Accepted: MSI or NSIS** produced by the Tauri bundler, with code signing added before public release. Android uses AAB.

### 4.9 End-to-end encryption (T7)

**Accepted at R1:** all ledger payloads are encrypted on the client.

- RustCrypto primitives: Argon2id for key derivation, an AEAD cipher for payloads, and `zeroize` for key material.
- The server stores ciphertext, operation metadata, device identity, timestamps, and sizes.
- The key hierarchy, recovery key, device revocation, and rotation are R4 decisions.

## 5. Stack summary

| Area | Choice |
| --- | --- |
| Core language | Rust, current stable |
| Core crates | `core-domain`, `core-application`, `core-persistence`, `core-sync`, `core-crypto`, `core-testkit` |
| Core libraries | `serde`, `uuid` (v7), `thiserror`, `rusqlite` (bundled SQLCipher), `argon2`, `chacha20poly1305`, `zeroize`; time-zone library selected in R3 |
| Windows client | Tauri 2 + WebView2 + TypeScript |
| Android client | Tauri 2 mobile + Android WebView + Kotlin plugins |
| Web UI framework | TypeScript framework selected in R5 (React, Svelte, or Solid); no UI framework in the core |
| Server | Go, `net/http` or `chi`, `pgx`/`sqlc`, versioned SQL migrations, S3 SDK |
| Local database | SQLite via `rusqlite`, one file per ledger, SQLCipher |
| Sync | Versioned JSON over HTTPS, append-only operation log |
| Build | Cargo workspace + Node package manager + Tauri CLI; Go modules |
| CI | GitHub Actions: Linux for core/server, Windows for the Windows client, Linux with Android SDK/NDK for Android |
| Packaging | MSI/NSIS for Windows; AAB for Android |

No third-party dependency is accepted without a license review and a reason to prefer it over the standard library or an already-approved dependency.

## 6. Data flow and layering rules

```text
Web UI
  -> Tauri command
  -> application use case
  -> domain policy
  -> repository trait
  -> rusqlite adapter (within one ledger database)
  -> outbox operation (for sync)
```

Rules:

- Views render state and send intent. They never open a database, construct money, calculate balances, or call the sync API.
- Use cases own multi-record atomicity and validation order.
- Domain policies are the only place that decides financial direction, balance effects, month membership, and rounding.
- Repository traits are storage-neutral; SQLite types never cross into presentation.
- Sync is an application concern. A local write commits locally first; the outbox is drained asynchronously. Network failure never rolls back a committed local ledger change.
- Each ledger has an isolated database and encryption key. Switching ledgers rebuilds the repository and presentation graph; no state crosses ledgers.
- The server stores encrypted operations and minimal metadata. It does not parse ledger amounts, categories, or notes.

## 7. Sync architecture (high level)

Detailed protocol design belongs to R4. The R2 contract is:

- Every local mutation produces an immutable operation with a stable operation ID, ledger ID, device ID, entity identity, base revision, client timestamp, idempotency key, and encrypted payload.
- The client maintains an outbox and a pull cursor. Push is idempotent; pull returns operations after the cursor in server sequence order.
- The server assigns a monotonic per-ledger sequence, deduplicates by operation ID, and stores ciphertext plus routing metadata.
- Applying operations is deterministic. Conflicts are detected per entity revision; financial conflicts are not silently resolved by last-writer-wins. Conflict policy, tombstones, retention, compaction, and recovery are R4 decisions.
- Periodic encrypted snapshots bound replay time. A new device restores from a snapshot plus subsequent operations.
- Sync works with multiple ledgers and multiple devices per account. Device revocation and key rotation are R4 decisions.

## 8. Security and privacy architecture

- **End-to-end encryption:** ledger operations and snapshots are encrypted on the client. The server stores ciphertext, operation metadata, device identity, timestamps, and sizes. It cannot read amounts, account names, categories, tags, counterparties, or notes.
- **Key hierarchy (proposed, detailed in R4):** account master key derived from a user passphrase with Argon2id; per-device key pair; per-ledger data key wrapped for each authorized device; a high-entropy recovery key. Account authentication is separate from the encryption passphrase.
- **Recovery:** losing all devices requires the recovery key. A password reset alone must not be able to decrypt data. The recovery flow must be explicit, tested, and documented before sync ships.
- **Transport:** TLS 1.3 for all server communication. Certificate pinning is not proposed for the open-source clients in v1 because it complicates independent builds and certificate rotation.
- **Local data:** SQLCipher with platform-protected keys where viable; otherwise OS-level encryption and app sandbox, recorded as a known limitation.
- **Logs and telemetry:** no financial values, notes, names, identifiers that resolve to financial data, credentials, or tokens in logs. Crash reporting is opt-in and redacted.
- **Export:** user-initiated, documented, versioned. Export may be plaintext JSON or an encrypted archive; the user chooses and is warned.
- **Supply chain:** pinned dependencies, license checks, SBOM generation, and no unreviewed transitive runtime dependencies.

## 9. Build, test, CI, and release

- **Core tests:** `cargo test`, property tests for money and aggregation, SQLite migration tests with in-memory and on-disk fixtures, crypto test vectors.
- **Client tests:** TypeScript unit/component tests, Tauri driver end-to-end tests, Android instrumented tests, keyboard and screen-reader checks on both platforms.
- **Sync tests:** protocol round-trip, idempotent push, cursor pull, multi-device simulation, conflict fixtures, offline/reconnect, snapshot restore.
- **Server tests:** Go tests for protocol compatibility, auth/device lifecycle, idempotency, retention, and encrypted-blob storage (private repository).
- **CI:** GitHub Actions: core and server on Linux, the Windows client on `windows-latest`, and Android on Linux with the Android SDK/NDK and an emulator. A pull request must not merge with unobserved relevant test results.
- **Release:** semantic versions for the public core and clients; Android AAB; Windows MSI/NSIS; signed artifacts; a compatibility note for protocol and export versions.
- **Public contracts:** the core's public API, local schema version, sync protocol version, and export schema version are documented and versioned. A breaking change requires a migration and release note.

## 10. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Tauri 2 Android maturity | Mobile quality and native integration | 10-day spike; custom Kotlin plugins; fallback to native Compose Android |
| WebView2/Android WebView differences | Layout, input, accessibility | Semantic HTML/ARIA, cross-webview tests, explicit minimum WebView policy |
| Rust learning curve and talent | Delivery speed | Spike, training, small well-tested core; Kotlin fallback if needed |
| Two languages and build systems | Coordination cost | Clear Tauri command boundary, generated TypeScript types, CI for both |
| SQLCipher on Windows/Android | Local at-rest encryption | Spike validation; fallback to OS encryption with a recorded limitation |
| E2EE recovery complexity | Data loss risk | Recovery key mandatory; tested restore flows; explicit UX |
| Sync conflicts on financial records | Data integrity | Revision-based detection; user-visible conflict handling; no silent last-writer-wins |
| Android background execution limits | Sync freshness | WorkManager plus sync on app open; no promise of real-time sync |
| Windows installer and code signing | Release friction | CI packaging, signing, and install/uninstall tests |
| 10,000-transaction performance | UX | Indexed queries, pagination, virtualized lists, performance fixtures |

## 11. Decision record

| ID | Decision | Result |
| --- | --- | --- |
| T1 | Core/client stack | Accepted: Rust core + Tauri 2; Android spike gate; native Compose fallback |
| T2 | UI strategy | Accepted: shared TypeScript web UI plus platform shell and Kotlin plugins |
| T3 | Server stack | Accepted: Go + PostgreSQL + S3-compatible object storage |
| T4 | Local database | Accepted: SQLite via `rusqlite`, one database per ledger, versioned migrations |
| T5 | Local at-rest encryption | Accepted: SQLCipher with platform keystore; spike validation |
| T6 | Sync protocol | Accepted: versioned JSON over HTTPS with append-only operation log |
| T7 | Encryption scope | Accepted: client-side E2EE for all ledger payloads; metadata minimized |
| T8 | Identifiers | Accepted: UUIDv7 |
| T9 | Windows packaging | Accepted: MSI/NSIS, signed before public release; Android AAB |

## 12. Next review

R3 — domain model and data contracts in `docs/domain/DATA_MODEL.md`: money and currency, ledger settings, account, category, tag, counterparty, transaction and transfer, exchange rates, deletion, identifiers, local schema versioning, and migration policy. R3 can proceed in parallel with the spike; implementation still waits for the spike gates and the R3 approval.
