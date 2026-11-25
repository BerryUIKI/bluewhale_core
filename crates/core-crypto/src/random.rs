//! Operating-system random source.

use crate::CryptoError;

/// Fills the buffer with cryptographically secure random bytes.
///
/// # Errors
///
/// Returns [`CryptoError::RandomnessUnavailable`] when the operating system
/// random source fails.
pub(crate) fn fill(buffer: &mut [u8]) -> Result<(), CryptoError> {
    getrandom::fill(buffer).map_err(|_| CryptoError::RandomnessUnavailable)
}
