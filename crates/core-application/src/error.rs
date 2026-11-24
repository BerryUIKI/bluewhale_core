//! Errors returned by application use cases.

use std::fmt;

use core_domain::{
    AccountId, AccountValidationError, CategoryId, CategoryKind, CategoryValidationError,
    CurrencyError, LedgerCalendarError, LedgerProfileValidationError, MoneyParsingError,
    RepositoryError, TagId, TransactionId, TransactionKind, TransactionValidationError,
};

/// A failure from a use case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApplicationError {
    /// The ledger has no settings yet.
    LedgerNotInitialized,
    /// The entry kind cannot be created through this use case.
    UnsupportedEntryKind(TransactionKind),
    /// An opening balance cannot change its kind.
    OpeningBalanceKindCannotChange(TransactionId),
    /// The source account does not exist.
    AccountNotFound(AccountId),
    /// The source account is archived.
    AccountArchived(AccountId),
    /// The destination account does not exist.
    DestinationAccountNotFound(AccountId),
    /// The destination account is archived.
    DestinationAccountArchived(AccountId),
    /// The category does not exist.
    CategoryNotFound(CategoryId),
    /// The category is archived.
    CategoryArchived(CategoryId),
    /// The category's kind does not match the transaction.
    CategoryKindMismatch {
        /// The kind required by the transaction.
        expected: CategoryKind,
        /// The category's actual kind.
        actual: CategoryKind,
    },
    /// A tag was supplied but no tag repository is available.
    TagRepositoryUnavailable,
    /// The tag does not exist.
    TagNotFound(TagId),
    /// The tag is archived.
    TagArchived(TagId),
    /// A counterparty was supplied but no counterparty repository is available.
    CounterpartyRepositoryUnavailable,
    /// The counterparty does not exist.
    CounterpartyNotFound(core_domain::CounterpartyId),
    /// The counterparty is archived.
    CounterpartyArchived(core_domain::CounterpartyId),
    /// The transaction does not exist.
    TransactionNotFound(TransactionId),
    /// The transaction is soft-deleted.
    TransactionDeleted(TransactionId),
    /// A cross-currency transfer needs a destination amount.
    DestinationAmountRequired,
    /// The entry kind does not allow a destination amount.
    DestinationAmountNotAllowed,
    /// A same-currency transfer must move equal amounts.
    SameCurrencyTransferAmountsMustMatch,
    /// The amount text is invalid.
    Amount(MoneyParsingError),
    /// The transaction entity rejected the fields.
    Transaction(TransactionValidationError),
    /// The account entity rejected the fields.
    Account(AccountValidationError),
    /// The category entity rejected the fields.
    Category(CategoryValidationError),
    /// The profile entity rejected the fields.
    Profile(LedgerProfileValidationError),
    /// The calendar boundary cannot be represented.
    Calendar(LedgerCalendarError),
    /// The currency is not supported by the policy.
    Currency(CurrencyError),
    /// The default expense categories are empty.
    EmptyExpenseCategoryDefaults,
    /// The default income categories are empty.
    EmptyIncomeCategoryDefaults,
    /// Another writer initialized the ledger and settings cannot be read.
    MissingSettingsAfterConcurrentInitialization,
    /// The query's day range is inverted.
    InvertedDateRange,
    /// The selected account does not exist in the context.
    SelectedAccountNotFound(AccountId),
    /// The selected category does not exist in the context.
    SelectedCategoryNotFound(CategoryId),
    /// The selected tag does not exist in the context.
    SelectedTagNotFound(TagId),
    /// The selected counterparty does not exist in the context.
    SelectedCounterpartyNotFound(core_domain::CounterpartyId),
    /// The repository failed.
    Repository(RepositoryError),
}

impl fmt::Display for ApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LedgerNotInitialized => formatter.write_str("ledger is not initialized"),
            Self::UnsupportedEntryKind(kind) => {
                write!(formatter, "unsupported entry kind: {kind:?}")
            }
            Self::OpeningBalanceKindCannotChange(id) => {
                write!(formatter, "opening balance {id} cannot change kind")
            }
            Self::AccountNotFound(id) => write!(formatter, "account not found: {id}"),
            Self::AccountArchived(id) => write!(formatter, "account is archived: {id}"),
            Self::DestinationAccountNotFound(id) => {
                write!(formatter, "destination account not found: {id}")
            }
            Self::DestinationAccountArchived(id) => {
                write!(formatter, "destination account is archived: {id}")
            }
            Self::CategoryNotFound(id) => write!(formatter, "category not found: {id}"),
            Self::CategoryArchived(id) => write!(formatter, "category is archived: {id}"),
            Self::CategoryKindMismatch { expected, actual } => {
                write!(
                    formatter,
                    "category kind mismatch: expected {expected:?}, got {actual:?}"
                )
            }
            Self::TagRepositoryUnavailable => formatter.write_str("tag repository unavailable"),
            Self::TagNotFound(id) => write!(formatter, "tag not found: {id}"),
            Self::TagArchived(id) => write!(formatter, "tag is archived: {id}"),
            Self::CounterpartyRepositoryUnavailable => {
                formatter.write_str("counterparty repository unavailable")
            }
            Self::CounterpartyNotFound(id) => write!(formatter, "counterparty not found: {id}"),
            Self::CounterpartyArchived(id) => {
                write!(formatter, "counterparty is archived: {id}")
            }
            Self::TransactionNotFound(id) => write!(formatter, "transaction not found: {id}"),
            Self::TransactionDeleted(id) => write!(formatter, "transaction is deleted: {id}"),
            Self::DestinationAmountRequired => {
                formatter.write_str("destination amount is required")
            }
            Self::DestinationAmountNotAllowed => {
                formatter.write_str("destination amount is not allowed")
            }
            Self::SameCurrencyTransferAmountsMustMatch => {
                formatter.write_str("same-currency transfer amounts must match")
            }
            Self::Amount(error) => error.fmt(formatter),
            Self::Transaction(error) => error.fmt(formatter),
            Self::Account(error) => error.fmt(formatter),
            Self::Category(error) => error.fmt(formatter),
            Self::Profile(error) => error.fmt(formatter),
            Self::Calendar(error) => error.fmt(formatter),
            Self::Currency(error) => error.fmt(formatter),
            Self::EmptyExpenseCategoryDefaults => {
                formatter.write_str("default expense categories are empty")
            }
            Self::EmptyIncomeCategoryDefaults => {
                formatter.write_str("default income categories are empty")
            }
            Self::MissingSettingsAfterConcurrentInitialization => {
                formatter.write_str("settings missing after concurrent initialization")
            }
            Self::InvertedDateRange => formatter.write_str("inverted date range"),
            Self::SelectedAccountNotFound(id) => {
                write!(formatter, "selected account not found: {id}")
            }
            Self::SelectedCategoryNotFound(id) => {
                write!(formatter, "selected category not found: {id}")
            }
            Self::SelectedTagNotFound(id) => write!(formatter, "selected tag not found: {id}"),
            Self::SelectedCounterpartyNotFound(id) => {
                write!(formatter, "selected counterparty not found: {id}")
            }
            Self::Repository(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<RepositoryError> for ApplicationError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}
