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

Workspace-wide `--all-features` is not used: `core-persistence` will expose mutually exclusive SQLite features, so CI runs explicit feature sets per crate instead.

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

Android client builds additionally need JDK 21, the Android SDK with the pinned platform, build-tools, and NDK, plus the Rust Android targets. The client setup is recorded in `bluewhale_clients` once the Phase 0 Android gates are complete.
