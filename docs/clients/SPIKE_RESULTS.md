# Phase 0 Spike Results

## Document Status

- Status: Phase 0 evidence complete on 2026-11-24; residual release items are listed in the Android gates section
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

Toolchain installed on 2026-11-23 and 2026-11-24:

- Temurin JDK 21.0.13.1+2 (portable zip), SHA-256 verified before extraction; renamed to a plus-free path (`D:\dev\android\jdk\jdk-21`).
- Android command-line tools 22.0, SHA-256 verified before extraction.
- SDK packages installed: `platform-tools`, `platforms;android-37.0`, `build-tools;37.0.0`, `ndk;27.2.12479018`, `emulator`, and `system-images;android-36;google_apis;x86_64`. Tauri CLI 2.12.6's Android template compiles and targets API 37, so platform 37.0 is required; no API 37 Google APIs x86_64 system image is published yet, so the emulator uses the API 36 image.
- Rust targets: all four Android targets (`aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, `x86_64-linux-android`) are installed on the pinned 1.97.1 toolchain. `cargo check --workspace --target aarch64-linux-android` and the `x86_64-linux-android` equivalent pass for the empty crates. (An earlier `aarch64-linux-android` target belonged to the default stable toolchain, not to the pinned toolchain.)
- Emulator: AVD `bluewhale-api36` (Pixel 6 profile, Android 16 / API 36, `google_apis`, x86_64) boots with WHPX acceleration; `adb` reports `emulator-5554` online, the display is 1080x2400, and the TalkBack package `com.google.android.marvin.talkback` is installed.
- `pnpm tauri android init --ci` generated the Gradle project with `compileSdk`/`targetSdk` 37, `minSdk` 24, application ID `com.berryui.bluewhale.spike`, and the Gradle 9.6.1 wrapper.

Observed on Android so far:

- A debug x86_64 APK with SQLCipher was assembled and installed; the app launches cold in about 2.5 seconds and renders the shared UI in the Android WebView.
- WebView checks over the Chrome DevTools Protocol: the SQLCipher button returned `sqlcipher ok: 8192 bytes, header encrypted, wrong key rejected`; the E2EE button returned `e2ee ok: argon2id + chacha20poly1305, 36 ciphertext bytes`; and the list generated 10,000 rows in 42.5 ms with 24 DOM rows and a 320,000 px scroll height.
- UI Automator exposes the WebView content: the text, the four buttons, the name field, and the named list are present as accessibility nodes. Tab focus moves through the controls and reaches the buttons.
- TalkBack is installed, enabled, bound as an accessibility service, and its notification permission was granted. While TalkBack is active, logcat shows Google TTS synthesis requests and speech audio-focus activity. The exact spoken text has not been captured; a short listening pass remains.
- WorkManager through the Tauri Kotlin plugin bridge: the app command calls a Rust command, which invokes the Kotlin `WorkManagerPlugin` through the Tauri mobile plugin handle; the Kotlin plugin enqueues a 5-second job. After the app was backgrounded, logcat recorded `workmanager_plugin_scheduled` and `workmanager_worker_ran pid=... source=plugin`. A direct `OneTimeWorkRequest` from `MainActivity` also ran with `source=activity`.
- Signed AAB: the release Rust library (11,269,056 bytes) was packaged into a 7,235,872-byte x86_64 AAB, signed with a temporary upload key (`CN=Bluewhale Spike, O=BerryUIKI, C=US`, 2048-bit RSA). `jarsigner -verify` returned `jar verified` (exit 0); `-strict` reports JarInputStream warnings that are typical for Gradle-built bundles. The bundle contains `base/lib/x86_64/libbluewhale_spike_app_lib.so`. An x86_64-only bundle is a spike artifact; a Play release needs the ARM ABIs.
- BiometricPrompt and Keystore: `BiometricManager.canAuthenticate(BIOMETRIC_WEAK or DEVICE_CREDENTIAL)` returned success (0); the device-credential prompt authenticated with the emulator PIN (`biometric_auth_succeeded`); a plain Android Keystore AES-GCM round-trip returned `keystore_roundtrip_ok=true`; and an AES key created with `setUserAuthenticationRequired(true)` and `AUTH_BIOMETRIC_STRONG` authenticated through a fingerprint-bound `CryptoObject`, logging `keystore_auth_callback_success crypto=true` and `keystore_auth_encrypt_ok=true`. This closes the BiometricPrompt/Keystore app-lock mechanism at the platform level; the product key hierarchy still needs to wrap the ledger key with this platform key.
- Synthetic taps: an earlier series of taps missed because the page had scrolled and the UI Automator bounds were stale. With fresh bounds, `adb input tap` activated both the E2EE button and the 10,000-row button (10,000 rows in 104.5 ms). The WebView touch path is functional; automation must re-read bounds after layout or scroll changes. Keyboard focus traversal reaches the buttons, but Enter activation was not observed.

Gates that remain as release work rather than Phase 0 blockers:

- TalkBack spoken-output listening pass (service and TTS confirmed active).
- Sync scheduling from the Rust core on top of the WorkManager plugin.
- ARM ABIs and a real upload key for a Play-bound AAB.

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
- The first Android APK build failed while configuring the vendored OpenSSL: `openssl-src` invoked Git's Cygwin Perl and could not load `Locale::Maketext::Simple` from `/usr/share/perl5`. This is the same class of failure as the earlier Windows build. The rebuild prepends Strawberry Perl's `perl\bin` and `c\bin` to `PATH` and unsets `PERL5LIB`, matching the Windows setup.
- For an Android target, `openssl-src` rejects Strawberry Perl because it does not produce Unix-like paths, and Git's Cygwin Perl lacks the required modules. The working local setup is MSYS2 Perl 5.44 and GNU Make 4.4, the NDK `toolchains/llvm/prebuilt/windows-x86_64/bin` directory on `PATH`, and `CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER` pointing at the NDK's `x86_64-linux-android24-clang.cmd`. With that, OpenSSL and SQLCipher compiled and linked, producing a 153,473,976-byte debug `libbluewhale_spike_app_lib.so`. The Android Gradle build still runs on Linux in CI; the Windows steps are a local workaround.
- Tauri's Android CLI creates a symbolic link from the Rust `.so` into the generated `jniLibs`; Windows refused it because Developer Mode is off and the shell lacks the symbolic-link privilege. For the spike, the `.so` and the frontend `dist` were copied into the generated project and Gradle's `assembleX86_64Debug` was run with the `rustBuild*` tasks excluded. The result was a 293,747,278-byte debug APK with SQLCipher that installs and runs.
- UI Automator exposes the Android WebView content without TalkBack. The WebView also allows Chrome DevTools Protocol access over `adb forward` in the debug build, which was used for the E2EE and 10,000-row checks because synthetic taps did not activate the buttons.
- The UI Automation tree is built lazily. The first query returned 17 Pane nodes; a second query after two seconds returned 38 nodes and exposed the document content: Text, Hyperlink, Image, Edit, four named Buttons, and a named List. Screen-reader-specific speech output has not been captured automatically, so a manual NVDA/Narrator pass remains.

## Next actions

1. Run a manual NVDA/Narrator pass over the release build and record the spoken output.
2. Finish the Android SDK installation, create the emulator, and run the Android gates.
3. Replace this document with final observed results and evidence links.
