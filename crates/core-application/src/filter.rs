//! Transaction filtering, search, and ordering.

use std::collections::BTreeMap;

use core_domain::{
    Account, AccountId, Category, CategoryGroup, CategoryId, Counterparty, CounterpartyId,
    LedgerCalendar, LedgerSettings, LedgerTag, TagId, Transaction, TransactionKind,
};
use jiff::Timestamp;
use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;

use crate::error::ApplicationError;

/// The filter, search, and date criteria for a transaction list.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransactionQuery {
    /// Case- and width-insensitive search text.
    pub search_text: String,
    /// Restrict to one kind.
    pub kind: Option<TransactionKind>,
    /// Restrict to transactions involving one account.
    pub account_id: Option<AccountId>,
    /// Restrict to one category.
    pub category_id: Option<CategoryId>,
    /// Restrict to transactions carrying one tag.
    pub tag_id: Option<TagId>,
    /// Restrict to one counterparty.
    pub counterparty_id: Option<CounterpartyId>,
    /// Inclusive first day in the ledger time zone.
    pub start_day: Option<Timestamp>,
    /// Inclusive last day in the ledger time zone.
    pub end_day: Option<Timestamp>,
}

/// The reference data needed to filter and search transactions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionContext {
    /// Ledger settings.
    pub settings: LedgerSettings,
    /// Accounts, including archived ones.
    pub accounts: Vec<Account>,
    /// Leaf categories.
    pub categories: Vec<Category>,
    /// Category groups.
    pub category_groups: Vec<CategoryGroup>,
    /// Tags.
    pub tags: Vec<LedgerTag>,
    /// Counterparties.
    pub counterparties: Vec<Counterparty>,
}

/// Filters and sorts transactions.
#[derive(Debug, Clone, Copy, Default)]
pub struct FilterTransactionsUseCase;

impl FilterTransactionsUseCase {
    /// Creates the use case.
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    /// Applies the query to the transactions.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] for an invalid time zone, an inverted
    /// date range, or a selected reference that is not in the context.
    pub fn execute(
        &self,
        transactions: &[Transaction],
        context: &TransactionContext,
        query: &TransactionQuery,
    ) -> Result<Vec<Transaction>, ApplicationError> {
        let accounts: BTreeMap<AccountId, &Account> = context
            .accounts
            .iter()
            .map(|account| (account.id(), account))
            .collect();
        let categories: BTreeMap<CategoryId, &Category> = context
            .categories
            .iter()
            .map(|category| (category.id(), category))
            .collect();
        let groups: BTreeMap<core_domain::CategoryGroupId, &CategoryGroup> = context
            .category_groups
            .iter()
            .map(|group| (group.id(), group))
            .collect();
        let tags: BTreeMap<TagId, &LedgerTag> =
            context.tags.iter().map(|tag| (tag.id(), tag)).collect();
        let counterparties: BTreeMap<CounterpartyId, &Counterparty> = context
            .counterparties
            .iter()
            .map(|item| (item.id(), item))
            .collect();

        if let Some(account_id) = query.account_id
            && !accounts.contains_key(&account_id)
        {
            return Err(ApplicationError::SelectedAccountNotFound(account_id));
        }
        if let Some(category_id) = query.category_id
            && !categories.contains_key(&category_id)
        {
            return Err(ApplicationError::SelectedCategoryNotFound(category_id));
        }
        if let Some(tag_id) = query.tag_id
            && !tags.contains_key(&tag_id)
        {
            return Err(ApplicationError::SelectedTagNotFound(tag_id));
        }
        if let Some(counterparty_id) = query.counterparty_id
            && !counterparties.contains_key(&counterparty_id)
        {
            return Err(ApplicationError::SelectedCounterpartyNotFound(
                counterparty_id,
            ));
        }

        let calendar = context.settings.calendar().clone();
        let (start, end_exclusive) = date_bounds(&calendar, query)?;
        if let (Some(start), Some(end)) = (start, end_exclusive)
            && start >= end
        {
            return Err(ApplicationError::InvertedDateRange);
        }
        let needle = normalize_search_text(&query.search_text);

        let mut filtered: Vec<Transaction> = transactions
            .iter()
            .filter(|transaction| {
                if query.kind.is_some_and(|kind| transaction.kind() != kind) {
                    return false;
                }
                if query.account_id.is_some_and(|account_id| {
                    transaction.account_id() != account_id
                        && transaction.fields().destination_account_id != Some(account_id)
                }) {
                    return false;
                }
                if query.category_id.is_some_and(|category_id| {
                    transaction.fields().category_id != Some(category_id)
                }) {
                    return false;
                }
                if query
                    .tag_id
                    .is_some_and(|tag_id| !transaction.fields().tag_ids.contains(&tag_id))
                {
                    return false;
                }
                if query.counterparty_id.is_some_and(|counterparty_id| {
                    transaction.fields().counterparty_id != Some(counterparty_id)
                }) {
                    return false;
                }
                if start.is_some_and(|start| transaction.occurred_at() < start) {
                    return false;
                }
                if end_exclusive.is_some_and(|end| transaction.occurred_at() >= end) {
                    return false;
                }
                if !needle.is_empty() {
                    let searchable = searchable_text(
                        transaction,
                        &accounts,
                        &categories,
                        &groups,
                        &tags,
                        &counterparties,
                    );
                    if !normalize_search_text(&searchable).contains(&needle) {
                        return false;
                    }
                }
                true
            })
            .cloned()
            .collect();

        filtered.sort_by(|left, right| {
            right
                .occurred_at()
                .cmp(&left.occurred_at())
                .then_with(|| right.created_at().cmp(&left.created_at()))
                .then_with(|| left.id().to_string().cmp(&right.id().to_string()))
        });
        Ok(filtered)
    }
}

fn date_bounds(
    calendar: &LedgerCalendar,
    query: &TransactionQuery,
) -> Result<(Option<Timestamp>, Option<Timestamp>), ApplicationError> {
    let start = match query.start_day {
        Some(day) => Some(
            calendar
                .day_interval(day)
                .map_err(ApplicationError::Calendar)?
                .start(),
        ),
        None => None,
    };
    let end_exclusive = match query.end_day {
        Some(day) => Some(
            calendar
                .day_interval(day)
                .map_err(ApplicationError::Calendar)?
                .end_exclusive(),
        ),
        None => None,
    };
    Ok((start, end_exclusive))
}

fn searchable_text(
    transaction: &Transaction,
    accounts: &BTreeMap<AccountId, &Account>,
    categories: &BTreeMap<CategoryId, &Category>,
    groups: &BTreeMap<core_domain::CategoryGroupId, &CategoryGroup>,
    tags: &BTreeMap<TagId, &LedgerTag>,
    counterparties: &BTreeMap<CounterpartyId, &Counterparty>,
) -> String {
    let mut parts: Vec<&str> = Vec::new();
    if let Some(note) = transaction.fields().note.as_deref() {
        parts.push(note);
    }
    if let Some(account) = accounts.get(&transaction.account_id()) {
        parts.push(account.name());
    }
    if let Some(destination_id) = transaction.fields().destination_account_id
        && let Some(account) = accounts.get(&destination_id)
    {
        parts.push(account.name());
    }
    if let Some(category_id) = transaction.fields().category_id
        && let Some(category) = categories.get(&category_id)
    {
        parts.push(category.name());
        if let Some(group_id) = category.options().group_id
            && let Some(group) = groups.get(&group_id)
        {
            parts.push(group.name());
        }
    }
    for tag_id in &transaction.fields().tag_ids {
        if let Some(tag) = tags.get(tag_id) {
            parts.push(tag.name());
        }
    }
    if let Some(counterparty_id) = transaction.fields().counterparty_id
        && let Some(counterparty) = counterparties.get(&counterparty_id)
    {
        parts.push(counterparty.name());
    }
    parts.join(" ")
}

fn normalize_search_text(value: &str) -> String {
    value
        .nfkd()
        .filter(|character| !is_combining_mark(*character))
        .collect::<String>()
        .to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        AccountOptions, AssetAccountType, CategoryKind, CategoryOptions, Currency,
        CurrencyScalePolicy, LedgerCalendar, LedgerSettings, LedgerSettingsId, Money,
        TransactionFields, TransactionId,
    };
    use uuid::Uuid;

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn settings() -> LedgerSettings {
        LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(99)),
            usd(),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    struct Fixture {
        context: TransactionContext,
        transactions: Vec<Transaction>,
        bank: AccountId,
        food: CategoryId,
    }

    fn fixture() -> Fixture {
        let cash = AccountId::from_uuid(Uuid::from_u128(1));
        let bank = AccountId::from_uuid(Uuid::from_u128(2));
        let food = CategoryId::from_uuid(Uuid::from_u128(3));
        let group = core_domain::CategoryGroupId::from_uuid(Uuid::from_u128(4));
        let travel = TagId::from_uuid(Uuid::from_u128(5));
        let alex = CounterpartyId::from_uuid(Uuid::from_u128(6));

        let context = TransactionContext {
            settings: settings(),
            accounts: vec![
                Account::new(
                    cash,
                    "Cash",
                    AssetAccountType::Cash,
                    usd(),
                    AccountOptions::default(),
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
                Account::new(
                    bank,
                    "Bank",
                    AssetAccountType::Checking,
                    usd(),
                    AccountOptions::default(),
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            ],
            categories: vec![
                Category::new(
                    food,
                    "Food",
                    CategoryKind::Expense,
                    CategoryOptions {
                        group_id: Some(group),
                        sort_order: 0,
                        is_archived: false,
                    },
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            ],
            category_groups: vec![
                CategoryGroup::new(
                    group,
                    "Essentials",
                    CategoryKind::Expense,
                    0,
                    false,
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            ],
            tags: vec![
                core_domain::LedgerTag::new(
                    travel,
                    "Travel",
                    "#112233",
                    core_domain::TagOptions::default(),
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            ],
            counterparties: vec![
                Counterparty::new(
                    alex,
                    core_domain::CounterpartyKind::Person,
                    "Alex",
                    0,
                    false,
                    timestamp(1_000),
                    timestamp(1_000),
                )
                .unwrap(),
            ],
        };

        let lunch = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(10)),
            TransactionKind::Expense,
            Money::new(1_234, usd()),
            timestamp(2_000),
            cash,
            TransactionFields {
                category_id: Some(food),
                tag_ids: [travel].into_iter().collect(),
                counterparty_id: Some(alex),
                note: Some("Lunch with Alex".to_string()),
                ..TransactionFields::default()
            },
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap();
        let salary = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(11)),
            TransactionKind::Income,
            Money::new(100_000, usd()),
            timestamp(3_000),
            bank,
            TransactionFields {
                category_id: Some(CategoryId::from_uuid(Uuid::from_u128(7))),
                ..TransactionFields::default()
            },
            timestamp(3_000),
            timestamp(3_000),
        )
        .unwrap();
        let transfer = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(12)),
            TransactionKind::Transfer,
            Money::new(5_000, usd()),
            timestamp(4_000),
            cash,
            TransactionFields {
                destination_account_id: Some(bank),
                ..TransactionFields::default()
            },
            timestamp(4_000),
            timestamp(4_000),
        )
        .unwrap();
        let deleted = lunch.soft_deleted(timestamp(5_000)).unwrap();

        Fixture {
            context,
            transactions: vec![lunch, salary, transfer, deleted],
            bank,
            food,
        }
    }

    fn ids(transactions: &[Transaction]) -> Vec<TransactionId> {
        transactions.iter().map(Transaction::id).collect()
    }

    #[test]
    fn filters_by_kind_account_category_and_tag() {
        let fixture = fixture();
        let use_case = FilterTransactionsUseCase::new();

        let expenses = use_case
            .execute(
                &fixture.transactions,
                &fixture.context,
                &TransactionQuery {
                    kind: Some(TransactionKind::Expense),
                    ..TransactionQuery::default()
                },
            )
            .unwrap();
        assert_eq!(expenses.len(), 2);

        let bank = use_case
            .execute(
                &fixture.transactions,
                &fixture.context,
                &TransactionQuery {
                    account_id: Some(fixture.bank),
                    ..TransactionQuery::default()
                },
            )
            .unwrap();
        assert_eq!(bank.len(), 2);

        let food = use_case
            .execute(
                &fixture.transactions,
                &fixture.context,
                &TransactionQuery {
                    category_id: Some(fixture.food),
                    ..TransactionQuery::default()
                },
            )
            .unwrap();
        assert_eq!(food.len(), 2);
    }

    #[test]
    fn search_matches_note_names_group_and_counterparty() {
        let fixture = fixture();
        let use_case = FilterTransactionsUseCase::new();

        for (needle, expected) in [
            ("lunch", 2),
            ("cash", 3),
            ("essentials", 2),
            ("alex", 2),
            ("travel", 2),
        ] {
            let result = use_case
                .execute(
                    &fixture.transactions,
                    &fixture.context,
                    &TransactionQuery {
                        search_text: needle.to_string(),
                        ..TransactionQuery::default()
                    },
                )
                .unwrap();
            assert_eq!(result.len(), expected, "needle {needle}");
        }
    }

    #[test]
    fn sorts_by_occurrence_descending_then_creation() {
        let fixture = fixture();
        let use_case = FilterTransactionsUseCase::new();
        let result = use_case
            .execute(
                &fixture.transactions,
                &fixture.context,
                &TransactionQuery::default(),
            )
            .unwrap();

        assert_eq!(
            ids(&result),
            vec![
                TransactionId::from_uuid(Uuid::from_u128(12)),
                TransactionId::from_uuid(Uuid::from_u128(11)),
                TransactionId::from_uuid(Uuid::from_u128(10)),
                TransactionId::from_uuid(Uuid::from_u128(10)),
            ]
        );
    }

    #[test]
    fn day_range_is_inclusive_in_the_ledger_time_zone() {
        let fixture = fixture();
        let use_case = FilterTransactionsUseCase::new();
        let day = timestamp(2_000);
        let result = use_case
            .execute(
                &fixture.transactions,
                &fixture.context,
                &TransactionQuery {
                    start_day: Some(day),
                    end_day: Some(day),
                    ..TransactionQuery::default()
                },
            )
            .unwrap();
        // All four events fall on 1970-01-01 in the UTC ledger time zone.
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn rejects_inverted_ranges_and_missing_references() {
        let fixture = fixture();
        let use_case = FilterTransactionsUseCase::new();

        assert_eq!(
            use_case
                .execute(
                    &fixture.transactions,
                    &fixture.context,
                    &TransactionQuery {
                        start_day: Some(timestamp(90_000)),
                        end_day: Some(timestamp(2_000)),
                        ..TransactionQuery::default()
                    },
                )
                .unwrap_err(),
            ApplicationError::InvertedDateRange
        );

        let missing = AccountId::from_uuid(Uuid::from_u128(77));
        assert_eq!(
            use_case
                .execute(
                    &fixture.transactions,
                    &fixture.context,
                    &TransactionQuery {
                        account_id: Some(missing),
                        ..TransactionQuery::default()
                    },
                )
                .unwrap_err(),
            ApplicationError::SelectedAccountNotFound(missing)
        );
    }
}
