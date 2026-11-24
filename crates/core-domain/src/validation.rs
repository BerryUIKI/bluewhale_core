//! Ledger-level invariants that span multiple transactions.

use std::collections::HashSet;
use std::fmt;

use crate::ids::AccountId;
use crate::transaction::{Transaction, TransactionKind};

/// Errors raised while validating a ledger's transactions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LedgerValidationError {
    /// More than one active opening balance exists for one account.
    DuplicateActiveOpeningBalance(AccountId),
}

impl fmt::Display for LedgerValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateActiveOpeningBalance(account_id) => write!(
                formatter,
                "duplicate active opening balance for account {account_id}"
            ),
        }
    }
}

impl std::error::Error for LedgerValidationError {}

/// Validates ledger invariants across the supplied transactions.
///
/// # Errors
///
/// Returns [`LedgerValidationError::DuplicateActiveOpeningBalance`] when an
/// account has more than one active opening balance.
pub fn validate_transactions(transactions: &[Transaction]) -> Result<(), LedgerValidationError> {
    let mut accounts_with_opening_balance = HashSet::new();
    for transaction in transactions {
        if transaction.is_deleted() || transaction.kind() != TransactionKind::OpeningBalance {
            continue;
        }
        if !accounts_with_opening_balance.insert(transaction.account_id()) {
            return Err(LedgerValidationError::DuplicateActiveOpeningBalance(
                transaction.account_id(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::currency::CurrencyScalePolicy;
    use crate::ids::TransactionId;
    use crate::money::Money;
    use crate::transaction::TransactionFields;
    use jiff::Timestamp;
    use uuid::Uuid;

    fn timestamp(second: i64) -> Timestamp {
        Timestamp::from_second(second).unwrap()
    }

    fn opening_balance(account: u128, minor_units: i64) -> Transaction {
        Transaction::new(
            TransactionId::from_uuid(Uuid::from_u128(account)),
            TransactionKind::OpeningBalance,
            Money::new(
                minor_units,
                CurrencyScalePolicy::v1().currency("USD").unwrap(),
            ),
            timestamp(1_000),
            AccountId::from_uuid(Uuid::from_u128(account)),
            TransactionFields::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    #[test]
    fn duplicate_active_opening_balances_are_rejected() {
        let first = opening_balance(1, 100);
        let second = opening_balance(1, 200);

        assert_eq!(
            validate_transactions(&[first, second]),
            Err(LedgerValidationError::DuplicateActiveOpeningBalance(
                AccountId::from_uuid(Uuid::from_u128(1))
            ))
        );
    }

    #[test]
    fn soft_deleted_opening_balances_do_not_conflict() {
        let first = opening_balance(1, 100);
        let deleted = first.soft_deleted(timestamp(2_000)).unwrap();

        assert_eq!(validate_transactions(&[first, deleted]), Ok(()));
    }
}
