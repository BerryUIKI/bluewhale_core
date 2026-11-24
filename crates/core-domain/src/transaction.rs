//! Transactions, transfers, and soft deletion.

use std::collections::BTreeSet;
use std::fmt;

use jiff::Timestamp;

use crate::ids::{
    AccountId, CategoryId, CounterpartyId, ReconciliationFlagId, TagId, TransactionId,
};
use crate::money::Money;
use crate::naming::normalize_optional_text;

/// The kind of ledger event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TransactionKind {
    /// An opening balance for an account.
    OpeningBalance,
    /// Money leaving the ledger.
    Expense,
    /// Money entering the ledger.
    Income,
    /// Money moving between two accounts as one event.
    Transfer,
}

/// Errors raised while validating a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransactionValidationError {
    /// The amount is zero or negative.
    AmountMustBePositive,
    /// The destination amount is zero or negative.
    DestinationAmountMustBePositive,
    /// An expense or income has no category.
    CategoryRequired,
    /// The kind does not allow a category.
    CategoryNotAllowed,
    /// A transfer has no destination account.
    DestinationAccountRequired,
    /// The kind does not allow a destination account.
    DestinationAccountNotAllowed,
    /// The kind does not allow a destination amount.
    DestinationAmountNotAllowed,
    /// The source and destination accounts are the same.
    TransferAccountsMustDiffer,
    /// The kind does not allow a counterparty.
    CounterpartyNotAllowed,
}

impl fmt::Display for TransactionValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::AmountMustBePositive => "amount must be positive",
            Self::DestinationAmountMustBePositive => "destination amount must be positive",
            Self::CategoryRequired => "category is required",
            Self::CategoryNotAllowed => "category is not allowed",
            Self::DestinationAccountRequired => "destination account is required",
            Self::DestinationAccountNotAllowed => "destination account is not allowed",
            Self::DestinationAmountNotAllowed => "destination amount is not allowed",
            Self::TransferAccountsMustDiffer => "transfer accounts must differ",
            Self::CounterpartyNotAllowed => "counterparty is not allowed",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for TransactionValidationError {}

/// Optional transaction fields.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TransactionFields {
    /// The amount received by the destination account of a transfer.
    pub destination_amount: Option<Money>,
    /// The destination account of a transfer.
    pub destination_account_id: Option<AccountId>,
    /// The classifying category.
    pub category_id: Option<CategoryId>,
    /// Tags attached to the transaction.
    pub tag_ids: BTreeSet<TagId>,
    /// The reconciliation flag.
    pub reconciliation_flag_id: Option<ReconciliationFlagId>,
    /// The counterparty.
    pub counterparty_id: Option<CounterpartyId>,
    /// A free-form note.
    pub note: Option<String>,
    /// The soft-deletion instant.
    pub deleted_at: Option<Timestamp>,
}

/// One ledger event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    id: TransactionId,
    kind: TransactionKind,
    amount: Money,
    occurred_at: Timestamp,
    account_id: AccountId,
    fields: TransactionFields,
    created_at: Timestamp,
    updated_at: Timestamp,
}

impl Transaction {
    /// Creates a validated transaction.
    ///
    /// # Errors
    ///
    /// Returns a [`TransactionValidationError`] when the amount, category,
    /// destination account, or counterparty does not match the kind's rules.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: TransactionId,
        kind: TransactionKind,
        amount: Money,
        occurred_at: Timestamp,
        account_id: AccountId,
        mut fields: TransactionFields,
        created_at: Timestamp,
        updated_at: Timestamp,
    ) -> Result<Self, TransactionValidationError> {
        if amount.minor_units() <= 0 {
            return Err(TransactionValidationError::AmountMustBePositive);
        }
        if fields
            .destination_amount
            .as_ref()
            .is_some_and(|destination| destination.minor_units() <= 0)
        {
            return Err(TransactionValidationError::DestinationAmountMustBePositive);
        }

        match kind {
            TransactionKind::OpeningBalance => {
                if fields.category_id.is_some() {
                    return Err(TransactionValidationError::CategoryNotAllowed);
                }
                if fields.destination_account_id.is_some() {
                    return Err(TransactionValidationError::DestinationAccountNotAllowed);
                }
                if fields.destination_amount.is_some() {
                    return Err(TransactionValidationError::DestinationAmountNotAllowed);
                }
                if fields.counterparty_id.is_some() {
                    return Err(TransactionValidationError::CounterpartyNotAllowed);
                }
            }
            TransactionKind::Expense | TransactionKind::Income => {
                if fields.category_id.is_none() {
                    return Err(TransactionValidationError::CategoryRequired);
                }
                if fields.destination_account_id.is_some() {
                    return Err(TransactionValidationError::DestinationAccountNotAllowed);
                }
                if fields.destination_amount.is_some() {
                    return Err(TransactionValidationError::DestinationAmountNotAllowed);
                }
            }
            TransactionKind::Transfer => {
                if fields.category_id.is_some() {
                    return Err(TransactionValidationError::CategoryNotAllowed);
                }
                let Some(destination) = fields.destination_account_id else {
                    return Err(TransactionValidationError::DestinationAccountRequired);
                };
                if destination == account_id {
                    return Err(TransactionValidationError::TransferAccountsMustDiffer);
                }
                if fields.counterparty_id.is_some() {
                    return Err(TransactionValidationError::CounterpartyNotAllowed);
                }
            }
        }

        fields.note = normalize_optional_text(fields.note.as_deref());
        Ok(Self {
            id,
            kind,
            amount,
            occurred_at,
            account_id,
            fields,
            created_at,
            updated_at,
        })
    }

    /// Returns the transaction identifier.
    #[must_use]
    pub fn id(&self) -> TransactionId {
        self.id
    }

    /// Returns the event kind.
    #[must_use]
    pub fn kind(&self) -> TransactionKind {
        self.kind
    }

    /// Returns the amount leaving the source account.
    #[must_use]
    pub fn amount(&self) -> &Money {
        &self.amount
    }

    /// Returns the amount received by the destination account, defaulting to
    /// the source amount.
    #[must_use]
    pub fn received_amount(&self) -> Money {
        self.fields
            .destination_amount
            .clone()
            .unwrap_or_else(|| self.amount.clone())
    }

    /// Returns the event instant.
    #[must_use]
    pub fn occurred_at(&self) -> Timestamp {
        self.occurred_at
    }

    /// Returns the source account.
    #[must_use]
    pub fn account_id(&self) -> AccountId {
        self.account_id
    }

    /// Returns the optional fields.
    #[must_use]
    pub fn fields(&self) -> &TransactionFields {
        &self.fields
    }

    /// Returns whether the transaction is soft-deleted.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.fields.deleted_at.is_some()
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

    /// Returns a soft-deleted copy.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the unchanged fields are invalid.
    pub fn soft_deleted(&self, at: Timestamp) -> Result<Self, TransactionValidationError> {
        self.replacing_deletion(Some(at), at)
    }

    /// Returns a restored copy.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the unchanged fields are invalid.
    pub fn restored(&self, at: Timestamp) -> Result<Self, TransactionValidationError> {
        self.replacing_deletion(None, at)
    }

    /// Returns a copy with a different reconciliation flag.
    ///
    /// # Errors
    ///
    /// Returns a validation error if the unchanged fields are invalid.
    pub fn replacing_reconciliation_flag(
        &self,
        reconciliation_flag_id: Option<ReconciliationFlagId>,
        at: Timestamp,
    ) -> Result<Self, TransactionValidationError> {
        let mut fields = self.fields.clone();
        fields.reconciliation_flag_id = reconciliation_flag_id;
        Self::new(
            self.id,
            self.kind,
            self.amount.clone(),
            self.occurred_at,
            self.account_id,
            fields,
            self.created_at,
            at,
        )
    }

    fn replacing_deletion(
        &self,
        deleted_at: Option<Timestamp>,
        updated_at: Timestamp,
    ) -> Result<Self, TransactionValidationError> {
        let mut fields = self.fields.clone();
        fields.deleted_at = deleted_at;
        Self::new(
            self.id,
            self.kind,
            self.amount.clone(),
            self.occurred_at,
            self.account_id,
            fields,
            self.created_at,
            updated_at,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use crate::money::Money;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn usd_minor(minor_units: i64) -> Money {
        Money::new(
            minor_units,
            CurrencyScalePolicy::v1().currency("USD").unwrap(),
        )
    }

    fn id(value: u128) -> AccountId {
        AccountId::from_uuid(Uuid::from_u128(value))
    }

    fn transaction(
        kind: TransactionKind,
        minor_units: i64,
        fields: TransactionFields,
    ) -> Result<Transaction, TransactionValidationError> {
        Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(10)),
            kind,
            usd_minor(minor_units),
            timestamp(1_000),
            id(1),
            fields,
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    #[test]
    fn expense_requires_category_and_positive_amount() {
        assert_eq!(
            transaction(TransactionKind::Expense, 100, TransactionFields::default()).unwrap_err(),
            TransactionValidationError::CategoryRequired
        );
        assert_eq!(
            transaction(
                TransactionKind::Expense,
                0,
                TransactionFields {
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(2))),
                    ..TransactionFields::default()
                }
            )
            .unwrap_err(),
            TransactionValidationError::AmountMustBePositive
        );
    }

    #[test]
    fn transfer_is_one_event_with_distinct_accounts_and_no_category() {
        let transfer = transaction(
            TransactionKind::Transfer,
            500,
            TransactionFields {
                destination_account_id: Some(id(2)),
                ..TransactionFields::default()
            },
        )
        .unwrap();
        assert_eq!(transfer.account_id(), id(1));
        assert_eq!(transfer.received_amount(), usd_minor(500));

        assert_eq!(
            transaction(
                TransactionKind::Transfer,
                500,
                TransactionFields {
                    destination_account_id: Some(id(1)),
                    ..TransactionFields::default()
                }
            )
            .unwrap_err(),
            TransactionValidationError::TransferAccountsMustDiffer
        );
        assert_eq!(
            transaction(
                TransactionKind::Transfer,
                500,
                TransactionFields {
                    destination_account_id: Some(id(2)),
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(2))),
                    ..TransactionFields::default()
                }
            )
            .unwrap_err(),
            TransactionValidationError::CategoryNotAllowed
        );
    }

    #[test]
    fn kind_specific_fields_are_rejected() {
        let category = Some(CategoryId::from_uuid(Uuid::from_u128(2)));
        let destination = Some(id(2));
        let counterparty = Some(CounterpartyId::from_uuid(Uuid::from_u128(3)));

        for (kind, fields, expected) in [
            (
                TransactionKind::OpeningBalance,
                TransactionFields {
                    category_id: category,
                    ..TransactionFields::default()
                },
                TransactionValidationError::CategoryNotAllowed,
            ),
            (
                TransactionKind::OpeningBalance,
                TransactionFields {
                    destination_account_id: destination,
                    ..TransactionFields::default()
                },
                TransactionValidationError::DestinationAccountNotAllowed,
            ),
            (
                TransactionKind::OpeningBalance,
                TransactionFields {
                    destination_amount: Some(usd_minor(10)),
                    ..TransactionFields::default()
                },
                TransactionValidationError::DestinationAmountNotAllowed,
            ),
            (
                TransactionKind::OpeningBalance,
                TransactionFields {
                    counterparty_id: counterparty,
                    ..TransactionFields::default()
                },
                TransactionValidationError::CounterpartyNotAllowed,
            ),
            (
                TransactionKind::Expense,
                TransactionFields {
                    category_id: category,
                    destination_account_id: destination,
                    ..TransactionFields::default()
                },
                TransactionValidationError::DestinationAccountNotAllowed,
            ),
            (
                TransactionKind::Income,
                TransactionFields {
                    category_id: category,
                    destination_amount: Some(usd_minor(10)),
                    ..TransactionFields::default()
                },
                TransactionValidationError::DestinationAmountNotAllowed,
            ),
            (
                TransactionKind::Transfer,
                TransactionFields::default(),
                TransactionValidationError::DestinationAccountRequired,
            ),
            (
                TransactionKind::Transfer,
                TransactionFields {
                    destination_account_id: destination,
                    counterparty_id: counterparty,
                    ..TransactionFields::default()
                },
                TransactionValidationError::CounterpartyNotAllowed,
            ),
            (
                TransactionKind::Transfer,
                TransactionFields {
                    destination_account_id: destination,
                    destination_amount: Some(usd_minor(0)),
                    ..TransactionFields::default()
                },
                TransactionValidationError::DestinationAmountMustBePositive,
            ),
        ] {
            assert_eq!(
                transaction(kind, 100, fields).unwrap_err(),
                expected,
                "{kind:?}"
            );
        }
    }

    #[test]
    fn cross_currency_transfer_keeps_exact_destination_amount() {
        let cny = Money::new(10_000, CurrencyScalePolicy::v1().currency("CNY").unwrap());
        let usd = usd_minor(1_400);
        let transfer = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(11)),
            TransactionKind::Transfer,
            cny,
            timestamp(1_000),
            id(1),
            TransactionFields {
                destination_account_id: Some(id(2)),
                destination_amount: Some(usd.clone()),
                ..TransactionFields::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();

        assert_eq!(transfer.received_amount(), usd);
    }

    #[test]
    fn soft_deletion_and_restoration_preserve_the_event() {
        let original = transaction(
            TransactionKind::Income,
            1_000,
            TransactionFields {
                category_id: Some(CategoryId::from_uuid(Uuid::from_u128(2))),
                ..TransactionFields::default()
            },
        )
        .unwrap();
        let deleted = original.soft_deleted(timestamp(2_000)).unwrap();
        let restored = deleted.restored(timestamp(3_000)).unwrap();

        assert!(deleted.is_deleted());
        assert!(!restored.is_deleted());
        assert_eq!(restored.id(), original.id());
        assert_eq!(restored.amount(), original.amount());
    }
}
