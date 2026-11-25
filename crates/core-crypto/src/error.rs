//! Errors returned by the cryptographic primitives.
//!
//! Errors never contain secret material, plaintext, or the reason an
//! authentication check failed. A failed decryption is reported as
//! [`CryptoError::DecryptionFailed`] whether the key, nonce, or associated data
//! was wrong.

use std::fmt;

/// A cryptographic operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptoError {
    /// The passphrase is empty or contains only whitespace.
    EmptyPassphrase,
    /// The Argon2id parameters are outside the supported bounds.
    InvalidParameters,
    /// Argon2id key derivation failed.
    KeyDerivationFailed,
    /// A serialized key record is malformed or has an unsupported version.
    InvalidKeyRecord,
    /// A serialized wrapped ledger key is malformed or has an unsupported
    /// version.
    InvalidWrappedKey,
    /// A recovery key code is malformed, non-canonical, or has a bad checksum.
    InvalidRecoveryKey,
    /// A serialized recovery envelope is malformed or has an unsupported
    /// version.
    InvalidRecoveryEnvelope,
    /// A public key is invalid, low-order, or non-contributory.
    InvalidPublicKey,
    /// A key version is zero or cannot be incremented.
    InvalidKeyVersion,
    /// A signature is malformed or does not verify.
    InvalidSignature,
    /// Authenticated encryption failed.
    EncryptionFailed,
    /// Authenticated decryption failed.
    DecryptionFailed,
    /// The operating system random source is unavailable.
    RandomnessUnavailable,
}

impl fmt::Display for CryptoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyPassphrase => "the passphrase is empty",
            Self::InvalidParameters => "invalid Argon2id parameters",
            Self::KeyDerivationFailed => "key derivation failed",
            Self::InvalidKeyRecord => "invalid key record",
            Self::InvalidWrappedKey => "invalid wrapped ledger key",
            Self::InvalidRecoveryKey => "invalid recovery key",
            Self::InvalidRecoveryEnvelope => "invalid recovery envelope",
            Self::InvalidPublicKey => "invalid public key",
            Self::InvalidKeyVersion => "invalid key version",
            Self::InvalidSignature => "invalid signature",
            Self::EncryptionFailed => "encryption failed",
            Self::DecryptionFailed => "decryption failed",
            Self::RandomnessUnavailable => "the random source is unavailable",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for CryptoError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_error_displays_a_message() {
        let errors = [
            CryptoError::EmptyPassphrase,
            CryptoError::InvalidParameters,
            CryptoError::KeyDerivationFailed,
            CryptoError::InvalidKeyRecord,
            CryptoError::InvalidWrappedKey,
            CryptoError::InvalidRecoveryKey,
            CryptoError::InvalidRecoveryEnvelope,
            CryptoError::InvalidPublicKey,
            CryptoError::InvalidKeyVersion,
            CryptoError::InvalidSignature,
            CryptoError::EncryptionFailed,
            CryptoError::DecryptionFailed,
            CryptoError::RandomnessUnavailable,
        ];
        for error in errors {
            assert!(!error.to_string().is_empty(), "{error:?}");
        }
    }
}
