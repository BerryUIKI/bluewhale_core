# Development setup

## Prerequisites

- Rust 1.97.1 through `rustup`. `rust-toolchain.toml` selects the toolchain and components; `rustup show` confirms it.
- A C toolchain for the host. On Windows, Visual Studio 2022 with the "Desktop development with C++" workload (MSVC and the Windows SDK).
- `cargo-deny` for the dependency policy check: `cargo install cargo-deny --locked`.
- Node 24.15 and pnpm 11.18 only when working on the client repositories.

## Commands

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo test -p core-domain <test_name>
cargo doc --workspace --no-deps
cargo deny check
```

Any workspace command that builds `core-persistence` (`cargo clippy --workspace`, `cargo test --workspace`, `cargo build`) needs this environment. Commands that only touch the other crates, such as `cargo test -p core-domain`, do not.

## Windows: vendored SQLCipher and OpenSSL

`core-persistence` builds SQLCipher with vendored OpenSSL. The OpenSSL build requires a native Windows Perl and `nmake`; the Cygwin Perl shipped with Git is rejected by OpenSSL's `VC-WIN64A` configuration because it does not produce Windows-style paths.

The Phase 0 spike validated this setup:

1. Install the Visual Studio C++ workload. `nmake.exe` is under `VC\Tools\MSVC\<version>\bin\Hostx64\x64`.
2. Install a native Windows Perl, for example [Strawberry Perl](https://strawberryperl.com/). A portable extraction works.
3. In the build shell, call `vcvars64.bat`, prepend the Perl `bin` directory to `PATH`, and run the cargo command in the same shell:

```text
call "C:\Program Files\Microsoft Visual Studio\2022\Community\VC\Auxiliary\Build\vcvars64.bat"
set PATH=C:\path\to\strawberry\perl\bin;C:\path\to\strawberry\c\bin;%PATH%
cargo test --workspace
```

Environment changes do not persist between separate CI steps; set `PATH` at job level or run the setup and the cargo command in one step.

## Troubleshooting

- **OpenSSL `nmake` exit code `0xc0000142`**: the build process was interrupted or started without a usable environment. Re-run from a clean shell with `vcvars64.bat` and the native Perl on `PATH`.
- **OpenSSL rejects the Perl implementation**: the shell is using Git's Cygwin Perl. Put the native Windows Perl first on `PATH` and confirm with `perl -v`.
- **Stale lock or partial build**: stop the running cargo process before retrying; do not delete another user's build directory.
- **`cargo deny` license failure**: check `deny.toml` and the dependency's license before adding it, and record the decision in the pull request.

## Android

Android client builds additionally need:

- JDK 21 (Temurin 21.0.13.1+2 was used for the Phase 0 spike).
- Android SDK command-line tools, platform 37.0, build-tools 37.0.0, NDK 27.2.12479018, an emulator, and a system image. Tauri 2.12 CLI compiles and targets API 37; the emulator can run a lower API image.
- `JAVA_HOME`, `ANDROID_HOME`, `ANDROID_SDK_ROOT`, and `NDK_HOME` set to those paths.
- The Rust Android targets: `aarch64-linux-android`, `armv7-linux-androideabi`, `i686-linux-android`, and `x86_64-linux-android`.

Windows host notes:

- Tauri's Android CLI creates a symbolic link from the built Rust library into the generated project. Without Windows Developer Mode or an elevated shell, symbolic-link creation is refused. Enable Developer Mode, or copy the `.so` into `src-tauri/gen/android/app/src/main/jniLibs/<abi>/`, copy the frontend `dist` into `app/src/main/assets/`, and run the Gradle `assemble<Abi>Debug` task with the `rustBuild*` tasks excluded.
- Cross-compiling the vendored OpenSSL for Android needs a Unix-like Perl and `make`. Git's Cygwin Perl lacks required modules, and Strawberry Perl is rejected because it produces Windows paths. Use an MSYS2 Perl and `make`, or build Android on the Linux CI runner where the cross-compile is supported. This applies only to the SQLCipher/OpenSSL dependency, not to the rest of the Android build.
- Keep the project path short and free of spaces; the Android Gradle build and the NDK are sensitive to long paths.

The full Android procedure is recorded in `docs/clients/SPIKE_RESULTS.md`.

### Signing an Android release

The upload key is never committed. Create it once per signing identity:

```text
keytool -genkeypair -v -keystore upload.jks -storetype JKS -keyalg RSA -keysize 2048 -validity 10000 -alias upload
```

Configure the generated Android project's release signing from environment variables (`BLUEWHALE_ANDROID_KEYSTORE`, `BLUEWHALE_ANDROID_KEYSTORE_PASSWORD`, `BLUEWHALE_ANDROID_KEY_ALIAS`, `BLUEWHALE_ANDROID_KEY_PASSWORD`) and build the bundle with the Tauri CLI or Gradle. Verify the result before distributing it:

```text
jarsigner -verify -strict app-release.aab
```

A Play-bound AAB needs the ARM ABIs as well; an x86_64-only bundle is a spike artifact and is never published. The application ID cannot change after the first Play release, so confirm the product name and identifier first.
