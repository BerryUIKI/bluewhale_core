//! Monthly income and expense statistics in the ledger time zone.

use std::fmt;

use jiff::Timestamp;

use crate::calendar::{InstantInterval, LedgerCalendar, LedgerCalendarError};
use crate::currency::Currency;
use crate::money::{Money, MoneyError};
use crate::transaction::{Transaction, TransactionKind};
use crate::validation::{LedgerValidationError, validate_transactions};

/// Income and expense totals for one ledger month.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthlyStatistics {
    interval: InstantInterval,
    income: Money,
    expense: Money,
}

impl MonthlyStatistics {
    /// Returns the month interval.
    #[must_use]
    pub fn interval(&self) -> InstantInterval {
        self.interval
    }

    /// Returns the income total.
    #[must_use]
    pub fn income(&self) -> &Money {
        &self.income
    }

    /// Returns the expense total.
    #[must_use]
    pub fn expense(&self) -> &Money {
        &self.expense
    }
}

/// Errors raised while calculating monthly statistics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MonthlyStatisticsError {
    /// The ledger invariants are violated.
    Validation(LedgerValidationError),
    /// The calendar boundary cannot be represented.
    Calendar(LedgerCalendarError),
    /// An amount cannot be combined or overflows.
    Money(MoneyError),
}

impl fmt::Display for MonthlyStatisticsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Calendar(error) => error.fmt(formatter),
            Self::Money(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for MonthlyStatisticsError {}

/// Calculates income and expense for the month containing the instant.
///
/// Transfers, opening balances, soft-deleted transactions, and events outside
/// the ledger-time-zone month are excluded.
///
/// # Errors
///
/// Returns a [`MonthlyStatisticsError`] when the ledger invariants are
/// violated, the calendar boundary cannot be represented, or an amount cannot
/// be combined.
pub fn monthly_statistics(
    transactions: &[Transaction],
    month_containing: Timestamp,
    calendar: &LedgerCalendar,
    currency: &Currency,
) -> Result<MonthlyStatistics, MonthlyStatisticsError> {
    validate_transactions(transactions).map_err(MonthlyStatisticsError::Validation)?;
    let interval = calendar
        .month_interval(month_containing)
        .map_err(MonthlyStatisticsError::Calendar)?;
    let mut income = Money::zero(currency.clone());
    let mut expense = Money::zero(currency.clone());

    for transaction in transactions {
        if transaction.is_deleted() || !interval.contains(transaction.occurred_at()) {
            continue;
        }
        match transaction.kind() {
            TransactionKind::Income => {
                income = income
                    .adding(transaction.amount())
                    .map_err(MonthlyStatisticsError::Money)?;
            }
            TransactionKind::Expense => {
                expense = expense
                    .adding(transaction.amount())
                    .map_err(MonthlyStatisticsError::Money)?;
            }
            TransactionKind::OpeningBalance | TransactionKind::Transfer => {}
        }
    }

    Ok(MonthlyStatistics {
        interval,
        income,
        expense,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use crate::ids::{AccountId, CategoryId, TransactionId};
    use crate::transaction::TransactionFields;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    fn account(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    fn transaction(
        id: u128,
        kind: TransactionKind,
        minor_units: i64,
        occurred_at: Timestamp,
        fields: TransactionFields,
    ) -> Transaction {
        Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(id)),
            kind,
            Money::new(minor_units, usd()),
            occurred_at,
            account(1),
            fields,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    #[test]
    fn statistics_include_income_and_expense_and_exclude_other_events() {
        let calendar = LedgerCalendar::new("UTC").unwrap();
        // 2026-02-10T12:00:00Z.
        let date = timestamp(1_770_724_800);
        let outside = timestamp(1_772_323_200); // 2026-03-01T00:00:00Z.
        let category = CategoryId::from_uuid(Uuid::from_u128(2));
        let expense = transaction(
            2,
            TransactionKind::Expense,
            2_500,
            date,
            TransactionFields {
                category_id: Some(category),
                ..TransactionFields::default()
            },
        );

        let transactions = vec![
            transaction(
                1,
                TransactionKind::OpeningBalance,
                50_000,
                date,
                TransactionFields::default(),
            ),
            transaction(
                3,
                TransactionKind::Income,
                10_000,
                date,
                TransactionFields {
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(3))),
                    ..TransactionFields::default()
                },
            ),
            expense.clone(),
            transaction(
                4,
                TransactionKind::Transfer,
                3_000,
                date,
                TransactionFields {
                    destination_account_id: Some(account(2)),
                    ..TransactionFields::default()
                },
            ),
            expense.soft_deleted(date).unwrap(),
            transaction(
                5,
                TransactionKind::Income,
                99_999,
                outside,
                TransactionFields {
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(4))),
                    ..TransactionFields::default()
                },
            ),
        ];

        let statistics = monthly_statistics(&transactions, date, &calendar, &usd()).unwrap();
        assert_eq!(statistics.income().minor_units(), 10_000);
        assert_eq!(statistics.expense().minor_units(), 2_500);
    }

    #[test]
    fn ledger_time_zone_determines_month_membership() {
        let calendar = LedgerCalendar::new("Asia/Singapore").unwrap();
        // 2026-01-31T16:30:00Z is 2026-02-01T00:30:00+08:00.
        let instant = timestamp(1_769_877_000);
        let income = transaction(
            1,
            TransactionKind::Income,
            1_000,
            instant,
            TransactionFields {
                category_id: Some(CategoryId::from_uuid(Uuid::from_u128(2))),
                ..TransactionFields::default()
            },
        );

        let statistics = monthly_statistics(&[income], instant, &calendar, &usd()).unwrap();
        assert_eq!(statistics.income().minor_units(), 1_000);
    }
}
