//! Currency codes and the versioned currency-scale policy.

use std::collections::BTreeMap;
use std::fmt;

/// A validated ISO-4217-style currency code and its minor-unit scale.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Currency {
    code: String,
    scale: u8,
}

/// Errors raised while validating a currency or a currency-scale policy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CurrencyError {
    /// The code is not exactly three ASCII letters.
    InvalidCode(String),
    /// The scale is greater than nine.
    InvalidScale(u8),
    /// The policy version is not positive.
    InvalidPolicyVersion(u32),
    /// The policy does not contain the requested currency.
    UnknownCurrency(String),
}

impl fmt::Display for CurrencyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCode(code) => {
                write!(formatter, "invalid currency code: {code}")
            }
            Self::InvalidScale(scale) => {
                write!(formatter, "invalid currency scale: {scale}")
            }
            Self::InvalidPolicyVersion(version) => {
                write!(formatter, "invalid currency policy version: {version}")
            }
            Self::UnknownCurrency(code) => {
                write!(formatter, "unknown currency: {code}")
            }
        }
    }
}

impl std::error::Error for CurrencyError {}

impl Currency {
    /// Creates a currency, normalizing the code to uppercase.
    ///
    /// # Errors
    ///
    /// Returns [`CurrencyError::InvalidCode`] when the code is not exactly
    /// three ASCII letters, and [`CurrencyError::InvalidScale`] when the scale
    /// is greater than nine.
    pub fn new(code: &str, scale: u8) -> Result<Self, CurrencyError> {
        let normalized = code.to_ascii_uppercase();
        let bytes = normalized.as_bytes();
        if bytes.len() != 3 || !bytes.iter().all(u8::is_ascii_uppercase) {
            return Err(CurrencyError::InvalidCode(code.to_string()));
        }
        if scale > 9 {
            return Err(CurrencyError::InvalidScale(scale));
        }
        Ok(Self {
            code: normalized,
            scale,
        })
    }

    /// Returns the uppercase currency code.
    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    /// Returns the number of fractional decimal digits.
    #[must_use]
    pub fn scale(&self) -> u8 {
        self.scale
    }
}

/// A versioned mapping from currency codes to minor-unit scales.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CurrencyScalePolicy {
    version: u32,
    scales: BTreeMap<String, u8>,
}

impl CurrencyScalePolicy {
    /// Creates a policy from validated currency codes and scales.
    ///
    /// # Errors
    ///
    /// Returns [`CurrencyError::InvalidPolicyVersion`] when the version is
    /// zero, and any error from [`Currency::new`] for an invalid currency.
    pub fn new(version: u32, scales: &[(&str, u8)]) -> Result<Self, CurrencyError> {
        if version == 0 {
            return Err(CurrencyError::InvalidPolicyVersion(version));
        }
        let mut normalized = BTreeMap::new();
        for (code, scale) in scales {
            let currency = Currency::new(code, *scale)?;
            normalized.insert(currency.code, currency.scale);
        }
        Ok(Self {
            version,
            scales: normalized,
        })
    }

    /// Returns the policy version.
    #[must_use]
    pub fn version(&self) -> u32 {
        self.version
    }

    /// Looks up a currency by code, normalizing the code to uppercase.
    ///
    /// # Errors
    ///
    /// Returns [`CurrencyError::UnknownCurrency`] when the policy does not
    /// contain the code.
    pub fn currency(&self, code: &str) -> Result<Currency, CurrencyError> {
        let normalized = code.to_ascii_uppercase();
        let scale = self
            .scales
            .get(&normalized)
            .copied()
            .ok_or_else(|| CurrencyError::UnknownCurrency(normalized.clone()))?;
        Currency::new(&normalized, scale)
    }

    /// Returns the supported codes in ascending order.
    #[must_use]
    pub fn supported_codes(&self) -> Vec<&str> {
        self.scales.keys().map(String::as_str).collect()
    }

    /// Returns the built-in version 1 policy used by Bluewhale.
    #[must_use]
    pub fn v1() -> Self {
        let mut scales = BTreeMap::new();
        for (code, scale) in V1_SCALES {
            scales.insert((*code).to_string(), *scale);
        }
        Self { version: 1, scales }
    }
}

/// The version 1 currency-scale policy: scale 0 for zero-decimal currencies,
/// scale 3 for three-decimal currencies, scale 2 for the rest.
const V1_SCALES: &[(&str, u8)] = &[
    ("AED", 2),
    ("AUD", 2),
    ("BHD", 3),
    ("BRL", 2),
    ("CAD", 2),
    ("CHF", 2),
    ("CLP", 0),
    ("CNY", 2),
    ("CZK", 2),
    ("DKK", 2),
    ("EUR", 2),
    ("GBP", 2),
    ("HKD", 2),
    ("HUF", 2),
    ("IDR", 2),
    ("ILS", 2),
    ("INR", 2),
    ("ISK", 0),
    ("JOD", 3),
    ("JPY", 0),
    ("KRW", 0),
    ("KWD", 3),
    ("MXN", 2),
    ("MYR", 2),
    ("NOK", 2),
    ("NZD", 2),
    ("OMR", 3),
    ("PHP", 2),
    ("PLN", 2),
    ("RUB", 2),
    ("SAR", 2),
    ("SEK", 2),
    ("SGD", 2),
    ("THB", 2),
    ("TND", 3),
    ("TRY", 2),
    ("TWD", 2),
    ("USD", 2),
    ("VND", 0),
    ("ZAR", 2),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_policy_supports_zero_two_and_three_decimal_currencies() {
        let policy = CurrencyScalePolicy::v1();

        assert_eq!(policy.currency("JPY").unwrap().scale(), 0);
        assert_eq!(policy.currency("USD").unwrap().scale(), 2);
        assert_eq!(policy.currency("KWD").unwrap().scale(), 3);
        assert!(policy.supported_codes().contains(&"CNY"));
        assert_eq!(policy.supported_codes().len(), 40);
    }

    #[test]
    fn policy_normalizes_codes_and_supports_explicit_scales() {
        let policy = CurrencyScalePolicy::new(1, &[("JPY", 0), ("USD", 2), ("KWD", 3)]).unwrap();

        assert_eq!(policy.currency("jpy").unwrap().scale(), 0);
        assert_eq!(policy.currency("usd").unwrap().scale(), 2);
        assert_eq!(policy.currency("kwd").unwrap().scale(), 3);
        assert_eq!(policy.version(), 1);
    }

    #[test]
    fn invalid_currency_is_rejected() {
        assert_eq!(
            Currency::new("US", 2),
            Err(CurrencyError::InvalidCode("US".to_string()))
        );
        assert_eq!(
            Currency::new("US1", 2),
            Err(CurrencyError::InvalidCode("US1".to_string()))
        );
        assert_eq!(
            Currency::new("USD", 10),
            Err(CurrencyError::InvalidScale(10))
        );
        assert_eq!(
            CurrencyScalePolicy::new(0, &[("USD", 2)]),
            Err(CurrencyError::InvalidPolicyVersion(0))
        );
    }
}
