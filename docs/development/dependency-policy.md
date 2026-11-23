# Dependency policy

## Principles

- Every dependency is a long-term maintenance and security obligation. Prefer the standard library and a small, well-maintained crate over a large dependency tree.
- A dependency must be usable under the project's license policy and compatible with the MSRV (`1.97.1`).
- Financial correctness must not depend on an unmaintained crate.

## Adding a dependency

1. State in the pull request what the dependency provides, why the standard library or an existing dependency is insufficient, and its license.
2. Confirm the license is on the allowlist in `deny.toml`.
3. Check the crate's maintenance status, open advisories, and whether it pulls a large tree. `cargo tree -e normal -p <crate>` shows the impact.
4. Run `cargo deny check` and the full test suite.
5. Update `CHANGELOG.md` only when the dependency changes user-visible behavior.

## License allowlist

`deny.toml` allows: Apache-2.0, MIT, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, and Zlib.

MPL-2.0 and other copyleft licenses are not allowed by default. Adding one requires a recorded exception in the pull request and an update to this document and `deny.toml` in the same change. The HTTP transport deliberately uses the platform trust store instead of an MPL-licensed root bundle.

## Sources and versions

- Dependencies come from crates.io. A git dependency, path dependency, or alternative registry requires an ADR.
- Wildcard version requirements are denied. Internal workspace dependencies use exact `=x.y.z` pins so a release cannot mix crate versions.
- `Cargo.lock` is committed for reproducible CI.

## Advisories and updates

- CI runs `cargo deny check` on every pull request and a weekly RustSec advisory audit.
- Dependabot opens grouped weekly updates. A security advisory is handled as a priority even if the grouped update is not ready.
- A yanked version is never kept in a release.

## Build-time and test-only dependencies

- Build scripts must not download code at build time. Vendored sources are permitted only when the license is recorded and the build is reproducible.
- Test-only dependencies follow the same license policy. A testkit dependency must never become a normal dependency of a production crate.

## Software bill of materials

A CycloneDX SBOM is generated for releases once the first functional release exists. It is attached to the release and retained with the release artifacts.
