# ADR-0004: Toolchain, CI, and release

- Status: proposed
- Date: 2026-11-23
- Deciders: Bluewhale maintainers

## Context

The core must be reproducible on Linux and Windows, must keep the dependency policy enforceable, and must not publish a release that was not built and tested from the release commit. The Phase 0 spike showed that the Windows SQLCipher/OpenSSL build needs a native Windows Perl and `nmake`.

## Decision

- Toolchain: Rust 1.97.1, pinned by `rust-toolchain.toml`; MSRV 1.97. Node 24.15 and pnpm 11.18 for clients; Go 1.25.5 for the server.
- CI: GitHub Actions. Core runs format, clippy with `-D warnings`, tests, documentation, and `cargo-deny` on Linux, tests on Windows, and a weekly RustSec audit. Clients add UI, Windows, and Android jobs. The server adds gofmt, vet, race tests, golangci-lint, govulncheck, and migration checks.
- Dependency policy: `deny.toml` license allowlist, wildcard and source bans, and `cargo-deny` on every pull request. A new dependency requires a license check and a note in the pull request.
- Release: semantic versions and tags; a release runs from a clean commit with all required checks green. Publishing crates or producing installers requires an explicit owner decision; the CI workflows verify but do not publish until that decision is recorded.
- Signing material and registry tokens live only in protected release environments.

## Consequences

- Windows contributors need the native Perl and `nmake` setup documented in `CONTRIBUTING.md`.
- A red dependency-policy job blocks the pull request, including for transitive license changes.
- Release automation is added incrementally; the first release is a manual, documented procedure.

## Alternatives considered

| Alternative | Why it was not chosen |
| --- | --- |
| Track stable Rust without a pin | Toolchain drift makes failures non-reproducible. |
| Publish on every tag automatically | Publishing must be a deliberate owner decision until the API is stable. |
| Skip cargo-deny in pull requests | License and advisory regressions would reach `main`. |

## References

- [QUALITY_AND_DELIVERY.md](../quality/QUALITY_AND_DELIVERY.md)
- [SPIKE_RESULTS.md](../clients/SPIKE_RESULTS.md)
