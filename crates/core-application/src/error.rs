//! Errors returned by application use cases.

use std::fmt;

use core_domain::{
    AccountGroupValidationError, AccountId, AccountValidationError, BalanceError, CategoryGroupId,
    CategoryGroupValidationError, CategoryId, CategoryKind, CategoryValidationError,
    CounterpartyValidationError, Currency, CurrencyError, ExchangeRateConversionError,
    ExchangeRateQuoteParsingError, ExchangeRateValidationError, LedgerCalendarError,
    LedgerProfileValidationError, LedgerValidationError, MoneyError, MoneyParsingError,
    ReconciliationFlagValidationError, RepositoryError, TagId, TagValidationError, TransactionId,
    TransactionKind, TransactionNotePresetValidationError, TransactionValidationError,
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
    /// The category group does not exist.
    CategoryGroupNotFound(CategoryGroupId),
    /// The category group is archived.
    CategoryGroupArchived(CategoryGroupId),
    /// The account group does not exist.
    AccountGroupNotFound(core_domain::AccountGroupId),
    /// The account group is archived.
    AccountGroupArchived(core_domain::AccountGroupId),
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
    /// The category-group entity rejected the fields.
    CategoryGroup(CategoryGroupValidationError),
    /// The account-group entity rejected the fields.
    AccountGroup(AccountGroupValidationError),
    /// The tag entity rejected the fields.
    Tag(TagValidationError),
    /// The counterparty entity rejected the fields.
    Counterparty(CounterpartyValidationError),
    /// The reconciliation-flag entity rejected the fields.
    ReconciliationFlag(ReconciliationFlagValidationError),
    /// The note-preset entity rejected the fields.
    NotePreset(TransactionNotePresetValidationError),
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
    /// No exchange rate is available for the requested conversion.
    MissingExchangeRate {
        /// The source currency.
        source: Currency,
        /// The reporting currency.
        reporting: Currency,
    },
    /// The exchange-rate conversion failed.
    Conversion(ExchangeRateConversionError),
    /// The exchange rate is invalid.
    ExchangeRate(ExchangeRateValidationError),
    /// The exchange-rate quote is invalid.
    Quote(ExchangeRateQuoteParsingError),
    /// The account balance cannot be calculated.
    Balance(BalanceError),
    /// The ledger invariants are violated.
    Validation(LedgerValidationError),
    /// A money operation failed.
    Money(MoneyError),
    /// The repository failed.
    Repository(RepositoryError),
    /// The ledger profile has not been created yet.
    ProfileNotInitialized,
    /// A time-zone change was not explicitly confirmed.
    TimeZoneChangeNotConfirmed,
    /// The reconciliation flag does not exist.
    ReconciliationFlagNotFound(core_domain::ReconciliationFlagId),
    /// The note preset does not exist.
    TransactionNotePresetNotFound(core_domain::TransactionNotePresetId),
    /// The application version supplied to an export is blank.
    EmptyAppVersion,
    /// The export document could not be serialized.
    Serialization(String),
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
            Self::CategoryGroupNotFound(id) => {
                write!(formatter, "category group not found: {id}")
            }
            Self::CategoryGroupArchived(id) => {
                write!(formatter, "category group is archived: {id}")
            }
            Self::AccountGroupNotFound(id) => {
                write!(formatter, "account group not found: {id}")
            }
            Self::AccountGroupArchived(id) => {
                write!(formatter, "account group is archived: {id}")
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
            Self::CategoryGroup(error) => error.fmt(formatter),
            Self::AccountGroup(error) => error.fmt(formatter),
            Self::Tag(error) => error.fmt(formatter),
            Self::Counterparty(error) => error.fmt(formatter),
            Self::ReconciliationFlag(error) => error.fmt(formatter),
            Self::NotePreset(error) => error.fmt(formatter),
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
            Self::MissingExchangeRate { source, reporting } => write!(
                formatter,
                "missing exchange rate from {} to {}",
                source.code(),
                reporting.code()
            ),
            Self::Conversion(error) => error.fmt(formatter),
            Self::ExchangeRate(error) => error.fmt(formatter),
            Self::Quote(error) => error.fmt(formatter),
            Self::Balance(error) => error.fmt(formatter),
            Self::Validation(error) => error.fmt(formatter),
            Self::Money(error) => error.fmt(formatter),
            Self::Repository(error) => error.fmt(formatter),
            Self::ProfileNotInitialized => formatter.write_str("ledger profile is not initialized"),
            Self::TimeZoneChangeNotConfirmed => {
                formatter.write_str("time-zone change requires explicit confirmation")
            }
            Self::ReconciliationFlagNotFound(id) => {
                write!(formatter, "reconciliation flag not found: {id}")
            }
            Self::TransactionNotePresetNotFound(id) => {
                write!(formatter, "note preset not found: {id}")
            }
            Self::EmptyAppVersion => formatter.write_str("application version must not be empty"),
            Self::Serialization(error) => write!(formatter, "serialization failed: {error}"),
        }
    }
}

impl std::error::Error for ApplicationError {}

impl From<RepositoryError> for ApplicationError {
    fn from(error: RepositoryError) -> Self {
        Self::Repository(error)
    }
}
