//! Shared text, color, and shortcut validation helpers.

use unicode_normalization::UnicodeNormalization;

/// Trims and NFC-normalizes a display name.
pub(crate) fn normalize_name(value: &str) -> String {
    value.trim().nfc().collect()
}

/// Produces the case-folded key used for uniqueness checks.
pub(crate) fn canonical_key(value: &str) -> String {
    normalize_name(value).to_lowercase()
}

/// Trims an optional text value and drops it when empty.
pub(crate) fn normalize_optional_text(value: Option<&str>) -> Option<String> {
    value
        .map(normalize_name)
        .filter(|normalized| !normalized.is_empty())
}

/// Validates a `#RRGGBB` color and returns it uppercased.
pub(crate) fn normalize_color(value: &str) -> Option<String> {
    let normalized = value.to_ascii_uppercase();
    let digits = normalized.strip_prefix('#')?;
    if normalized.len() == 7 && digits.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Some(normalized)
    } else {
        None
    }
}

/// Validates a single-letter shortcut and returns it uppercased.
pub(crate) fn normalize_shortcut(value: Option<&str>) -> Result<Option<String>, ()> {
    let Some(trimmed) = value.map(str::trim) else {
        return Ok(None);
    };
    if trimmed.is_empty() {
        return Ok(None);
    }
    let normalized = trimmed.to_ascii_uppercase();
    let bytes = normalized.as_bytes();
    if bytes.len() == 1 && bytes[0].is_ascii_uppercase() {
        Ok(Some(normalized))
    } else {
        Err(())
    }
}
