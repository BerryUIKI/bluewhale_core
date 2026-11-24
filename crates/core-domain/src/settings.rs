//! Ledger settings: reporting currency, calendar, and time zone.

use jiff::Timestamp;

use crate::calendar::LedgerCalendar;
use crate::currency::Currency;
use crate::ids::LedgerSettingsId;

/// The calendar used for ledger day, month, and year boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum LedgerCalendarIdentifier {
    /// The Gregorian calendar.
    #[default]
    Gregorian,
}

/// The settings of one ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerSettings {
    id: LedgerSettingsId,
    currency: Currency,
    currency_policy_version: u32,
    calendar_identifier: LedgerCalendarIdentifier,
    calendar: LedgerCalendar,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl LedgerSettings {
    /// Creates validated ledger settings.
    #[must_use]
    pub fn new(
        id: LedgerSettingsId,
        currency: Currency,
        currency_policy_version: u32,
        calendar: LedgerCalendar,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Self {
        Self {
            id,
            currency,
            currency_policy_version,
            calendar_identifier: LedgerCalendarIdentifier::Gregorian,
            calendar,
            created_at,
            updated_at,
        }
    }

    /// Returns the settings identifier.
    #[must_use]
    pub fn id(&self) -> LedgerSettingsId {
        self.id
    }

    /// Returns the reporting currency.
    #[must_use]
    pub fn currency(&self) -> &Currency {
        &self.currency
    }

    /// Returns the currency-scale policy version.
    #[must_use]
    pub fn currency_policy_version(&self) -> u32 {
        self.currency_policy_version
    }

    /// Returns the calendar identifier.
    #[must_use]
    pub fn calendar_identifier(&self) -> LedgerCalendarIdentifier {
        self.calendar_identifier
    }

    /// Returns the ledger calendar.
    #[must_use]
    pub fn calendar(&self) -> &LedgerCalendar {
        &self.calendar
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

    /// Returns settings with a new reporting currency and policy version.
    #[must_use]
    pub fn changing_reporting_currency(
        &self,
        currency: Currency,
        currency_policy_version: u32,
        at: Timestamp,
    ) -> Self {
        Self {
            id: self.id,
            currency,
            currency_policy_version,
            calendar_identifier: self.calendar_identifier,
            calendar: self.calendar.clone(),
            created_at: self.created_at,
            updated_at: at,
        }
    }

    /// Returns settings with a new ledger time zone.
    ///
    /// The caller is responsible for obtaining the user's explicit
    /// confirmation. Changing the time zone changes how instants are grouped
    /// into ledger days and months; it never rewrites stored instants.
    #[must_use]
    pub fn changing_time_zone(&self, calendar: LedgerCalendar, at: Timestamp) -> Self {
        Self {
            id: self.id,
            currency: self.currency.clone(),
            currency_policy_version: self.currency_policy_version,
            calendar_identifier: self.calendar_identifier,
            calendar,
            created_at: self.created_at,
            updated_at: at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    #[test]
    fn settings_expose_currency_calendar_and_timestamps() {
        let policy = CurrencyScalePolicy::v1();
        let currency = policy.currency("USD").unwrap();
        let calendar = LedgerCalendar::new("Asia/Singapore").unwrap();
        let settings = LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(1)),
            currency.clone(),
            1,
            calendar.clone(),
            timestamp(1_000),
            timestamp(1_000),
        );

        assert_eq!(settings.currency(), &currency);
        assert_eq!(settings.currency_policy_version(), 1);
        assert_eq!(settings.calendar(), &calendar);
        assert_eq!(
            settings.calendar_identifier(),
            LedgerCalendarIdentifier::Gregorian
        );
        assert_eq!(settings.created_at(), timestamp(1_000));
    }

    #[test]
    fn time_zone_change_preserves_identity_and_currency() {
        let policy = CurrencyScalePolicy::v1();
        let settings = LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(3)),
            policy.currency("USD").unwrap(),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        );

        let changed = settings.changing_time_zone(
            LedgerCalendar::new("Europe/Berlin").unwrap(),
            timestamp(2_000),
        );

        assert_eq!(changed.id(), settings.id());
        assert_eq!(changed.currency(), settings.currency());
        assert_eq!(changed.calendar().identifier(), "Europe/Berlin");
        assert_eq!(changed.created_at(), settings.created_at());
        assert_eq!(changed.updated_at(), timestamp(2_000));
    }

    #[test]
    fn reporting_currency_change_preserves_identity_and_calendar() {
        let policy = CurrencyScalePolicy::v1();
        let calendar = LedgerCalendar::new("Europe/Berlin").unwrap();
        let settings = LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(2)),
            policy.currency("EUR").unwrap(),
            1,
            calendar.clone(),
            timestamp(1_000),
            timestamp(1_000),
        );

        let changed = settings.changing_reporting_currency(
            policy.currency("USD").unwrap(),
            1,
            timestamp(2_000),
        );

        assert_eq!(changed.id(), settings.id());
        assert_eq!(changed.calendar(), &calendar);
        assert_eq!(changed.created_at(), settings.created_at());
        assert_eq!(changed.updated_at(), timestamp(2_000));
        assert_eq!(changed.currency().code(), "USD");
    }
}
