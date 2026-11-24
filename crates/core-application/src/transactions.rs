//! Transaction create, update, soft-delete, and list use cases.

use std::collections::BTreeSet;

use core_domain::{
    Account, AccountId, AccountRepository, CategoryId, CategoryKind, CategoryRepository,
    CounterpartyRepository, LedgerSettingsRepository, Money, MoneyParser, TagId, TagRepository,
    Transaction, TransactionFields, TransactionId, TransactionKind, TransactionRepository,
};
use jiff::Timestamp;

use crate::error::ApplicationError;
use crate::ports::{Clock, IdSource};

/// The user-provided transaction fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionDraft {
    /// The event kind.
    pub kind: TransactionKind,
    /// The amount magnitude.
    pub amount_text: String,
    /// The optional destination amount magnitude for transfers.
    pub destination_amount_text: Option<String>,
    /// The event instant.
    pub occurred_at: Timestamp,
    /// The source account.
    pub account_id: AccountId,
    /// The destination account for transfers.
    pub destination_account_id: Option<AccountId>,
    /// The category for expenses and income.
    pub category_id: Option<CategoryId>,
    /// Tags to attach.
    pub tag_ids: BTreeSet<TagId>,
    /// The optional counterparty.
    pub counterparty_id: Option<core_domain::CounterpartyId>,
    /// A free-form note.
    pub note: Option<String>,
}

struct ValidatedReferences {
    source: Account,
    destination: Option<Account>,
}

/// Creates transactions after validating references and amounts.
pub struct CreateTransactionUseCase<'a> {
    transactions: &'a dyn TransactionRepository,
    settings: &'a dyn LedgerSettingsRepository,
    accounts: &'a dyn AccountRepository,
    categories: &'a dyn CategoryRepository,
    tags: Option<&'a dyn TagRepository>,
    counterparties: Option<&'a dyn CounterpartyRepository>,
    parser: MoneyParser,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateTransactionUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        transactions: &'a dyn TransactionRepository,
        settings: &'a dyn LedgerSettingsRepository,
        accounts: &'a dyn AccountRepository,
        categories: &'a dyn CategoryRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            transactions,
            settings,
            accounts,
            categories,
            tags: None,
            counterparties: None,
            parser: MoneyParser::default(),
            clock,
            ids,
        }
    }

    /// Enables tag validation.
    #[must_use]
    pub fn with_tags(mut self, tags: &'a dyn TagRepository) -> Self {
        self.tags = Some(tags);
        self
    }

    /// Enables counterparty validation.
    #[must_use]
    pub fn with_counterparties(mut self, counterparties: &'a dyn CounterpartyRepository) -> Self {
        self.counterparties = Some(counterparties);
        self
    }

    /// Creates and saves a transaction.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the ledger is not initialized,
    /// references are missing or archived, the kind is not supported, or the
    /// amounts are invalid for the kind.
    pub fn execute(&self, draft: &TransactionDraft) -> Result<Transaction, ApplicationError> {
        if draft.kind == TransactionKind::OpeningBalance {
            return Err(ApplicationError::UnsupportedEntryKind(draft.kind));
        }
        if self.settings.settings()?.is_none() {
            return Err(ApplicationError::LedgerNotInitialized);
        }
        self.validate_tags(&draft.tag_ids)?;
        self.validate_counterparty(draft.counterparty_id)?;
        let references = self.validate_references(draft)?;
        let (amount, destination_amount) = self.parse_amounts(draft, &references)?;
        let timestamp = self.clock.now();
        let transaction = Transaction::new(
            TransactionId::from_uuid(self.ids.next_uuid()),
            draft.kind,
            amount,
            draft.occurred_at,
            draft.account_id,
            TransactionFields {
                destination_amount,
                destination_account_id: draft.destination_account_id,
                category_id: draft.category_id,
                tag_ids: draft.tag_ids.clone(),
                reconciliation_flag_id: None,
                counterparty_id: draft.counterparty_id,
                note: draft.note.clone(),
                deleted_at: None,
            },
            timestamp,
            timestamp,
        )
        .map_err(ApplicationError::Transaction)?;
        self.transactions.save_transaction(&transaction)?;
        Ok(transaction)
    }

    fn validate_tags(&self, tag_ids: &BTreeSet<TagId>) -> Result<(), ApplicationError> {
        if tag_ids.is_empty() {
            return Ok(());
        }
        let repository = self
            .tags
            .ok_or(ApplicationError::TagRepositoryUnavailable)?;
        for tag_id in tag_ids {
            let tag = repository
                .tag(*tag_id)?
                .ok_or(ApplicationError::TagNotFound(*tag_id))?;
            if tag.is_archived() {
                return Err(ApplicationError::TagArchived(*tag_id));
            }
        }
        Ok(())
    }

    fn validate_counterparty(
        &self,
        counterparty_id: Option<core_domain::CounterpartyId>,
    ) -> Result<(), ApplicationError> {
        let Some(counterparty_id) = counterparty_id else {
            return Ok(());
        };
        let repository = self
            .counterparties
            .ok_or(ApplicationError::CounterpartyRepositoryUnavailable)?;
        let counterparty = repository
            .counterparty(counterparty_id)?
            .ok_or(ApplicationError::CounterpartyNotFound(counterparty_id))?;
        if counterparty.is_archived() {
            return Err(ApplicationError::CounterpartyArchived(counterparty_id));
        }
        Ok(())
    }

    fn validate_references(
        &self,
        draft: &TransactionDraft,
    ) -> Result<ValidatedReferences, ApplicationError> {
        let source = self
            .accounts
            .account(draft.account_id)?
            .ok_or(ApplicationError::AccountNotFound(draft.account_id))?;
        if source.is_archived() {
            return Err(ApplicationError::AccountArchived(draft.account_id));
        }
        let destination = match draft.destination_account_id {
            Some(id) => {
                let account = self
                    .accounts
                    .account(id)?
                    .ok_or(ApplicationError::DestinationAccountNotFound(id))?;
                if account.is_archived() {
                    return Err(ApplicationError::DestinationAccountArchived(id));
                }
                Some(account)
            }
            None => None,
        };
        let category = match draft.category_id {
            Some(id) => {
                let category = self
                    .categories
                    .category(id)?
                    .ok_or(ApplicationError::CategoryNotFound(id))?;
                if category.is_archived() {
                    return Err(ApplicationError::CategoryArchived(id));
                }
                Some(category)
            }
            None => None,
        };
        if let (Some(category), TransactionKind::Expense | TransactionKind::Income) =
            (category.as_ref(), draft.kind)
        {
            let expected = if draft.kind == TransactionKind::Expense {
                CategoryKind::Expense
            } else {
                CategoryKind::Income
            };
            if category.kind() != expected {
                return Err(ApplicationError::CategoryKindMismatch {
                    expected,
                    actual: category.kind(),
                });
            }
        }
        Ok(ValidatedReferences {
            source,
            destination,
        })
    }

    fn parse_amounts(
        &self,
        draft: &TransactionDraft,
        references: &ValidatedReferences,
    ) -> Result<(Money, Option<Money>), ApplicationError> {
        let amount = self
            .parser
            .parse_magnitude(&draft.amount_text, references.source.currency())
            .map_err(ApplicationError::Amount)?;
        let destination_amount = match draft.kind {
            TransactionKind::Transfer => match references.destination.as_ref() {
                Some(destination) if destination.currency() != references.source.currency() => {
                    let text = draft
                        .destination_amount_text
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .ok_or(ApplicationError::DestinationAmountRequired)?;
                    Some(
                        self.parser
                            .parse_magnitude(text, destination.currency())
                            .map_err(ApplicationError::Amount)?,
                    )
                }
                Some(_) => match draft
                    .destination_amount_text
                    .as_deref()
                    .filter(|value| !value.trim().is_empty())
                {
                    Some(text) => {
                        let parsed = self
                            .parser
                            .parse_magnitude(text, references.source.currency())
                            .map_err(ApplicationError::Amount)?;
                        if parsed != amount {
                            return Err(ApplicationError::SameCurrencyTransferAmountsMustMatch);
                        }
                        Some(parsed)
                    }
                    None => None,
                },
                None => None,
            },
            _ => {
                if draft
                    .destination_amount_text
                    .as_deref()
                    .is_some_and(|value| !value.trim().is_empty())
                {
                    return Err(ApplicationError::DestinationAmountNotAllowed);
                }
                None
            }
        };
        Ok((amount, destination_amount))
    }
}

/// Updates an existing transaction.
pub struct UpdateTransactionUseCase<'a> {
    create: CreateTransactionUseCase<'a>,
    transactions: &'a dyn TransactionRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateTransactionUseCase<'a> {
    /// Creates the use case from the create use case's dependencies.
    #[must_use]
    pub fn new(
        transactions: &'a dyn TransactionRepository,
        settings: &'a dyn LedgerSettingsRepository,
        accounts: &'a dyn AccountRepository,
        categories: &'a dyn CategoryRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            create: CreateTransactionUseCase::new(
                transactions,
                settings,
                accounts,
                categories,
                clock,
                ids,
            ),
            transactions,
            clock,
        }
    }

    /// Enables tag validation.
    #[must_use]
    pub fn with_tags(mut self, tags: &'a dyn TagRepository) -> Self {
        self.create = self.create.with_tags(tags);
        self
    }

    /// Enables counterparty validation.
    #[must_use]
    pub fn with_counterparties(mut self, counterparties: &'a dyn CounterpartyRepository) -> Self {
        self.create = self.create.with_counterparties(counterparties);
        self
    }

    /// Updates and saves a transaction.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the transaction is missing or
    /// deleted, an opening balance changes kind, validation fails, or the
    /// repository fails.
    pub fn execute(
        &self,
        id: TransactionId,
        draft: &TransactionDraft,
    ) -> Result<Transaction, ApplicationError> {
        let existing = self
            .transactions
            .transaction(id)?
            .ok_or(ApplicationError::TransactionNotFound(id))?;
        if existing.is_deleted() {
            return Err(ApplicationError::TransactionDeleted(id));
        }
        if draft.kind == TransactionKind::OpeningBalance {
            return Err(ApplicationError::UnsupportedEntryKind(draft.kind));
        }
        if existing.kind() == TransactionKind::OpeningBalance {
            return Err(ApplicationError::OpeningBalanceKindCannotChange(id));
        }
        if self.create.settings.settings()?.is_none() {
            return Err(ApplicationError::LedgerNotInitialized);
        }
        self.create.validate_tags(&draft.tag_ids)?;
        self.create.validate_counterparty(draft.counterparty_id)?;
        let references = self.create.validate_references(draft)?;
        let (amount, destination_amount) = self.create.parse_amounts(draft, &references)?;
        let timestamp = self.clock.now();
        let updated = Transaction::new(
            existing.id(),
            draft.kind,
            amount,
            draft.occurred_at,
            draft.account_id,
            TransactionFields {
                destination_amount,
                destination_account_id: draft.destination_account_id,
                category_id: draft.category_id,
                tag_ids: draft.tag_ids.clone(),
                reconciliation_flag_id: existing.fields().reconciliation_flag_id,
                counterparty_id: draft.counterparty_id,
                note: draft.note.clone(),
                deleted_at: existing.fields().deleted_at,
            },
            existing.created_at(),
            timestamp,
        )
        .map_err(ApplicationError::Transaction)?;
        self.transactions.save_transaction(&updated)?;
        Ok(updated)
    }
}

/// Soft-deletes or restores a transaction.
pub struct SetTransactionDeletedUseCase<'a> {
    transactions: &'a dyn TransactionRepository,
    clock: &'a dyn Clock,
}

impl<'a> SetTransactionDeletedUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(transactions: &'a dyn TransactionRepository, clock: &'a dyn Clock) -> Self {
        Self {
            transactions,
            clock,
        }
    }

    /// Applies the requested deletion state.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the transaction is missing or the
    /// repository fails.
    pub fn execute(
        &self,
        id: TransactionId,
        is_deleted: bool,
    ) -> Result<Transaction, ApplicationError> {
        let existing = self
            .transactions
            .transaction(id)?
            .ok_or(ApplicationError::TransactionNotFound(id))?;
        if existing.is_deleted() == is_deleted {
            return Ok(existing);
        }
        let timestamp = self.clock.now();
        let updated = if is_deleted {
            existing.soft_deleted(timestamp)
        } else {
            existing.restored(timestamp)
        }
        .map_err(ApplicationError::Transaction)?;
        self.transactions.save_transaction(&updated)?;
        Ok(updated)
    }
}

/// Lists transactions.
pub struct ListTransactionsUseCase<'a> {
    transactions: &'a dyn TransactionRepository,
}

impl<'a> ListTransactionsUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(transactions: &'a dyn TransactionRepository) -> Self {
        Self { transactions }
    }

    /// Returns transactions ordered by occurrence.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the repository fails.
    pub fn execute(&self, include_deleted: bool) -> Result<Vec<Transaction>, ApplicationError> {
        Ok(self.transactions.transactions(include_deleted)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixedClock, SequenceIds};
    use core_domain::{
        AccountId, AccountOptions, AssetAccountType, Category, CategoryOptions, Currency,
        CurrencyScalePolicy, LedgerCalendar, LedgerSettings, LedgerSettingsId,
    };
    use core_testkit::InMemoryLedger;
    use uuid::Uuid;

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    fn clock() -> FixedClock {
        FixedClock::new(Timestamp::from_second(1_000).unwrap())
    }

    fn settings() -> LedgerSettings {
        LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(99)),
            usd(),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            Timestamp::from_second(1_000).unwrap(),
            Timestamp::from_second(1_000).unwrap(),
        )
    }

    struct Fixture {
        ledger: InMemoryLedger,
        account_id: AccountId,
        destination_id: AccountId,
        category_id: CategoryId,
    }

    fn fixture() -> Fixture {
        let ledger = InMemoryLedger::new();
        ledger.seed_settings(settings());
        let account_id = AccountId::from_uuid(Uuid::from_u128(1));
        let destination_id = AccountId::from_uuid(Uuid::from_u128(2));
        let category_id = CategoryId::from_uuid(Uuid::from_u128(3));
        for (id, name) in [(account_id, "Cash"), (destination_id, "Bank")] {
            ledger
                .save_account(
                    &Account::new(
                        id,
                        name,
                        AssetAccountType::Cash,
                        usd(),
                        AccountOptions::default(),
                        Timestamp::from_second(1_000).unwrap(),
                        Timestamp::from_second(1_000).unwrap(),
                    )
                    .unwrap(),
                )
                .unwrap();
        }
        ledger
            .save_category(
                &Category::new(
                    category_id,
                    "Food",
                    CategoryKind::Expense,
                    CategoryOptions::default(),
                    Timestamp::from_second(1_000).unwrap(),
                    Timestamp::from_second(1_000).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        Fixture {
            ledger,
            account_id,
            destination_id,
            category_id,
        }
    }

    fn expense_draft(fixture: &Fixture) -> TransactionDraft {
        TransactionDraft {
            kind: TransactionKind::Expense,
            amount_text: "12.34".to_string(),
            destination_amount_text: None,
            occurred_at: Timestamp::from_second(2_000).unwrap(),
            account_id: fixture.account_id,
            destination_account_id: None,
            category_id: Some(fixture.category_id),
            tag_ids: BTreeSet::new(),
            counterparty_id: None,
            note: Some("  Lunch  ".to_string()),
        }
    }

    #[test]
    fn create_expense_saves_normalized_transaction() {
        let fixture = fixture();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );

        let transaction = use_case.execute(&expense_draft(&fixture)).unwrap();

        assert_eq!(transaction.amount().minor_units(), 1_234);
        assert_eq!(transaction.fields().note.as_deref(), Some("Lunch"));
        assert_eq!(fixture.ledger.transactions(true).unwrap().len(), 1);
    }

    #[test]
    fn opening_balance_kind_is_rejected() {
        let fixture = fixture();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let mut draft = expense_draft(&fixture);
        draft.kind = TransactionKind::OpeningBalance;

        assert_eq!(
            use_case.execute(&draft).unwrap_err(),
            ApplicationError::UnsupportedEntryKind(TransactionKind::OpeningBalance)
        );
    }

    #[test]
    fn same_currency_transfer_requires_matching_destination_amount() {
        let fixture = fixture();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let mut draft = expense_draft(&fixture);
        draft.kind = TransactionKind::Transfer;
        draft.destination_account_id = Some(fixture.destination_id);
        draft.category_id = None;
        draft.destination_amount_text = Some("13.00".to_string());

        assert_eq!(
            use_case.execute(&draft).unwrap_err(),
            ApplicationError::SameCurrencyTransferAmountsMustMatch
        );
    }

    #[test]
    fn cross_currency_transfer_requires_destination_amount() {
        let fixture = fixture();
        let eur = CurrencyScalePolicy::v1().currency("EUR").unwrap();
        let destination = AccountId::from_uuid(Uuid::from_u128(4));
        fixture
            .ledger
            .save_account(
                &Account::new(
                    destination,
                    "Euro",
                    AssetAccountType::Checking,
                    eur,
                    AccountOptions::default(),
                    Timestamp::from_second(1_000).unwrap(),
                    Timestamp::from_second(1_000).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let mut draft = expense_draft(&fixture);
        draft.kind = TransactionKind::Transfer;
        draft.destination_account_id = Some(destination);
        draft.category_id = None;

        assert_eq!(
            use_case.execute(&draft).unwrap_err(),
            ApplicationError::DestinationAmountRequired
        );

        draft.destination_amount_text = Some("10.00".to_string());
        let transaction = use_case.execute(&draft).unwrap();
        assert_eq!(transaction.received_amount().minor_units(), 1_000);
    }

    #[test]
    fn archived_account_is_rejected() {
        let fixture = fixture();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let archived = Account::new(
            fixture.account_id,
            "Cash",
            AssetAccountType::Cash,
            usd(),
            AccountOptions {
                is_archived: true,
                ..AccountOptions::default()
            },
            Timestamp::from_second(1_000).unwrap(),
            Timestamp::from_second(1_000).unwrap(),
        )
        .unwrap();
        fixture.ledger.save_account(&archived).unwrap();

        assert_eq!(
            use_case.execute(&expense_draft(&fixture)).unwrap_err(),
            ApplicationError::AccountArchived(fixture.account_id)
        );
    }

    #[test]
    fn update_preserves_identity_and_soft_delete_round_trips() {
        let fixture = fixture();
        let clock = clock();
        let ids = SequenceIds::new();
        let create = CreateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let created = create.execute(&expense_draft(&fixture)).unwrap();
        let update = UpdateTransactionUseCase::new(
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &fixture.ledger,
            &clock,
            &ids,
        );
        let mut draft = expense_draft(&fixture);
        draft.amount_text = "20.00".to_string();
        let updated = update.execute(created.id(), &draft).unwrap();
        assert_eq!(updated.id(), created.id());
        assert_eq!(updated.created_at(), created.created_at());
        assert_eq!(updated.amount().minor_units(), 2_000);

        let deleted = SetTransactionDeletedUseCase::new(&fixture.ledger, &clock)
            .execute(created.id(), true)
            .unwrap();
        assert!(deleted.is_deleted());
        let restored = SetTransactionDeletedUseCase::new(&fixture.ledger, &clock)
            .execute(created.id(), false)
            .unwrap();
        assert!(!restored.is_deleted());
    }
}
