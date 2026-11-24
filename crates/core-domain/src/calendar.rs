//! Ledger-time-zone calendar intervals.
//!
//! All day, month, and year boundaries use the ledger's IANA time zone and the
//! Gregorian calendar. Intervals are half-open: `[start, end_exclusive)`.

use std::fmt;

use jiff::Timestamp;
use jiff::civil::Date;
use jiff::tz::TimeZone;

/// Validates a ledger time zone and computes its calendar intervals.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerCalendar {
    identifier: String,
    time_zone: TimeZone,
}

/// Errors raised while validating or using a ledger calendar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LedgerCalendarError {
    /// The identifier is not a known IANA time zone.
    InvalidTimeZoneIdentifier(String),
    /// A month boundary could not be represented.
    UnableToCalculateMonth,
    /// A year boundary could not be represented.
    UnableToCalculateYear,
    /// A day boundary could not be represented.
    UnableToCalculateDay,
    /// The inclusive day range is inverted.
    InvalidDayRange,
}

impl fmt::Display for LedgerCalendarError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidTimeZoneIdentifier(identifier) => {
                write!(formatter, "invalid time zone identifier: {identifier}")
            }
            Self::UnableToCalculateMonth => formatter.write_str("unable to calculate month"),
            Self::UnableToCalculateYear => formatter.write_str("unable to calculate year"),
            Self::UnableToCalculateDay => formatter.write_str("unable to calculate day"),
            Self::InvalidDayRange => formatter.write_str("invalid day range"),
        }
    }
}

impl std::error::Error for LedgerCalendarError {}

/// A half-open interval of UTC instants: `[start, end_exclusive)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InstantInterval {
    start: Timestamp,
    end_exclusive: Timestamp,
}

impl InstantInterval {
    /// Creates an interval from its start and exclusive end.
    #[must_use]
    pub const fn new(start: Timestamp, end_exclusive: Timestamp) -> Self {
        Self {
            start,
            end_exclusive,
        }
    }

    /// Returns the inclusive start.
    #[must_use]
    pub const fn start(&self) -> Timestamp {
        self.start
    }

    /// Returns the exclusive end.
    #[must_use]
    pub const fn end_exclusive(&self) -> Timestamp {
        self.end_exclusive
    }

    /// Returns whether the instant lies within the interval.
    #[must_use]
    pub fn contains(&self, instant: Timestamp) -> bool {
        self.start <= instant && instant < self.end_exclusive
    }
}

impl LedgerCalendar {
    /// Validates an IANA time zone identifier.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerCalendarError::InvalidTimeZoneIdentifier`] when the
    /// identifier is not a known time zone.
    pub fn new(identifier: &str) -> Result<Self, LedgerCalendarError> {
        let time_zone = TimeZone::get(identifier)
            .map_err(|_| LedgerCalendarError::InvalidTimeZoneIdentifier(identifier.to_string()))?;
        Ok(Self {
            identifier: identifier.to_string(),
            time_zone,
        })
    }

    /// Returns the IANA time zone identifier.
    #[must_use]
    pub fn identifier(&self) -> &str {
        &self.identifier
    }

    fn date(&self, instant: Timestamp) -> Date {
        instant.to_zoned_datetime(self.time_zone.clone()).date()
    }

    fn midnight(
        &self,
        date: Date,
        error: LedgerCalendarError,
    ) -> Result<Timestamp, LedgerCalendarError> {
        date.to_zoned_datetime(self.time_zone.clone())
            .map(|zoned| zoned.timestamp())
            .map_err(|_| error)
    }

    /// Returns the half-open month interval containing the instant.
    ///
    /// # Errors
    ///
    /// Returns a calendar error when the boundary cannot be represented.
    pub fn month_interval(
        &self,
        containing: Timestamp,
    ) -> Result<InstantInterval, LedgerCalendarError> {
        let date = self.date(containing);
        let first = date.first_of_month();
        let next = first_of_next_month(first)?;
        Ok(InstantInterval::new(
            self.midnight(first, LedgerCalendarError::UnableToCalculateMonth)?,
            self.midnight(next, LedgerCalendarError::UnableToCalculateMonth)?,
        ))
    }

    /// Returns the half-open year interval containing the instant.
    ///
    /// # Errors
    ///
    /// Returns a calendar error when the boundary cannot be represented.
    pub fn year_interval(
        &self,
        containing: Timestamp,
    ) -> Result<InstantInterval, LedgerCalendarError> {
        let date = self.date(containing);
        let first =
            Date::new(date.year(), 1, 1).map_err(|_| LedgerCalendarError::UnableToCalculateYear)?;
        let next = date
            .year()
            .checked_add(1)
            .and_then(|year| Date::new(year, 1, 1).ok())
            .ok_or(LedgerCalendarError::UnableToCalculateYear)?;
        Ok(InstantInterval::new(
            self.midnight(first, LedgerCalendarError::UnableToCalculateYear)?,
            self.midnight(next, LedgerCalendarError::UnableToCalculateYear)?,
        ))
    }

    /// Returns the half-open day interval containing the instant.
    ///
    /// # Errors
    ///
    /// Returns a calendar error when the boundary cannot be represented.
    pub fn day_interval(
        &self,
        containing: Timestamp,
    ) -> Result<InstantInterval, LedgerCalendarError> {
        let date = self.date(containing);
        let next = date
            .tomorrow()
            .map_err(|_| LedgerCalendarError::UnableToCalculateDay)?;
        Ok(InstantInterval::new(
            self.midnight(date, LedgerCalendarError::UnableToCalculateDay)?,
            self.midnight(next, LedgerCalendarError::UnableToCalculateDay)?,
        ))
    }

    /// Returns the half-open interval covering the inclusive day range.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerCalendarError::InvalidDayRange`] when the start day is
    /// after the end day, or another calendar error when a boundary cannot be
    /// represented.
    pub fn inclusive_day_interval(
        &self,
        from: Timestamp,
        through: Timestamp,
    ) -> Result<InstantInterval, LedgerCalendarError> {
        let start_date = self.date(from);
        let end_date = self.date(through);
        let start_key = (start_date.year(), start_date.month(), start_date.day());
        let end_key = (end_date.year(), end_date.month(), end_date.day());
        if start_key > end_key {
            return Err(LedgerCalendarError::InvalidDayRange);
        }
        let end_exclusive = end_date
            .tomorrow()
            .map_err(|_| LedgerCalendarError::UnableToCalculateDay)?;
        Ok(InstantInterval::new(
            self.midnight(start_date, LedgerCalendarError::UnableToCalculateDay)?,
            self.midnight(end_exclusive, LedgerCalendarError::UnableToCalculateDay)?,
        ))
    }
}

fn first_of_next_month(date: Date) -> Result<Date, LedgerCalendarError> {
    let (year, month) = if date.month() == 12 {
        (
            date.year()
                .checked_add(1)
                .ok_or(LedgerCalendarError::UnableToCalculateMonth)?,
            1,
        )
    } else {
        (date.year(), date.month() + 1)
    };
    Date::new(year, month, 1).map_err(|_| LedgerCalendarError::UnableToCalculateMonth)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    #[test]
    fn month_membership_uses_the_ledger_time_zone() {
        let calendar = LedgerCalendar::new("Asia/Singapore").unwrap();
        // 2026-01-31T16:30:00Z is 2026-02-01T00:30:00+08:00.
        let instant = timestamp(1_769_877_000);
        let interval = calendar.month_interval(instant).unwrap();

        assert_eq!(interval.start(), timestamp(1_769_875_200));
        assert_eq!(interval.end_exclusive(), timestamp(1_772_294_400));
        assert!(interval.contains(instant));
        assert!(interval.contains(interval.start()));
        assert!(!interval.contains(interval.end_exclusive()));
    }

    #[test]
    fn day_and_inclusive_ranges_are_half_open() {
        let calendar = LedgerCalendar::new("UTC").unwrap();
        // 2026-01-01T00:00:00Z.
        let start = timestamp(1_767_225_600);

        let day = calendar.day_interval(start).unwrap();
        assert_eq!(day.start(), start);
        assert_eq!(day.end_exclusive(), timestamp(1_767_312_000));

        let two_days = calendar
            .inclusive_day_interval(start, timestamp(1_767_312_000))
            .unwrap();
        assert_eq!(two_days.start(), start);
        assert_eq!(two_days.end_exclusive(), timestamp(1_767_398_400));

        assert_eq!(
            calendar.inclusive_day_interval(timestamp(1_767_398_400), start),
            Err(LedgerCalendarError::InvalidDayRange)
        );
    }

    #[test]
    fn year_interval_uses_the_ledger_time_zone() {
        let calendar = LedgerCalendar::new("UTC").unwrap();
        let interval = calendar.year_interval(timestamp(1_767_225_600)).unwrap();
        assert_eq!(interval.start(), timestamp(1_767_225_600));
        assert_eq!(interval.end_exclusive(), timestamp(1_798_761_600));
    }

    #[test]
    fn december_month_interval_crosses_the_year() {
        let calendar = LedgerCalendar::new("UTC").unwrap();
        let instant: Timestamp = "2026-12-15T12:00:00Z".parse().unwrap();
        let interval = calendar.month_interval(instant).unwrap();

        assert_eq!(
            interval.start(),
            "2026-12-01T00:00:00Z".parse::<Timestamp>().unwrap()
        );
        assert_eq!(
            interval.end_exclusive(),
            "2027-01-01T00:00:00Z".parse::<Timestamp>().unwrap()
        );
    }

    #[test]
    fn month_interval_follows_dst_offsets() {
        let calendar = LedgerCalendar::new("Europe/Berlin").unwrap();
        let instant: Timestamp = "2026-03-15T12:00:00Z".parse().unwrap();
        let interval = calendar.month_interval(instant).unwrap();

        assert_eq!(
            interval.start(),
            "2026-02-28T23:00:00Z".parse::<Timestamp>().unwrap()
        );
        assert_eq!(
            interval.end_exclusive(),
            "2026-03-31T22:00:00Z".parse::<Timestamp>().unwrap()
        );
    }

    #[test]
    fn invalid_time_zone_is_rejected() {
        assert_eq!(
            LedgerCalendar::new("Not/A_Time_Zone"),
            Err(LedgerCalendarError::InvalidTimeZoneIdentifier(
                "Not/A_Time_Zone".to_string()
            ))
        );
    }
}
