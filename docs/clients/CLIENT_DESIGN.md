# Bluewhale Client Technical Design

## Document Status

- Status: Approved (R5 review; Tauri-specific decisions conditional on spike evidence)
- Review: R5 — client technical design
- Date: 2026-11-23
- Scope: `bluewhale_clients` (Windows and Android) and client-side core integration
- Depends on: approved R1 product specification; approved R2 architecture; approved R3 data model; approved R4 sync and server design
- Decision owner: project owner
- Approval: approved by the project owner on 2026-11-23
- Supersedes: none

This document defines the approved client technical design. Decisions **D5.1–D5.10** are accepted as recommended. Tauri-specific decisions remain conditional until the 10-day Rust + Tauri spike in `docs/architecture/CLIENT_STACK_DECISION.md` produces observed evidence; approval does not authorize implementation.

## 1. Goals and constraints

- Windows 10/11 and Android 10+ clients from one shared TypeScript UI and one Rust core.
- Local-first operation; sync is optional and never blocks entry.
- Exact integer money; all financial rules live in the Rust core.
- SQLCipher local storage with platform-protected keys.
- English and Simplified Chinese, including Chinese IME on Windows.
- Keyboard operation and screen-reader support on both platforms.
- Lean v1; no placeholder controls.
- Pre-approved fallback: if the Android Tauri spike fails, replace the Android app with native Jetpack Compose while keeping the Rust core and the Windows Tauri app.

## 2. Repository layout

```text
bluewhale_clients/
  ui/                         # shared React + TypeScript web UI
    src/
      app/                    # shell, routing, providers
      features/               # accounts, transactions, reports, settings, sync
      components/             # shared accessible primitives
      lib/                    # IPC client, formatting, i18n, errors
      locales/                # en and zh-Hans resources
  desktop/                    # Tauri 2 Windows application
    src-tauri/
      src/                    # thin Tauri commands and app state
      capabilities/           # minimal Tauri permissions
      tauri.conf.json
  android/                    # Tauri 2 Android application
    src-tauri/
    gen/android/              # generated Tauri Android project
    platform/                 # custom Kotlin plugins
  packages/                   # generated TypeScript IPC types
  docs/                       # client-specific design notes and spike reports
```

- `desktop` and `android` are separate Tauri applications that share `ui` and the Rust core crates through path dependencies.
- Separation keeps the Android fallback isolated: replacing `android` with a Compose app does not affect `desktop` or `ui`.
- No financial logic exists in `ui`, Tauri command handlers, or Kotlin plugins.

## 3. Tauri architecture

### 3.1 Process model

- Each client embeds the Rust core crates in its Tauri backend; there is no separate background daemon.
- Tauri commands are thin adapters that validate transport-level input and call `core-application` use cases.
- A per-ledger database runtime owns the `rusqlite` connection and serializes writes. Blocking SQLite work runs on a dedicated executor, never on the UI thread.
- Application state holds the device registry, active ledger descriptor, and the active ledger runtime. Switching ledgers tears down the old runtime and builds a new one; no repository or presentation state crosses ledgers.

### 3.2 IPC contract

- `specta`/`tauri-specta` generates TypeScript bindings from Rust command signatures.
- Every command returns a typed result or `CommandError { code, message_key, details? }`.
- Panics never cross the IPC boundary; all fallible paths return typed errors.
- Commands accept and return presentation DTOs. SQLite row types and domain internals never cross into TypeScript.
- Monetary values cross the boundary as exact decimal strings or `{ minor_units, currency, scale }`; TypeScript never performs money arithmetic.
- Mutations return the updated aggregate or a result code; the UI invalidates affected queries.

### 3.3 Security

- Minimal Tauri capabilities: no shell, process, or arbitrary filesystem access.
- Strict content security policy; navigation restricted to the application origin.
- Devtools disabled in release builds.
- File access only through explicit export/import commands and dialogs.
- Secrets and keys never appear in IPC responses, logs, or crash reports.

## 4. Shared web UI

### 4.1 Framework

**Recommendation: React + TypeScript + Vite.**

- Largest ecosystem for accessible components, forms, testing, and hiring.
- Strong fit for data-dense finance screens with tables, filters, dialogs, and keyboard workflows.
- Svelte remains an alternative if bundle size becomes a hard constraint; it is not recommended for v1 because the accessible component ecosystem is smaller.

### 4.2 Components and styling

- Radix UI primitives for dialogs, menus, popovers, tooltips, and form controls; Tailwind CSS for layout and tokens.
- One project design-token layer for color, spacing, typography, focus rings, and dark/light appearance.
- No heavy component framework that hides focus management or accessibility behavior.

### 4.3 State and data

- TanStack Query for command-backed queries and mutations, with explicit invalidation after writes.
- Local transient UI state in React state or a small store; no second copy of domain data.
- The UI treats Rust as the source of truth and never recalculates balances, totals, or currency conversion.
- Money display uses strings returned by Rust plus locale formatting; amount input is sent as a locale-normalized decimal string and parsed exactly in Rust.

### 4.4 Forms and validation

- React Hook Form for form state; Zod for transport-shape validation only.
- Business validation (category intent, account currency, transfer endpoints, amount scale) runs in Rust and returns stable error codes.
- The UI maps error codes to localized messages and field-level feedback.
- Save is atomic and duplicate-safe; the UI disables duplicate submission and surfaces retry state.

### 4.5 Localization

- `i18next` + `react-i18next` with ICU-style messages and `en` / `zh-Hans` resources.
- Language preference persists locally; system default is supported.
- No localized identifiers, currency codes, enum values, dates, or machine-readable keys.
- Localization completeness is a test gate: every key has non-empty English and Simplified Chinese values.

### 4.6 Accessibility

- Semantic HTML, visible focus, logical tab order, skip links, and keyboard operation for every flow.
- Radix focus management for dialogs and menus; no keyboard traps.
- Screen-reader validation with NVDA/Narrator on Windows and TalkBack on Android.
- Charts expose accessible summaries and data tables; color is never the only signal.
- Scalable text and high-contrast appearance are supported without layout breakage.

### 4.7 Screens for v1

- **Ledger selector and first-run setup:** reporting currency, time zone, name, emoji.
- **Dashboard:** balances, monthly income/expense, day detail.
- **Accounts:** list, create/edit, archive, tags, owner.
- **Transactions:** history, search/filter, editor, soft delete/restore, recovery view.
- **Categories and tags:** two-level categories, tags, counterparties.
- **Reports:** cash-flow summary and category ranking, with explicit missing-rate state.
- **Settings:** language, reporting currency, manual exchange rates, ledger profile, sync.
- **Sync:** status, manual sync, and a conflict review queue.
- **Export:** user-initiated versioned JSON export.

### 4.8 Navigation

- Windows: persistent sidebar plus content area; keyboard shortcuts for primary destinations and creation.
- Android: adaptive bottom navigation or navigation drawer; native back handling.
- Shared route model with platform-specific shells; deep links are deferred.

## 5. Platform services

### 5.1 Local storage and key protection

- SQLCipher database per ledger using `rusqlite`; the database key is generated on first setup.
- Android: key wrapped by Android Keystore; biometric or device-credential unlock can gate access.
- Windows: key protected by DPAPI or Credential Manager; Windows Hello is an optional gate.
- The device-local ledger registry contains descriptors and active selection only; no financial data.
- **Spike-dependent:** SQLCipher build and key-storage behavior on Windows and Android.

### 5.2 Android plugins

Custom Kotlin plugins behind Tauri:

- **Background sync:** WorkManager periodic work with a network constraint, invoking the Rust sync engine through a Tauri plugin. Work is idempotent and respects OS battery limits; minimum interval is 15 minutes.
- **Biometric unlock:** BiometricPrompt with Keystore-backed key; fallback to device credential when configured.
- **Secure storage:** Keystore-wrapped database key and refresh token.
- **File export:** Storage Access Framework document picker; streams the export without buffering the whole ledger.
- **Notifications:** minimal local notification for sync failures; opt-in.
- **Deferred:** share intents, deep links, widgets, and background push messaging.

### 5.3 Windows integration

- Native menu bar and keyboard shortcuts; Tauri file dialogs for export.
- DPAPI or Credential Manager for database and token keys.
- Single-instance guard to avoid two writers on one ledger.
- Optional Windows Hello unlock.
- **Deferred:** tray icon, auto-update, protocol handlers, and shell integration.

### 5.4 Background sync

- Android: WorkManager periodic sync plus sync on app open. No real-time guarantee and no foreground service in v1.
- Windows: sync on launch, on foreground, and on a manual action. No Windows service or scheduled task in v1.
- The UI always exposes a manual "Sync now" and an accurate last-sync state; it never implies real-time synchronization.

### 5.5 App lock

- Optional biometric or platform-credential lock for the active ledger; configurable grace period.
- Locking clears decrypted key material from memory and hides ledger content.
- **Decision D5.9:** enable the app lock by default in v1 or defer it to v1.1.

## 6. Core integration contracts

| Area | Commands |
| --- | --- |
| Ledger | list, create, switch, profile load/update |
| Accounts | list, create, update, archive |
| Categories | list groups/categories, create, update, archive |
| Tags and counterparties | list, create, update, archive, assign |
| Transactions | list, load context, create, update, delete, restore, filter |
| Dashboard | load balances, monthly summary, day detail |
| Reports | cash-flow summary, category ranking |
| Settings | load/update language, reporting currency, exchange rates |
| Sync | status, push, pull, resolve conflict, enroll/revoke device |
| Export | run versioned JSON export |

- Every command maps to an approved use case; no ad-hoc SQL or financial logic in command handlers.
- Query results are presentation DTOs. Mutations are atomic and return the new aggregate.
- Every command uses the active ledger runtime; there is no cross-ledger state.

## 7. Testing

- **Rust core:** unit tests, property tests for money and aggregation, migration fixtures, crypto vectors.
- **Tauri commands:** integration tests against temporary ledgers and in-memory or on-disk SQLite.
- **Web UI:** Vitest and Testing Library for components; Playwright or Tauri driver for end-to-end flows on Windows.
- **Android:** instrumented tests for plugins, sync, biometric gate, and file export; TalkBack and manual device checks.
- **Accessibility:** axe-core automated checks plus manual screen-reader and keyboard audits.
- **Localization:** key completeness, zh-Hans layout, and no hardcoded user-facing strings.
- **Sync:** two-client simulation against a test server, offline/reconnect, conflict fixtures, snapshot restore.
- **Performance:** 10,000-transaction fixtures, virtualized lists, startup and search budgets.

## 8. Build, CI, and release

- **Workspace:** Cargo workspace for the core and Tauri backends; a Node workspace for `ui` and `packages`; Gradle/NDK for Android.
- **CI:** Linux for core tests; Windows for the desktop build and Tauri tests; Linux with Android SDK/NDK and an emulator for the Android build and tests.
- **Signing:** Windows code-signing certificate; Android keystore; secrets in CI only.
- **Packages:** MSI or NSIS for Windows; AAB for Android.
- **Versioning:** semantic versions for clients; protocol, schema, and export versions evolve independently.
- **Auto-update:** deferred to v1.1. V1 distributions are manual, signed installers.
- **Release evidence:** exact builds, tests, manual accessibility checks, and known limitations are recorded for each release.

## 9. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Tauri 2 Android plugin complexity | Background sync, biometrics, file access | 10-day spike; custom Kotlin plugins; Compose fallback |
| WebView2/Android WebView differences | Layout, IME, accessibility | Semantic HTML/ARIA, cross-webview tests, minimum WebView policy |
| SQLCipher and key storage on Windows | Local data protection | DPAPI spike; fallback to OS encryption with a recorded limitation |
| Money handling in TypeScript | Incorrect amounts | Rust owns parsing/arithmetic; UI only formats strings |
| Accessibility of a web UI | Compliance and usability | Radix primitives, keyboard audits, screen-reader testing |
| Two Tauri apps diverge | Inconsistent behavior | Shared `ui` package, shared core, visual and E2E tests |
| Android background limits | Stale sync | WorkManager plus sync on open; honest sync status |
| Bundle size and startup | Desktop/mobile quality | Vite code splitting, lazy routes, performance budgets |
| IPC type drift | Runtime errors | Generated TypeScript bindings; CI check |

## 10. Decisions accepted at R5

Approved by the project owner on 2026-11-23, conditional on the spike evidence in section 11. The recommendations below are accepted.

| ID | Decision | Recommendation |
| --- | --- | --- |
| D5.1 | Web framework | React + TypeScript + Vite |
| D5.2 | UI primitives | Radix UI + Tailwind CSS |
| D5.3 | Data layer | TanStack Query over typed Tauri commands |
| D5.4 | IPC typing | `specta`/`tauri-specta` generated TypeScript bindings |
| D5.5 | Repository layout | Separate `desktop` and `android` Tauri apps sharing `ui` and the core |
| D5.6 | Android plugins | WorkManager, BiometricPrompt, Keystore, Storage Access Framework |
| D5.7 | Localization | `i18next` with en and zh-Hans resources |
| D5.8 | Charts | Accessible SVG charts plus data tables |
| D5.9 | App lock | Optional biometric/platform-credential lock, off by default in v1 |
| D5.10 | Auto-update | Deferred to v1.1 |

## 11. Spike evidence and next review

The following must be observed before R5 is final:

- **Windows:** WebView2 accessibility with NVDA/Narrator, Chinese IME, keyboard-only flows, SQLCipher open/reopen, E2EE round-trip, 10,000-transaction list, MSI/NSIS install.
- **Android:** Tauri 2 app on Android 10 and a current release, TalkBack, SQLCipher, E2EE, WorkManager background sync through a Kotlin plugin, biometric unlock, signed AAB.
- **Kotlin baseline:** Compose Desktop accessibility and Compose Android behavior for comparison.
- Results are recorded in `docs/clients/SPIKE_RESULTS.md` with exact devices, builds, and observed outcomes.

R6 — quality and delivery: test strategy, CI, release gates, contribution and license policy. R5 approval is conditional: the spike evidence must be recorded in `docs/clients/SPIKE_RESULTS.md` before implementation begins.
