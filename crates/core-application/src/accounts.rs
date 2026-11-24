//! Account creation with an optional opening balance.

use core_domain::{
    Account, AccountId, AccountOptions, AccountSetupRepository, AssetAccountType, Currency,
    LedgerSettingsRepository, MoneyParser, Transaction, TransactionFields, TransactionId,
    TransactionKind,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, IdSource};

/// The request to create an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccountRequest {
    /// The account name.
    pub name: String,
    /// The asset type.
    pub asset_type: AssetAccountType,
    /// The account currency.
    pub currency: Currency,
    /// The optional account fields.
    pub options: AccountOptions,
    /// The optional opening balance magnitude.
    pub opening_balance_text: Option<String>,
}

/// The created account and its optional opening balance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccountOutcome {
    /// The created account.
    pub account: Account,
    /// The created opening balance, when the text was non-zero.
    pub opening_balance: Option<Transaction>,
}

/// Creates an account and its opening balance atomically.
pub struct CreateAccountWithOpeningBalanceUseCase<'a> {
    settings: &'a dyn LedgerSettingsRepository,
    repository: &'a dyn AccountSetupRepository,
    parser: MoneyParser,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateAccountWithOpeningBalanceUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        settings: &'a dyn LedgerSettingsRepository,
        repository: &'a dyn AccountSetupRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            settings,
            repository,
            parser: MoneyParser::default(),
            clock,
            ids,
        }
    }

    /// Creates the account.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the ledger is not initialized, the
    /// name is empty, the opening balance is invalid, or the repository fails.
    pub fn execute(
        &self,
        request: &CreateAccountRequest,
    ) -> Result<CreateAccountOutcome, ApplicationError> {
        if self.settings.settings()?.is_none() {
            return Err(ApplicationError::LedgerNotInitialized);
        }
        let timestamp = self.clock.now();
        let account = Account::new(
            AccountId::from_uuid(self.ids.next_uuid()),
            &request.name,
            request.asset_type,
            request.currency.clone(),
            request.options.clone(),
            timestamp,
            timestamp,
        )
        .map_err(ApplicationError::Account)?;

        let opening_balance = match request.opening_balance_text.as_deref() {
            Some(text) if !text.trim().is_empty() => {
                let amount = self
                    .parser
                    .parse_magnitude(text, &request.currency)
                    .map_err(ApplicationError::Amount)?;
                if amount.minor_units() == 0 {
                    None
                } else {
                    Some(
                        Transaction::new(
                            TransactionId::from_uuid(self.ids.next_uuid()),
                            TransactionKind::OpeningBalance,
                            amount,
                            timestamp,
                            account.id(),
                            TransactionFields::default(),
                            timestamp,
                            timestamp,
                        )
                        .map_err(ApplicationError::Transaction)?,
                    )
                }
            }
            _ => None,
        };

        self.repository
            .create_account_with_opening_balance(&account, opening_balance.as_ref())?;
        Ok(CreateAccountOutcome {
            account,
            opening_balance,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixedClock, SequenceIds};
    use core_domain::{
        AccountRepository, CurrencyScalePolicy, LedgerCalendar, LedgerSettings, LedgerSettingsId,
        TransactionRepository,
    };
    use core_testkit::InMemoryLedger;
    use jiff::Timestamp;
    use uuid::Uuid;

    fn usd() -> Currency {
        CurrencyScalePolicy::v1().currency("USD").unwrap()
    }

    fn clock() -> FixedClock {
        FixedClock::new(Timestamp::from_second(1_000).unwrap())
    }

    fn seed_settings(ledger: &InMemoryLedger) {
        let timestamp = Timestamp::from_second(1_000).unwrap();
        ledger.seed_settings(LedgerSettings::new(
            LedgerSettingsId::from_uuid(Uuid::from_u128(99)),
            usd(),
            1,
            LedgerCalendar::new("UTC").unwrap(),
            timestamp,
            timestamp,
        ));
    }

    #[test]
    fn creates_account_and_opening_balance() {
        let ledger = InMemoryLedger::new();
        seed_settings(&ledger);
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateAccountWithOpeningBalanceUseCase::new(&ledger, &ledger, &clock, &ids);
        let request = CreateAccountRequest {
            name: "  Savings  ".to_string(),
            asset_type: AssetAccountType::Savings,
            currency: usd(),
            options: AccountOptions::default(),
            opening_balance_text: Some("250.00".to_string()),
        };

        let outcome = use_case.execute(&request).unwrap();

        assert_eq!(outcome.account.name(), "Savings");
        assert_eq!(
            outcome
                .opening_balance
                .as_ref()
                .unwrap()
                .amount()
                .minor_units(),
            25_000
        );
        assert_eq!(ledger.accounts(true).unwrap().len(), 1);
        assert_eq!(ledger.transactions(true).unwrap().len(), 1);
    }

    #[test]
    fn zero_opening_balance_is_omitted() {
        let ledger = InMemoryLedger::new();
        seed_settings(&ledger);
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateAccountWithOpeningBalanceUseCase::new(&ledger, &ledger, &clock, &ids);
        let request = CreateAccountRequest {
            name: "Cash".to_string(),
            asset_type: AssetAccountType::Cash,
            currency: usd(),
            options: AccountOptions::default(),
            opening_balance_text: Some("0".to_string()),
        };

        let outcome = use_case.execute(&request).unwrap();

        assert!(outcome.opening_balance.is_none());
        assert!(ledger.transactions(true).unwrap().is_empty());
    }

    #[test]
    fn rejects_creation_before_initialization() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case = CreateAccountWithOpeningBalanceUseCase::new(&ledger, &ledger, &clock, &ids);
        let request = CreateAccountRequest {
            name: "Cash".to_string(),
            asset_type: AssetAccountType::Cash,
            currency: usd(),
            options: AccountOptions::default(),
            opening_balance_text: None,
        };

        assert_eq!(
            use_case.execute(&request).unwrap_err(),
            ApplicationError::LedgerNotInitialized
        );
    }
}
