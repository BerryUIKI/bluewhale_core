# Contributing to Bluewhale Core

Thank you for your interest in Bluewhale. This repository is in a pre-alpha state: the design review is complete, the Phase 0 spike is still running, and the workspace is a skeleton. The most useful contributions right now are design feedback, bug reports with reproductions, and tests.

By participating you agree to follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- Read the [documentation index](docs/README.md) and the design document that covers your change. Behavior, schema, and protocol changes need an approved design before code. Open a design-decision issue first when the change is not already covered.
- Check existing issues and pull requests before starting work.
- Keep changes small and focused. Trunk-based development: `main` must stay releasable.

## Licensing and sign-off

- The project is licensed under the Apache License, Version 2.0. By contributing you agree that your contribution is licensed under the same terms.
- Every commit must carry a Developer Certificate of Origin sign-off: use `git commit -s`. The sign-off certifies that you wrote the change or have the right to submit it under the project license.
- Do not copy code, assets, test fixtures, or documents from the private Apple application or from any other project without a recorded licensing decision.

## Development setup

Prerequisites:

- Rust 1.97.1, installed through [rustup](https://rustup.rs/). `rust-toolchain.toml` selects the correct version and components automatically.
- A C compiler toolchain for your platform. On Windows install Visual Studio 2022 Build Tools with the C++ workload. Building the vendored SQLCipher/OpenSSL dependencies on Windows also needs a native Windows Perl distribution (for example Strawberry Perl) and `nmake` on `PATH`; the Phase 0 spike records the exact setup.
- `cargo-deny` for the dependency check: `cargo install cargo-deny --locked`.

Commands:

```text
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo test -p core-domain <test_name>                    # single test by name
cargo doc --workspace --no-deps
cargo deny check
```

CI runs the same commands on Linux and Windows. A pull request must not merge with failing or unobserved checks.

## Commit and pull-request conventions

- Use Conventional Commits: `type(scope): summary`, with `type` in `feat`, `fix`, `docs`, `refactor`, `test`, `build`, `ci`, `chore`, `perf`.
- Write commit subjects in the imperative mood and keep them under 72 characters.
- Open a pull request against `main`. Describe the scope, link the design document or issue, list the commands you ran, and state the schema or migration impact.
- Never include real financial data, credentials, encryption keys, or ciphertext in commits, issues, screenshots, or logs.

## Documentation

Behavior and schema changes update their design document and `CHANGELOG.md` in the same pull request. Documents are written in English and use LF line endings. See [docs/README.md](docs/README.md) for the index and the ADR process.

## Security

Do not report vulnerabilities as public issues. Follow [SECURITY.md](SECURITY.md).
