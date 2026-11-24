//! Money values, exact magnitude parsing, and minor-unit rounding.

use std::fmt;

use crate::currency::Currency;

/// A monetary amount stored as an integer number of minor units.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Money {
    minor_units: i64,
    currency: Currency,
}

/// Errors raised by money arithmetic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyError {
    /// The two amounts use different currencies.
    CurrencyMismatch {
        /// The currency of the left-hand amount.
        expected: Currency,
        /// The currency of the right-hand amount.
        actual: Currency,
    },
    /// The result does not fit in an `i64`.
    Overflow,
}

impl fmt::Display for MoneyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrencyMismatch { expected, actual } => write!(
                formatter,
                "currency mismatch: expected {}, got {}",
                expected.code(),
                actual.code()
            ),
            Self::Overflow => formatter.write_str("money overflow"),
        }
    }
}

impl std::error::Error for MoneyError {}

impl Money {
    /// Creates an amount from minor units.
    #[must_use]
    pub fn new(minor_units: i64, currency: Currency) -> Self {
        Self {
            minor_units,
            currency,
        }
    }

    /// Creates a zero amount in the given currency.
    #[must_use]
    pub fn zero(currency: Currency) -> Self {
        Self::new(0, currency)
    }

    /// Returns the integer minor units.
    #[must_use]
    pub fn minor_units(&self) -> i64 {
        self.minor_units
    }

    /// Returns the amount's currency.
    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Adds another amount in the same currency.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ and
    /// [`MoneyError::Overflow`] when the result does not fit in an `i64`.
    pub fn adding(&self, other: &Self) -> Result<Self, MoneyError> {
        self.require_same_currency(other)?;
        let minor_units = self
            .minor_units
            .checked_add(other.minor_units)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self::new(minor_units, self.currency.clone()))
    }

    /// Subtracts another amount in the same currency.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyError::CurrencyMismatch`] when the currencies differ and
    /// [`MoneyError::Overflow`] when the result does not fit in an `i64`.
    pub fn subtracting(&self, other: &Self) -> Result<Self, MoneyError> {
        self.require_same_currency(other)?;
        let minor_units = self
            .minor_units
            .checked_sub(other.minor_units)
            .ok_or(MoneyError::Overflow)?;
        Ok(Self::new(minor_units, self.currency.clone()))
    }

    fn require_same_currency(&self, other: &Self) -> Result<(), MoneyError> {
        if self.currency == other.currency {
            Ok(())
        } else {
            Err(MoneyError::CurrencyMismatch {
                expected: self.currency.clone(),
                actual: other.currency.clone(),
            })
        }
    }
}

/// The policy applied when a parsed magnitude has more fractional digits than
/// the currency allows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MoneyRoundingPolicy {
    /// Reject the input instead of rounding it.
    #[default]
    RejectExcessFraction,
}

/// Parses non-negative decimal magnitudes into [`Money`] without floating
/// point.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MoneyParser {
    rounding_policy: MoneyRoundingPolicy,
}

/// Errors raised while parsing a money magnitude.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoneyParsingError {
    /// The text is empty or not a plain decimal magnitude.
    InvalidFormat(String),
    /// The text starts with a minus sign.
    NegativeMagnitude,
    /// The text has more fractional digits than the currency allows.
    ExcessFractionalDigits {
        /// The maximum number of fractional digits for the currency.
        allowed: u8,
    },
    /// The value does not fit in an `i64` number of minor units.
    Overflow,
}

impl fmt::Display for MoneyParsingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(text) => write!(formatter, "invalid money format: {text}"),
            Self::NegativeMagnitude => formatter.write_str("negative magnitude"),
            Self::ExcessFractionalDigits { allowed } => {
                write!(formatter, "excess fractional digits: at most {allowed}")
            }
            Self::Overflow => formatter.write_str("money overflow"),
        }
    }
}

impl std::error::Error for MoneyParsingError {}

impl MoneyParser {
    /// Creates a parser with the given excess-fraction policy.
    #[must_use]
    pub fn new(rounding_policy: MoneyRoundingPolicy) -> Self {
        Self { rounding_policy }
    }

    /// Parses a non-negative decimal magnitude into the currency's minor units.
    ///
    /// # Errors
    ///
    /// Returns [`MoneyParsingError::InvalidFormat`] for anything other than
    /// ASCII digits with at most one decimal point,
    /// [`MoneyParsingError::NegativeMagnitude`] for a leading minus sign,
    /// [`MoneyParsingError::ExcessFractionalDigits`] when the fraction is
    /// longer than the currency scale, and [`MoneyParsingError::Overflow`] when
    /// the value does not fit in an `i64`.
    pub fn parse_magnitude(
        &self,
        text: &str,
        currency: &Currency,
    ) -> Result<Money, MoneyParsingError> {
        let value = text.trim();
        if value.is_empty() {
            return Err(MoneyParsingError::InvalidFormat(text.to_string()));
        }
        if value.starts_with('-') {
            return Err(MoneyParsingError::NegativeMagnitude);
        }

        let mut components = value.split('.');
        let whole = components.next().unwrap_or_default();
        let fraction = components.next();
        if components.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(MoneyParsingError::InvalidFormat(text.to_string()));
        }
        let fraction = fraction.unwrap_or_default();
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(MoneyParsingError::InvalidFormat(text.to_string()));
        }

        let allowed = currency.scale();
        let fraction_length = u8::try_from(fraction.len())
            .map_err(|_| MoneyParsingError::ExcessFractionalDigits { allowed })?;
        if fraction_length > allowed {
            match self.rounding_policy {
                MoneyRoundingPolicy::RejectExcessFraction => {
                    return Err(MoneyParsingError::ExcessFractionalDigits { allowed });
                }
            }
        }

        let whole_value = parse_digits(whole)?;
        let factor = power_of_ten(allowed)?;
        let scaled_whole = whole_value
            .checked_mul(factor)
            .ok_or(MoneyParsingError::Overflow)?;

        let fraction_value = parse_digits(fraction)?;
        let padding = allowed - fraction_length;
        let padding_factor = power_of_ten(padding)?;
        let scaled_fraction = fraction_value
            .checked_mul(padding_factor)
            .ok_or(MoneyParsingError::Overflow)?;

        let minor_units = scaled_whole
            .checked_add(scaled_fraction)
            .ok_or(MoneyParsingError::Overflow)?;
        Ok(Money::new(minor_units, currency.clone()))
    }
}

fn parse_digits(value: &str) -> Result<i64, MoneyParsingError> {
    let mut result: i64 = 0;
    for byte in value.bytes() {
        let digit = i64::from(byte - b'0');
        result = result
            .checked_mul(10)
            .and_then(|current| current.checked_add(digit))
            .ok_or(MoneyParsingError::Overflow)?;
    }
    Ok(result)
}

fn power_of_ten(exponent: u8) -> Result<i64, MoneyParsingError> {
    let mut result: i64 = 1;
    for _ in 0..exponent {
        result = result.checked_mul(10).ok_or(MoneyParsingError::Overflow)?;
    }
    Ok(result)
}

/// Errors raised while rounding a ratio to a whole number of minor units.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoneyRoundingError {
    /// The divisor is not positive.
    InvalidDivisor,
    /// The dividend is negative.
    NegativeDividend,
    /// The rounded result does not fit in an `i64`.
    Overflow,
}

impl fmt::Display for MoneyRoundingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidDivisor => formatter.write_str("invalid divisor"),
            Self::NegativeDividend => formatter.write_str("negative dividend"),
            Self::Overflow => formatter.write_str("rounding overflow"),
        }
    }
}

impl std::error::Error for MoneyRoundingError {}

/// Rounds `dividend / divisor` to the nearest whole minor unit, with ties
/// rounded up (away from zero for non-negative values).
///
/// # Errors
///
/// Returns [`MoneyRoundingError::InvalidDivisor`] for a non-positive divisor,
/// [`MoneyRoundingError::NegativeDividend`] for a negative dividend, and
/// [`MoneyRoundingError::Overflow`] if the result does not fit in an `i64`.
pub fn nearest_minor_unit(dividend: i64, divisor: i64) -> Result<i64, MoneyRoundingError> {
    if divisor <= 0 {
        return Err(MoneyRoundingError::InvalidDivisor);
    }
    if dividend < 0 {
        return Err(MoneyRoundingError::NegativeDividend);
    }

    let divisor = u64::try_from(divisor).map_err(|_| MoneyRoundingError::InvalidDivisor)?;
    let dividend = u64::try_from(dividend).map_err(|_| MoneyRoundingError::NegativeDividend)?;
    let quotient = dividend / divisor;
    let remainder = dividend % divisor;
    let rounds_up = remainder.saturating_mul(2) >= divisor;
    let rounded = if rounds_up {
        quotient
            .checked_add(1)
            .ok_or(MoneyRoundingError::Overflow)?
    } else {
        quotient
    };
    i64::try_from(rounded).map_err(|_| MoneyRoundingError::Overflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn currency(code: &str, scale: u8) -> Currency {
        Currency::new(code, scale).unwrap()
    }

    #[test]
    fn parser_converts_magnitudes_without_floating_point() {
        let parser = MoneyParser::default();
        let usd = currency("USD", 2);
        let jpy = currency("JPY", 0);
        let kwd = currency("KWD", 3);

        assert_eq!(
            parser
                .parse_magnitude("123.45", &usd)
                .unwrap()
                .minor_units(),
            12_345
        );
        assert_eq!(
            parser.parse_magnitude("123", &jpy).unwrap().minor_units(),
            123
        );
        assert_eq!(
            parser.parse_magnitude("1.2", &kwd).unwrap().minor_units(),
            1_200
        );
    }

    #[test]
    fn parser_rejects_negative_and_excess_fractional_input() {
        let parser = MoneyParser::default();
        let usd = currency("USD", 2);

        assert_eq!(
            parser.parse_magnitude("-1.00", &usd),
            Err(MoneyParsingError::NegativeMagnitude)
        );
        assert_eq!(
            parser.parse_magnitude("1.234", &usd),
            Err(MoneyParsingError::ExcessFractionalDigits { allowed: 2 })
        );
    }

    #[test]
    fn parser_reports_overflow() {
        let parser = MoneyParser::default();
        let usd = currency("USD", 2);

        assert_eq!(
            parser.parse_magnitude("92233720368547758.08", &usd),
            Err(MoneyParsingError::Overflow)
        );
    }

    #[test]
    fn arithmetic_checks_currency_and_overflow() {
        let usd = currency("USD", 2);
        let eur = currency("EUR", 2);
        let maximum = Money::new(i64::MAX, usd.clone());

        assert_eq!(
            maximum.adding(&Money::new(1, usd.clone())),
            Err(MoneyError::Overflow)
        );
        assert!(matches!(
            Money::new(100, usd).adding(&Money::new(100, eur)),
            Err(MoneyError::CurrencyMismatch { .. })
        ));
        assert_eq!(
            Money::new(100, currency("USD", 2))
                .adding(&Money::new(50, currency("USD", 2)))
                .unwrap()
                .minor_units(),
            150
        );
    }

    #[test]
    fn rounding_uses_half_up_for_non_negative_dividends() {
        assert_eq!(nearest_minor_unit(5, 2).unwrap(), 3);
        assert_eq!(nearest_minor_unit(4, 2).unwrap(), 2);
        assert_eq!(nearest_minor_unit(3, 2).unwrap(), 2);
        assert_eq!(nearest_minor_unit(2, 3).unwrap(), 1);
        assert_eq!(nearest_minor_unit(1, 3).unwrap(), 0);
        assert_eq!(nearest_minor_unit(0, 7).unwrap(), 0);
    }

    #[test]
    fn rounding_rejects_invalid_inputs() {
        assert_eq!(
            nearest_minor_unit(1, 0),
            Err(MoneyRoundingError::InvalidDivisor)
        );
        assert_eq!(
            nearest_minor_unit(1, -2),
            Err(MoneyRoundingError::InvalidDivisor)
        );
        assert_eq!(
            nearest_minor_unit(-1, 2),
            Err(MoneyRoundingError::NegativeDividend)
        );
    }

    use proptest::prelude::*;

    proptest! {
        #[test]
        fn addition_is_commutative(
            left in -1_000_000_i64..1_000_000,
            right in -1_000_000_i64..1_000_000,
        ) {
            let usd = currency("USD", 2);
            let sum = Money::new(left, usd.clone())
                .adding(&Money::new(right, usd.clone()))
                .unwrap();
            let reverse = Money::new(right, usd.clone())
                .adding(&Money::new(left, usd))
                .unwrap();
            prop_assert_eq!(sum, reverse);
        }

        #[test]
        fn add_then_subtract_round_trips(left in any::<i64>(), right in any::<i64>()) {
            let usd = currency("USD", 2);
            if let Ok(sum) = Money::new(left, usd.clone()).adding(&Money::new(right, usd.clone())) {
                let restored = sum.subtracting(&Money::new(right, usd)).unwrap();
                prop_assert_eq!(restored.minor_units(), left);
            }
        }

        #[test]
        fn parsed_magnitude_matches_scaled_integer(
            whole in 0_i64..1_000_000,
            cents in 0_i64..100,
        ) {
            let usd = currency("USD", 2);
            let text = format!("{whole}.{cents:02}");
            let parsed = MoneyParser::default()
                .parse_magnitude(&text, &usd)
                .unwrap();
            prop_assert_eq!(parsed.minor_units(), whole * 100 + cents);
        }

        #[test]
        fn parser_rejects_excess_fraction(
            whole in 0_i64..1_000_000,
            fraction in 100_i64..1000,
        ) {
            let usd = currency("USD", 2);
            let text = format!("{whole}.{fraction}");
            prop_assert_eq!(
                MoneyParser::default().parse_magnitude(&text, &usd),
                Err(MoneyParsingError::ExcessFractionalDigits { allowed: 2 })
            );
        }
    }
}
