//! The user-held recovery key and its encrypted user-master-key envelope.
//!
//! The recovery key is 256 random bits encoded in Crockford Base32 with a
//! 20-bit checksum. The encoding is case-insensitive, ignores separators, maps
//! the ambiguous `O`, `I`, and `L` to digits, and rejects the alphabet's
//! excluded `U`. The checksum catches transcription errors.
//!
//! The recovery envelope is 61 bytes: a format version, a nonce, and the
//! encrypted UMK. The server may store the envelope but never the recovery key
//! or UMK.

use std::fmt;

use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::aead;
use crate::key::{KEY_LEN, SymmetricKey};
use crate::{CryptoError, random};

/// The recovery-key length, in bytes.
pub const RECOVERY_KEY_LEN: usize = 32;

/// The number of data characters in a recovery code.
const RECOVERY_PAYLOAD_CHARS: usize = 52;

/// The number of checksum characters in a recovery code.
const CHECKSUM_CHARS: usize = 4;

/// The number of data and checksum characters, without separators.
pub const RECOVERY_CODE_CHARS: usize = RECOVERY_PAYLOAD_CHARS + CHECKSUM_CHARS;

/// The serialized recovery-envelope length, in bytes.
pub const RECOVERY_ENVELOPE_LEN: usize = 1 + aead::NONCE_LEN + KEY_LEN + aead::TAG_LEN;

const ENVELOPE_FORMAT_VERSION: u8 = 1;
const ENVELOPE_DOMAIN: &[u8] = b"bluewhale/recovery-envelope/v1";
const CHECKSUM_DOMAIN: &[u8] = b"bluewhale/recovery-checksum/v1";
const ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// A user-held 256-bit recovery key.
///
/// The key is zeroized on drop and redacted from debug output. The encoded form
/// is the only representation the user should store; the server must never see
/// either form.
pub struct RecoveryKey(SymmetricKey);

impl RecoveryKey {
    /// Generates a recovery key from the operating system random source.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::RandomnessUnavailable`] when the random source
    /// fails.
    pub fn generate() -> Result<Self, CryptoError> {
        Ok(Self(SymmetricKey::generate()?))
    }

    /// Creates a recovery key from raw bytes.
    ///
    /// The caller must zeroize its own copy of `bytes`.
    #[must_use]
    pub fn from_bytes(bytes: [u8; RECOVERY_KEY_LEN]) -> Self {
        Self(SymmetricKey::from_bytes(bytes))
    }

    /// Exposes the raw key bytes.
    ///
    /// The caller must zeroize the returned buffer and must not log it.
    #[must_use]
    pub fn to_bytes(&self) -> Zeroizing<[u8; RECOVERY_KEY_LEN]> {
        Zeroizing::new(*self.0.expose_secret())
    }

    /// Encodes the key for human transcription.
    ///
    /// The result is 14 groups of four Crockford Base32 characters separated by
    /// hyphens. It is zeroized on drop.
    #[must_use]
    pub fn encode(&self) -> Zeroizing<String> {
        let payload = encode_payload(self.0.expose_secret());
        let checksum = encode_checksum(checksum_bits(self.0.expose_secret()));
        let mut text = Zeroizing::new(String::with_capacity(RECOVERY_CODE_CHARS + 13));
        for (index, value) in payload.iter().chain(checksum.iter()).enumerate() {
            if index > 0 && index % 4 == 0 {
                text.push('-');
            }
            text.push(char::from(*value));
        }
        text
    }

    /// Parses a recovery code.
    ///
    /// Separators are optional, case is ignored, and `O`, `I`, and `L` are
    /// accepted as `0`, `1`, and `1`. The code must be canonical and its
    /// checksum must match.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidRecoveryKey`] for a wrong length, an
    /// excluded character, non-zero padding bits, or a bad checksum.
    pub fn parse(text: &str) -> Result<Self, CryptoError> {
        let normalized = normalize(text)?;
        if normalized.len() != RECOVERY_CODE_CHARS {
            return Err(CryptoError::InvalidRecoveryKey);
        }
        let (payload, checksum) = normalized.split_at(RECOVERY_PAYLOAD_CHARS);
        let mut bytes = Zeroizing::new([0_u8; RECOVERY_KEY_LEN]);
        let mut buffer = 0_u32;
        let mut bits = 0_u32;
        let mut index = 0;
        for value in payload.chars() {
            let digit = decode_char(value).ok_or(CryptoError::InvalidRecoveryKey)?;
            buffer = (buffer << 5) | u32::from(digit);
            bits += 5;
            if bits >= 8 {
                bits -= 8;
                bytes[index] = (buffer >> bits) as u8;
                index += 1;
                buffer &= (1_u32 << bits) - 1;
            }
        }
        if index != RECOVERY_KEY_LEN || bits != 4 || buffer != 0 {
            return Err(CryptoError::InvalidRecoveryKey);
        }

        let mut expected = 0_u32;
        for value in checksum.chars() {
            let digit = decode_char(value).ok_or(CryptoError::InvalidRecoveryKey)?;
            expected = (expected << 5) | u32::from(digit);
        }
        if expected != checksum_bits(&bytes) {
            return Err(CryptoError::InvalidRecoveryKey);
        }

        let key = Self(SymmetricKey::from_bytes(*bytes));
        let canonical = key.encode();
        if canonical.replace('-', "") != *normalized {
            return Err(CryptoError::InvalidRecoveryKey);
        }
        Ok(key)
    }
}

impl fmt::Debug for RecoveryKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RecoveryKey(<redacted>)")
    }
}

/// The user master key encrypted by the recovery key.
///
/// The envelope contains no plaintext secret and may be stored by the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveryEnvelope {
    nonce: [u8; aead::NONCE_LEN],
    ciphertext: [u8; KEY_LEN + aead::TAG_LEN],
}

impl RecoveryEnvelope {
    /// Encrypts the UMK with the recovery key.
    ///
    /// # Errors
    ///
    /// Returns a [`CryptoError`] when the random source fails or encryption
    /// fails.
    pub fn wrap(umk: &SymmetricKey, recovery: &RecoveryKey) -> Result<Self, CryptoError> {
        let mut nonce = [0_u8; aead::NONCE_LEN];
        random::fill(&mut nonce)?;
        let sealed = aead::seal(&recovery.0, &nonce, &associated_data(), umk.expose_secret())?;
        Ok(Self {
            nonce,
            ciphertext: sealed
                .try_into()
                .map_err(|_| CryptoError::EncryptionFailed)?,
        })
    }

    /// Decrypts the UMK with the recovery key.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::DecryptionFailed`] for a wrong recovery key or a
    /// modified envelope.
    pub fn unwrap(&self, recovery: &RecoveryKey) -> Result<SymmetricKey, CryptoError> {
        let plaintext = aead::open(
            &recovery.0,
            &self.nonce,
            &associated_data(),
            &self.ciphertext,
        )?;
        let key: [u8; KEY_LEN] = plaintext
            .as_slice()
            .try_into()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(SymmetricKey::from_bytes(key))
    }

    /// Serializes the envelope.
    #[must_use]
    pub fn to_bytes(&self) -> [u8; RECOVERY_ENVELOPE_LEN] {
        let mut bytes = [0_u8; RECOVERY_ENVELOPE_LEN];
        bytes[0] = ENVELOPE_FORMAT_VERSION;
        bytes[1..1 + aead::NONCE_LEN].copy_from_slice(&self.nonce);
        bytes[1 + aead::NONCE_LEN..].copy_from_slice(&self.ciphertext);
        bytes
    }

    /// Deserializes an envelope.
    ///
    /// # Errors
    ///
    /// Returns [`CryptoError::InvalidRecoveryEnvelope`] for a wrong length or
    /// an unsupported format version.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, CryptoError> {
        if bytes.len() != RECOVERY_ENVELOPE_LEN || bytes[0] != ENVELOPE_FORMAT_VERSION {
            return Err(CryptoError::InvalidRecoveryEnvelope);
        }
        let mut nonce = [0_u8; aead::NONCE_LEN];
        nonce.copy_from_slice(&bytes[1..1 + aead::NONCE_LEN]);
        let mut ciphertext = [0_u8; KEY_LEN + aead::TAG_LEN];
        ciphertext.copy_from_slice(&bytes[1 + aead::NONCE_LEN..]);
        Ok(Self { nonce, ciphertext })
    }
}

fn normalize(text: &str) -> Result<Zeroizing<String>, CryptoError> {
    let mut normalized = Zeroizing::new(String::with_capacity(RECOVERY_CODE_CHARS));
    for value in text.chars() {
        if value == '-' || value.is_ascii_whitespace() {
            continue;
        }
        if !value.is_ascii() {
            return Err(CryptoError::InvalidRecoveryKey);
        }
        let upper = value.to_ascii_uppercase();
        normalized.push(match upper {
            'O' => '0',
            'I' | 'L' => '1',
            _ => upper,
        });
    }
    Ok(normalized)
}

fn decode_char(value: char) -> Option<u8> {
    let normalized = match value {
        'O' => '0',
        'I' | 'L' => '1',
        _ => value,
    };
    ALPHABET
        .iter()
        .position(|candidate| char::from(*candidate) == normalized)
        .map(|index| index as u8)
}

fn encode_payload(bytes: &[u8; RECOVERY_KEY_LEN]) -> [u8; RECOVERY_PAYLOAD_CHARS] {
    let mut output = [0_u8; RECOVERY_PAYLOAD_CHARS];
    let mut buffer = 0_u32;
    let mut bits = 0_u32;
    let mut index = 0;
    for byte in bytes {
        buffer = (buffer << 8) | u32::from(*byte);
        bits += 8;
        while bits >= 5 {
            bits -= 5;
            output[index] = ALPHABET[((buffer >> bits) & 0x1f) as usize];
            index += 1;
            buffer &= (1_u32 << bits) - 1;
        }
    }
    if bits > 0 {
        output[index] = ALPHABET[((buffer << (5 - bits)) & 0x1f) as usize];
        index += 1;
    }
    debug_assert_eq!(index, RECOVERY_PAYLOAD_CHARS);
    output
}

fn encode_checksum(value: u32) -> [u8; CHECKSUM_CHARS] {
    let mut output = [0_u8; CHECKSUM_CHARS];
    for (index, slot) in output.iter_mut().enumerate() {
        *slot = ALPHABET[((value >> (15 - index * 5)) & 0x1f) as usize];
    }
    output
}

fn checksum_bits(key: &[u8; RECOVERY_KEY_LEN]) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(CHECKSUM_DOMAIN);
    hasher.update(key);
    let digest = hasher.finalize();
    (u32::from(digest[0]) << 12) | (u32::from(digest[1]) << 4) | (u32::from(digest[2]) >> 4)
}

fn associated_data() -> Vec<u8> {
    let mut associated_data = Vec::with_capacity(ENVELOPE_DOMAIN.len() + 1);
    associated_data.extend_from_slice(ENVELOPE_DOMAIN);
    associated_data.push(ENVELOPE_FORMAT_VERSION);
    associated_data
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vector_key() -> RecoveryKey {
        let mut bytes = [0_u8; RECOVERY_KEY_LEN];
        for (index, byte) in bytes.iter_mut().enumerate() {
            *byte = index as u8;
        }
        RecoveryKey::from_bytes(bytes)
    }

    const VECTOR: &str = "000G-40R4-0M30-E209-185G-R38E-1W81-24GK-2GAH-C5RR-34D1-P70X-3RFG-QSD7";

    #[test]
    fn the_fixed_vector_encodes_and_parses() {
        assert_eq!(vector_key().encode().as_str(), VECTOR);
        let parsed = RecoveryKey::parse(VECTOR).unwrap();
        assert_eq!(parsed.to_bytes().as_ref(), vector_key().to_bytes().as_ref());
        let parsed_code = parsed.encode();
        let expected_code = vector_key().encode();
        assert_eq!(parsed_code.as_str(), expected_code.as_str());
    }

    #[test]
    fn parsing_ignores_case_separators_and_ambiguous_characters() {
        assert_eq!(
            RecoveryKey::parse(&VECTOR.to_lowercase())
                .unwrap()
                .encode()
                .as_str(),
            VECTOR
        );
        assert_eq!(
            RecoveryKey::parse(&VECTOR.replace('-', " "))
                .unwrap()
                .encode()
                .as_str(),
            VECTOR
        );
        assert_eq!(
            RecoveryKey::parse(&VECTOR.replace('-', ""))
                .unwrap()
                .encode()
                .as_str(),
            VECTOR
        );

        let ambiguous = VECTOR.replace('0', "O").replace('1', "I");
        assert_eq!(
            RecoveryKey::parse(&ambiguous).unwrap().encode().as_str(),
            VECTOR
        );
    }

    #[test]
    fn malformed_codes_are_rejected() {
        assert_eq!(
            RecoveryKey::parse("").err(),
            Some(CryptoError::InvalidRecoveryKey)
        );
        assert_eq!(
            RecoveryKey::parse(&VECTOR[..VECTOR.len() - 1]).err(),
            Some(CryptoError::InvalidRecoveryKey)
        );
        assert_eq!(
            RecoveryKey::parse(&VECTOR.replace('F', "U")).err(),
            Some(CryptoError::InvalidRecoveryKey)
        );

        let mut wrong_checksum = VECTOR.to_string();
        wrong_checksum.pop();
        wrong_checksum.push('F');
        assert_eq!(
            RecoveryKey::parse(&wrong_checksum).err(),
            Some(CryptoError::InvalidRecoveryKey)
        );

        let mut wrong_payload = VECTOR.to_string();
        wrong_payload.replace_range(0..1, "1");
        assert_eq!(
            RecoveryKey::parse(&wrong_payload).err(),
            Some(CryptoError::InvalidRecoveryKey)
        );

        let mut bad_padding = VECTOR.to_string();
        bad_padding.replace_range(63..64, "H");
        assert_eq!(
            RecoveryKey::parse(&bad_padding).err(),
            Some(CryptoError::InvalidRecoveryKey)
        );

        assert_eq!(
            RecoveryKey::parse("000G-40R4-0M30-🐋").err(),
            Some(CryptoError::InvalidRecoveryKey)
        );
    }

    #[test]
    fn recovery_keys_are_redacted_and_round_trip() {
        assert_eq!(format!("{:?}", vector_key()), "RecoveryKey(<redacted>)");
        let generated = RecoveryKey::generate().unwrap();
        let code = generated.encode();
        assert_eq!(
            RecoveryKey::parse(code.as_str())
                .unwrap()
                .to_bytes()
                .as_ref(),
            generated.to_bytes().as_ref()
        );
    }

    #[test]
    fn the_recovery_envelope_round_trips_and_rejects_a_wrong_key() {
        let umk = SymmetricKey::from_bytes([0x5a; KEY_LEN]);
        let recovery = RecoveryKey::generate().unwrap();
        let envelope = RecoveryEnvelope::wrap(&umk, &recovery).unwrap();
        assert_eq!(
            envelope.unwrap(&recovery).unwrap().expose_secret(),
            umk.expose_secret()
        );
        assert_eq!(
            envelope.unwrap(&RecoveryKey::generate().unwrap()).err(),
            Some(CryptoError::DecryptionFailed)
        );

        let bytes = envelope.to_bytes();
        assert_eq!(bytes.len(), RECOVERY_ENVELOPE_LEN);
        assert_eq!(RecoveryEnvelope::from_bytes(&bytes).unwrap(), envelope);

        let mut wrong_version = bytes;
        wrong_version[0] = 2;
        assert_eq!(
            RecoveryEnvelope::from_bytes(&wrong_version).err(),
            Some(CryptoError::InvalidRecoveryEnvelope)
        );
        assert_eq!(
            RecoveryEnvelope::from_bytes(&bytes[..RECOVERY_ENVELOPE_LEN - 1]).err(),
            Some(CryptoError::InvalidRecoveryEnvelope)
        );

        for index in [1, 13, 60] {
            let mut tampered = bytes;
            tampered[index] ^= 1;
            let parsed = RecoveryEnvelope::from_bytes(&tampered).unwrap();
            assert_eq!(
                parsed.unwrap(&recovery).err(),
                Some(CryptoError::DecryptionFailed),
                "offset {index}"
            );
        }
    }
}
