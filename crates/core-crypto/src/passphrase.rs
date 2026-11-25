//! Argon2id passphrase key derivation and the passphrase-wrapped user master
//! key.
//!
//! The key record layout is documented in `docs/reference/crypto.md`. Its
//! associated data binds the format version, Argon2id parameters, and salt, so
//! a modified parameter set cannot silently downgrade the derivation.

use std::fmt;

use argon2::{Algorithm, Argon2, Params, Version};
use unicode_normalization::UnicodeNormalization;
use zeroize::Zeroizing;

use crate::aead;
use crate::key::{KEY_LEN, SymmetricKey};
use crate::{CryptoError, random};

/// The Argon2id salt length, in bytes.
pub const SALT_LEN: usize = 16;

/// The wrapped-key length: a 32-byte key plus the 16-byte authentication tag.
const WRAPPED_KEY_LEN: usize = KEY_LEN + aead::TAG_LEN;

/// The key-record header length: version, parameters, and salt.
const HEADER_LEN: usize = 1 + 12 + SALT_LEN;

/// The serialized key-record length, in bytes.
pub const KEY_RECORD_LEN: usize = HEADER_LEN + aead::NONCE_LEN + WRAPPED_KEY_LEN;

/// The key-record format version.
const FORMAT_VERSION: u8 = 1;

/// A user passphrase.
///
/// The value is normalized to Unicode NFC so the same typed passphrase derives
/// the same key on every platform. It is zeroized on drop and redacted from
/// debug output. Leading and trailing whitespace is significant; a passphrase
/// that is empty or only whitespace is rejected.
pub struct Passphrase(Zeroizing<String>);

impl Passphrase {
    /// Creates a passphrase.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::EmptyPassphrase`] when the value is empty or
    /// contains only whitespace.
    pub fn new(value: impl Into<String>) -> Result<Self, CryptoError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(CryptoError::EmptyPassphrase);
        }
        let normalized: String = value.nfc().collect();
        Ok(Self(Zeroizing::new(normalized)))
    }

    fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }
}

impl fmt::Debug for Passphrase {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("Passphrase(<redacted>)")
    }
}

/// Argon2id parameters stored with a key record.
///
/// The supported bounds are deliberately narrower than Argon2's own limits:
/// memory up to 4 GiB, at most 64 iterations, and at most 16 lanes. Version 1
/// is the default used for new accounts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Argon2idParameters {
    memory_kib: u32,
    iterations: u32,
    parallelism: u32,
}

impl Argon2idParameters {
    /// The version 1 parameters: 64 MiB, 3 iterations, one lane.
    pub const V1: Self = Self {
        memory_kib: 65_536,
        iterations: 3,
        parallelism: 1,
    };

    /// Validates and creates a parameter set.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidParameters`] when a value is zero, above
    /// the supported bound, or rejected by Argon2.
    pub fn new(memory_kib: u32, iterations: u32, parallelism: u32) -> Result<Self, CryptoError> {
        let parameters = Self {
            memory_kib,
            iterations,
            parallelism,
        };
        parameters.validate()?;
        Ok(parameters)
    }

    /// Returns the memory cost in kibibytes.
    #[must_use]
    pub const fn memory_kib(self) -> u32 {
        self.memory_kib
    }

    /// Returns the iteration count.
    #[must_use]
    pub const fn iterations(self) -> u32 {
        self.iterations
    }

    /// Returns the parallelism (lane) count.
    #[must_use]
    pub const fn parallelism(self) -> u32 {
        self.parallelism
    }

    /// Derives a 256-bit key-encryption key from a passphrase and salt.
    ///
    /// The salt must be unique for the passphrase and stored with the derived
    /// key. Prefer [`KeyRecord`], which generates the salt and wraps the key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidParameters`] for invalid parameters and
    /// [`CryptoError::KeyDerivationFailed`] when Argon2 rejects the input.
    pub fn derive_kek(
        self,
        passphrase: &Passphrase,
        salt: &[u8; SALT_LEN],
    ) -> Result<SymmetricKey, CryptoError> {
        let parameters = self.to_argon2_parameters()?;
        let argon2 = Argon2::new(Algorithm::Argon2id, Version::V0x13, parameters);
        let mut output = Zeroizing::new([0_u8; KEY_LEN]);
        argon2
            .hash_password_into(passphrase.as_bytes(), salt, &mut output[..])
            .map_err(|_| CryptoError::KeyDerivationFailed)?;
        Ok(SymmetricKey::from_bytes(*output))
    }

    fn validate(self) -> Result<(), CryptoError> {
        if self.parallelism == 0
            || self.parallelism > 16
            || self.iterations == 0
            || self.iterations > 64
            || self.memory_kib < self.parallelism.saturating_mul(8)
            || self.memory_kib > 4 * 1024 * 1024
        {
            return Err(CryptoError::InvalidParameters);
        }
        self.to_argon2_parameters().map(|_| ())
    }

    fn to_argon2_parameters(self) -> Result<Params, CryptoError> {
        Params::new(
            self.memory_kib,
            self.iterations,
            self.parallelism,
            Some(KEY_LEN),
        )
        .map_err(|_| CryptoError::InvalidParameters)
    }
}

/// The user master key wrapped by the Argon2id-derived key-encryption key.
///
/// The record contains no plaintext secret. It is stored by the client and is
/// never sent to the server. The nonce is generated once and must never be
/// reused with a rewritten record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyRecord {
    params: Argon2idParameters,
    salt: [u8; SALT_LEN],
    nonce: [u8; aead::NONCE_LEN],
    wrapped_umk: [u8; WRAPPED_KEY_LEN],
}

impl KeyRecord {
    /// Wraps a user master key with a passphrase.
    ///
    /// # Errors
    ///
    /// Returns a [`CryptoError`] when the random source fails, the parameters
    /// are invalid, or encryption fails.
    pub fn create(
        passphrase: &Passphrase,
        umk: &SymmetricKey,
        params: Argon2idParameters,
    ) -> Result<Self, CryptoError> {
        let mut salt = [0_u8; SALT_LEN];
        random::fill(&mut salt)?;
        let mut nonce = [0_u8; aead::NONCE_LEN];
        random::fill(&mut nonce)?;
        let kek = params.derive_kek(passphrase, &salt)?;
        let header = header_bytes(params, &salt);
        let sealed = aead::seal(&kek, &nonce, &header, umk.expose_secret())?;
        let wrapped_umk = sealed
            .try_into()
            .map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(Self {
            params,
            salt,
            nonce,
            wrapped_umk,
        })
    }

    /// Unwraps the user master key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::DecryptionFailed`] for a wrong passphrase or a
    /// modified record.
    pub fn unwrap_umk(&self, passphrase: &Passphrase) -> Result<SymmetricKey, CryptoError> {
        let kek = self.params.derive_kek(passphrase, &self.salt)?;
        let header = header_bytes(self.params, &self.salt);
        let plaintext = aead::open(&kek, &self.nonce, &header, &self.wrapped_umk)?;
        let key: [u8; KEY_LEN] = plaintext
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(SymmetricKey::from_bytes(key))
    }

    /// Returns the Argon2id parameters stored in the record.
    #[must_use]
    pub const fn params(&self) -> Argon2idParameters {
        self.params
    }

    /// Serializes the record.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; KEY_RECORD_LEN] {
        let mut bytes = [0_u8; KEY_RECORD_LEN];
        bytes[0] = FORMAT_VERSION;
        bytes[1..5].copy_from_slice(&self.params.memory_kib.to_be_bytes());
        bytes[5..9].copy_from_slice(&self.params.iterations.to_be_bytes());
        bytes[9..13].copy_from_slice(&self.params.parallelism.to_be_bytes());
        bytes[13..HEADER_LEN].copy_from_slice(&self.salt);
        bytes[HEADER_LEN..HEADER_LEN + aead::NONCE_LEN].copy_from_slice(&self.nonce);
        bytes[HEADER_LEN + aead::NONCE_LEN..].copy_from_slice(&self.wrapped_umk);
        bytes
    }

    /// Deserializes a record.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidKeyRecord`] for a wrong length, an
    /// unsupported format version, or invalid parameters.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != KEY_RECORD_LEN || bytes[0] != FORMAT_VERSION {
            return Err(CryptoError::InvalidKeyRecord);
        }
        let params = Argon2idParameters::new(
            u32::from_be_bytes([bytes[1], bytes[2], bytes[3], bytes[4]]),
            u32::from_be_bytes([bytes[5], bytes[6], bytes[7], bytes[8]]),
            u32::from_be_bytes([bytes[9], bytes[10], bytes[11], bytes[12]]),
        )
        .map_err(|_| CryptoError::InvalidKeyRecord)?;
        let mut salt = [0_u8; SALT_LEN];
        salt.copy_from_slice(&bytes[13..HEADER_LEN]);
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&bytes[HEADER_LEN..HEADER_LEN + aead::NONCE_LEN]);
        let mut wrapped_umk = [0_u8; WRAPPED_KEY_LEN];
        wrapped_umk.copy_from_slice(&bytes[HEADER_LEN + aead::NONCE_LEN..]);
        Ok(Self {
            params,
            salt,
            nonce,
            wrapped_umk,
        })
    }
}

fn header_bytes(params: Argon2idParameters, salt: &[u8; SALT_LEN]) -> [u8; HEADER_LEN] {
    let mut header = [0_u8; HEADER_LEN];
    header[0] = FORMAT_VERSION;
    header[1..5].copy_from_slice(&params.memory_kib.to_be_bytes());
    header[5..9].copy_from_slice(&params.iterations.to_be_bytes());
    header[9..13].copy_from_slice(&params.parallelism.to_be_bytes());
    header[13..].copy_from_slice(salt);
    header
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_parameters() -> Argon2idParameters {
        Argon2idParameters::new(1_024, 1, 1).unwrap()
    }

    fn passphrase() -> Passphrase {
        Passphrase::new("correct horse battery staple").unwrap()
    }

    fn umk() -> SymmetricKey {
        SymmetricKey::from_bytes([0x42; KEY_LEN])
    }

    #[test]
    fn version_one_parameters_are_pinned() {
        assert_eq!(Argon2idParameters::V1.memory_kib(), 65_536);
        assert_eq!(Argon2idParameters::V1.iterations(), 3);
        assert_eq!(Argon2idParameters::V1.parallelism(), 1);
    }

    #[test]
    fn passphrases_are_validated_and_redacted() {
        assert_eq!(
            Passphrase::new("").err(),
            Some(CryptoError::EmptyPassphrase)
        );
        assert_eq!(
            Passphrase::new("   ").err(),
            Some(CryptoError::EmptyPassphrase)
        );
        assert_eq!(format!("{:?}", passphrase()), "Passphrase(<redacted>)");
    }

    #[test]
    fn passphrases_are_normalized_to_nfc() {
        let composed = Passphrase::new("\u{e9}").unwrap();
        let decomposed = Passphrase::new("e\u{301}").unwrap();
        let parameters = test_parameters();
        let salt = [0x11; SALT_LEN];
        assert_eq!(
            parameters
                .derive_kek(&composed, &salt)
                .unwrap()
                .expose_secret(),
            parameters
                .derive_kek(&decomposed, &salt)
                .unwrap()
                .expose_secret()
        );
    }

    #[test]
    fn parameters_are_validated() {
        assert_eq!(
            Argon2idParameters::new(0, 1, 1),
            Err(CryptoError::InvalidParameters)
        );
        assert_eq!(
            Argon2idParameters::new(1_024, 0, 1),
            Err(CryptoError::InvalidParameters)
        );
        assert_eq!(
            Argon2idParameters::new(1_024, 1, 0),
            Err(CryptoError::InvalidParameters)
        );
        assert_eq!(
            Argon2idParameters::new(7, 1, 1),
            Err(CryptoError::InvalidParameters)
        );
        assert_eq!(
            Argon2idParameters::new(4 * 1024 * 1024 + 1, 1, 1),
            Err(CryptoError::InvalidParameters)
        );
        assert!(Argon2idParameters::new(1_024, 1, 1).is_ok());
    }

    #[test]
    fn a_key_record_round_trips_and_rejects_a_wrong_passphrase() {
        let record = KeyRecord::create(&passphrase(), &umk(), test_parameters()).unwrap();
        assert_eq!(
            record.unwrap_umk(&passphrase()).unwrap().expose_secret(),
            umk().expose_secret()
        );
        assert_eq!(
            record
                .unwrap_umk(&Passphrase::new("wrong passphrase").unwrap())
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(record.params(), test_parameters());
    }

    #[test]
    fn serialized_records_round_trip() {
        let record = KeyRecord::create(&passphrase(), &umk(), test_parameters()).unwrap();
        let bytes = record.to_bytes();
        assert_eq!(bytes.len(), KEY_RECORD_LEN);
        let restored = KeyRecord::from_bytes(&bytes).unwrap();
        assert_eq!(restored, record);
        assert_eq!(
            restored.unwrap_umk(&passphrase()).unwrap().expose_secret(),
            umk().expose_secret()
        );
    }

    #[test]
    fn malformed_records_are_rejected() {
        let record = KeyRecord::create(&passphrase(), &umk(), test_parameters()).unwrap();
        let bytes = record.to_bytes();

        assert_eq!(
            KeyRecord::from_bytes(&bytes[..KEY_RECORD_LEN - 1]).err(),
            Some(CryptoError::InvalidKeyRecord)
        );

        let mut wrong_version = bytes;
        wrong_version[0] = 2;
        assert_eq!(
            KeyRecord::from_bytes(&wrong_version).err(),
            Some(CryptoError::InvalidKeyRecord)
        );

        let mut invalid_parameters = bytes;
        invalid_parameters[1..5].copy_from_slice(&0_u32.to_be_bytes());
        assert_eq!(
            KeyRecord::from_bytes(&invalid_parameters).err(),
            Some(CryptoError::InvalidKeyRecord)
        );
    }

    #[test]
    fn modified_records_fail_authentication() {
        for index in [13, 29, 41, 88] {
            let mut bytes = KeyRecord::create(&passphrase(), &umk(), test_parameters())
                .unwrap()
                .to_bytes();
            bytes[index] ^= 1;
            let record = KeyRecord::from_bytes(&bytes).unwrap();
            assert_eq!(
                record.unwrap_umk(&passphrase()).err(),
                Some(CryptoError::DecryptionFailed),
                "offset {index}"
            );
        }
    }
}
