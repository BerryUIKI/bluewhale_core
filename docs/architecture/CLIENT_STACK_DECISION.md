# Client Stack Decision: Tauri + Rust vs Kotlin

## Document Status

- Status: Accepted (R2 decision)
- Date: 2026-11-23
- Depends on: R1 product specification; R2 architecture
- Decision: T1 and T2 accepted — Option B, Rust core + Tauri 2, subject to the spike gates in section 11
- Approved by: project owner on 2026-11-23
- Supersedes: the provisional Kotlin Multiplatform recommendation in R2 revision 1

## 1. Decision to make

Choose the implementation stack for:

- the shared open-source core in `bluewhale_core`;
- the Windows and Android clients in `bluewhale_clients`.

The server is already decided: Go + PostgreSQL + object storage, coupled only through the versioned sync protocol. This document therefore focuses on the client side and the core language.

Candidates:

| Option | Core | Windows UI | Android UI |
| --- | --- | --- | --- |
| A. Kotlin Multiplatform | Kotlin Multiplatform | Compose Multiplatform (JVM) | Compose Multiplatform (native) |
| B. Rust + Tauri 2 | Rust | Tauri 2 + WebView2 | Tauri 2 + Android WebView |
| C. Rust + Tauri/Compose hybrid | Rust | Tauri 2 + WebView2 | Jetpack Compose (native) |
| D. Rust + Flutter | Rust | Flutter Windows | Flutter Android |
| E. Rust + native Windows | Rust | WinUI 3 (C#) | Jetpack Compose |

## 2. Requirements that drive the decision

- Windows 10/11 and Android 10+ are both first-class targets.
- Local-first, offline-complete core; optional E2EE synchronization.
- Exact integer money and a domain core that is testable without UI.
- Open-source core and clients (Apache-2.0); the core should remain reusable beyond one UI stack.
- Windows accessibility and IME are explicit requirements (keyboard operation, screen readers).
- Lean v1; a small team must be able to deliver and maintain it.
- Future features will expand the domain (budgets, plans, liabilities, reports), so the core must scale.
- English and Simplified Chinese, including non-Latin input on Windows.

## 3. Option A — Kotlin Multiplatform + Compose Multiplatform

**Shape**

- Core: Kotlin Multiplatform, targets `jvm` and `android`.
- Windows: JVM desktop application rendered by Compose Multiplatform (Skia).
- Android: native Kotlin/Compose.
- Persistence: SQLDelight; crypto via Tink and an Argon2id library.

**Strengths**

- One language for core, Android, Windows, and tests.
- Android is first-class; Compose is the native Android UI toolkit.
- Mature Gradle/Android Studio tooling, coroutines, sealed types, value classes.
- One UI codebase; SQLDelight and Tink are proven.
- Fastest path for a Kotlin/Android-centric team.

**Weaknesses**

- Windows is a JVM application, not a native WinUI application. Startup, memory, installer size, and native integration are weaker than a native shell.
- Compose Multiplatform desktop accessibility and IME support are improving but less mature than Chromium-based WebView2. Screen-reader behavior on Windows needs validation.
- Native menus, file dialogs, tray, and deep OS integration require JNA or platform-specific code.
- The core is reusable mainly from Kotlin/JVM; reuse from a future CLI, iOS, or other runtimes is narrower than a Rust core.
- The server is Go, so the core cannot be shared with it anyway; the benefit of Kotlin on the server disappears.

## 4. Option B — Rust core + Tauri 2 + web UI

**Shape**

- Core: Rust crate containing domain, application use cases, persistence, sync, and crypto.
- Windows: Tauri 2 shell with WebView2 and a TypeScript web UI.
- Android: Tauri 2 mobile with Android WebView and the same web UI.
- Persistence: `rusqlite` with bundled SQLCipher; migrations embedded in the crate.
- Crypto: RustCrypto (`argon2`, `chacha20poly1305`, `zeroize`) or libsodium bindings.
- FFI: Tauri calls Rust directly on both platforms; UniFFI/C ABI remains available for future clients.

**Strengths**

- Rust core is memory-safe, fast, language-neutral, and reusable by future CLI, web, or native clients.
- Strong ecosystem for exact arithmetic, SQLite/SQLCipher, and cryptography.
- Tauri produces a small Windows application using the system WebView2 runtime.
- WebView2 is Chromium: mature accessibility (UIA), IME, text rendering, and keyboard behavior when the web UI uses semantic HTML and ARIA.
- One UI codebase for Windows and Android; web tooling and UI ecosystem are mature.
- Tauri's capability model is a security advantage for a finance application.
- The Go server fits a language-neutral JSON protocol; no shared runtime is required.

**Weaknesses**

- Tauri 2 mobile is younger than Kotlin/Compose or Flutter. Background sync, biometric unlock, secure key storage, notifications, and share targets may require custom Kotlin plugins.
- Android WebView behavior varies with the installed system WebView; performance and accessibility need device testing.
- Two languages and two build systems: Rust/Cargo plus TypeScript/Node/Tauri.
- Rust has a steeper learning curve and slower iteration than Kotlin.
- Windows installer defaults are MSI/NSIS; MSIX requires additional packaging work.
- End-to-end encryption, key recovery, and background sync are core code that must be testable on both platforms.

## 5. Option C — Rust core + Tauri Windows + Jetpack Compose Android

**Shape**

- Rust core shared by both clients.
- Windows: Tauri 2 + WebView2 + TypeScript UI.
- Android: native Kotlin/Jetpack Compose, with the Rust core exposed through UniFFI/JNI.

**Strengths**

- Best realistic platform quality on both targets without writing a C#/WinUI client.
- Native Android (Compose) avoids Tauri mobile risk.
- Windows gets WebView2 accessibility and a small app.
- The Rust core remains the single domain implementation.

**Weaknesses**

- Two UI codebases and three languages (Rust, TypeScript, Kotlin).
- Two build, test, and release pipelines; more coordination and more surface for divergence.
- Higher delivery cost than a single UI; may not fit a lean v1.

## 6. Option D — Rust core + Flutter

**Shape**

- Rust core exposed through `flutter_rust_bridge`.
- Flutter UI on Windows and Android.

**Strengths**

- One UI codebase with mature Android support and stable Windows desktop support.
- Rust core remains portable and reusable.
- Avoids both Tauri mobile risk and Compose Desktop risk.
- Strong UI testing and animation/rendering consistency.

**Weaknesses**

- Dart + Rust: two languages, code generation, and an FFI boundary.
- Flutter's desktop rendering is custom, not native; Windows accessibility and IME are good but less proven than WebView2.
- Larger application size; desktop plugin ecosystem is smaller than web.
- Flutter desktop is less common for finance/enterprise data-entry applications than web or native shells.

## 7. Option E — Rust core + native Windows + Compose Android

- Windows: WinUI 3 (C#/XAML) with the Rust core through a C ABI.
- Android: Jetpack Compose with the Rust core through UniFFI.
- Best native Windows quality and accessibility; highest cost: three languages and two UI stacks.
- Recommended only if Windows-native quality is non-negotiable after a Tauri/Compose spike fails.

## 8. Comparison

Ratings are qualitative and relative to this project; 5 is best.

| Criterion | A. KMP + Compose | B. Rust + Tauri | C. Hybrid | D. Rust + Flutter | E. Native |
| --- | --- | --- | --- | --- | --- |
| Android maturity | 5 | 3 | 5 | 5 | 5 |
| Windows accessibility/IME | 3 | 4.5 | 4.5 | 4 | 5 |
| Core longevity and reuse | 3.5 | 5 | 5 | 5 | 5 |
| One UI codebase | 5 | 5 | 1.5 | 5 | 1 |
| Language simplicity | 5 | 2 | 1 | 2 | 1 |
| Time to MVP | 4.5 | 3.5 | 2 | 3.5 | 1.5 |
| Native OS integration | 3 | 4 | 5 | 3.5 | 5 |
| Security and crypto fit | 4.5 | 5 | 5 | 5 | 5 |
| App size and startup | 3 | 4.5 | 4 | 3.5 | 5 |
| Team availability | 4.5 | 3 | 2.5 | 3.5 | 2 |

Interpretation:

- Kotlin wins on team velocity and Android simplicity, but gives up Windows quality and core portability.
- Rust + Tauri wins on core quality and Windows accessibility, but carries Android mobile risk and a two-language cost.
- The hybrid is the highest-quality realistic option, but the most work.
- Flutter is the best one-UI alternative if Tauri mobile is unacceptable.

## 9. Deep analysis

### 9.1 Core language

The core is the long-lived asset: money rules, ledger policies, validation, sync, and encryption. Rust offers stronger guarantees for a portable financial core: no garbage collector, explicit error handling, strong crypto/SQLite libraries, and reuse through UniFFI or a C ABI. Kotlin is also safe and productive, but its reuse is concentrated in JVM/Android.

Neither language is shared with the Go server; the server is coupled through the protocol, so the core language should be chosen for the clients and long-term reuse, not for server convenience.

### 9.2 Windows

This is where the options separate most:

- **Compose Desktop**: JVM/Skia, non-native widgets, weaker accessibility/IME story, larger runtime.
- **Tauri/WebView2**: Chromium accessibility, IME, and text rendering; small app; native window/menus; web UI quality determines the result.
- **WinUI 3**: best native integration and accessibility, but a separate C#/XAML UI and the highest cost.

For a data-entry finance app with Chinese input and keyboard operation, Tauri/WebView2 is the better pragmatic choice unless native WinUI is funded.

### 9.3 Android

- **Compose**: the native, lowest-risk option.
- **Tauri 2 mobile**: viable but requires custom Kotlin plugins for WorkManager background sync, BiometricPrompt, Keystore-backed key storage, notifications, and share intents. WebView performance and accessibility need device tests.
- **Flutter**: mature and consistent, with a custom renderer.

Android is the main reason to prefer Option C or D over B. A timeboxed spike must prove the Tauri mobile path before it is locked.

### 9.4 Team and velocity

- One language and one UI (Kotlin) minimizes coordination and is fastest for an Android-centric team.
- Rust + TypeScript adds a second ecosystem and a Rust learning curve, but web UI development is fast and the Rust core is small and testable.
- The hybrid adds a third language and a second UI; it should be chosen for quality, not speed.

### 9.5 Security and financial correctness

All options can satisfy exact money and E2EE. Rust has the edge for the core: memory safety without a GC, strong crypto crates, and a small trusted surface. Tauri's capability model and WebView2's sandbox are sound. Kotlin/JVM is memory-safe but carries a larger runtime and dependency surface.

### 9.6 Distribution

- Tauri Windows: MSI/NSIS, small installer, WebView2 runtime dependency (normally present on Windows 10/11).
- Compose Windows: bundled JRE, larger installer, MSIX possible.
- Flutter Windows: larger app, MSIX via tooling.
- Android: native AAB for Kotlin/Flutter; Tauri produces an AAB with a Rust library and web assets.

## 10. Recommendation

**Primary: Option B — Rust core + Tauri 2 — conditional on a timeboxed spike.**

Reasons:

- The Rust core is the strongest long-term foundation for an open-source financial engine.
- Tauri gives a materially better Windows accessibility/IME story than Compose Desktop at a smaller footprint.
- One web UI keeps the lean v1 scope manageable.
- The Go server already makes this a polyglot project; a language-neutral core and protocol are the right boundary.

**Pre-approved fallback: Option C — Rust core + Tauri Windows + Jetpack Compose Android.**

- Use it if the Tauri Android spike fails background sync, biometric unlock, secure storage, accessibility, or Play packaging.
- The Rust core investment is preserved; only the Android UI changes.

**Choose Option A (Kotlin) instead if:**

- the team is Kotlin/Android-centric and Rust capacity is not available;
- Windows is a secondary platform and time-to-MVP dominates;
- a Windows Compose accessibility/IME spike is acceptable.

**Choose Option D (Rust + Flutter) if:**

- one UI codebase is required;
- Tauri Android fails the spike;
- the team prefers Flutter over maintaining two UIs.

Option E is reserved for a later decision if Windows-native quality becomes a hard requirement.

## 11. Spike plan and hard gates

Timebox: 10 working days. The spike is throwaway code and does not enter the production repositories.

### Windows gates

- Tauri 2 window with semantic web UI, keyboard-only navigation, focus order, and error announcements.
- NVDA or Narrator reads labels, values, validation errors, and table content.
- Chinese IME input works in transaction fields.
- `rusqlite` + SQLCipher opens, writes, and reopens an encrypted database.
- E2EE round-trip with Argon2id and an AEAD cipher; keys never appear in logs.
- 10,000-transaction list remains responsive with virtualization and search.
- Native menu, file export dialog, and MSI/NSIS install/uninstall.

### Android gates

- Tauri 2 app runs on Android 10 and a current Android release, on emulator and a physical device.
- TalkBack reads labels, values, and errors; touch targets and focus order pass a manual audit.
- SQLCipher database and E2EE round-trip work on device.
- Background sync runs through WorkManager via a custom Kotlin plugin, survives app backgrounding, and is idempotent.
- Biometric unlock works through BiometricPrompt; the database key is protected by Android Keystore.
- Notifications and a signed AAB build succeed.
- 10,000-transaction list remains responsive.

### Kotlin baseline

For a fair comparison, run the equivalent Windows Compose Desktop and Android Compose spikes:

- screen-reader and IME behavior on Windows;
- SQLDelight + SQLCipher;
- startup time, memory, installer size;
- Android background sync and biometric integration.

### Decision rule

- B passes all hard gates -> lock Option B.
- Android hard gates fail -> lock Option C (or D if one UI is mandatory).
- Windows hard gates fail -> evaluate Option E or A before locking.
- Record observed evidence; do not approve a stack based on documentation alone.

## 12. Decision record

- T1 (core/client stack): accepted — Rust core + Tauri 2 (Option B), subject to the spike gates in section 11.
- T2 (UI strategy): accepted — shared TypeScript web UI plus platform-specific shell and integrations.
- T3 (server): accepted — Go + PostgreSQL + object storage.
- Approved by the project owner on 2026-11-23.
- Next action: run the 10-day spike, record observed results here, and lock or fall back before implementation.
