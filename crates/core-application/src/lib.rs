//! Application use cases for Bluewhale.
//!
//! Orchestrates domain operations over repository traits, with time and
//! identifiers injected through ports. The crate has no database or network
//! dependency.

pub mod accounts;
pub mod error;
pub mod filter;
pub mod ledger;
pub mod ports;
#[cfg(test)]
mod test_support;
pub mod transactions;

pub use accounts::{
    CreateAccountOutcome, CreateAccountRequest, CreateAccountWithOpeningBalanceUseCase,
};
pub use error::ApplicationError;
pub use filter::{FilterTransactionsUseCase, TransactionContext, TransactionQuery};
pub use ledger::{
    DefaultLedgerContent, InitializeLedgerRequest, InitializeLedgerUseCase,
    LedgerInitializationOutcome,
};
pub use ports::{Clock, IdSource};
pub use transactions::{
    CreateTransactionUseCase, ListTransactionsUseCase, SetTransactionDeletedUseCase,
    TransactionDraft, UpdateTransactionUseCase,
};
