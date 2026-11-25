//! Application use cases for Bluewhale.
//!
//! Orchestrates domain operations over repository traits, with time and
//! identifiers injected through ports. The crate has no database or network
//! dependency.

pub mod accounts;
pub mod dashboard;
pub mod error;
pub mod exchange_rates;
pub mod export;
pub mod filter;
pub mod ledger;
pub mod management;
pub mod ports;
pub mod sync_payload;
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
pub use export::{
    EXPORT_VERSION, ExportAccount, ExportAccountGroup, ExportAssetType, ExportCalendar,
    ExportCategory, ExportCategoryGroup, ExportCategoryKind, ExportCounterparty,
    ExportCounterpartyKind, ExportDocument, ExportExchangeRate, ExportExchangeRateSource,
    ExportLedgerUseCase, ExportMoney, ExportNotePreset, ExportProfile, ExportReconciliationFlag,
    ExportReconciliationFlagKind, ExportSettings, ExportTag, ExportTransaction,
    ExportTransactionKind,
};
pub use filter::{FilterTransactionsUseCase, TransactionContext, TransactionQuery};
pub use ledger::{
    DefaultLedgerContent, InitializeLedgerRequest, InitializeLedgerUseCase,
    LedgerInitializationOutcome, UpdateLedgerProfileRequest, UpdateLedgerProfileUseCase,
    UpdateLedgerSettingsRequest, UpdateLedgerSettingsUseCase,
};
pub use management::{
    CreateAccountGroupRequest, CreateAccountGroupUseCase, CreateCategoryGroupRequest,
    CreateCategoryGroupUseCase, CreateCategoryRequest, CreateCategoryUseCase,
    CreateCounterpartyRequest, CreateCounterpartyUseCase, CreateReconciliationFlagRequest,
    CreateReconciliationFlagUseCase, CreateTagRequest, CreateTagUseCase,
    CreateTransactionNotePresetRequest, CreateTransactionNotePresetUseCase,
    UpdateAccountGroupRequest, UpdateAccountGroupUseCase, UpdateAccountRequest,
    UpdateAccountUseCase, UpdateCategoryGroupRequest, UpdateCategoryGroupUseCase,
    UpdateCategoryRequest, UpdateCategoryUseCase, UpdateCounterpartyRequest,
    UpdateCounterpartyUseCase, UpdateReconciliationFlagRequest, UpdateReconciliationFlagUseCase,
    UpdateTagRequest, UpdateTagUseCase, UpdateTransactionNotePresetRequest,
    UpdateTransactionNotePresetUseCase,
};
pub use ports::{Clock, IdSource};
pub use sync_payload::{SyncPayloadError, decode_entity, encode_entity, entities_from_document};
pub use transactions::{
    CreateTransactionUseCase, ListTransactionsUseCase, SetTransactionDeletedUseCase,
    TransactionDraft, UpdateTransactionUseCase,
};
