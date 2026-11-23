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
| Perl | Strawberry Perl 5.40.5 portable, native Windows build |
| nmake | MSVC 14.44.35207 Hostx64/x64 |
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
| SQLCipher open/reopen | Passed | `cargo test --lib`: encrypted header, reopen, wrong key rejected; 2 tests passed |
| Argon2id + AEAD E2EE round-trip | Passed | Argon2id + ChaCha20Poly1305 round-trip; 2 tests passed in 0.70s |
| 10,000-transaction list | Passed in Chromium preview | 10,000 rows in 4.4 ms; 23 DOM rows initially, 35 after scrolling; 320,000 px scroll height. Tauri WebView2 check pending. |
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
- The built Vite UI was served to the in-app Chromium preview. Generating 10,000 rows took 4.4 ms; only 23 rows were in the DOM initially and 35 after scrolling to 160,000 px, with a 320,000 px scroll height. This validates the virtualization approach, not the final WebView2 runtime check.
- Strawberry Perl 5.40.5 resolved the native Windows Perl requirement. OpenSSL 3.6.3 and SQLCipher built from source with MSVC `nmake`; the linker PDB warnings are benign.
- `cargo test --lib` passed both spike checks: SQLCipher wrote an encrypted database, reopened it, and rejected a wrong key; Argon2id + ChaCha20Poly1305 completed a round-trip. The wrong-key SQLCipher "hmac check failed" log lines are the expected rejection and not a test failure.
- The same checks are exposed as Tauri commands and UI buttons for the WebView2 application.
- `cargo test --lib` failed in `openssl-sys` while building vendored OpenSSL. The shell pipeline used `tee` without `pipefail`, so an earlier background task reported exit code 0 even though cargo failed; it was not a test pass.
- Git's Cygwin Perl is not usable for OpenSSL's `VC-WIN64A` target. Patching missing modules (`Locale::Maketext::Simple`, `ExtUtils::MakeMaker`) through a `/tmp` `PERL5LIB` let `Configure` progress, but it then rejected Cygwin Perl because it does not produce Windows-style paths: "This perl implementation doesn't produce Windows like paths".
- Next attempt: use Strawberry Perl portable as the build Perl, prepend its `perl/bin` to `PATH`, add MSVC `nmake`, and rerun `cargo test --lib`. A 304 MB portable download is in progress and is resumed from the partial file.
- If SQLCipher remains blocked, record the gap and evaluate the approved OS-level-encryption fallback.

## Next actions

1. Build the MSI/NSIS installer with the release profile.
2. Launch the Tauri app and exercise the SQLCipher and E2EE buttons in WebView2.
3. Run NVDA/Narrator, Chinese IME, and keyboard checks.
4. Install the Android toolchain and run the Android gates.
5. Replace this document with final observed results and evidence links.
