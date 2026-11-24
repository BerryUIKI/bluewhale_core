//! Domain model and policies for Bluewhale.
//!
//! This crate owns money, accounts, categories, transactions, transfers,
//! reports, and the repository traits implemented by the other core crates.
//! It must not depend on persistence, sync, networking, or client code.

pub mod calendar;
pub mod currency;
pub mod ids;
pub mod money;
pub mod profile;
pub mod settings;

pub use calendar::{InstantInterval, LedgerCalendar, LedgerCalendarError};
pub use currency::{Currency, CurrencyError, CurrencyScalePolicy};
pub use ids::{
    AccountGroupId, AccountId, CategoryGroupId, CategoryId, CounterpartyId, ExchangeRateId,
    LedgerId, LedgerProfileId, LedgerSettingsId, ReconciliationFlagId, TagId, TransactionId,
    TransactionNotePresetId,
};
pub use money::{
    Money, MoneyError, MoneyParser, MoneyParsingError, MoneyRoundingError, MoneyRoundingPolicy,
    nearest_minor_unit,
};
pub use profile::{
    LedgerProfile, LedgerProfileValidationError, MAXIMUM_EMOJI_SCALARS, MAXIMUM_NAME_CHARACTERS,
};
pub use settings::{LedgerCalendarIdentifier, LedgerSettings};
