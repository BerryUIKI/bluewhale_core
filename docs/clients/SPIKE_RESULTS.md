# Phase 0 Spike Results

## Document Status

- Status: In progress
- Date: 2026-11-23
- Gate: Phase 0 — Rust + Tauri spike
- Spike workspace: temporary directory outside the production repositories; spike code is not committed

## Environment

| Component | Observed |
| --- | --- |
| OS | Windows 10 Pro 19045 |
| Rust | 1.97.1, `x86_64-pc-windows-msvc` |
| MSVC | Visual Studio 2022 Community with VC tools |
| Node | 24.15.0 |
| pnpm | 11.18.0 |
| Tauri CLI | 2.12.6 (local dev dependency) |
| WebView2 Runtime | 136.x installed |
| Go | 1.25.5 (server work; not part of the Windows spike) |
| Android JDK / SDK / NDK | Not installed |

## Spike workspace

- Scaffolded with `create-tauri-app` 4.7.5 using the React + TypeScript template and Tauri 2.
- Identifier: `com.berryui.bluewhale.spike`.
- Generated stack: React 19, Vite 8, TypeScript 6, Rust edition 2024.
- `pnpm install` completed successfully.

## Windows gates

| Gate | Status | Evidence |
| --- | --- | --- |
| Tauri 2 scaffold | Passed | React + TypeScript template generated |
| Dependency install | Passed | `pnpm install` completed |
| Debug build without bundle | Passed | `pnpm tauri build --debug --no-bundle`, 3m 24s, 13,718,528-byte exe |
| WebView2 launch | Passed | Process stayed alive for 5 seconds, then stopped by the test harness |
| NVDA/Narrator accessibility | Pending | Not yet run |
| Chinese IME | Pending | Not yet run |
| Keyboard-only flow | Pending | Not yet run |
| SQLCipher open/reopen | Pending | Not yet implemented |
| Argon2id + AEAD E2EE round-trip | Pending | Not yet implemented |
| 10,000-transaction list | Pending | Not yet implemented |
| MSI/NSIS install | Pending | Not yet built |

## Android gates

Blocked until the toolchain is installed:

- JDK 17 or 21.
- Android SDK platform, build-tools, and NDK.
- Emulator and system images.
- Rust targets `x86_64-linux-android` and `aarch64-linux-android`.

## Observations

- The scaffold sets `app.security.csp` to `null`. Production must use a strict content security policy.
- The default capability includes `core:default` and `opener:default`. Production must minimize capabilities and remove unused plugins.
- The scaffold is suitable for validation only; production project structure follows R5.
- Build artifact: `src-tauri/target/debug/bluewhale-spike-app.exe`.
- Launch smoke test started the executable and observed a live process for five seconds; visual, IME, and screen-reader checks are still pending.

## Next actions

1. Complete the Windows debug build and launch the app.
2. Add SQLCipher, Argon2id/AEAD, and a 10,000-row virtualized list to the spike.
3. Run NVDA/Narrator, Chinese IME, and keyboard checks.
4. Build the MSI/NSIS installer.
5. Install the Android toolchain and run the Android gates.
6. Replace this document with observed results and evidence links.
