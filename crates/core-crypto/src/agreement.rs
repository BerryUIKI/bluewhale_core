//! X25519 device agreement keys and ledger data-key wrapping.
//!
//! A ledger data key is wrapped for one device with a fresh ephemeral X25519
//! exchange. HKDF-SHA256 binds the shared secret to both public keys, the
//! ledger identity, the device identity, and the key version. The resulting
//! wrapping key encrypts the data key with ChaCha20-Poly1305. The server stores
//! the resulting opaque envelope and cannot decrypt it.

use std::fmt;

use hkdf::Hkdf;
use sha2::Sha256;
use uuid::Uuid;
use x25519_dalek::{PublicKey, StaticSecret};
use zeroize::Zeroizing;

use crate::aead;
use crate::key::{KEY_LEN, KeyVersion, SymmetricKey};
use crate::{CryptoError, random};

/// The X25519 public-key length, in bytes.
pub const PUBLIC_KEY_LEN: usize = 32;

/// The serialized wrapped-ledger-key length, in bytes.
pub const WRAPPED_LEDGER_KEY_LEN: usize =
    1 + PUBLIC_KEY_LEN + aead::NONCE_LEN + KEY_LEN + aead::TAG_LEN;

/// The wrapped-ledger-key format version.
const FORMAT_VERSION: u8 = 1;

/// HKDF domain separation for ledger data-key wrapping.
const WRAPPING_DOMAIN: &[u8] = b"bluewhale/ldk-wrap/v1";

/// A device X25519 private key used to receive wrapped ledger data keys.
///
/// The secret is zeroized on drop and redacted from debug output.
pub struct DeviceAgreementKey(StaticSecret);

impl DeviceAgreementKey {
    /// Generates a new agreement key from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomnessUnavailable`] when the random source
    /// fails.
    pub fn generate() -> Result<Self, CryptoError> {
        let mut bytes = Zeroizing::new([0_u8; PUBLIC_KEY_LEN]);
        random::fill(&mut bytes[..])?;
        Ok(Self(StaticSecret::from(*bytes)))
    }

    /// Creates a key from raw bytes.
    ///
    /// The caller must zeroize its own copy of `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; PUBLIC_KEY_LEN]) -> Self {
        Self(StaticSecret::from(bytes))
    }

    /// Exposes the raw private key.
    ///
    /// The caller must zeroize the returned buffer and must not log it.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<[u8; PUBLIC_KEY_LEN]> {
        Zeroizing::new(self.0.to_bytes())
    }

    /// Returns the corresponding public key.
    #[must_use]
    pub fn public_key(&self) -> DeviceAgreementPublicKey {
        DeviceAgreementPublicKey(PublicKey::from(&self.0))
    }

    /// Computes the X25519 shared secret with a peer public key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] when the peer key is
    /// low-order and produces an all-zero shared secret.
    pub fn shared_secret(
        &self,
        public: &DeviceAgreementPublicKey,
    ) -> Result<Zeroizing<[u8; PUBLIC_KEY_LEN]>, CryptoError> {
        let shared = self.0.diffie_hellman(&public.0);
        if !shared.was_contributory() {
            return Err(CryptoError::InvalidPublicKey);
        }
        Ok(Zeroizing::new(shared.to_bytes()))
    }
}

impl fmt::Debug for DeviceAgreementKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("DeviceAgreementKey(<redacted>)")
    }
}

/// A device X25519 public key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeviceAgreementPublicKey(PublicKey);

impl DeviceAgreementPublicKey {
    /// Creates a public key from raw bytes.
    ///
    /// Low-order keys are rejected when the key participates in a key
    /// exchange, not here.
    #[must_use]
    pub fn from_bytes(bytes: [u8; PUBLIC_KEY_LEN]) -> Self {
        Self(PublicKey::from(bytes))
    }

    /// Returns the raw public key.
    #[must_use]
    pub fn to_bytes(self) -> [u8; PUBLIC_KEY_LEN] {
        self.0.to_bytes()
    }
}

/// A ledger data key wrapped for one device and key version.
///
/// The serialized form is 93 bytes: a format version, the ephemeral public key,
/// the nonce, and the authenticated ciphertext. See
/// `docs/reference/crypto.md`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrappedLedgerKey {
    ephemeral_public: [u8; PUBLIC_KEY_LEN],
    nonce: [u8; aead::NONCE_LEN],
    ciphertext: [u8; KEY_LEN + aead::TAG_LEN],
}

impl WrappedLedgerKey {
    /// Wraps a ledger data key for a device.
    ///
    /// The ledger identity, device identity, and key version are bound into
    /// the derivation, so an envelope cannot be moved between ledgers,
    /// devices, or key versions.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomnessUnavailable`] when the random source
    /// fails, [`CryptoError::InvalidPublicKey`] for a low-order recipient key,
    /// and [`CryptoError::EncryptionFailed`] when encryption fails.
    pub fn wrap(
        ldk: &SymmetricKey,
        recipient: &DeviceAgreementPublicKey,
        ledger_id: Uuid,
        device_id: Uuid,
        key_version: KeyVersion,
    ) -> Result<Self, CryptoError> {
        let mut ephemeral_bytes = Zeroizing::new([0_u8; PUBLIC_KEY_LEN]);
        random::fill(&mut ephemeral_bytes[..])?;
        let ephemeral_secret = StaticSecret::from(*ephemeral_bytes);
        let ephemeral_public = PublicKey::from(&ephemeral_secret);
        let shared = ephemeral_secret.diffie_hellman(&recipient.0);
        if !shared.was_contributory() {
            return Err(CryptoError::InvalidPublicKey);
        }
        let shared = Zeroizing::new(shared.to_bytes());
        let wrapping_key = derive_wrapping_key(
            &shared,
            &ephemeral_public.to_bytes(),
            &recipient.to_bytes(),
            ledger_id,
            device_id,
            key_version,
        )?;
        let mut nonce = [0_u8; aead::NONCE_LEN];
        random::fill(&mut nonce)?;
        let ephemeral_public = ephemeral_public.to_bytes();
        let sealed = aead::seal(
            &wrapping_key,
            &nonce,
            &associated_data(&ephemeral_public),
            ldk.expose_secret(),
        )?;
        Ok(Self {
            ephemeral_public,
            nonce,
            ciphertext: sealed
                .try_into()
                .map_err(|_| CryptoError::EncryptionFailed)?,
        })
    }

    /// Unwraps the ledger data key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidPublicKey`] for a low-order ephemeral key
    /// and [`CryptoError::DecryptionFailed`] when the key, ledger, device,
    /// version, or envelope does not match.
    pub fn unwrap(
        &self,
        recipient: &DeviceAgreementKey,
        ledger_id: Uuid,
        device_id: Uuid,
        key_version: KeyVersion,
    ) -> Result<SymmetricKey, CryptoError> {
        let ephemeral_public = PublicKey::from(self.ephemeral_public);
        let shared = recipient.0.diffie_hellman(&ephemeral_public);
        if !shared.was_contributory() {
            return Err(CryptoError::InvalidPublicKey);
        }
        let shared = Zeroizing::new(shared.to_bytes());
        let wrapping_key = derive_wrapping_key(
            &shared,
            &self.ephemeral_public,
            &recipient.public_key().to_bytes(),
            ledger_id,
            device_id,
            key_version,
        )?;
        let plaintext = aead::open(
            &wrapping_key,
            &self.nonce,
            &associated_data(&self.ephemeral_public),
            &self.ciphertext,
        )?;
        let key: [u8; KEY_LEN] = plaintext
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(SymmetricKey::from_bytes(key))
    }

    /// Serializes the wrapped key.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; WRAPPED_LEDGER_KEY_LEN] {
        let mut bytes = [0_u8; WRAPPED_LEDGER_KEY_LEN];
        bytes[0] = FORMAT_VERSION;
        bytes[1..1 + PUBLIC_KEY_LEN].copy_from_slice(&self.ephemeral_public);
        bytes[1 + PUBLIC_KEY_LEN..1 + PUBLIC_KEY_LEN + aead::NONCE_LEN]
            .copy_from_slice(&self.nonce);
        bytes[1 + PUBLIC_KEY_LEN + aead::NONCE_LEN..].copy_from_slice(&self.ciphertext);
        bytes
    }

    /// Deserializes a wrapped key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidWrappedKey`] for a wrong length or an
    /// unsupported format version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != WRAPPED_LEDGER_KEY_LEN || bytes[0] != FORMAT_VERSION {
            return Err(CryptoError::InvalidWrappedKey);
        }
        let mut ephemeral_public = [0_u8; PUBLIC_KEY_LEN];
        ephemeral_public.copy_from_slice(&bytes[1..1 + PUBLIC_KEY_LEN]);
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&bytes[1 + PUBLIC_KEY_LEN..1 + PUBLIC_KEY_LEN + aead::NONCE_LEN]);
        let mut ciphertext = [0_u8; KEY_LEN + aead::TAG_LEN];
        ciphertext.copy_from_slice(&bytes[1 + PUBLIC_KEY_LEN + aead::NONCE_LEN..]);
        Ok(Self {
            ephemeral_public,
            nonce,
            ciphertext,
        })
    }
}

fn derive_wrapping_key(
    shared: &[u8; PUBLIC_KEY_LEN],
    ephemeral_public: &[u8; PUBLIC_KEY_LEN],
    recipient_public: &[u8; PUBLIC_KEY_LEN],
    ledger_id: Uuid,
    device_id: Uuid,
    key_version: KeyVersion,
) -> Result<SymmetricKey, CryptoError> {
    let mut salt = [0_u8; PUBLIC_KEY_LEN * 2];
    salt[..PUBLIC_KEY_LEN].copy_from_slice(ephemeral_public);
    salt[PUBLIC_KEY_LEN..].copy_from_slice(recipient_public);
    let mut info = Vec::with_capacity(WRAPPING_DOMAIN.len() + 16 + 4 + 16);
    info.extend_from_slice(WRAPPING_DOMAIN);
    info.extend_from_slice(ledger_id.as_bytes());
    info.extend_from_slice(&key_version.get().to_be_bytes());
    info.extend_from_slice(device_id.as_bytes());
    let hkdf = Hkdf::<Sha256>::new(Some(&salt), shared);
    let mut output = Zeroizing::new([0_u8; KEY_LEN]);
    hkdf.expand(&info, &mut output[..])
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    Ok(SymmetricKey::from_bytes(*output))
}

fn associated_data(ephemeral_public: &[u8; PUBLIC_KEY_LEN]) -> [u8; 1 + PUBLIC_KEY_LEN] {
    let mut associated_data = [0_u8; 1 + PUBLIC_KEY_LEN];
    associated_data[0] = FORMAT_VERSION;
    associated_data[1..].copy_from_slice(ephemeral_public);
    associated_data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger() -> Uuid {
        Uuid::parse_str("0199c0f0-0000-7000-8000-000000000001").unwrap()
    }

    fn device(value: u128) -> Uuid {
        Uuid::from_u128(value)
    }

    fn ldk(value: u8) -> SymmetricKey {
        SymmetricKey::from_bytes([value; KEY_LEN])
    }

    #[test]
    fn a_wrapped_key_round_trips_for_the_intended_device() {
        let recipient = DeviceAgreementKey::generate().unwrap();
        let key = ldk(0x11);
        let wrapped = WrappedLedgerKey::wrap(
            &key,
            &recipient.public_key(),
            ledger(),
            device(2),
            KeyVersion::INITIAL,
        )
        .unwrap();
        let unwrapped = wrapped
            .unwrap(&recipient, ledger(), device(2), KeyVersion::INITIAL)
            .unwrap();
        assert_eq!(unwrapped.expose_secret(), key.expose_secret());
    }

    #[test]
    fn the_wrapped_key_is_bound_to_the_ledger_device_and_version() {
        let recipient = DeviceAgreementKey::generate().unwrap();
        let key = ldk(0x22);
        let wrapped = WrappedLedgerKey::wrap(
            &key,
            &recipient.public_key(),
            ledger(),
            device(2),
            KeyVersion::INITIAL,
        )
        .unwrap();

        assert_eq!(
            wrapped
                .unwrap(
                    &recipient,
                    Uuid::from_u128(9),
                    device(2),
                    KeyVersion::INITIAL
                )
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(
            wrapped
                .unwrap(&recipient, ledger(), device(3), KeyVersion::INITIAL)
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(
            wrapped
                .unwrap(&recipient, ledger(), device(2), KeyVersion::new(2).unwrap())
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(
            wrapped
                .unwrap(
                    &DeviceAgreementKey::generate().unwrap(),
                    ledger(),
                    device(2),
                    KeyVersion::INITIAL
                )
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
    }

    #[test]
    fn serialized_wrapped_keys_round_trip_and_reject_tampering() {
        let recipient = DeviceAgreementKey::generate().unwrap();
        let key = ldk(0x33);
        let wrapped = WrappedLedgerKey::wrap(
            &key,
            &recipient.public_key(),
            ledger(),
            device(2),
            KeyVersion::INITIAL,
        )
        .unwrap();
        let bytes = wrapped.to_bytes();
        assert_eq!(bytes.len(), WRAPPED_LEDGER_KEY_LEN);
        assert_eq!(WrappedLedgerKey::from_bytes(&bytes).unwrap(), wrapped);

        let mut wrong_version = bytes;
        wrong_version[0] = 2;
        assert_eq!(
            WrappedLedgerKey::from_bytes(&wrong_version).err(),
            Some(CryptoError::InvalidWrappedKey)
        );
        assert_eq!(
            WrappedLedgerKey::from_bytes(&bytes[..WRAPPED_LEDGER_KEY_LEN - 1]).err(),
            Some(CryptoError::InvalidWrappedKey)
        );

        for index in [1, 33, 45, 92] {
            let mut tampered = bytes;
            tampered[index] ^= 1;
            let parsed = WrappedLedgerKey::from_bytes(&tampered).unwrap();
            assert_eq!(
                parsed
                    .unwrap(&recipient, ledger(), device(2), KeyVersion::INITIAL)
                    .err(),
                Some(CryptoError::DecryptionFailed),
                "offset {index}"
            );
        }
    }

    #[test]
    fn key_rotation_preserves_old_keys_and_issues_a_new_version() {
        let first = DeviceAgreementKey::generate().unwrap();
        let second = DeviceAgreementKey::generate().unwrap();
        let version_one = KeyVersion::INITIAL;
        let version_two = version_one.next().unwrap();
        let old_key = ldk(0x44);
        let new_key = ldk(0x55);

        let old_for_first = WrappedLedgerKey::wrap(
            &old_key,
            &first.public_key(),
            ledger(),
            device(1),
            version_one,
        )
        .unwrap();
        let old_for_second = WrappedLedgerKey::wrap(
            &old_key,
            &second.public_key(),
            ledger(),
            device(2),
            version_one,
        )
        .unwrap();
        let new_for_first = WrappedLedgerKey::wrap(
            &new_key,
            &first.public_key(),
            ledger(),
            device(1),
            version_two,
        )
        .unwrap();
        let new_for_second = WrappedLedgerKey::wrap(
            &new_key,
            &second.public_key(),
            ledger(),
            device(2),
            version_two,
        )
        .unwrap();

        assert_eq!(
            old_for_first
                .unwrap(&first, ledger(), device(1), version_one)
                .unwrap()
                .expose_secret(),
            old_key.expose_secret()
        );
        assert_eq!(
            old_for_second
                .unwrap(&second, ledger(), device(2), version_one)
                .unwrap()
                .expose_secret(),
            old_key.expose_secret()
        );
        assert_eq!(
            new_for_first
                .unwrap(&first, ledger(), device(1), version_two)
                .unwrap()
                .expose_secret(),
            new_key.expose_secret()
        );
        assert_eq!(
            new_for_second
                .unwrap(&second, ledger(), device(2), version_two)
                .unwrap()
                .expose_secret(),
            new_key.expose_secret()
        );
        assert_eq!(
            new_for_first
                .unwrap(&second, ledger(), device(1), version_two)
                .err(),
            Some(CryptoError::DecryptionFailed)
        );
    }

    #[test]
    fn low_order_keys_are_rejected() {
        let low_order = DeviceAgreementPublicKey::from_bytes([0; PUBLIC_KEY_LEN]);
        let recipient = DeviceAgreementKey::generate().unwrap();
        assert_eq!(
            WrappedLedgerKey::wrap(
                &ldk(1),
                &low_order,
                ledger(),
                device(2),
                KeyVersion::INITIAL
            )
            .err(),
            Some(CryptoError::InvalidPublicKey)
        );

        let mut bytes = WrappedLedgerKey::wrap(
            &ldk(1),
            &recipient.public_key(),
            ledger(),
            device(2),
            KeyVersion::INITIAL,
        )
        .unwrap()
        .to_bytes();
        bytes[1..1 + PUBLIC_KEY_LEN].fill(0);
        let wrapped = WrappedLedgerKey::from_bytes(&bytes).unwrap();
        assert_eq!(
            wrapped
                .unwrap(&recipient, ledger(), device(2), KeyVersion::INITIAL)
                .err(),
            Some(CryptoError::InvalidPublicKey)
        );
    }

    #[test]
    fn agreement_keys_are_redacted_and_round_trip() {
        let key = DeviceAgreementKey::from_bytes([9; PUBLIC_KEY_LEN]);
        assert_eq!(format!("{key:?}"), "DeviceAgreementKey(<redacted>)");
        assert_eq!(key.to_bytes().as_ref(), &[9; PUBLIC_KEY_LEN]);
        let public = key.public_key();
        assert_eq!(
            DeviceAgreementPublicKey::from_bytes(public.to_bytes()),
            public
        );
    }
}
