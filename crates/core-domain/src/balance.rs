//! Account balances derived from active ledger events.

use std::fmt;

use crate::currency::Currency;
use crate::ids::AccountId;
use crate::money::{Money, MoneyError};
use crate::transaction::{Transaction, TransactionKind};
use crate::validation::{LedgerValidationError, validate_transactions};

/// Errors raised while calculating a balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BalanceError {
    /// The ledger invariants are violated.
    Validation(LedgerValidationError),
    /// An amount uses a different currency or overflows.
    Money(MoneyError),
}

impl fmt::Display for BalanceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Validation(error) => error.fmt(formatter),
            Self::Money(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for BalanceError {}

/// Calculates an account balance from active transactions.
///
/// Opening balances and income add to the source account, expenses subtract,
/// and transfers subtract from the source and add the received amount to the
/// destination.
///
/// # Errors
///
/// Returns a [`BalanceError`] when the ledger invariants are violated or an
/// amount cannot be combined with the account currency.
pub fn account_balance(
    account_id: AccountId,
    currency: &Currency,
    transactions: &[Transaction],
) -> Result<Money, BalanceError> {
    validate_transactions(transactions).map_err(BalanceError::Validation)?;

    let mut total = Money::zero(currency.clone());
    for transaction in transactions {
        if transaction.is_deleted() {
            continue;
        }
        match transaction.kind() {
            TransactionKind::OpeningBalance | TransactionKind::Income => {
                if transaction.account_id() == account_id {
                    total = total
                        .adding(transaction.amount())
                        .map_err(BalanceError::Money)?;
                }
            }
            TransactionKind::Expense => {
                if transaction.account_id() == account_id {
                    total = total
                        .subtracting(transaction.amount())
                        .map_err(BalanceError::Money)?;
                }
            }
            TransactionKind::Transfer => {
                if transaction.account_id() == account_id {
                    total = total
                        .subtracting(transaction.amount())
                        .map_err(BalanceError::Money)?;
                } else if transaction.fields().destination_account_id == Some(account_id) {
                    total = total
                        .adding(&transaction.received_amount())
                        .map_err(BalanceError::Money)?;
                }
            }
        }
    }
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use crate::ids::{CategoryId, TransactionId};
    use crate::transaction::TransactionFields;
    use jiff::Timestamp;
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
        currency: Currency,
        fields: TransactionFields,
    ) -> Transaction {
        Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(id)),
            kind,
            Money::new(minor_units, currency),
            timestamp(1_000),
            account(1),
            fields,
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    #[test]
    fn balance_applies_directions_and_atomic_transfer() {
        let source = account(1);
        let destination = account(2);
        let transactions = vec![
            transaction(
                1,
                TransactionKind::OpeningBalance,
                10_000,
                usd(),
                TransactionFields::default(),
            ),
            transaction(
                2,
                TransactionKind::Income,
                5_000,
                usd(),
                TransactionFields {
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(3))),
                    ..TransactionFields::default()
                },
            ),
            transaction(
                3,
                TransactionKind::Expense,
                1_200,
                usd(),
                TransactionFields {
                    category_id: Some(CategoryId::from_uuid(Uuid::from_u128(4))),
                    ..TransactionFields::default()
                },
            ),
            transaction(
                4,
                TransactionKind::Transfer,
                2_000,
                usd(),
                TransactionFields {
                    destination_account_id: Some(destination),
                    ..TransactionFields::default()
                },
            ),
        ];

        assert_eq!(
            account_balance(source, &usd(), &transactions)
                .unwrap()
                .minor_units(),
            11_800
        );
        assert_eq!(
            account_balance(destination, &usd(), &transactions)
                .unwrap()
                .minor_units(),
            2_000
        );
    }

    #[test]
    fn soft_deleted_transactions_are_excluded() {
        let source = account(1);
        let expense = transaction(
            1,
            TransactionKind::Expense,
            500,
            usd(),
            TransactionFields {
                category_id: Some(CategoryId::from_uuid(Uuid::from_u128(2))),
                ..TransactionFields::default()
            },
        );
        let deleted = expense.soft_deleted(timestamp(2_000)).unwrap();

        assert_eq!(
            account_balance(source, &usd(), &[deleted])
                .unwrap()
                .minor_units(),
            0
        );
    }

    #[test]
    fn cross_currency_transfer_uses_destination_amount() {
        let source = account(1);
        let destination = account(2);
        let policy = CurrencyScalePolicy::v1();
        let cny = policy.currency("CNY").unwrap();
        let usd = policy.currency("USD").unwrap();
        let transfer = Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(1)),
            TransactionKind::Transfer,
            Money::new(10_000, cny.clone()),
            timestamp(1_000),
            source,
            TransactionFields {
                destination_account_id: Some(destination),
                destination_amount: Some(Money::new(1_400, usd.clone())),
                ..TransactionFields::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap();

        let transfer_slice = std::slice::from_ref(&transfer);
        assert_eq!(
            account_balance(source, &cny, transfer_slice)
                .unwrap()
                .minor_units(),
            -10_000
        );
        assert_eq!(
            account_balance(destination, &usd, transfer_slice)
                .unwrap()
                .minor_units(),
            1_400
        );
    }
}
