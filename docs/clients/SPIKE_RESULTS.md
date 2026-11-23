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
| Android JDK | Temurin 21.0.13.1+2, portable at `D:\dev\android\jdk` |
| Android SDK | Command-line tools 22.0 at `D:\dev\android\sdk`; packages installing |
| Android NDK | 27.2.12479018, queued with the SDK packages |

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
| Release build and bundles | Passed | `pnpm tauri build`, 13m 29s, 10,448,896-byte exe; MSI 4.48 MiB; NSIS 3.33 MiB |
| WebView2 launch | Passed | Process stayed alive for 5 seconds, then stopped by the test harness |
| WebView2 UI checks | Passed | Release exe and installed exe: SQLCipher and E2EE buttons returned expected text; 10,000-row list rendered |
| NVDA/Narrator accessibility | Partially verified | UI Automation exposes Text, Hyperlink, Image, Edit, Button (four named buttons) and a named List; a manual NVDA/Narrator listening pass is still pending |
| Chinese IME | Passed | Microsoft Pinyin composed `你好` in the WebView2 text field through OS-level input |
| Keyboard-only flow | Passed | OS-level Tab sequence reached every control; Enter on the focused SQLCipher button produced `sqlcipher ok` |
| SQLCipher open/reopen | Passed | `cargo test --lib`: encrypted header, reopen, wrong key rejected; 2 tests passed |
| Argon2id + AEAD E2EE round-trip | Passed | Argon2id + ChaCha20Poly1305 round-trip; 2 tests passed in 0.70s |
| 10,000-transaction list | Passed | Release WebView2: 10,000 rows in 4.0 ms, 23 DOM rows, 320,000 px scroll height; installed copy: 3.6 ms |
| MSI install | Blocked by test environment | MSI built; silent installation requires elevation and the standard-user test account could not complete it |
| NSIS install/uninstall | Passed | Per-user silent install to `%LOCALAPPDATA%\\bluewhale-spike-app`; installed app passed the three UI checks; silent uninstall removed the files and uninstall entry |

## Android gates

Toolchain installation started on 2026-11-23:

- Temurin JDK 21.0.13.1+2 (portable zip), SHA-256 verified before extraction.
- Android command-line tools 22.0, SHA-256 verified before extraction.
- SDK package installation queued: `platform-tools`, `platforms;android-37.0`, `build-tools;37.0.0`, `ndk;27.2.12479018`, `emulator`, and `system-images;android-36;google_apis;x86_64`. Tauri CLI 2.12.6's Android template compiles and targets API 37, so platform 37.0 is required; no API 37 Google APIs x86_64 system image is published yet, so the emulator uses the API 36 image. The download is several gigabytes and the observed network throughput is roughly 50-150 KB/s, so completion will take hours.
- Rust targets: all four Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`) are installed on the pinned 1.97.1 toolchain. `cargo check --workspace --target aarch64-linux-android` and the `x86_64-linux-android` equivalent pass for the empty crates. (An earlier `aarch64-linux-android` target belonged to the default stable toolchain, not to the pinned toolchain.)

Gates that remain:

- Tauri Android build and debug APK on an emulator.
- TalkBack accessibility check.
- WorkManager background sync through a Kotlin plugin.
- BiometricPrompt/Keystore app lock.
- Signed AAB.

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
- The first release build failed inside vendored OpenSSL with `nmake` exit code `0xc0000142` after the previous background build was terminated. Re-running from a clean `cmd.exe` wrapper that calls `vcvars64.bat`, prepends Strawberry Perl, and then runs `pnpm tauri build` completed in 13m 29s. The wrapper is the supported build entry point for this spike.
- The release exe and both installers were produced with `bundle.targets = "all"`. Tauri downloaded the WiX and NSIS tools itself; no manual installer toolchain setup was needed.
- WebView2 automation used `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and the Chrome DevTools Protocol. `msedgedriver` and the Microsoft download pages were unreachable from this network (TLS failure), so `tauri-driver` was not usable; CDP provided equivalent DOM, key, and accessibility-tree access.
- In the release WebView2 window the three checks returned `sqlcipher ok: 8192 bytes, header encrypted, wrong key rejected`, `e2ee ok: argon2id + chacha20poly1305, 36 ciphertext bytes`, and `10,000 rows generated in 4.0 ms` with 23 DOM rows and a 320,000 px scroll height.
- The NSIS package is per-user: silent `/S` installation to `%LOCALAPPDATA%\\bluewhale-spike-app` succeeded without elevation, the installed executable reproduced all three checks (10,000 rows in 3.6 ms), and the silent uninstaller removed both the files and the uninstall entry. The MSI package is per-machine; a standard-user account cannot complete a silent installation and the attempt was cancelled.
- OS-level keyboard input was sent through `SendKeys` after focusing the WebView document with UI Automation. Six Tab presses reached the links, the name field, Greet, and Run SQLCipher check; Enter produced the SQLCipher result. CDP key events independently produced the same focus order, and typed characters reached the name field.
- Microsoft Pinyin was activated with Win+Space and typed at the OS level; the WebView2 input received the composed text `你好`. The first attempt stayed in the English layout and was repeated after another layout switch.
- Tauri CLI 2.12.6's Android template pins Android Gradle Plugin 9.3.1, Gradle 9.6.1, Kotlin 2.2.10, and `compileSdk`/`targetSdk` 37. Gradle 9.6.1 supports JVM 17-26, so the installed Temurin JDK 21 is accepted. The JDK path still contains a `+` character; it is renamed to a plus-free path after the SDK installation finishes.
- The UI Automation tree is built lazily. The first query returned 17 Pane nodes; a second query after two seconds returned 38 nodes and exposed the document content: Text, Hyperlink, Image, Edit, four named Buttons, and a named List. Screen-reader-specific speech output has not been captured automatically, so a manual NVDA/Narrator pass remains.

## Next actions

1. Run a manual NVDA/Narrator pass over the release build and record the spoken output.
2. Finish the Android SDK installation, create the emulator, and run the Android gates.
3. Replace this document with final observed results and evidence links.
