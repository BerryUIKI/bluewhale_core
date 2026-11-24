//! First-run ledger initialization.

use core_domain::{
    Account, AccountId, AccountOptions, AssetAccountType, Category, CategoryId, CategoryKind,
    CategoryOptions, CurrencyScalePolicy, LedgerBootstrapRepository, LedgerCalendar, LedgerProfile,
    LedgerProfileId, LedgerSettings, LedgerSettingsId, MoneyParser, Transaction, TransactionFields,
    TransactionId, TransactionKind,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, IdSource};

/// The default content created with a new ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultLedgerContent {
    /// The name of the default cash account.
    pub cash_account_name: String,
    /// Names of the default expense categories.
    pub expense_category_names: Vec<String>,
    /// Names of the default income categories.
    pub income_category_names: Vec<String>,
}

/// The request for first-run initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InitializeLedgerRequest {
    /// The ledger name.
    pub ledger_name: String,
    /// The ledger emoji.
    pub ledger_emoji: String,
    /// The reporting currency code.
    pub currency_code: String,
    /// The IANA time zone identifier.
    pub time_zone_identifier: String,
    /// The optional opening balance text for the default account.
    pub opening_balance_text: String,
    /// The default content to create.
    pub default_content: DefaultLedgerContent,
}

/// The result of initialization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerInitializationOutcome {
    /// The active settings.
    pub settings: LedgerSettings,
    /// Whether this call initialized the ledger.
    pub did_initialize: bool,
    /// Whether the default account was created.
    pub did_create_default_account: bool,
}

/// Creates the first ledger settings, profile, account, categories, and
/// optional opening balance.
pub struct InitializeLedgerUseCase<'a> {
    repository: &'a dyn LedgerBootstrapRepository,
    currency_policy: CurrencyScalePolicy,
    parser: MoneyParser,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> InitializeLedgerUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn LedgerBootstrapRepository,
        currency_policy: CurrencyScalePolicy,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            currency_policy,
            parser: MoneyParser::default(),
            clock,
            ids,
        }
    }

    /// Initializes the ledger, or returns the existing settings.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] for invalid defaults, currency, time
    /// zone, profile, account, or opening balance, or when the repository
    /// fails.
    pub fn execute(
        &self,
        request: &InitializeLedgerRequest,
    ) -> Result<LedgerInitializationOutcome, ApplicationError> {
        if let Some(existing) = self.repository.settings()? {
            return Ok(LedgerInitializationOutcome {
                settings: existing,
                did_initialize: false,
                did_create_default_account: false,
            });
        }
        if request.default_content.expense_category_names.is_empty() {
            return Err(ApplicationError::EmptyExpenseCategoryDefaults);
        }
        if request.default_content.income_category_names.is_empty() {
            return Err(ApplicationError::EmptyIncomeCategoryDefaults);
        }

        let timestamp = self.clock.now();
        let currency = self
            .currency_policy
            .currency(&request.currency_code)
            .map_err(ApplicationError::Currency)?;
        let calendar = LedgerCalendar::new(&request.time_zone_identifier)
            .map_err(ApplicationError::Calendar)?;
        let settings = LedgerSettings::new(
            LedgerSettingsId::from_uuid(self.ids.next_uuid()),
            currency.clone(),
            self.currency_policy.version(),
            calendar,
            timestamp,
            timestamp,
        );
        let profile = LedgerProfile::new(
            LedgerProfileId::from_uuid(self.ids.next_uuid()),
            settings.id(),
            &request.ledger_name,
            &request.ledger_emoji,
            timestamp,
            timestamp,
        )
        .map_err(ApplicationError::Profile)?;
        let account = Account::new(
            AccountId::from_uuid(self.ids.next_uuid()),
            &request.default_content.cash_account_name,
            AssetAccountType::Cash,
            currency.clone(),
            AccountOptions::default(),
            timestamp,
            timestamp,
        )
        .map_err(ApplicationError::Account)?;
        let categories = self.make_categories(request, timestamp)?;
        let opening_balance = self.make_opening_balance(
            &request.opening_balance_text,
            &currency,
            account.id(),
            timestamp,
        )?;

        let did_initialize = self.repository.initialize(
            &settings,
            &profile,
            &account,
            &categories,
            opening_balance.as_ref(),
        )?;
        if did_initialize {
            return Ok(LedgerInitializationOutcome {
                settings,
                did_initialize: true,
                did_create_default_account: true,
            });
        }
        let persisted = self
            .repository
            .settings()?
            .ok_or(ApplicationError::MissingSettingsAfterConcurrentInitialization)?;
        Ok(LedgerInitializationOutcome {
            settings: persisted,
            did_initialize: false,
            did_create_default_account: false,
        })
    }

    fn make_categories(
        &self,
        request: &InitializeLedgerRequest,
        timestamp: jiff::Timestamp,
    ) -> Result<Vec<Category>, ApplicationError> {
        let mut categories = Vec::new();
        for (index, name) in request
            .default_content
            .expense_category_names
            .iter()
            .enumerate()
        {
            categories.push(
                Category::new(
                    CategoryId::from_uuid(self.ids.next_uuid()),
                    name,
                    CategoryKind::Expense,
                    CategoryOptions {
                        sort_order: i32::try_from(index).unwrap_or(i32::MAX),
                        ..CategoryOptions::default()
                    },
                    timestamp,
                    timestamp,
                )
                .map_err(ApplicationError::Category)?,
            );
        }
        for (index, name) in request
            .default_content
            .income_category_names
            .iter()
            .enumerate()
        {
            categories.push(
                Category::new(
                    CategoryId::from_uuid(self.ids.next_uuid()),
                    name,
                    CategoryKind::Income,
                    CategoryOptions {
                        sort_order: i32::try_from(index).unwrap_or(i32::MAX),
                        ..CategoryOptions::default()
                    },
                    timestamp,
                    timestamp,
                )
                .map_err(ApplicationError::Category)?,
            );
        }
        Ok(categories)
    }

    fn make_opening_balance(
        &self,
        text: &str,
        currency: &core_domain::Currency,
        account_id: AccountId,
        timestamp: jiff::Timestamp,
    ) -> Result<Option<Transaction>, ApplicationError> {
        let normalized = text.trim();
        if normalized.is_empty() {
            return Ok(None);
        }
        let amount = self
            .parser
            .parse_magnitude(normalized, currency)
            .map_err(ApplicationError::Amount)?;
        if amount.minor_units() == 0 {
            return Ok(None);
        }
        Transaction::new(
            TransactionId::from_uuid(self.ids.next_uuid()),
            TransactionKind::OpeningBalance,
            amount,
            timestamp,
            account_id,
            TransactionFields::default(),
            timestamp,
            timestamp,
        )
        .map(Some)
        .map_err(ApplicationError::Transaction)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixedClock, SequenceIds};
    use core_domain::{AccountRepository, CategoryRepository, TransactionRepository};
    use core_testkit::InMemoryLedger;
    use jiff::Timestamp;

    fn clock() -> FixedClock {
        FixedClock::new(Timestamp::from_second(1_000).unwrap())
    }

    fn request() -> InitializeLedgerRequest {
        InitializeLedgerRequest {
            ledger_name: "  Household  ".to_string(),
            ledger_emoji: "🏠".to_string(),
            currency_code: "USD".to_string(),
            time_zone_identifier: "Asia/Singapore".to_string(),
            opening_balance_text: "100.50".to_string(),
            default_content: DefaultLedgerContent {
                cash_account_name: "Cash".to_string(),
                expense_category_names: vec!["Food".to_string(), "Transport".to_string()],
                income_category_names: vec!["Salary".to_string()],
            },
        }
    }

    #[test]
    fn initialization_creates_settings_profile_account_categories_and_opening_balance() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case =
            InitializeLedgerUseCase::new(&ledger, CurrencyScalePolicy::v1(), &clock, &ids);

        let outcome = use_case.execute(&request()).unwrap();

        assert!(outcome.did_initialize);
        assert!(outcome.did_create_default_account);
        assert_eq!(outcome.settings.currency().code(), "USD");
        assert_eq!(outcome.settings.calendar().identifier(), "Asia/Singapore");
        assert_eq!(ledger.accounts(true).unwrap().len(), 1);
        assert_eq!(ledger.categories(true).unwrap().len(), 3);
        let transactions = ledger.transactions(true).unwrap();
        assert_eq!(transactions.len(), 1);
        assert_eq!(transactions[0].amount().minor_units(), 10_050);
    }

    #[test]
    fn initialization_is_idempotent() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case =
            InitializeLedgerUseCase::new(&ledger, CurrencyScalePolicy::v1(), &clock, &ids);

        let first = use_case.execute(&request()).unwrap();
        let second = use_case.execute(&request()).unwrap();

        assert!(first.did_initialize);
        assert!(!second.did_initialize);
        assert_eq!(first.settings.id(), second.settings.id());
        assert_eq!(ledger.accounts(true).unwrap().len(), 1);
    }

    #[test]
    fn empty_default_categories_are_rejected() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case =
            InitializeLedgerUseCase::new(&ledger, CurrencyScalePolicy::v1(), &clock, &ids);
        let mut invalid = request();
        invalid.default_content.expense_category_names.clear();

        assert_eq!(
            use_case.execute(&invalid).unwrap_err(),
            ApplicationError::EmptyExpenseCategoryDefaults
        );
    }

    #[test]
    fn zero_opening_balance_is_not_created() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let use_case =
            InitializeLedgerUseCase::new(&ledger, CurrencyScalePolicy::v1(), &clock, &ids);
        let mut request = request();
        request.opening_balance_text = "0.00".to_string();

        use_case.execute(&request).unwrap();

        assert!(ledger.transactions(true).unwrap().is_empty());
    }
}
