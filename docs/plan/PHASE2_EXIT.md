# Phase 2 exit record

Status: complete on 2026-11-26. This record closes Phase 2 (sync and server,
P2.1-P2.6) against the criteria in
[DEVELOPMENT_PLAN.md](DEVELOPMENT_PLAN.md#5-phase-2--sync-and-server).

## Exit criteria

| Criterion | Result | Evidence |
| --- | --- | --- |
| Two clients converge on the same ledger state | Met | `crates/core-sync/tests/integration.rs::deterministic_operation_rounds_converge` runs six seeded rounds of interleaved edits over two devices and asserts an identical export document and identical per-entity revisions, with a contiguous server sequence. The engine tests cover the same property for focused cases. |
| Financial conflicts produce reviewable records, not silent overwrites | Met | `a_concurrent_edit_is_reviewable_and_resolves_to_the_local_side` and `a_concurrent_delete_and_edit_keeps_both_versions_reviewable` assert that the losing local aggregate is preserved in a conflict record, that both resolution choices converge, and that the winning device is not left with an unresolved record. |
| Snapshot restore works on a fresh client | Met | `a_snapshot_restores_a_fresh_client_and_catches_up`: a fresh device restores an encrypted snapshot, matches the source export and revisions, then catches up through the normal pull path. Restore into a non-empty ledger is refused. |
| Server stores no plaintext ledger data, display names, or keys | Met | The server stores signed envelopes, token hashes, and routing metadata only. The P2.4 verification record and the server test suite assert the schema, log, and API behavior; the contract schemas for every response are validated in both implementations. |
| Protocol fixtures pass on both Rust and Go | Met | All 36 fixtures (22 valid, 14 invalid) are validated by `core-sync/tests/contract.rs` and by the server's contract tests. At P2.5 the 39 shared contract files were byte-identical to the server's pinned copy. |

## Verification runs (2026-11-26)

| Repository | Command or run | Result |
| --- | --- | --- |
| core | `cargo test --workspace` | 209 passed, 0 failed, 1 ignored (Argon2id timing). |
| core | `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --all --check` | Clean. |
| core | `cargo deny check` | advisories, bans, licenses, sources ok. |
| core | Core CI on `main`, run 45745866579 for the exit commit `ef24d53` | All jobs passed, including the secret scan (6 s) and SBOM (1 min 14 s). |
| server | `go test -count=1 ./...` against PostgreSQL 17.7 | All packages pass. |
| server | `go vet ./...`, `gofmt -l .`, `golangci-lint run` | Clean; 0 issues. |
| server | `govulncheck ./...` | No vulnerabilities found. |

## Open items for the owner

- **Private server CI has never started.** Every Server CI run fails within
  about three seconds with "The job was not started because recent account
  payments have failed or your spending limit needs to be increased." This is
  an account billing setting for private-repository Actions, not a repository
  defect. Until it is resolved, the server pipeline (including `go test -race`)
  cannot run on GitHub, and the server's local evidence stands alone.
- **GitHub-native security features** were enabled on 2026-11-26: the
  dependency graph (so dependency review now runs on pull requests), secret
  scanning with push protection, and Dependabot alerts. Dependabot security
  updates remain off; enabling them is a repository settings action.

## Deferred beyond Phase 2

- The blocking HTTP transport and client command wiring are P3.1.
- New-device enrollment still lacks a key-envelope read endpoint; the gap is
  recorded in the server's protocol-compatibility document.
- Docker artifacts and real SMTP delivery were not exercised end to end.
- Payload padding and further metadata minimization remain deferred (D4.9).

## Conclusion

The Phase 2 exit criteria are met with observed local evidence, and the public
core pipeline is green on `main`. The private server's pipeline awaits the
billing action above; no Phase 2 functionality depends on it.
