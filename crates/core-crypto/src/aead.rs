//! ChaCha20-Poly1305 authenticated encryption with associated data.
//!
//! Higher-level key wrapping in this crate generates a fresh random nonce and
//! never reuses a nonce with the same key. Callers that use these low-level
//! functions directly are responsible for that invariant.

use chacha20poly1305::aead::{Aead, Payload};
use chacha20poly1305::{ChaCha20Poly1305, KeyInit};
use zeroize::Zeroizing;

use crate::{CryptoError, SymmetricKey};

/// The AEAD nonce length, in bytes.
pub const NONCE_LEN: usize = 12;

/// The Poly1305 authentication tag length, in bytes.
pub const TAG_LEN: usize = 16;

/// Encrypts a plaintext and appends the authentication tag.
///
/// # Errors
///
/// Returns [`CryptoError::EncryptionFailed`] when the cipher rejects the input.
/// A 32-byte key and a 12-byte nonce are always accepted, so this is defensive.
pub fn seal(
    key: &SymmetricKey,
    nonce: &[u8; NONCE_LEN],
    associated_data: &[u8],
    plaintext: &[u8],
) -> Result<Vec<u8>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| CryptoError::EncryptionFailed)?;
    cipher
        .encrypt(
            nonce.into(),
            Payload {
                msg: plaintext,
                aad: associated_data,
            },
        )
        .map_err(|_| CryptoError::EncryptionFailed)
}

/// Decrypts a ciphertext and verifies its authentication tag.
///
/// # Errors
///
/// Returns [`CryptoError::DecryptionFailed`] for any authentication failure,
/// including a wrong key, nonce, or associated data.
pub fn open(
    key: &SymmetricKey,
    nonce: &[u8; NONCE_LEN],
    associated_data: &[u8],
    ciphertext: &[u8],
) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
    let cipher = ChaCha20Poly1305::new_from_slice(key.expose_secret())
        .map_err(|_| CryptoError::DecryptionFailed)?;
    let plaintext = cipher
        .decrypt(
            nonce.into(),
            Payload {
                msg: ciphertext,
                aad: associated_data,
            },
        )
        .map_err(|_| CryptoError::DecryptionFailed)?;
    Ok(Zeroizing::new(plaintext))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_binds_the_associated_data() {
        let key = SymmetricKey::from_bytes([3; 32]);
        let nonce = [9; NONCE_LEN];
        let sealed = seal(&key, &nonce, b"header", b"secret").unwrap();
        assert_eq!(sealed.len(), b"secret".len() + TAG_LEN);
        assert_eq!(
            &open(&key, &nonce, b"header", &sealed).unwrap()[..],
            b"secret"
        );
        assert_eq!(
            open(&key, &nonce, b"other", &sealed).err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(
            open(&key, &[8; NONCE_LEN], b"header", &sealed).err(),
            Some(CryptoError::DecryptionFailed)
        );
        assert_eq!(
            open(
                &SymmetricKey::from_bytes([4; 32]),
                &nonce,
                b"header",
                &sealed
            )
            .err(),
            Some(CryptoError::DecryptionFailed)
        );
    }

    #[test]
    fn tampering_is_rejected() {
        let key = SymmetricKey::from_bytes([3; 32]);
        let nonce = [9; NONCE_LEN];
        let mut sealed = seal(&key, &nonce, b"", b"secret").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 1;
        assert_eq!(
            open(&key, &nonce, b"", &sealed).err(),
            Some(CryptoError::DecryptionFailed)
        );
    }
}
