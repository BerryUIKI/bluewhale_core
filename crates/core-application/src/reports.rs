//! Category ranking report.

use std::collections::BTreeMap;

use core_domain::{
    CategoryGroup, CategoryGroupId, CategoryId, CategoryKind, Currency, InstantInterval,
    LedgerReadSnapshotRepository, LedgerSettings, Money, TransactionKind, validate_transactions,
};
use jiff::Timestamp;

use crate::error::ApplicationError;
use crate::exchange_rates::HistoricalMoneyConverter;

/// One ranked category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRankingRow {
    /// The category identity.
    pub category_id: CategoryId,
    /// The category display name.
    pub category_name: String,
    /// The optional containing group.
    pub group_id: Option<CategoryGroupId>,
    /// The group display name, when grouped.
    pub group_name: Option<String>,
    /// The converted total in the reporting currency.
    pub total: Money,
    /// The number of matching transactions.
    pub transaction_count: u32,
}

/// A category ranking for one ledger month and kind.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryRankingReport {
    /// The ledger settings.
    pub settings: LedgerSettings,
    /// The reporting currency used for the totals.
    pub reporting_currency: Currency,
    /// The month interval in the ledger time zone.
    pub interval: InstantInterval,
    /// The ranked category kind.
    pub kind: CategoryKind,
    /// The sum of all ranked totals.
    pub total: Money,
    /// The rows ordered by total descending, then by name.
    pub rows: Vec<CategoryRankingRow>,
}

/// Builds the category ranking for a ledger month.
pub struct LoadCategoryRankingUseCase<'a> {
    repository: &'a dyn LedgerReadSnapshotRepository,
}

impl<'a> LoadCategoryRankingUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn LedgerReadSnapshotRepository) -> Self {
        Self { repository }
    }

    /// Builds the ranking for the month containing the instant.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] for an invalid time zone or calendar
    /// boundary, an invalid transaction, a missing exchange rate, or a money
    /// overflow.
    pub fn execute(
        &self,
        month_containing: Timestamp,
        kind: CategoryKind,
        requested_currency: Option<Currency>,
    ) -> Result<CategoryRankingReport, ApplicationError> {
        let snapshot = self.repository.snapshot()?;
        validate_transactions(&snapshot.active_transactions)
            .map_err(ApplicationError::Validation)?;
        let reporting_currency =
            requested_currency.unwrap_or_else(|| snapshot.settings.currency().clone());
        let converter =
            HistoricalMoneyConverter::new(&snapshot.exchange_rates, reporting_currency.clone());
        let interval = snapshot
            .settings
            .calendar()
            .month_interval(month_containing)
            .map_err(ApplicationError::Calendar)?;
        let transaction_kind = match kind {
            CategoryKind::Expense => TransactionKind::Expense,
            CategoryKind::Income => TransactionKind::Income,
        };

        let mut totals: BTreeMap<CategoryId, (Money, u32)> = BTreeMap::new();
        for transaction in &snapshot.active_transactions {
            if transaction.kind() != transaction_kind
                || !interval.contains(transaction.occurred_at())
            {
                continue;
            }
            let Some(category_id) = transaction.fields().category_id else {
                continue;
            };
            let converted = converter.convert(transaction.amount(), transaction.occurred_at())?;
            let entry = totals
                .entry(category_id)
                .or_insert_with(|| (Money::zero(reporting_currency.clone()), 0));
            entry.0 = entry
                .0
                .adding(&converted)
                .map_err(ApplicationError::Money)?;
            entry.1 = entry.1.saturating_add(1);
        }

        let categories: BTreeMap<CategoryId, &core_domain::Category> = snapshot
            .categories
            .iter()
            .map(|category| (category.id(), category))
            .collect();
        let groups: BTreeMap<CategoryGroupId, &CategoryGroup> = snapshot
            .category_groups
            .iter()
            .map(|group| (group.id(), group))
            .collect();

        let mut rows: Vec<CategoryRankingRow> = totals
            .into_iter()
            .map(|(category_id, (total, transaction_count))| {
                let (category_name, group_id, group_name) =
                    categories.get(&category_id).map_or_else(
                        || (category_id.to_string(), None, None),
                        |category| {
                            let group_id = category.options().group_id;
                            let group_name = group_id
                                .and_then(|id| groups.get(&id))
                                .map(|group| group.name().to_string());
                            (category.name().to_string(), group_id, group_name)
                        },
                    );
                CategoryRankingRow {
                    category_id,
                    category_name,
                    group_id,
                    group_name,
                    total,
                    transaction_count,
                }
            })
            .collect();
        rows.sort_by(|left, right| {
            right
                .total
                .minor_units()
                .cmp(&left.total.minor_units())
                .then_with(|| left.category_name.cmp(&right.category_name))
        });

        let total = rows
            .iter()
            .try_fold(Money::zero(reporting_currency.clone()), |sum, row| {
                sum.adding(&row.total).map_err(ApplicationError::Money)
            })?;

        Ok(CategoryRankingReport {
            settings: snapshot.settings,
            reporting_currency,
            interval,
            kind,
            total,
            rows,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        Account, AccountId, AccountOptions, AccountRepository, AssetAccountType, Category,
        CategoryGroupRepository, CategoryOptions, CategoryRepository, CurrencyScalePolicy,
        ExchangeRate, ExchangeRateId, ExchangeRateRepository, ExchangeRateSource, LedgerCalendar,
        LedgerSettingsId, Transaction, TransactionFields, TransactionId, TransactionRepository,
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

    fn account(ledger: &InMemoryLedger, id: u128, code: &str) {
        ledger
            .save_account(
                &Account::new(
                    AccountId::from_uuid(Uuid::from_u128(id)),
                    "Account",
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

    fn category(
        ledger: &InMemoryLedger,
        id: u128,
        name: &str,
        kind: CategoryKind,
        group: Option<CategoryGroupId>,
    ) {
        ledger
            .save_category(
                &Category::new(
                    CategoryId::from_uuid(Uuid::from_u128(id)),
                    name,
                    kind,
                    CategoryOptions {
                        group_id: group,
                        ..CategoryOptions::default()
                    },
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn expense(
        ledger: &InMemoryLedger,
        id: u128,
        category_id: CategoryId,
        minor_units: i64,
        code: &str,
        at: i64,
    ) {
        ledger
            .save_transaction(
                &Transaction::new(
                    TransactionId::from_uuid(Uuid::from_u128(id)),
                    TransactionKind::Expense,
                    Money::new(minor_units, currency(code)),
                    timestamp(at),
                    AccountId::from_uuid(Uuid::from_u128(1)),
                    TransactionFields {
                        category_id: Some(category_id),
                        ..TransactionFields::default()
                    },
                    timestamp(at),
                    timestamp(at),
                )
                .unwrap(),
            )
            .unwrap();
    }

    fn seed(ledger: &InMemoryLedger) {
        ledger.seed_settings(settings());
        account(ledger, 1, "USD");
        account(ledger, 2, "CNY");
        category(ledger, 10, "Food", CategoryKind::Expense, None);
        category(ledger, 11, "Transport", CategoryKind::Expense, None);
        category(ledger, 12, "Salary", CategoryKind::Income, None);
        category(ledger, 13, "Groceries", CategoryKind::Expense, None);
        let group = CategoryGroupId::from_uuid(Uuid::from_u128(20));
        ledger
            .save_category_group(
                &CategoryGroup::new(
                    group,
                    "Essentials",
                    CategoryKind::Expense,
                    0,
                    false,
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            )
            .unwrap();
        category(ledger, 14, "Rent", CategoryKind::Expense, Some(group));
    }

    #[test]
    fn ranks_categories_by_converted_total() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        expense(
            &ledger,
            100,
            CategoryId::from_uuid(Uuid::from_u128(10)),
            1_000,
            "USD",
            2_000,
        );
        expense(
            &ledger,
            101,
            CategoryId::from_uuid(Uuid::from_u128(10)),
            500,
            "USD",
            2_100,
        );
        expense(
            &ledger,
            102,
            CategoryId::from_uuid(Uuid::from_u128(11)),
            2_000,
            "USD",
            2_200,
        );
        expense(
            &ledger,
            103,
            CategoryId::from_uuid(Uuid::from_u128(13)),
            8_000,
            "CNY",
            2_300,
        );
        ledger
            .save_exchange_rate(
                &ExchangeRate::new(
                    ExchangeRateId::from_uuid(Uuid::from_u128(30)),
                    currency("CNY"),
                    currency("USD"),
                    core_domain::ExactRatio::new(15, 100).unwrap(),
                    timestamp(1_000),
                    ExchangeRateSource::Manual,
                    None,
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            )
            .unwrap();

        let report = LoadCategoryRankingUseCase::new(&ledger)
            .execute(timestamp(2_500), CategoryKind::Expense, None)
            .unwrap();

        assert_eq!(report.reporting_currency.code(), "USD");
        assert_eq!(report.rows.len(), 3);
        assert_eq!(report.rows[0].category_name, "Transport");
        assert_eq!(report.rows[0].total.minor_units(), 2_000);
        assert_eq!(report.rows[0].transaction_count, 1);
        assert_eq!(report.rows[1].category_name, "Food");
        assert_eq!(report.rows[1].total.minor_units(), 1_500);
        assert_eq!(report.rows[1].transaction_count, 2);
        assert_eq!(report.rows[2].category_name, "Groceries");
        assert_eq!(report.rows[2].total.minor_units(), 1_200);
        assert_eq!(report.total.minor_units(), 4_700);
    }

    #[test]
    fn includes_group_names_and_excludes_other_kinds() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        expense(
            &ledger,
            100,
            CategoryId::from_uuid(Uuid::from_u128(14)),
            1_000,
            "USD",
            2_000,
        );
        let income = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(101)),
            TransactionKind::Income,
            Money::new(5_000, currency("USD")),
            timestamp(2_000),
            AccountId::from_uuid(Uuid::from_u128(1)),
            TransactionFields {
                category_id: Some(CategoryId::from_uuid(Uuid::from_u128(12))),
                ..TransactionFields::default()
            },
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap();
        ledger.save_transaction(&income).unwrap();
        let transfer = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(102)),
            TransactionKind::Transfer,
            Money::new(2_000, currency("USD")),
            timestamp(2_000),
            AccountId::from_uuid(Uuid::from_u128(1)),
            TransactionFields {
                destination_account_id: Some(AccountId::from_uuid(Uuid::from_u128(2))),
                ..TransactionFields::default()
            },
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap();
        ledger.save_transaction(&transfer).unwrap();

        let expenses = LoadCategoryRankingUseCase::new(&ledger)
            .execute(timestamp(2_500), CategoryKind::Expense, None)
            .unwrap();
        assert_eq!(expenses.rows.len(), 1);
        assert_eq!(expenses.rows[0].category_name, "Rent");
        assert_eq!(expenses.rows[0].group_name.as_deref(), Some("Essentials"));
        assert_eq!(expenses.total.minor_units(), 1_000);

        let income = LoadCategoryRankingUseCase::new(&ledger)
            .execute(timestamp(2_500), CategoryKind::Income, None)
            .unwrap();
        assert_eq!(income.rows.len(), 1);
        assert_eq!(income.rows[0].category_name, "Salary");
        assert_eq!(income.total.minor_units(), 5_000);
    }

    #[test]
    fn reports_missing_rates() {
        let ledger = InMemoryLedger::new();
        seed(&ledger);
        expense(
            &ledger,
            100,
            CategoryId::from_uuid(Uuid::from_u128(10)),
            10_000,
            "CNY",
            2_000,
        );

        assert_eq!(
            LoadCategoryRankingUseCase::new(&ledger)
                .execute(timestamp(2_500), CategoryKind::Expense, None)
                .unwrap_err(),
            ApplicationError::MissingExchangeRate {
                source: currency("CNY"),
                reporting: currency("USD"),
            }
        );
    }

    #[test]
    fn month_boundaries_use_the_ledger_time_zone() {
        let ledger = InMemoryLedger::new();
        ledger.seed_settings(LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(99)),
            currency("USD"),
            1,
            LedgerCalendar::new("Asia/Singapore").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        ));
        account(&ledger, 1, "USD");
        category(&ledger, 10, "Food", CategoryKind::Expense, None);
        // 2026-01-31T16:30:00Z is 2026-02-01T00:30:00+08:00.
        let instant: Timestamp = "2026-01-31T16:30:00Z".parse().unwrap();
        expense(
            &ledger,
            100,
            CategoryId::from_uuid(Uuid::from_u128(10)),
            1_000,
            "USD",
            instant.as_second(),
        );

        let january = LoadCategoryRankingUseCase::new(&ledger)
            .execute(
                "2026-01-15T12:00:00Z".parse().unwrap(),
                CategoryKind::Expense,
                None,
            )
            .unwrap();
        assert!(january.rows.is_empty());
        let february = LoadCategoryRankingUseCase::new(&ledger)
            .execute(
                "2026-02-15T12:00:00Z".parse().unwrap(),
                CategoryKind::Expense,
                None,
            )
            .unwrap();
        assert_eq!(february.rows.len(), 1);
    }
}
