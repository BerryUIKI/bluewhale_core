//! Ed25519 device signing keys and signatures.
//!
//! Every operation envelope is signed by the device Ed25519 key. The signature
//! covers the RFC 8785 canonical JSON of the envelope with the signature field
//! omitted; `core-sync` produces that input, and this module signs and verifies
//! raw bytes without knowing the envelope shape.

use std::fmt;

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use zeroize::Zeroizing;

use crate::{CryptoError, random};

/// The Ed25519 public-key length, in bytes.
pub const VERIFYING_KEY_LEN: usize = 32;

/// The Ed25519 private-key seed length, in bytes.
pub const SIGNING_SEED_LEN: usize = 32;

/// The Ed25519 signature length, in bytes.
pub const SIGNATURE_LEN: usize = 64;

/// A device Ed25519 private key.
///
/// The seed is zeroized on drop and redacted from debug output.
pub struct DeviceSigningKey(SigningKey);

impl DeviceSigningKey {
    /// Generates a signing key from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomnessUnavailable`] when the random source
    /// fails.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut seed = Zeroizing::new([0_u8; SIGNING_SEED_LEN]);
        random::fill(&mut seed[..])?;
        Ok(Self(SigningKey::from_bytes(&seed)))
    }

    /// Creates a signing key from a 32-byte seed.
    ///
    /// The caller must zeroize its own copy of `seed`.
    #[must_use]
    pub fn from_seed(seed: [u8; SIGNING_SEED_LEN]) -> Self {
        Self(SigningKey::from_bytes(&seed))
    }

    /// Exposes the 32-byte seed.
    ///
    /// The caller must zeroize the returned buffer and must not log it.
    #[must_use]
    pub fn to_seed(&self) -> Zeroizing<[u8; SIGNING_SEED_LEN]> {
        Zeroizing::new(self.0.to_bytes())
    }

    /// Returns the corresponding verifying key.
    #[must_use]
    pub fn public_key(&self) -> DeviceVerifyingKey {
        DeviceVerifyingKey(self.0.verifying_key())
    }

    /// Signs a message. Ed25519 signatures are deterministic.
    #[must_use]
    pub fn sign(&self, message: &[u8]) -> DeviceSignature {
        DeviceSignature(self.0.sign(message))
    }
}

impl fmt::Debug for DeviceSigningKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceSigningKey(<redacted>)")
    }
}

/// A device Ed25519 verifying key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceVerifyingKey(VerifyingKey);

impl DeviceVerifyingKey {
    /// Creates a verifying key from raw compressed-point bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] when the bytes are not a valid
    /// Ed25519 point.
    pub fn from_bytes(bytes: &[u8; VERIFYING_KEY_LEN]) -> Result<Self, CryptoError> {
        VerifyingKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| CryptoError::InvalidPublicKey)
    }

    /// Returns the raw compressed-point bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; VERIFYING_KEY_LEN] {
        self.0.to_bytes()
    }

    /// Verifies a signature over a message.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] when the signature does not
    /// verify or the key is too weak for strict verification.
    pub fn verify(&self, message: &[u8], signature: &DeviceSignature) -> Result<(), CryptoError> {
        self.0
            .verify_strict(message, &signature.0)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

/// An Ed25519 signature.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceSignature(Signature);

impl DeviceSignature {
    /// Creates a signature from exactly 64 bytes.
    #[must_use]
    pub fn from_bytes(bytes: &[u8; SIGNATURE_LEN]) -> Self {
        Self(Signature::from_bytes(bytes))
    }

    /// Creates a signature from a byte slice.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidSignature`] when the slice is not 64
    /// bytes.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, CryptoError> {
        Signature::from_slice(bytes)
            .map(Self)
            .map_err(|_| CryptoError::InvalidSignature)
    }

    /// Returns the raw 64 signature bytes.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; SIGNATURE_LEN] {
        self.0.to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_verify_and_reject_tampering() {
        let key = DeviceSigningKey::from_seed([7; SIGNING_SEED_LEN]);
        let verifying = key.public_key();
        let signature = key.sign(b"operation envelope");
        verifying.verify(b"operation envelope", &signature).unwrap();
        assert_eq!(
            verifying.verify(b"other envelope", &signature).err(),
            Some(CryptoError::InvalidSignature)
        );
        assert_eq!(
            DeviceSigningKey::from_seed([8; SIGNING_SEED_LEN])
                .public_key()
                .verify(b"operation envelope", &signature)
                .err(),
            Some(CryptoError::InvalidSignature)
        );
    }

    #[test]
    fn signatures_are_deterministic() {
        let key = DeviceSigningKey::from_seed([7; SIGNING_SEED_LEN]);
        assert_eq!(key.sign(b"message"), key.sign(b"message"));
    }

    #[test]
    fn keys_and_signatures_round_trip_through_bytes() {
        let key = DeviceSigningKey::from_seed([5; SIGNING_SEED_LEN]);
        assert_eq!(key.to_seed().as_ref(), &[5; SIGNING_SEED_LEN]);
        let public = key.public_key();
        assert_eq!(
            DeviceVerifyingKey::from_bytes(&public.to_bytes()).unwrap(),
            public
        );
        let signature = key.sign(b"message");
        assert_eq!(
            DeviceSignature::from_bytes(&signature.to_bytes()),
            signature
        );
        assert_eq!(
            DeviceSignature::from_slice(&signature.to_bytes()).unwrap(),
            signature
        );
        assert_eq!(
            DeviceSignature::from_slice(&signature.to_bytes()[..63]).err(),
            Some(CryptoError::InvalidSignature)
        );
        assert_eq!(format!("{key:?}"), "DeviceSigningKey(<redacted>)");
    }

    #[test]
    fn generated_keys_differ() {
        let first = DeviceSigningKey::generate().unwrap();
        let second = DeviceSigningKey::generate().unwrap();
        assert_ne!(first.public_key(), second.public_key());
    }
}
