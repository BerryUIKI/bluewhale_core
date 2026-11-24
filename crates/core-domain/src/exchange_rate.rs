//! Exchange rates: exact quotes, conversion, and selection.

use std::cmp::Ordering;
use std::fmt;

use jiff::Timestamp;

use crate::currency::Currency;
use crate::ids::ExchangeRateId;
use crate::money::Money;

/// Where an exchange-rate revision came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeRateSource {
    /// Entered by the user.
    Manual,
    /// Imported from the European Central Bank reference table.
    EuropeanCentralBank,
}

impl ExchangeRateSource {
    fn selection_priority(self) -> u8 {
        match self {
            Self::Manual => 1,
            Self::EuropeanCentralBank => 0,
        }
    }
}

/// Errors raised while constructing or dividing exact ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactRatioError {
    /// The numerator is not positive.
    NonPositiveNumerator,
    /// The denominator is not positive.
    NonPositiveDenominator,
    /// The reduced ratio does not fit in an `i64`.
    Overflow,
}

impl fmt::Display for ExactRatioError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonPositiveNumerator => formatter.write_str("non-positive numerator"),
            Self::NonPositiveDenominator => formatter.write_str("non-positive denominator"),
            Self::Overflow => formatter.write_str("ratio overflow"),
        }
    }
}

impl std::error::Error for ExactRatioError {}

/// A positive rational number in reduced form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExactRatio {
    numerator: i64,
    denominator: i64,
}

impl ExactRatio {
    /// Creates a reduced ratio.
    ///
    /// # Errors
    ///
    /// Returns an error when either term is not positive.
    pub fn new(numerator: i64, denominator: i64) -> Result<Self, ExactRatioError> {
        if numerator <= 0 {
            return Err(ExactRatioError::NonPositiveNumerator);
        }
        if denominator <= 0 {
            return Err(ExactRatioError::NonPositiveDenominator);
        }
        let divisor = greatest_common_divisor(numerator.unsigned_abs(), denominator.unsigned_abs());
        let divisor = i64::try_from(divisor).map_err(|_| ExactRatioError::Overflow)?;
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    /// Returns the numerator.
    #[must_use]
    pub fn numerator(&self) -> i64 {
        self.numerator
    }

    /// Returns the denominator.
    #[must_use]
    pub fn denominator(&self) -> i64 {
        self.denominator
    }

    /// Divides this ratio by another, cross-cancelling first.
    ///
    /// # Errors
    ///
    /// Returns [`ExactRatioError::Overflow`] when the result does not fit in an
    /// `i64`.
    pub fn divided_by(&self, other: &Self) -> Result<Self, ExactRatioError> {
        let mut left_numerator = u128::from(self.numerator.unsigned_abs());
        let mut left_denominator = u128::from(self.denominator.unsigned_abs());
        let mut right_numerator = u128::from(other.numerator.unsigned_abs());
        let mut right_denominator = u128::from(other.denominator.unsigned_abs());

        let numerator_divisor = greatest_common_divisor_u128(left_numerator, right_numerator);
        left_numerator /= numerator_divisor;
        right_numerator /= numerator_divisor;

        let denominator_divisor = greatest_common_divisor_u128(right_denominator, left_denominator);
        right_denominator /= denominator_divisor;
        left_denominator /= denominator_divisor;

        let result_numerator = left_numerator
            .checked_mul(right_denominator)
            .ok_or(ExactRatioError::Overflow)?;
        let result_denominator = left_denominator
            .checked_mul(right_numerator)
            .ok_or(ExactRatioError::Overflow)?;
        let maximum = u128::from(i64::MAX as u64);
        if result_numerator == 0
            || result_denominator == 0
            || result_numerator > maximum
            || result_denominator > maximum
        {
            return Err(ExactRatioError::Overflow);
        }
        ExactRatio::new(
            i64::try_from(result_numerator).map_err(|_| ExactRatioError::Overflow)?,
            i64::try_from(result_denominator).map_err(|_| ExactRatioError::Overflow)?,
        )
    }
}

fn greatest_common_divisor(mut first: u64, mut second: u64) -> u64 {
    while second != 0 {
        (first, second) = (second, first % second);
    }
    first
}

fn greatest_common_divisor_u128(mut first: u128, mut second: u128) -> u128 {
    while second != 0 {
        (first, second) = (second, first % second);
    }
    first
}

/// Errors raised while parsing an exchange-rate quote.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeRateQuoteParsingError {
    /// The text is not a plain positive decimal.
    InvalidFormat(String),
    /// The quote is zero.
    NonPositive,
    /// The quote has too many fractional digits.
    ExcessFractionalDigits {
        /// The maximum number of fractional digits.
        allowed: usize,
    },
    /// The quote does not fit in an `i64` ratio.
    Overflow,
}

impl fmt::Display for ExchangeRateQuoteParsingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidFormat(text) => write!(formatter, "invalid exchange rate: {text}"),
            Self::NonPositive => formatter.write_str("exchange rate must be positive"),
            Self::ExcessFractionalDigits { allowed } => {
                write!(
                    formatter,
                    "exchange rate has more than {allowed} fractional digits"
                )
            }
            Self::Overflow => formatter.write_str("exchange rate overflow"),
        }
    }
}

impl std::error::Error for ExchangeRateQuoteParsingError {}

/// Parses positive decimal quotes into exact ratios.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExchangeRateQuoteParser {
    maximum_fractional_digits: usize,
}

impl Default for ExchangeRateQuoteParser {
    fn default() -> Self {
        Self::new(9)
    }
}

impl ExchangeRateQuoteParser {
    /// Creates a parser with a maximum number of fractional digits.
    #[must_use]
    pub fn new(maximum_fractional_digits: usize) -> Self {
        Self {
            maximum_fractional_digits,
        }
    }

    /// Parses a quote.
    ///
    /// # Errors
    ///
    /// Returns a [`ExchangeRateQuoteParsingError`] for invalid syntax, a zero
    /// quote, excess fractional digits, or overflow.
    pub fn parse(&self, text: &str) -> Result<ExactRatio, ExchangeRateQuoteParsingError> {
        let value = text.trim();
        if value.is_empty() || value.starts_with('-') || value.starts_with('+') {
            return Err(ExchangeRateQuoteParsingError::InvalidFormat(
                text.to_string(),
            ));
        }

        let mut components = value.split('.');
        let whole = components.next().unwrap_or_default();
        let fraction = components.next();
        if components.next().is_some()
            || whole.is_empty()
            || !whole.bytes().all(|byte| byte.is_ascii_digit())
        {
            return Err(ExchangeRateQuoteParsingError::InvalidFormat(
                text.to_string(),
            ));
        }
        let fraction = fraction.unwrap_or_default();
        if fraction.is_empty() && value.contains('.') {
            return Err(ExchangeRateQuoteParsingError::InvalidFormat(
                text.to_string(),
            ));
        }
        if !fraction.bytes().all(|byte| byte.is_ascii_digit()) {
            return Err(ExchangeRateQuoteParsingError::InvalidFormat(
                text.to_string(),
            ));
        }
        if fraction.len() > self.maximum_fractional_digits {
            return Err(ExchangeRateQuoteParsingError::ExcessFractionalDigits {
                allowed: self.maximum_fractional_digits,
            });
        }

        let mut numerator: i64 = 0;
        append_digits(whole, &mut numerator)?;
        append_digits(fraction, &mut numerator)?;
        if numerator <= 0 {
            return Err(ExchangeRateQuoteParsingError::NonPositive);
        }

        let mut denominator: i64 = 1;
        for _ in 0..fraction.len() {
            denominator = denominator
                .checked_mul(10)
                .ok_or(ExchangeRateQuoteParsingError::Overflow)?;
        }
        ExactRatio::new(numerator, denominator).map_err(|_| ExchangeRateQuoteParsingError::Overflow)
    }
}

fn append_digits(value: &str, result: &mut i64) -> Result<(), ExchangeRateQuoteParsingError> {
    for byte in value.bytes() {
        let digit = i64::from(byte - b'0');
        *result = result
            .checked_mul(10)
            .and_then(|current| current.checked_add(digit))
            .ok_or(ExchangeRateQuoteParsingError::Overflow)?;
    }
    Ok(())
}

/// Errors raised while validating an exchange-rate revision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeRateValidationError {
    /// The source and reporting currencies are the same.
    IdenticalCurrencies(Currency),
    /// The update instant precedes creation.
    UpdatedBeforeCreation,
    /// The supersession instant precedes creation.
    SupersededBeforeCreation,
    /// The scale-adjusted ratio cannot be represented.
    UnrepresentableMinorUnitRatio,
}

impl fmt::Display for ExchangeRateValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IdenticalCurrencies(currency) => {
                write!(formatter, "identical currencies: {}", currency.code())
            }
            Self::UpdatedBeforeCreation => formatter.write_str("updated before creation"),
            Self::SupersededBeforeCreation => formatter.write_str("superseded before creation"),
            Self::UnrepresentableMinorUnitRatio => {
                formatter.write_str("unrepresentable minor-unit ratio")
            }
        }
    }
}

impl std::error::Error for ExchangeRateValidationError {}

/// Errors raised while converting money with an exchange rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExchangeRateConversionError {
    /// The money uses a different source currency.
    SourceCurrencyMismatch {
        /// The rate's source currency.
        expected: Currency,
        /// The money's currency.
        actual: Currency,
    },
    /// The converted amount does not fit in an `i64`.
    Overflow,
}

impl fmt::Display for ExchangeRateConversionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceCurrencyMismatch { expected, actual } => write!(
                formatter,
                "source currency mismatch: expected {}, got {}",
                expected.code(),
                actual.code()
            ),
            Self::Overflow => formatter.write_str("conversion overflow"),
        }
    }
}

impl std::error::Error for ExchangeRateConversionError {}

/// One exchange-rate revision from a source currency to a reporting currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExchangeRate {
    id: ExchangeRateId,
    source_currency: Currency,
    reporting_currency: Currency,
    quote: ExactRatio,
    effective_at: Timestamp,
    source: ExchangeRateSource,
    superseded_at: Option<Timestamp>,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl ExchangeRate {
    /// Creates a validated exchange-rate revision.
    ///
    /// # Errors
    ///
    /// Returns a [`ExchangeRateValidationError`] for identical currencies,
    /// timestamps out of order, or an unrepresentable scale-adjusted ratio.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: ExchangeRateId,
        source_currency: Currency,
        reporting_currency: Currency,
        quote: ExactRatio,
        effective_at: Timestamp,
        source: ExchangeRateSource,
        superseded_at: Option<Timestamp>,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, ExchangeRateValidationError> {
        if source_currency == reporting_currency {
            return Err(ExchangeRateValidationError::IdenticalCurrencies(
                source_currency,
            ));
        }
        if updated_at < created_at {
            return Err(ExchangeRateValidationError::UpdatedBeforeCreation);
        }
        if superseded_at.is_some_and(|superseded| superseded < created_at) {
            return Err(ExchangeRateValidationError::SupersededBeforeCreation);
        }

        let rate = Self {
            id,
            source_currency,
            reporting_currency,
            quote,
            effective_at,
            source,
            superseded_at,
            created_at,
            updated_at,
        };
        rate.minor_unit_ratio()?;
        Ok(rate)
    }

    /// Returns the rate identifier.
    #[must_use]
    pub fn id(&self) -> ExchangeRateId {
        self.id
    }

    /// Returns the source currency.
    #[must_use]
    pub fn source_currency(&self) -> &Currency {
        &self.source_currency
    }

    /// Returns the reporting currency.
    #[must_use]
    pub fn reporting_currency(&self) -> &Currency {
        &self.reporting_currency
    }

    /// Returns the exact quote.
    #[must_use]
    pub fn quote(&self) -> ExactRatio {
        self.quote
    }

    /// Returns the instant the rate takes effect.
    #[must_use]
    pub fn effective_at(&self) -> Timestamp {
        self.effective_at
    }

    /// Returns the rate source.
    #[must_use]
    pub fn source(&self) -> ExchangeRateSource {
        self.source
    }

    /// Returns the supersession instant, when superseded.
    #[must_use]
    pub fn superseded_at(&self) -> Option<Timestamp> {
        self.superseded_at
    }

    /// Returns whether the rate is superseded.
    #[must_use]
    pub fn is_superseded(&self) -> bool {
        self.superseded_at.is_some()
    }

    /// Returns the creation instant.
    #[must_use]
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }

    /// Returns the last update instant.
    #[must_use]
    pub fn updated_at(&self) -> Timestamp {
        self.updated_at
    }

    /// Returns a superseded copy.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the unchanged fields are invalid.
    pub fn superseding(&self, at: Timestamp) -> Result<Self, ExchangeRateValidationError> {
        Self::new(
            self.id,
            self.source_currency.clone(),
            self.reporting_currency.clone(),
            self.quote,
            self.effective_at,
            self.source,
            Some(at),
            self.created_at,
            at,
        )
    }

    /// Converts money from the source currency to the reporting currency,
    /// rounding ties away from zero.
    ///
    /// # Errors
    ///
    /// Returns [`ExchangeRateConversionError::SourceCurrencyMismatch`] for the
    /// wrong currency and [`ExchangeRateConversionError::Overflow`] when the
    /// result does not fit in an `i64`.
    pub fn convert(&self, money: &Money) -> Result<Money, ExchangeRateConversionError> {
        if money.currency() != &self.source_currency {
            return Err(ExchangeRateConversionError::SourceCurrencyMismatch {
                expected: self.source_currency.clone(),
                actual: money.currency().clone(),
            });
        }
        let ratio = self
            .minor_unit_ratio()
            .map_err(|_| ExchangeRateConversionError::Overflow)?;
        let magnitude = u128::from(money.minor_units().unsigned_abs());
        let numerator = u128::from(ratio.numerator.unsigned_abs());
        let denominator = u128::from(ratio.denominator.unsigned_abs());
        let product = magnitude
            .checked_mul(numerator)
            .ok_or(ExchangeRateConversionError::Overflow)?;
        let quotient = product / denominator;
        let remainder = product % denominator;
        let threshold = denominator / 2 + denominator % 2;
        let rounded = if remainder >= threshold {
            quotient
                .checked_add(1)
                .ok_or(ExchangeRateConversionError::Overflow)?
        } else {
            quotient
        };

        let is_negative = money.minor_units() < 0;
        let maximum = if is_negative {
            u128::from(i64::MAX as u64) + 1
        } else {
            u128::from(i64::MAX as u64)
        };
        if rounded > maximum {
            return Err(ExchangeRateConversionError::Overflow);
        }
        let minor_units = if is_negative {
            if rounded == u128::from(i64::MAX as u64) + 1 {
                i64::MIN
            } else {
                -i64::try_from(rounded).map_err(|_| ExchangeRateConversionError::Overflow)?
            }
        } else {
            i64::try_from(rounded).map_err(|_| ExchangeRateConversionError::Overflow)?
        };
        Ok(Money::new(minor_units, self.reporting_currency.clone()))
    }

    fn minor_unit_ratio(&self) -> Result<ExactRatio, ExchangeRateValidationError> {
        let mut numerator = u128::from(self.quote.numerator.unsigned_abs());
        let mut denominator = u128::from(self.quote.denominator.unsigned_abs());
        let mut reporting_factor = power_of_ten(self.reporting_currency.scale());
        let mut source_factor = power_of_ten(self.source_currency.scale());

        cancel(&mut numerator, &mut denominator);
        cancel(&mut numerator, &mut source_factor);
        cancel(&mut reporting_factor, &mut denominator);
        cancel(&mut reporting_factor, &mut source_factor);

        let minor_numerator = numerator
            .checked_mul(reporting_factor)
            .ok_or(ExchangeRateValidationError::UnrepresentableMinorUnitRatio)?;
        let minor_denominator = denominator
            .checked_mul(source_factor)
            .ok_or(ExchangeRateValidationError::UnrepresentableMinorUnitRatio)?;
        let maximum = u128::from(i64::MAX as u64);
        if minor_numerator == 0
            || minor_denominator == 0
            || minor_numerator > maximum
            || minor_denominator > maximum
        {
            return Err(ExchangeRateValidationError::UnrepresentableMinorUnitRatio);
        }
        ExactRatio::new(
            i64::try_from(minor_numerator)
                .map_err(|_| ExchangeRateValidationError::UnrepresentableMinorUnitRatio)?,
            i64::try_from(minor_denominator)
                .map_err(|_| ExchangeRateValidationError::UnrepresentableMinorUnitRatio)?,
        )
        .map_err(|_| ExchangeRateValidationError::UnrepresentableMinorUnitRatio)
    }
}

fn cancel(left: &mut u128, right: &mut u128) {
    let divisor = greatest_common_divisor_u128(*left, *right);
    *left /= divisor;
    *right /= divisor;
}

fn power_of_ten(exponent: u8) -> u128 {
    let mut result: u128 = 1;
    for _ in 0..exponent {
        result *= 10;
    }
    result
}

/// Selects the applicable rate: latest effective date, then manual over
/// provider, then latest update, then greatest UUID string.
#[must_use]
pub fn latest_rate<'a>(
    rates: &'a [ExchangeRate],
    source_currency: &Currency,
    reporting_currency: &Currency,
    at: Timestamp,
) -> Option<&'a ExchangeRate> {
    rates
        .iter()
        .filter(|rate| {
            !rate.is_superseded()
                && &rate.source_currency == source_currency
                && &rate.reporting_currency == reporting_currency
                && rate.effective_at <= at
        })
        .max_by(|left, right| selection_order(left, right))
}

fn selection_order(left: &ExchangeRate, right: &ExchangeRate) -> Ordering {
    left.effective_at
        .cmp(&right.effective_at)
        .then_with(|| {
            left.source
                .selection_priority()
                .cmp(&right.source.selection_priority())
        })
        .then_with(|| left.updated_at.cmp(&right.updated_at))
        .then_with(|| left.id.to_string().cmp(&right.id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use crate::ids::ExchangeRateId;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn currency(code: &str) -> Currency {
        CurrencyScalePolicy::v1().currency(code).unwrap()
    }

    fn parser() -> ExchangeRateQuoteParser {
        ExchangeRateQuoteParser::default()
    }

    fn make_rate(
        id: u128,
        source: &str,
        reporting: &str,
        quote: &str,
        effective_at: i64,
        rate_source: ExchangeRateSource,
    ) -> ExchangeRate {
        ExchangeRate::new(
            ExchangeRateId::from_uuid(Uuid::from_u128(id)),
            currency(source),
            currency(reporting),
            parser().parse(quote).unwrap(),
            timestamp(effective_at),
            rate_source,
            None,
            timestamp(effective_at),
            timestamp(effective_at),
        )
        .unwrap()
    }

    #[test]
    fn quote_parser_reduces_exact_decimal_values() {
        assert_eq!(
            parser().parse("0.147761").unwrap(),
            ExactRatio::new(147_761, 1_000_000).unwrap()
        );
        assert_eq!(
            parser().parse("1.2500").unwrap(),
            ExactRatio::new(5, 4).unwrap()
        );
        assert_eq!(
            parser().parse("3150").unwrap(),
            ExactRatio::new(3_150, 1).unwrap()
        );
    }

    #[test]
    fn quote_parser_rejects_invalid_values() {
        assert_eq!(
            parser().parse("0"),
            Err(ExchangeRateQuoteParsingError::NonPositive)
        );
        for value in ["-1", "+1", "1,25", "1e2", ".25", "1."] {
            assert!(parser().parse(value).is_err(), "{value} should be rejected");
        }
        assert_eq!(
            parser().parse("1.1234567890"),
            Err(ExchangeRateQuoteParsingError::ExcessFractionalDigits { allowed: 9 })
        );
    }

    #[test]
    fn conversion_accounts_for_scales_and_rounds_ties_away_from_zero() {
        let rate = make_rate(1, "JPY", "USD", "0.005", 1_000, ExchangeRateSource::Manual);

        assert_eq!(
            rate.convert(&Money::new(1, currency("JPY"))).unwrap(),
            Money::new(1, currency("USD"))
        );
        assert_eq!(
            rate.convert(&Money::new(-1, currency("JPY"))).unwrap(),
            Money::new(-1, currency("USD"))
        );
        assert_eq!(
            rate.convert(&Money::new(3, currency("JPY"))).unwrap(),
            Money::new(2, currency("USD"))
        );
    }

    #[test]
    fn conversion_handles_three_decimal_source_and_zero_decimal_target() {
        let rate = make_rate(2, "KWD", "JPY", "500", 1_000, ExchangeRateSource::Manual);
        assert_eq!(
            rate.convert(&Money::new(1_234, currency("KWD"))).unwrap(),
            Money::new(617, currency("JPY"))
        );
    }

    #[test]
    fn conversion_supports_full_width_and_reports_overflow() {
        let quote = ExactRatio::new(i64::MAX, i64::MAX - 1).unwrap();
        let rate = ExchangeRate::new(
            ExchangeRateId::from_uuid(Uuid::from_u128(3)),
            currency("CNY"),
            currency("USD"),
            quote,
            timestamp(1_000),
            ExchangeRateSource::Manual,
            None,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();

        assert_eq!(
            rate.convert(&Money::new(i64::MAX - 1, currency("CNY")))
                .unwrap(),
            Money::new(i64::MAX, currency("USD"))
        );
        assert_eq!(
            rate.convert(&Money::new(i64::MAX, currency("CNY"))),
            Err(ExchangeRateConversionError::Overflow)
        );
    }

    #[test]
    fn conversion_rejects_wrong_source_currency() {
        let rate = make_rate(4, "CNY", "USD", "0.15", 1_000, ExchangeRateSource::Manual);
        assert_eq!(
            rate.convert(&Money::new(100, currency("EUR"))),
            Err(ExchangeRateConversionError::SourceCurrencyMismatch {
                expected: currency("CNY"),
                actual: currency("EUR"),
            })
        );
    }

    #[test]
    fn selection_uses_latest_day_then_manual_precedence() {
        let day_one = 86_400;
        let day_two = 172_800;
        let imported = make_rate(
            2,
            "CNY",
            "USD",
            "0.14",
            day_two,
            ExchangeRateSource::EuropeanCentralBank,
        );
        let manual = make_rate(1, "CNY", "USD", "0.15", day_two, ExchangeRateSource::Manual);
        let prior = make_rate(3, "CNY", "USD", "0.13", day_one, ExchangeRateSource::Manual);
        let rates = vec![prior.clone(), imported, manual.clone()];

        assert_eq!(
            latest_rate(
                &rates,
                &currency("CNY"),
                &currency("USD"),
                timestamp(day_two)
            ),
            Some(&manual)
        );
        assert_eq!(
            latest_rate(
                &rates,
                &currency("CNY"),
                &currency("USD"),
                timestamp(day_one)
            ),
            Some(&prior)
        );
    }

    #[test]
    fn selection_excludes_superseded_rates() {
        let rate = make_rate(5, "CNY", "USD", "0.15", 86_400, ExchangeRateSource::Manual);
        let superseded = rate.superseding(timestamp(86_401)).unwrap();

        assert_eq!(
            latest_rate(
                &[superseded],
                &currency("CNY"),
                &currency("USD"),
                timestamp(86_500)
            ),
            None
        );
    }

    #[test]
    fn exact_ratio_division_derives_provider_cross_rate() {
        let usd_per_eur = parser().parse("1.2").unwrap();
        let cny_per_eur = parser().parse("7.8").unwrap();

        assert_eq!(
            cny_per_eur.divided_by(&usd_per_eur).unwrap(),
            ExactRatio::new(13, 2).unwrap()
        );
    }

    #[test]
    fn rate_validation_rejects_bad_timestamps_and_identical_currencies() {
        let quote = parser().parse("1.2").unwrap();
        assert_eq!(
            ExchangeRate::new(
                ExchangeRateId::from_uuid(Uuid::from_u128(6)),
                currency("USD"),
                currency("USD"),
                quote,
                timestamp(1_000),
                ExchangeRateSource::Manual,
                None,
                timestamp(1_000),
                timestamp(1_000),
            )
            .unwrap_err(),
            ExchangeRateValidationError::IdenticalCurrencies(currency("USD"))
        );

        assert_eq!(
            ExchangeRate::new(
                ExchangeRateId::from_uuid(Uuid::from_u128(7)),
                currency("CNY"),
                currency("USD"),
                quote,
                timestamp(1_000),
                ExchangeRateSource::Manual,
                None,
                timestamp(2_000),
                timestamp(1_000),
            )
            .unwrap_err(),
            ExchangeRateValidationError::UpdatedBeforeCreation
        );
    }
}
