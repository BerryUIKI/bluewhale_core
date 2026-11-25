//! Cryptographic primitives for Bluewhale.
//!
//! The crate implements the approved key hierarchy from R4 with RustCrypto
//! primitives:
//!
//! - Argon2id derives a key-encryption key from the user passphrase.
//! - A random user master key (UMK) is wrapped by that key and by the
//!   user-held recovery key.
//! - Each ledger has an independent data key (LDK), wrapped for each device
//!   with an ephemeral X25519 exchange, HKDF-SHA256, and ChaCha20-Poly1305.
//! - Each device has an Ed25519 signing key for operation envelopes.
//!
//! Every secret is zeroized on drop and is redacted from debug output. The
//! crate performs no I/O, no key storage, and no network access; callers own
//! platform key storage and never log secret material.
//!
//! The concrete algorithms, parameters, binary formats, recovery-key encoding,
//! and recorded vectors are documented in `docs/reference/crypto.md`.

pub mod aead;

mod agreement;
mod error;
mod key;
mod passphrase;
mod random;
mod recovery;
mod signing;

pub use agreement::{
    DeviceAgreementKey, DeviceAgreementPublicKey, PUBLIC_KEY_LEN, WRAPPED_LEDGER_KEY_LEN,
    WrappedLedgerKey,
};
pub use error::CryptoError;
pub use key::{KEY_LEN, KeyVersion, SymmetricKey};
pub use passphrase::{Argon2idParameters, KEY_RECORD_LEN, KeyRecord, Passphrase, SALT_LEN};
pub use recovery::{
    RECOVERY_CODE_CHARS, RECOVERY_ENVELOPE_LEN, RECOVERY_KEY_LEN, RecoveryEnvelope, RecoveryKey,
};
pub use signing::{
    DeviceSignature, DeviceSigningKey, DeviceVerifyingKey, SIGNATURE_LEN, VERIFYING_KEY_LEN,
};
