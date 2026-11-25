# core-crypto

Cryptographic primitives for Bluewhale: Argon2id passphrase key derivation, the
passphrase-wrapped user master key, the user-held recovery key, per-device
X25519 wrapping of each ledger data key, and Ed25519 device signatures.

The crate performs no I/O and no network access. Secrets zeroize on drop and are
redacted from debug output.

- [Reference](../../docs/reference/crypto.md): algorithms, parameters, binary
  key formats, recovery-key encoding, signature procedure, and test vectors.
- `crates/core-crypto/tests/vectors.rs`: RFC 8032, RFC 7748, RFC 8439, and
  Argon2id known-answer tests.
- `core-sync`: produces the RFC 8785 canonical envelope bytes that
  `DeviceSigningKey` signs.

Platform key storage, SQLCipher key management, payload encryption, and the
outbox are not part of this crate.
