//! Domain model and policies for Bluewhale.
//!
//! This crate owns money, accounts, categories, transactions, transfers,
//! reports, and the repository traits implemented by the other core crates.
//! It must not depend on persistence, sync, networking, or client code.

pub mod account;
pub mod balance;
pub mod calendar;
pub mod category;
pub mod counterparty;
pub mod currency;
pub mod exchange_rate;
pub mod ids;
pub mod money;
mod naming;
pub mod note_preset;
pub mod profile;
pub mod reconciliation;
pub mod repository;
pub mod settings;
pub mod statistics;
pub mod tag;
pub mod transaction;
pub mod validation;

pub use account::{
    Account, AccountGroup, AccountGroupValidationError, AccountOptions, AccountValidationError,
    AssetAccountType,
};
pub use balance::{BalanceError, account_balance};
pub use calendar::{InstantInterval, LedgerCalendar, LedgerCalendarError};
pub use category::{
    Category, CategoryGroup, CategoryGroupValidationError, CategoryKind, CategoryOptions,
    CategoryValidationError,
};
pub use counterparty::{Counterparty, CounterpartyKind, CounterpartyValidationError};
pub use currency::{Currency, CurrencyError, CurrencyScalePolicy};
pub use exchange_rate::{
    ExactRatio, ExactRatioError, ExchangeRate, ExchangeRateConversionError,
    ExchangeRateQuoteParser, ExchangeRateQuoteParsingError, ExchangeRateSource,
    ExchangeRateValidationError, latest_rate,
};
pub use ids::{
    AccountGroupId, AccountId, CategoryGroupId, CategoryId, CounterpartyId, ExchangeRateId,
    LedgerId, LedgerProfileId, LedgerSettingsId, ReconciliationFlagId, TagId, TransactionId,
    TransactionNotePresetId,
};
pub use money::{
    Money, MoneyError, MoneyParser, MoneyParsingError, MoneyRoundingError, MoneyRoundingPolicy,
    nearest_minor_unit,
};
pub use note_preset::{TransactionNotePreset, TransactionNotePresetValidationError};
pub use profile::{
    LedgerProfile, LedgerProfileValidationError, MAXIMUM_EMOJI_SCALARS, MAXIMUM_NAME_CHARACTERS,
};
pub use reconciliation::{
    ReconciliationFlag, ReconciliationFlagOptions, ReconciliationFlagSystemKind,
    ReconciliationFlagValidationError,
};
pub use repository::{
    AccountGroupRepository, AccountRepository, AccountSetupRepository, CategoryGroupRepository,
    CategoryRepository, CounterpartyRepository, ExchangeRateRepository, LedgerBootstrapRepository,
    LedgerExportRepository, LedgerExportSnapshot, LedgerProfileRepository, LedgerReadSnapshot,
    LedgerReadSnapshotRepository, LedgerSettingsRepository, ReconciliationFlagRepository,
    RepositoryError, TagRepository, TransactionNotePresetRepository, TransactionRepository,
};
pub use settings::{LedgerCalendarIdentifier, LedgerSettings};
pub use statistics::{MonthlyStatistics, MonthlyStatisticsError, monthly_statistics};
pub use tag::{LedgerTag, TagOptions, TagValidationError};
pub use transaction::{
    Transaction, TransactionFields, TransactionKind, TransactionValidationError,
};
pub use validation::{LedgerValidationError, validate_transactions};
