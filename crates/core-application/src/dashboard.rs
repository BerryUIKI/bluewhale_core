//! Ledger dashboard: balances, converted net balance, monthly statistics, and
//! daily cash flow.

use std::collections::BTreeMap;

use core_domain::{
    Account, Currency, InstantInterval, LedgerCalendar, LedgerReadSnapshotRepository,
    LedgerSettings, Money, MonthlyStatistics, Transaction, TransactionKind, account_balance,
    validate_transactions,
};
use jiff::Timestamp;

use crate::error::ApplicationError;
use crate::exchange_rates::HistoricalMoneyConverter;

/// One account and its balance in the account currency.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountBalanceSummary {
    /// The account.
    pub account: Account,
    /// The account balance.
    pub balance: Money,
}

/// One day of converted income, expense, and transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyCashFlowSummary {
    /// The day interval.
    pub interval: InstantInterval,
    /// Converted income.
    pub income: Money,
    /// Converted expense.
    pub expense: Money,
    /// Converted savings (income minus expense).
    pub savings: Money,
    /// Transactions on the day.
    pub transactions: Vec<Transaction>,
}

/// The dashboard payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerDashboard {
    /// The ledger settings.
    pub settings: LedgerSettings,
    /// The reporting currency used for totals.
    pub reporting_currency: Currency,
    /// Currencies used by active accounts.
    pub available_currencies: Vec<Currency>,
    /// Per-account balances in native currency.
    pub account_balances: Vec<AccountBalanceSummary>,
    /// Net balance converted to the reporting currency.
    pub net_balance: Money,
    /// Income and expense for the month.
    pub monthly_statistics: MonthlyStatistics,
    /// Monthly income minus expense.
    pub monthly_savings: Money,
    /// Daily cash flow for the month.
    pub daily_cash_flow: Vec<DailyCashFlowSummary>,
}

/// Loads a ledger dashboard from a snapshot.
pub struct LoadLedgerDashboardUseCase<'a> {
    repository: &'a dyn LedgerReadSnapshotRepository,
}

impl<'a> LoadLedgerDashboardUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn LedgerReadSnapshotRepository) -> Self {
        Self { repository }
    }

    /// Builds the dashboard for the month containing the instant.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] for invalid transactions, balances,
    /// calendar boundaries, missing exchange rates, or money overflow.
    pub fn execute(
        &self,
        month_containing: Timestamp,
        requested_currency: Option<Currency>,
    ) -> Result<LedgerDashboard, ApplicationError> {
        let snapshot = self.repository.snapshot()?;
        validate_transactions(&snapshot.active_transactions)
            .map_err(ApplicationError::Validation)?;
        let calendar = snapshot.settings.calendar().clone();
        let reporting_currency =
            requested_currency.unwrap_or_else(|| snapshot.settings.currency().clone());
        let converter =
            HistoricalMoneyConverter::new(&snapshot.exchange_rates, reporting_currency.clone());

        let mut available: BTreeMap<String, Currency> = BTreeMap::new();
        available.insert(
            reporting_currency.code().to_string(),
            reporting_currency.clone(),
        );
        let mut account_balances = Vec::new();
        for account in &snapshot.active_accounts {
            available
                .entry(account.currency().code().to_string())
                .or_insert_with(|| account.currency().clone());
            let balance = account_balance(
                account.id(),
                account.currency(),
                &snapshot.active_transactions,
            )
            .map_err(ApplicationError::Balance)?;
            account_balances.push(AccountBalanceSummary {
                account: account.clone(),
                balance,
            });
        }

        let mut net_balance = Money::zero(reporting_currency.clone());
        for transaction in &snapshot.active_transactions {
            match transaction.kind() {
                TransactionKind::OpeningBalance | TransactionKind::Income => {
                    net_balance = net_balance
                        .adding(
                            &converter.convert(transaction.amount(), transaction.occurred_at())?,
                        )
                        .map_err(ApplicationError::Money)?;
                }
                TransactionKind::Expense => {
                    net_balance = net_balance
                        .subtracting(
                            &converter.convert(transaction.amount(), transaction.occurred_at())?,
                        )
                        .map_err(ApplicationError::Money)?;
                }
                TransactionKind::Transfer => {
                    net_balance = net_balance
                        .subtracting(
                            &converter.convert(transaction.amount(), transaction.occurred_at())?,
                        )
                        .map_err(ApplicationError::Money)?;
                    net_balance =
                        net_balance
                            .adding(&converter.convert(
                                &transaction.received_amount(),
                                transaction.occurred_at(),
                            )?)
                            .map_err(ApplicationError::Money)?;
                }
            }
        }

        let interval = calendar
            .month_interval(month_containing)
            .map_err(ApplicationError::Calendar)?;
        let mut income = Money::zero(reporting_currency.clone());
        let mut expense = Money::zero(reporting_currency.clone());
        for transaction in &snapshot.active_transactions {
            if !interval.contains(transaction.occurred_at()) {
                continue;
            }
            match transaction.kind() {
                TransactionKind::Income => {
                    income = income
                        .adding(
                            &converter.convert(transaction.amount(), transaction.occurred_at())?,
                        )
                        .map_err(ApplicationError::Money)?;
                }
                TransactionKind::Expense => {
                    expense = expense
                        .adding(
                            &converter.convert(transaction.amount(), transaction.occurred_at())?,
                        )
                        .map_err(ApplicationError::Money)?;
                }
                TransactionKind::OpeningBalance | TransactionKind::Transfer => {}
            }
        }
        let monthly_savings = income
            .subtracting(&expense)
            .map_err(ApplicationError::Money)?;
        let monthly_statistics = MonthlyStatistics::new(interval, income, expense);

        let daily_cash_flow = self.daily_cash_flow(
            &snapshot.active_transactions,
            &interval,
            &calendar,
            &converter,
            &reporting_currency,
        )?;

        Ok(LedgerDashboard {
            settings: snapshot.settings,
            reporting_currency,
            available_currencies: available.into_values().collect(),
            account_balances,
            net_balance,
            monthly_statistics,
            monthly_savings,
            daily_cash_flow,
        })
    }

    fn daily_cash_flow(
        &self,
        transactions: &[Transaction],
        month_interval: &InstantInterval,
        calendar: &LedgerCalendar,
        converter: &HistoricalMoneyConverter<'_>,
        reporting_currency: &Currency,
    ) -> Result<Vec<DailyCashFlowSummary>, ApplicationError> {
        let mut days = Vec::new();
        let mut cursor = month_interval.start();
        while cursor < month_interval.end_exclusive() {
            let day = calendar
                .day_interval(cursor)
                .map_err(ApplicationError::Calendar)?;
            let mut income = Money::zero(reporting_currency.clone());
            let mut expense = Money::zero(reporting_currency.clone());
            let mut day_transactions = Vec::new();
            for transaction in transactions {
                if !day.contains(transaction.occurred_at()) {
                    continue;
                }
                day_transactions.push(transaction.clone());
                match transaction.kind() {
                    TransactionKind::Income => {
                        income = income
                            .adding(
                                &converter
                                    .convert(transaction.amount(), transaction.occurred_at())?,
                            )
                            .map_err(ApplicationError::Money)?;
                    }
                    TransactionKind::Expense => {
                        expense = expense
                            .adding(
                                &converter
                                    .convert(transaction.amount(), transaction.occurred_at())?,
                            )
                            .map_err(ApplicationError::Money)?;
                    }
                    TransactionKind::OpeningBalance | TransactionKind::Transfer => {}
                }
            }
            day_transactions.sort_by(|left, right| {
                left.occurred_at()
                    .cmp(&right.occurred_at())
                    .then_with(|| left.id().to_string().cmp(&right.id().to_string()))
            });
            let savings = income
                .subtracting(&expense)
                .map_err(ApplicationError::Money)?;
            days.push(DailyCashFlowSummary {
                interval: day,
                income,
                expense,
                savings,
                transactions: day_transactions,
            });
            cursor = day.end_exclusive();
        }
        Ok(days)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        AccountId, AccountOptions, AccountRepository, AssetAccountType, CategoryId, CategoryKind,
        CategoryOptions, CategoryRepository, CurrencyScalePolicy, ExchangeRate, ExchangeRateId,
        ExchangeRateRepository, ExchangeRateSource, LedgerSettingsId, TransactionFields,
        TransactionId, TransactionRepository,
    };
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

    fn seed(ledger: &InMemoryLedger) {
        ledger.seed_settings(settings());
        for (id, name, code) in [
            (1u128, "Cash", "USD"),
            (2, "Bank", "USD"),
            (3, "Yuan", "CNY"),
        ] {
            ledger
                .save_account(
                    &Account::new(
                        AccountId::from_uuid(Uuid::from_u128(id)),
                        name,
                        AssetAccountType::Cash,
                        currency(code),
                        AccountOptions::default(),
                        timestamp(1_000),
                        timestamp(1_000),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        let category = core_domain::Category::new(
            CategoryId::from_uuid(Uuid::from_u128(4)),
            "Food",
            CategoryKind::Expense,
            CategoryOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        ledger.save_category(&category).unwrap();

        let opening = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(10)),
            TransactionKind::OpeningBalance,
            Money::new(10_000, currency("USD")),
            timestamp(2_000),
            AccountId::from_uuid(Uuid::from_u128(1)),
            TransactionFields::default(),
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap();
        let expense = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(11)),
            TransactionKind::Expense,
            Money::new(1_000, currency("USD")),
            timestamp(2_100),
            AccountId::from_uuid(Uuid::from_u128(1)),
            TransactionFields {
                category_id: Some(category.id()),
                ..TransactionFields::default()
            },
            timestamp(2_100),
            timestamp(2_100),
        )
        .unwrap();
        let transfer = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(12)),
            TransactionKind::Transfer,
            Money::new(2_000, currency("USD")),
            timestamp(2_200),
            AccountId::from_uuid(Uuid::from_u128(1)),
            TransactionFields {
                destination_account_id: Some(AccountId::from_uuid(Uuid::from_u128(2))),
                ..TransactionFields::default()
            },
            timestamp(2_200),
            timestamp(2_200),
        )
        .unwrap();
        ledger.save_transaction(&opening).unwrap();
        ledger.save_transaction(&expense).unwrap();
        ledger.save_transaction(&transfer).unwrap();
    }

    #[test]
    fn dashboard_reports_balances_net_and_monthly_totals() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        let dashboard = LoadLedgerDashboardUseCase::new(&ledger)
            .execute(timestamp(2_500), None)
            .unwrap();

        assert_eq!(dashboard.reporting_currency.code(), "USD");
        assert_eq!(dashboard.account_balances[0].balance.minor_units(), 7_000);
        assert_eq!(dashboard.account_balances[1].balance.minor_units(), 2_000);
        assert_eq!(dashboard.net_balance.minor_units(), 9_000);
        assert_eq!(dashboard.monthly_statistics.expense().minor_units(), 1_000);
        assert_eq!(dashboard.monthly_statistics.income().minor_units(), 0);
        assert_eq!(dashboard.monthly_savings.minor_units(), -1_000);
        assert!(!dashboard.daily_cash_flow.is_empty());
    }

    #[test]
    fn dashboard_converts_foreign_accounts_with_rates() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        let opening = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(13)),
            TransactionKind::OpeningBalance,
            Money::new(100_000, currency("CNY")),
            timestamp(2_300),
            AccountId::from_uuid(Uuid::from_u128(3)),
            TransactionFields::default(),
            timestamp(2_300),
            timestamp(2_300),
        )
        .unwrap();
        ledger.save_transaction(&opening).unwrap();
        let rate = ExchangeRate::new(
            ExchangeRateId::from_uuid(Uuid::from_u128(20)),
            currency("CNY"),
            currency("USD"),
            core_domain::ExactRatio::new(15, 100).unwrap(),
            timestamp(1_000),
            ExchangeRateSource::Manual,
            None,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();
        ledger.save_exchange_rate(&rate).unwrap();

        let dashboard = LoadLedgerDashboardUseCase::new(&ledger)
            .execute(timestamp(2_500), None)
            .unwrap();

        assert_eq!(dashboard.net_balance.minor_units(), 24_000);
        assert_eq!(dashboard.available_currencies.len(), 2);
    }

    #[test]
    fn dashboard_reports_missing_rates() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        let opening = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(14)),
            TransactionKind::OpeningBalance,
            Money::new(100_000, currency("CNY")),
            timestamp(2_300),
            AccountId::from_uuid(Uuid::from_u128(3)),
            TransactionFields::default(),
            timestamp(2_300),
            timestamp(2_300),
        )
        .unwrap();
        ledger.save_transaction(&opening).unwrap();

        assert_eq!(
            LoadLedgerDashboardUseCase::new(&ledger)
                .execute(timestamp(2_500), None)
                .unwrap_err(),
            ApplicationError::MissingExchangeRate {
                source: currency("CNY"),
                reporting: currency("USD"),
            }
        );
    }
}
