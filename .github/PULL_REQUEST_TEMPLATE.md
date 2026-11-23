# Pull request

## Summary

<!-- What changes, and why? Keep it short. -->

## Design reference

<!-- Link the design document, ADR, or issue that covers this change. -->

## Tests run

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo deny check`
- [ ] Documentation built or links checked where relevant

## Impact

- [ ] No schema, protocol, export, or public API change
- [ ] Schema or migration change (describe the forward migration and backup behavior)
- [ ] Sync protocol or export contract change (link the versioning note)
- [ ] Accessibility or localization impact

## Data safety

- [ ] No real financial data, credentials, encryption keys, recovery keys, or ciphertext are included
- [ ] Logs and screenshots use synthetic data only

## DCO

- [ ] Every commit is signed off with `git commit -s`
