# Cryptography

Status: active since P2.2 (2026-11-25). The implementation is `core-crypto`; the
signing input for operation envelopes is produced by `core-sync`. The approved
key hierarchy is in [SYNC_AND_SERVER_DESIGN.md §4](../sync/SYNC_AND_SERVER_DESIGN.md),
and the protocol envelope is in [sync-protocol.md](sync-protocol.md).

## Scope

`core-crypto` implements:

- Argon2id passphrase key derivation and the passphrase-wrapped user master key
  (UMK);
- the user-held recovery key, its human-transcription encoding, and the
  recovery envelope that wraps the UMK;
- per-ledger data keys (LDKs) wrapped for each device with X25519, HKDF-SHA256,
  and ChaCha20-Poly1305;
- Ed25519 device signing keys and signature verification;
- key versions and key-rotation primitives.

It does not implement platform key storage (DPAPI or Android Keystore),
SQLCipher key management, the outbox or payload encryption, server-side
envelope storage, or client key-enrollment policy. Those land in P2.3, P2.4, and
the client phases. It has no I/O and no network access.

## Algorithms

| Purpose | Algorithm | Parameters |
| --- | --- | --- |
| Passphrase KDF | Argon2id v1.3 | m = 65,536 KiB, t = 3, p = 1, output 32 bytes, salt 16 bytes |
| Authenticated encryption | ChaCha20-Poly1305 (RFC 8439) | 32-byte key, 96-bit random nonce, 128-bit tag |
| Device agreement | X25519 (RFC 7748) | 32-byte keys, ephemeral per wrap |
| Key derivation for wrapping | HKDF-SHA256 (RFC 5869) | 32-byte output, salt and info below |
| Device signatures | Ed25519 (RFC 8032) | 32-byte seed, 32-byte public key, 64-byte signature |
| Randomness | OS CSPRNG | `getrandom` |

The AEAD choice matches the Phase 0 spike. All primitives come from RustCrypto
and the `dalek` crates; their licenses are on the repository allowlist.

### Argon2id parameters and timing

Version 1 is 64 MiB, 3 iterations, and one lane. The parameters are stored in
each key record, so a future default can coexist with existing accounts.

Measured on the Windows development host on 2026-11-25 with

```text
cargo test -p core-crypto --release --test vectors argon2id_version_one -- --ignored --nocapture
```

the derivation took 108.4 ms, 109.0 ms, 109.5 ms, and 110.7 ms across four
release runs. Host: Windows 10 Pro 10.0.19045, `x86_64-pc-windows-msvc`, Rust
1.97.1.

This is host evidence only. Android and low-end Windows device timings are
recorded in P4; if they are too slow, the default can move to a lower memory
cost without invalidating existing records, because every record stores its own
parameters.

## Key hierarchy

```text
passphrase --Argon2id--> KEK (32 bytes)
KEK --ChaCha20-Poly1305--> KeyRecord (UMK wrapped)
UMK --ChaCha20-Poly1305 with recovery key--> RecoveryEnvelope
UMK --wraps--> LDK (32 random bytes per ledger)
LDK --X25519 + HKDF-SHA256 + ChaCha20-Poly1305--> WrappedLedgerKey per device
LDK --ChaCha20-Poly1305--> operation payloads (P2.3)
device Ed25519 key --signs--> canonical operation envelope
```

The UMK is random, not derived from the passphrase. A passphrase change rewraps
the UMK and does not re-encrypt ledger data. Passphrases are normalized to
Unicode NFC before derivation, so the same typed passphrase derives the same
key on every platform. Leading and trailing whitespace is significant. The
server may store a recovery envelope but never the UMK, recovery key, LDKs,
passphrase, or plaintext.

## Binary formats

All integers are unsigned big-endian. The first byte is the format version.
Unknown versions are rejected. Binary bodies are carried in the API as unpadded
base64url; see [sync-protocol.md](sync-protocol.md).

### Key record (89 bytes)

| Offset | Length | Field |
| ---: | ---: | --- |
| 0 | 1 | Format version, `1` |
| 1 | 4 | Argon2id memory cost, KiB |
| 5 | 4 | Argon2id iterations |
| 9 | 4 | Argon2id parallelism |
| 13 | 16 | Salt |
| 29 | 12 | ChaCha20-Poly1305 nonce |
| 41 | 48 | Wrapped UMK (32-byte key plus 16-byte tag) |

The AEAD associated data is bytes 0–28: the format version, parameters, and
salt. Changing the parameters or salt therefore invalidates the record instead
of silently deriving a different key.

### Wrapped ledger key (93 bytes)

| Offset | Length | Field |
| ---: | ---: | --- |
| 0 | 1 | Format version, `1` |
| 1 | 32 | Ephemeral X25519 public key |
| 33 | 12 | ChaCha20-Poly1305 nonce |
| 45 | 48 | Wrapped LDK (32-byte key plus 16-byte tag) |

The wrapping key is derived as follows:

```text
shared = X25519(ephemeral_private, recipient_public)
salt   = ephemeral_public || recipient_public
info   = "bluewhale/ldk-wrap/v1" || ledger_id (16) || key_version (u32 BE) || device_id (16)
key    = HKDF-SHA256(ikm = shared, salt, info, L = 32)
```

The AEAD associated data is bytes 0–32: the format version and ephemeral public
key. A low-order peer key that produces a non-contributory shared secret is
rejected. The ledger, device, and version binding prevents an envelope from
being moved between ledgers, devices, or key versions.

### Recovery envelope (61 bytes)

| Offset | Length | Field |
| ---: | ---: | --- |
| 0 | 1 | Format version, `1` |
| 1 | 12 | ChaCha20-Poly1305 nonce |
| 13 | 48 | Wrapped UMK (32-byte key plus 16-byte tag) |

The recovery key is used directly as the 32-byte AEAD key. The associated data
is `bluewhale/recovery-envelope/v1` followed by the format version byte.

## Recovery key encoding

A recovery key is 32 random bytes encoded as:

- 52 Crockford Base32 characters for the key, most significant bit first, with
  zero padding in the final character;
- 4 Crockford Base32 characters for a 20-bit checksum: the first 20 bits of
  `SHA-256("bluewhale/recovery-checksum/v1" || key)`;
- displayed as 14 groups of four characters separated by hyphens.

The alphabet is `0123456789ABCDEFGHJKMNPQRSTVWXYZ`. It excludes `I`, `L`, `O`,
and `U`. Parsing ignores case, whitespace, and hyphens; it accepts `O` as `0`
and `I` or `L` as `1`; it rejects `U`, non-ASCII characters, non-zero padding
bits, and a bad checksum. The parsed code is re-encoded and compared, so only a
canonical code is accepted.

Vector for key bytes `00 01 02 … 1f`:

```text
000G-40R4-0M30-E209-185G-R38E-1W81-24GK-2GAH-C5RR-34D1-P70X-3RFG-QSD7
```

## Signatures

The signing input is the RFC 8785 canonical JSON of the envelope object with
the `signature` field omitted. `OperationEnvelope::signing_bytes()` produces it:
object keys are sorted, the JSON is compact, integers are decimal, and strings
use the required escaping. The signed fields are:

```text
base_revision, ciphertext, client_timestamp, device_id, entity_id,
entity_type, idempotency_key, key_version, ledger_id, nonce, op_id,
op_type, payload_version, protocol_version
```

Sign with `DeviceSigningKey::sign(&message)`, encode the 64-byte signature as
unpadded base64url, and set it in the envelope. A verifier reconstructs the
canonical bytes from the envelope, decodes the signature, and calls
`DeviceVerifyingKey::verify`, which uses strict Ed25519 verification.

`crates/core-sync/tests/signing.rs` signs and verifies a canonical envelope
end-to-end.

## Randomness and nonces

- Keys, salts, and nonces come from the operating system CSPRNG through
  `getrandom`.
- Every AEAD operation that this crate performs generates a fresh 96-bit nonce.
- The low-level `core_crypto::aead` functions accept a caller-supplied nonce.
  A caller must never reuse a nonce with the same key; the higher-level key
  wrapping APIs avoid the risk.
- No deterministic nonce is exposed by the public API. Tests that need one work
  from fixed vectors, not from production entry points.

## Secret handling

- `SymmetricKey`, `Passphrase`, the device private keys, and the recovery key
  zeroize on drop through `zeroize`.
- Their `Debug` output is redacted. Errors never contain secret material or
  the reason an authentication check failed.
- `to_bytes`, `to_seed`, and `expose_secret` are explicit boundary calls for
  platform key storage or a cryptographic operation. Callers must not log or
  serialize them to untrusted destinations.
- A wrong passphrase, wrong device, wrong ledger, wrong key version, and a
  tampered envelope all report `DecryptionFailed`.

## Test vectors

`crates/core-crypto/tests/vectors.rs` contains:

| Vector | Source | Result |
| --- | --- | --- |
| Ed25519 seed, public key, empty-message signature | RFC 8032 §7.1 test 1 | Exact match |
| X25519 public keys and shared secret | RFC 7748 §6.1 | Exact match |
| ChaCha20-Poly1305 key, nonce, AAD, ciphertext, tag | RFC 8439 §2.8.2 | Exact match |
| Argon2id m=32 KiB, t=3, p=4 | `argon2-cffi` 25.1.0 reference output | Exact match |
| Argon2id version 1 defaults | `argon2-cffi` 25.1.0 reference output | Exact match; ignored by default |

The version 1 Argon2id test is ignored in the normal suite because it is slow in
debug builds; run it in release to reproduce the timing evidence:

```text
cargo test -p core-crypto --release --test vectors argon2id_version_one -- --ignored --nocapture
```

The recovery-code vector and the key-record, wrapped-key, and recovery-envelope
round-trips and tamper checks are unit tests in their modules.

## Commands

```text
cargo test -p core-crypto
cargo test -p core-sync
cargo llvm-cov -p core-crypto --summary-only
```

Line coverage for `core-crypto` was 99.2% when measured on 2026-11-25; the
recorded figure and the workspace gate are in [COVERAGE.md](../quality/COVERAGE.md).

## Residual risks

- Mobile KDF timing is not measured yet; P4 must record it on target devices.
- Platform key storage and biometric gating are client concerns; the core only
  provides raw key material at an explicit boundary.
- A revoked device can still decrypt operations it received before revocation
  and any LDK version it already held. Rotation protects future data, as stated
  in R4.
- Payload encryption and the outbox are not wired yet; P2.3 uses these
  primitives.
- No claim is made about FIPS 140 validation or post-quantum resistance.
