# Licensing

## Repository licenses

| Repository | License | Attribution |
| --- | --- | --- |
| `bluewhale_core` | Apache License 2.0 | `NOTICE`: Bluewhale, Copyright 2026 BerryUIKI |
| `bluewhale_clients` | Apache License 2.0 | `NOTICE`: Bluewhale, Copyright 2026 BerryUIKI |
| `bluewhale_server` | Proprietary, all rights reserved | No open-source license; no `NOTICE` |
| `bluewhale-ledger` | Proprietary | Behavioral reference only; never copied |

## Source files

The public repositories do not require per-file license headers. The root `LICENSE` and `NOTICE` are the authoritative terms, and each published crate carries copies so a packaged crate is self-contained. Contributions are accepted under the Apache License 2.0 and require a Developer Certificate of Origin sign-off.

## Third-party components

- Rust and JavaScript dependencies are checked against the allowlist in `deny.toml` and `cargo-deny` on every pull request.
- The Windows and Android clients embed OpenSSL and SQLCipher. A generated third-party notices file is included in the installers and the Android about screen before the first release; this is a release gate, not an optional step.
- Do not copy code from a dependency into this repository; depend on it instead, so its license and attribution remain intact.

## Protocol and contract artifacts

The sync protocol schemas, OpenAPI document, and fixtures are authored in `bluewhale_core`. The private server may consume them in one of two ways, recorded in an ADR before the server pins a contract:

1. Commit the copied bundle under `api/protocol/` together with the upstream Apache-2.0 `LICENSE` and `NOTICE` and a provenance file with the tag, commit, and per-file SHA-256. Those files, and only those files, carry Apache-2.0 terms.
2. Fetch the bundle at build time from a tagged core commit without committing it.

The first option is recommended for reproducible private builds; it requires legal sign-off.

## Apple reference

`bluewhale-ledger` is a private Apple application used only as a behavioral reference. Its source code, assets, test fixtures, and private documents must not be copied into any public repository. Any reuse decision is recorded separately with its licensing and provenance.

## Reporting a licensing concern

Open an issue in the affected repository without including confidential material, or report privately through the security advisory channel when the concern is sensitive.
