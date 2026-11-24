//! Application use cases for Bluewhale.
//!
//! Orchestrates domain operations over repository traits, with time and
//! identifiers injected through ports. The crate has no database or network
//! dependency.

pub mod accounts;
pub mod dashboard;
pub mod error;
pub mod exchange_rates;
pub mod filter;
pub mod ledger;
pub mod management;
pub mod ports;
#[cfg(test)]
mod test_support;
pub mod transactions;

pub use accounts::{
    CreateAccountOutcome, CreateAccountRequest, CreateAccountWithOpeningBalanceUseCase,
};
pub use dashboard::{
    AccountBalanceSummary, DailyCashFlowSummary, LedgerDashboard, LoadLedgerDashboardUseCase,
};
pub use error::ApplicationError;
pub use exchange_rates::{
    HistoricalMoneyConverter, ListExchangeRatesUseCase, SetManualExchangeRateRequest,
    SetManualExchangeRateUseCase,
};
pub use filter::{FilterTransactionsUseCase, TransactionContext, TransactionQuery};
pub use ledger::{
    DefaultLedgerContent, InitializeLedgerRequest, InitializeLedgerUseCase,
    LedgerInitializationOutcome,
};
pub use management::{
    CreateAccountGroupRequest, CreateAccountGroupUseCase, CreateCategoryGroupRequest,
    CreateCategoryGroupUseCase, CreateCategoryRequest, CreateCategoryUseCase,
    CreateCounterpartyRequest, CreateCounterpartyUseCase, CreateTagRequest, CreateTagUseCase,
    UpdateAccountGroupRequest, UpdateAccountGroupUseCase, UpdateAccountRequest,
    UpdateAccountUseCase, UpdateCategoryGroupRequest, UpdateCategoryGroupUseCase,
    UpdateCategoryRequest, UpdateCategoryUseCase, UpdateCounterpartyRequest,
    UpdateCounterpartyUseCase, UpdateTagRequest, UpdateTagUseCase,
};
pub use ports::{Clock, IdSource};
pub use transactions::{
    CreateTransactionUseCase, ListTransactionsUseCase, SetTransactionDeletedUseCase,
    TransactionDraft, UpdateTransactionUseCase,
};
