//! Symmetric key material and ledger key versions.

use std::fmt;

use zeroize::Zeroizing;

use crate::CryptoError;
use crate::random;

/// The length of a symmetric key, in bytes.
pub const KEY_LEN: usize = 32;

/// A 256-bit symmetric key used as the UMK, an LDK, or a derived encryption
/// key.
///
/// The key is zeroized on drop and never appears in debug output. Use
/// [`SymmetricKey::expose_secret`] only at the point where the key is handed to
/// a cryptographic primitive or stored in a platform key store.
pub struct SymmetricKey(Zeroizing<[u8; KEY_LEN]>);

impl SymmetricKey {
    /// Generates a random key from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomnessUnavailable`] when the random source
    /// fails.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = Zeroizing::new([0_u8; KEY_LEN]);
        random::fill(&mut bytes[..])?;
        Ok(Self(bytes))
    }

    /// Creates a key from raw bytes.
    ///
    /// The caller must zeroize its own copy of `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; KEY_LEN]) -> Self {
        Self(Zeroizing::new(bytes))
    }

    /// Exposes the raw key bytes.
    ///
    /// The returned reference must not be logged, serialized to an untrusted
    /// destination, or kept longer than the operation that needs it.
    #[must_use]
    pub fn expose_secret(&self) -> &[u8; KEY_LEN] {
        &self.0
    }
}

impl fmt::Debug for SymmetricKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SymmetricKey(<redacted>)")
    }
}

/// A positive ledger data-key version.
///
/// The initial ledger key version is 1. Device revocation rotates the ledger
/// data key to the next version for future operations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyVersion(u32);

impl KeyVersion {
    /// The first ledger key version.
    pub const INITIAL: Self = Self(1);

    /// Creates a key version, rejecting zero.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKeyVersion`] when `value` is zero.
    pub fn new(value: u32) -> Result<Self, CryptoError> {
        if value == 0 {
            Err(CryptoError::InvalidKeyVersion)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the numeric version.
    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Returns the next version.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKeyVersion`] when the version is
    /// `u32::MAX`.
    pub fn next(self) -> Result<Self, CryptoError> {
        self.0
            .checked_add(1)
            .map(Self)
            .ok_or(CryptoError::InvalidKeyVersion)
    }
}

impl fmt::Display for KeyVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_keys_differ() {
        let first = SymmetricKey::generate().unwrap();
        let second = SymmetricKey::generate().unwrap();
        assert_ne!(first.expose_secret(), second.expose_secret());
    }

    #[test]
    fn keys_round_trip_through_bytes() {
        let key = SymmetricKey::from_bytes([7; KEY_LEN]);
        assert_eq!(key.expose_secret(), &[7; KEY_LEN]);
        assert_eq!(format!("{key:?}"), "SymmetricKey(<redacted>)");
    }

    #[test]
    fn key_versions_are_positive_and_monotonic() {
        assert_eq!(KeyVersion::INITIAL.get(), 1);
        assert_eq!(KeyVersion::new(0), Err(CryptoError::InvalidKeyVersion));
        assert_eq!(KeyVersion::new(7).unwrap().get(), 7);
        assert_eq!(KeyVersion::INITIAL.next().unwrap().get(), 2);
        assert_eq!(
            KeyVersion::new(u32::MAX).unwrap().next(),
            Err(CryptoError::InvalidKeyVersion)
        );
        assert_eq!(KeyVersion::new(3).unwrap().to_string(), "3");
    }
}
