//! Domain model and policies for Bluewhale.
//!
//! This crate owns money, accounts, categories, transactions, transfers,
//! reports, and the repository traits implemented by the other core crates.
//! It must not depend on persistence, sync, networking, or client code.

pub mod currency;
pub mod money;

pub use currency::{Currency, CurrencyError, CurrencyScalePolicy};
pub use money::{
    Money, MoneyError, MoneyParser, MoneyParsingError, MoneyRoundingError, MoneyRoundingPolicy,
    nearest_minor_unit,
};
