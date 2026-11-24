//! Exchange-rate use cases and historical conversion.

use core_domain::{
    Currency, ExchangeRate, ExchangeRateId, ExchangeRateQuoteParser, ExchangeRateRepository,
    ExchangeRateSource, LedgerSettingsRepository, Money, latest_rate,
};
use jiff::Timestamp;

use crate::error::ApplicationError;
use crate::ports::{Clock, IdSource};

/// Converts money at a historical instant using the latest applicable rate.
pub struct HistoricalMoneyConverter<'a> {
    rates: &'a [ExchangeRate],
    reporting_currency: Currency,
}

impl<'a> HistoricalMoneyConverter<'a> {
    /// Creates the converter.
    #[must_use]
    pub fn new(rates: &'a [ExchangeRate], reporting_currency: Currency) -> Self {
        Self {
            rates,
            reporting_currency,
        }
    }

    /// Converts money to the reporting currency at the instant.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::MissingExchangeRate`] when no rate applies
    /// and [`ApplicationError::Conversion`] when conversion overflows.
    pub fn convert(&self, money: &Money, at: Timestamp) -> Result<Money, ApplicationError> {
        if money.currency() == &self.reporting_currency {
            return Ok(money.clone());
        }
        let rate = latest_rate(self.rates, money.currency(), &self.reporting_currency, at)
            .ok_or_else(|| ApplicationError::MissingExchangeRate {
                source: money.currency().clone(),
                reporting: self.reporting_currency.clone(),
            })?;
        rate.convert(money).map_err(ApplicationError::Conversion)
    }
}

/// The request to set a manual rate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetManualExchangeRateRequest {
    /// The source currency.
    pub source_currency: Currency,
    /// The reporting currency.
    pub reporting_currency: Currency,
    /// The positive decimal quote.
    pub quote_text: String,
    /// The day the rate takes effect; it is normalized to the ledger day start.
    pub effective_at: Timestamp,
}

/// Sets a manual rate and supersedes the manual revision for the same day.
pub struct SetManualExchangeRateUseCase<'a> {
    settings: &'a dyn LedgerSettingsRepository,
    repository: &'a dyn ExchangeRateRepository,
    parser: ExchangeRateQuoteParser,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> SetManualExchangeRateUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        settings: &'a dyn LedgerSettingsRepository,
        repository: &'a dyn ExchangeRateRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            settings,
            repository,
            parser: ExchangeRateQuoteParser::default(),
            clock,
            ids,
        }
    }

    /// Parses, stores, and returns the new manual rate.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the ledger is not initialized, the
    /// quote or rate is invalid, or the repository fails.
    pub fn execute(
        &self,
        request: &SetManualExchangeRateRequest,
    ) -> Result<ExchangeRate, ApplicationError> {
        let settings = self
            .settings
            .settings()?
            .ok_or(ApplicationError::LedgerNotInitialized)?;
        let calendar = settings.calendar().clone();
        let effective_at = calendar
            .day_interval(request.effective_at)
            .map_err(ApplicationError::Calendar)?
            .start();
        let quote = self
            .parser
            .parse(&request.quote_text)
            .map_err(ApplicationError::Quote)?;
        let now = self.clock.now();
        for existing in self.repository.exchange_rates(true)? {
            if existing.source() == ExchangeRateSource::Manual
                && !existing.is_superseded()
                && existing.source_currency() == &request.source_currency
                && existing.reporting_currency() == &request.reporting_currency
                && existing.effective_at() == effective_at
            {
                let superseded = existing
                    .superseding(now)
                    .map_err(ApplicationError::ExchangeRate)?;
                self.repository.save_exchange_rate(&superseded)?;
            }
        }
        let rate = ExchangeRate::new(
            ExchangeRateId::from_uuid(self.ids.next_uuid()),
            request.source_currency.clone(),
            request.reporting_currency.clone(),
            quote,
            effective_at,
            ExchangeRateSource::Manual,
            None,
            now,
            now,
        )
        .map_err(ApplicationError::ExchangeRate)?;
        self.repository.save_exchange_rate(&rate)?;
        Ok(rate)
    }
}

/// Lists exchange-rate revisions.
pub struct ListExchangeRatesUseCase<'a> {
    repository: &'a dyn ExchangeRateRepository,
}

impl<'a> ListExchangeRatesUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn ExchangeRateRepository) -> Self {
        Self { repository }
    }

    /// Returns rates sorted by effective date descending.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the repository fails.
    pub fn execute(&self, include_superseded: bool) -> Result<Vec<ExchangeRate>, ApplicationError> {
        let mut rates = self.repository.exchange_rates(include_superseded)?;
        rates.sort_by(|left, right| {
            right
                .effective_at()
                .cmp(&left.effective_at())
                .then_with(|| right.id().to_string().cmp(&left.id().to_string()))
        });
        Ok(rates)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixedClock, SequenceIds};
    use core_domain::{CurrencyScalePolicy, LedgerCalendar, LedgerSettings, LedgerSettingsId};
    use core_testkit::InMemoryLedger;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn currency(code: &str) -> Currency {
        CurrencyScalePolicy::v1().currency(code).unwrap()
    }

    fn settings() -> LedgerSettings {
        LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(99)),
            currency("USD"),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    fn ledger_with_settings() -> InMemoryLedger {
        let ledger = InMemoryLedger::new();
        ledger.seed_settings(settings());
        ledger
    }

    fn set_manual(
        ledger: &InMemoryLedger,
        clock: &FixedClock,
        ids: &SequenceIds,
        quote: &str,
        effective_at: i64,
    ) -> ExchangeRate {
        SetManualExchangeRateUseCase::new(ledger, ledger, clock, ids)
            .execute(&SetManualExchangeRateRequest {
                source_currency: currency("CNY"),
                reporting_currency: currency("USD"),
                quote_text: quote.to_string(),
                effective_at: timestamp(effective_at),
            })
            .unwrap()
    }

    #[test]
    fn setting_a_manual_rate_supersedes_the_same_day_revision() {
        let ledger = ledger_with_settings();
        let clock = FixedClock::new(timestamp(1_000));
        let ids = SequenceIds::new();
        let first = set_manual(&ledger, &clock, &ids, "0.14", 86_400);
        let second = set_manual(&ledger, &clock, &ids, "0.15", 86_500);

        assert_eq!(second.effective_at(), timestamp(86_400));
        let rates = ledger.exchange_rates(true).unwrap();
        assert_eq!(rates.len(), 2);
        assert!(
            rates
                .iter()
                .find(|rate| rate.id() == first.id())
                .unwrap()
                .is_superseded()
        );
        assert!(!second.is_superseded());
    }

    #[test]
    fn converter_uses_the_latest_rate_at_the_instant() {
        let ledger = ledger_with_settings();
        let clock = FixedClock::new(timestamp(1_000));
        let ids = SequenceIds::new();
        set_manual(&ledger, &clock, &ids, "0.14", 86_400);
        let latest = set_manual(&ledger, &clock, &ids, "0.15", 172_800);
        let rates = ledger.exchange_rates(true).unwrap();
        let converter = HistoricalMoneyConverter::new(&rates, currency("USD"));

        assert_eq!(
            converter
                .convert(&Money::new(10_000, currency("CNY")), timestamp(90_000))
                .unwrap(),
            Money::new(1_400, currency("USD"))
        );
        assert_eq!(
            converter
                .convert(&Money::new(10_000, currency("CNY")), timestamp(180_000))
                .unwrap(),
            Money::new(1_500, currency("USD"))
        );
        assert_eq!(latest.quote(), core_domain::ExactRatio::new(3, 20).unwrap());
    }

    #[test]
    fn converter_reports_missing_rates() {
        let ledger = InMemoryLedger::new();
        let rates = ledger.exchange_rates(true).unwrap();
        let converter = HistoricalMoneyConverter::new(&rates, currency("USD"));
        assert_eq!(
            converter
                .convert(&Money::new(100, currency("EUR")), timestamp(1_000))
                .unwrap_err(),
            ApplicationError::MissingExchangeRate {
                source: currency("EUR"),
                reporting: currency("USD"),
            }
        );
    }

    #[test]
    fn list_sorts_by_effective_date() {
        let ledger = ledger_with_settings();
        let clock = FixedClock::new(timestamp(1_000));
        let ids = SequenceIds::new();
        let earlier = set_manual(&ledger, &clock, &ids, "0.14", 86_400);
        let later = set_manual(&ledger, &clock, &ids, "0.15", 172_800);

        let rates = ListExchangeRatesUseCase::new(&ledger)
            .execute(true)
            .unwrap();
        assert_eq!(rates[0].id(), later.id());
        assert_eq!(rates[1].id(), earlier.id());
    }
}
