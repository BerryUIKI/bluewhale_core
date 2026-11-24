//! Versioned JSON export of a complete ledger.
//!
//! The document shape is the public export contract in
//! `contracts/export/v1/export.schema.json`. Money is exact integer minor
//! units, timestamps are UTC RFC 3339 with millisecond precision, and every
//! collection is sorted by identifier so the same ledger always exports to
//! the same bytes.

use core_domain::{
    Account, AccountGroup, AssetAccountType, Category, CategoryGroup, CategoryKind, Counterparty,
    CounterpartyKind, ExchangeRate, ExchangeRateSource, LedgerCalendarIdentifier,
    LedgerExportRepository, LedgerExportSnapshot, LedgerProfile, LedgerSettings, LedgerTag, Money,
    ReconciliationFlag, ReconciliationFlagSystemKind, Transaction, TransactionKind,
    TransactionNotePreset,
};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};

use crate::error::ApplicationError;
use crate::ports::Clock;

/// The export document version written by this build.
pub const EXPORT_VERSION: u32 = 1;

/// A complete ledger export document.
///
/// Field order is part of the emitted shape. Every array is sorted by `id`
/// in ascending UUID order, and every object is serialized in declaration
/// order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportDocument {
    /// The export contract version.
    pub export_version: u32,
    /// The ledger identity, equal to the settings record identifier.
    pub ledger_id: String,
    /// The instant the export was produced, in UTC.
    pub exported_at: String,
    /// The client application version that produced the export.
    pub app_version: String,
    /// Ledger settings.
    pub settings: ExportSettings,
    /// Ledger profile.
    pub profile: ExportProfile,
    /// Account groups, including archived groups.
    pub account_groups: Vec<ExportAccountGroup>,
    /// Accounts, including archived accounts.
    pub accounts: Vec<ExportAccount>,
    /// Category groups, including archived groups.
    pub category_groups: Vec<ExportCategoryGroup>,
    /// Categories, including archived categories.
    pub categories: Vec<ExportCategory>,
    /// Tags, including archived tags.
    pub tags: Vec<ExportTag>,
    /// Counterparties, including archived counterparties.
    pub counterparties: Vec<ExportCounterparty>,
    /// Reconciliation flags, including archived flags.
    pub reconciliation_flags: Vec<ExportReconciliationFlag>,
    /// Note presets, including archived presets.
    pub note_presets: Vec<ExportNotePreset>,
    /// Transactions, including soft-deleted transactions.
    pub transactions: Vec<ExportTransaction>,
    /// Every exchange-rate revision, including superseded rates.
    pub exchange_rates: Vec<ExportExchangeRate>,
}

/// Ledger settings.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportSettings {
    /// The settings record identifier.
    pub id: String,
    /// The reporting currency code.
    pub currency: String,
    /// The reporting currency scale.
    pub currency_scale: u8,
    /// The currency-scale policy version.
    pub currency_policy_version: u32,
    /// The calendar identifier.
    pub calendar: ExportCalendar,
    /// The IANA time zone identifier.
    pub time_zone: String,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// The calendar used for ledger boundaries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportCalendar {
    /// The proleptic Gregorian calendar.
    Gregorian,
}

/// The ledger profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportProfile {
    /// The profile record identifier.
    pub id: String,
    /// The settings record this profile belongs to.
    pub settings_id: String,
    /// The ledger name.
    pub name: String,
    /// The ledger emoji.
    pub emoji: String,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// An account group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportAccountGroup {
    /// The group identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the group is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// An asset account.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportAccount {
    /// The account identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The asset type.
    pub asset_type: ExportAssetType,
    /// The account currency code.
    pub currency: String,
    /// The account currency scale.
    pub currency_scale: u8,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the account is archived.
    pub is_archived: bool,
    /// The owning counterparty.
    pub owner_id: Option<String>,
    /// The custom account group.
    pub account_group_id: Option<String>,
    /// A free-form note.
    pub note: String,
    /// Whether the account is hidden from the main list.
    pub is_hidden: bool,
    /// Whether the account contributes to net worth.
    pub is_included_in_net_worth: bool,
    /// Attached tag identifiers.
    pub tag_ids: Vec<String>,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// The kind of an asset account.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportAssetType {
    /// Cash on hand.
    Cash,
    /// A checking account.
    Checking,
    /// A savings account.
    Savings,
    /// Stored value, such as a transit or gift card balance.
    StoredValue,
}

/// A category group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportCategoryGroup {
    /// The group identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The category kind.
    pub kind: ExportCategoryKind,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the group is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// A leaf category.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportCategory {
    /// The category identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The category kind.
    pub kind: ExportCategoryKind,
    /// The containing category group.
    pub group_id: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the category is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// Whether a category records expenses or income.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportCategoryKind {
    /// An expense category.
    Expense,
    /// An income category.
    Income,
}

/// A tag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportTag {
    /// The tag identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The display color.
    pub color_hex: String,
    /// An optional description.
    pub description: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the tag is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// A counterparty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportCounterparty {
    /// The counterparty identifier.
    pub id: String,
    /// The counterparty kind.
    pub kind: ExportCounterpartyKind,
    /// The display name.
    pub name: String,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the counterparty is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// The kind of a counterparty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportCounterpartyKind {
    /// A member of the household.
    HouseholdMember,
    /// Another person.
    Person,
    /// An organization.
    Organization,
}

/// A reconciliation flag.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportReconciliationFlag {
    /// The flag identifier.
    pub id: String,
    /// The display name.
    pub name: String,
    /// The display color.
    pub color_hex: String,
    /// The built-in kind, when this is a system flag.
    pub system_kind: Option<ExportReconciliationFlagKind>,
    /// Whether the flag prevents editing attached transactions.
    pub locks_transactions: bool,
    /// A single-letter keyboard shortcut.
    pub shortcut_key: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the flag is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// A built-in reconciliation flag kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportReconciliationFlagKind {
    /// Cleared by the bank.
    Cleared,
    /// Awaiting reimbursement.
    NeedsReimbursement,
    /// Marks an issue.
    Issue,
}

/// A transaction note preset.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportNotePreset {
    /// The preset identifier.
    pub id: String,
    /// The preset text.
    pub text: String,
    /// The manual sort position.
    pub sort_order: i32,
    /// Whether the preset is archived.
    pub is_archived: bool,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// An exact amount in integer minor units.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportMoney {
    /// The amount in minor units.
    pub minor_units: i64,
    /// The currency code.
    pub currency: String,
    /// The currency scale.
    pub scale: u8,
}

/// A ledger transaction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportTransaction {
    /// The transaction identifier.
    pub id: String,
    /// The transaction kind.
    pub kind: ExportTransactionKind,
    /// The source amount.
    pub amount: ExportMoney,
    /// The destination amount for a cross-currency transfer.
    pub destination_amount: Option<ExportMoney>,
    /// The instant the transaction occurred.
    pub occurred_at: String,
    /// The source account.
    pub account_id: String,
    /// The destination account of a transfer.
    pub destination_account_id: Option<String>,
    /// The classifying category.
    pub category_id: Option<String>,
    /// The reconciliation flag.
    pub reconciliation_flag_id: Option<String>,
    /// The counterparty.
    pub counterparty_id: Option<String>,
    /// A free-form note.
    pub note: Option<String>,
    /// Attached tag identifiers.
    pub tag_ids: Vec<String>,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
    /// The soft-deletion instant.
    pub deleted_at: Option<String>,
}

/// The kind of a transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportTransactionKind {
    /// The initial balance of an account.
    OpeningBalance,
    /// Money spent.
    Expense,
    /// Money received.
    Income,
    /// Money moved between accounts.
    Transfer,
}

/// One exchange-rate revision.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportExchangeRate {
    /// The rate identifier.
    pub id: String,
    /// The source currency code.
    pub source_currency: String,
    /// The source currency scale.
    pub source_currency_scale: u8,
    /// The reporting currency code.
    pub reporting_currency: String,
    /// The reporting currency scale.
    pub reporting_currency_scale: u8,
    /// The exact quote numerator.
    pub quote_numerator: i64,
    /// The exact quote denominator.
    pub quote_denominator: i64,
    /// The instant the rate takes effect.
    pub effective_at: String,
    /// The rate source.
    pub source: ExportExchangeRateSource,
    /// The instant a newer revision superseded this rate.
    pub superseded_at: Option<String>,
    /// The creation instant.
    pub created_at: String,
    /// The last update instant.
    pub updated_at: String,
}

/// Where an exchange rate came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportExchangeRateSource {
    /// Entered by the user.
    Manual,
    /// Published by the European Central Bank.
    EuropeanCentralBank,
}

/// Produces the versioned export document for a ledger.
pub struct ExportLedgerUseCase<'a> {
    repository: &'a dyn LedgerExportRepository,
    clock: &'a dyn Clock,
}

impl<'a> ExportLedgerUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn LedgerExportRepository, clock: &'a dyn Clock) -> Self {
        Self { repository, clock }
    }

    /// Builds the export document.
    ///
    /// # Errors
    ///
    /// Returns [`ApplicationError::EmptyAppVersion`] when the application
    /// version is blank, or [`ApplicationError::Repository`] when the ledger
    /// cannot be read.
    pub fn export(&self, app_version: &str) -> Result<ExportDocument, ApplicationError> {
        if app_version.trim().is_empty() {
            return Err(ApplicationError::EmptyAppVersion);
        }
        let snapshot = self.repository.export_snapshot()?;
        Ok(build_document(&snapshot, self.clock.now(), app_version))
    }

    /// Builds the export document and serializes it as pretty-printed JSON.
    ///
    /// # Errors
    ///
    /// Returns the same errors as [`Self::export`], plus
    /// [`ApplicationError::Serialization`] when serialization fails.
    pub fn export_json(&self, app_version: &str) -> Result<String, ApplicationError> {
        let document = self.export(app_version)?;
        serde_json::to_string_pretty(&document)
            .map_err(|error| ApplicationError::Serialization(error.to_string()))
    }
}

fn build_document(
    snapshot: &LedgerExportSnapshot,
    exported_at: Timestamp,
    app_version: &str,
) -> ExportDocument {
    ExportDocument {
        export_version: EXPORT_VERSION,
        ledger_id: snapshot.settings.id().to_string(),
        exported_at: format_timestamp(exported_at),
        app_version: app_version.to_string(),
        settings: export_settings(&snapshot.settings),
        profile: export_profile(&snapshot.profile),
        account_groups: sorted(snapshot.account_groups.iter().map(export_account_group)),
        accounts: sorted(snapshot.accounts.iter().map(export_account)),
        category_groups: sorted(snapshot.category_groups.iter().map(export_category_group)),
        categories: sorted(snapshot.categories.iter().map(export_category)),
        tags: sorted(snapshot.tags.iter().map(export_tag)),
        counterparties: sorted(snapshot.counterparties.iter().map(export_counterparty)),
        reconciliation_flags: sorted(
            snapshot
                .reconciliation_flags
                .iter()
                .map(export_reconciliation_flag),
        ),
        note_presets: sorted(snapshot.note_presets.iter().map(export_note_preset)),
        transactions: sorted(snapshot.transactions.iter().map(export_transaction)),
        exchange_rates: sorted(snapshot.exchange_rates.iter().map(export_exchange_rate)),
    }
}

fn sorted<T>(items: impl Iterator<Item = T>) -> Vec<T>
where
    T: HasId,
{
    let mut items: Vec<T> = items.collect();
    items.sort_by(|left, right| left.id().cmp(right.id()));
    items
}

trait HasId {
    fn id(&self) -> &str;
}

macro_rules! impl_has_id {
    ($($type:ty),* $(,)?) => {
        $(
            impl HasId for $type {
                fn id(&self) -> &str {
                    &self.id
                }
            }
        )*
    };
}

impl_has_id!(
    ExportAccountGroup,
    ExportAccount,
    ExportCategoryGroup,
    ExportCategory,
    ExportTag,
    ExportCounterparty,
    ExportReconciliationFlag,
    ExportNotePreset,
    ExportTransaction,
    ExportExchangeRate,
);

fn format_timestamp(value: Timestamp) -> String {
    value.strftime("%Y-%m-%dT%H:%M:%S%.3fZ").to_string()
}

fn export_settings(settings: &LedgerSettings) -> ExportSettings {
    ExportSettings {
        id: settings.id().to_string(),
        currency: settings.currency().code().to_string(),
        currency_scale: settings.currency().scale(),
        currency_policy_version: settings.currency_policy_version(),
        calendar: match settings.calendar_identifier() {
            LedgerCalendarIdentifier::Gregorian => ExportCalendar::Gregorian,
        },
        time_zone: settings.calendar().identifier().to_string(),
        created_at: format_timestamp(settings.created_at()),
        updated_at: format_timestamp(settings.updated_at()),
    }
}

fn export_profile(profile: &LedgerProfile) -> ExportProfile {
    ExportProfile {
        id: profile.id().to_string(),
        settings_id: profile.settings_id().to_string(),
        name: profile.name().to_string(),
        emoji: profile.emoji().to_string(),
        created_at: format_timestamp(profile.created_at()),
        updated_at: format_timestamp(profile.updated_at()),
    }
}

fn export_account_group(group: &AccountGroup) -> ExportAccountGroup {
    ExportAccountGroup {
        id: group.id().to_string(),
        name: group.name().to_string(),
        sort_order: group.sort_order(),
        is_archived: group.is_archived(),
        created_at: format_timestamp(group.created_at()),
        updated_at: format_timestamp(group.updated_at()),
    }
}

fn export_account(account: &Account) -> ExportAccount {
    let options = account.options();
    ExportAccount {
        id: account.id().to_string(),
        name: account.name().to_string(),
        asset_type: match account.asset_type() {
            AssetAccountType::Cash => ExportAssetType::Cash,
            AssetAccountType::Checking => ExportAssetType::Checking,
            AssetAccountType::Savings => ExportAssetType::Savings,
            AssetAccountType::StoredValue => ExportAssetType::StoredValue,
        },
        currency: account.currency().code().to_string(),
        currency_scale: account.currency().scale(),
        sort_order: options.sort_order,
        is_archived: options.is_archived,
        owner_id: options.owner_id.map(|id| id.to_string()),
        account_group_id: options.custom_group_id.map(|id| id.to_string()),
        note: options.note.clone(),
        is_hidden: options.is_hidden,
        is_included_in_net_worth: options.is_included_in_net_worth,
        tag_ids: options.tag_ids.iter().map(ToString::to_string).collect(),
        created_at: format_timestamp(account.created_at()),
        updated_at: format_timestamp(account.updated_at()),
    }
}

fn export_category_group(group: &CategoryGroup) -> ExportCategoryGroup {
    ExportCategoryGroup {
        id: group.id().to_string(),
        name: group.name().to_string(),
        kind: export_category_kind(group.kind()),
        sort_order: group.sort_order(),
        is_archived: group.is_archived(),
        created_at: format_timestamp(group.created_at()),
        updated_at: format_timestamp(group.updated_at()),
    }
}

fn export_category(category: &Category) -> ExportCategory {
    let options = category.options();
    ExportCategory {
        id: category.id().to_string(),
        name: category.name().to_string(),
        kind: export_category_kind(category.kind()),
        group_id: options.group_id.map(|id| id.to_string()),
        sort_order: options.sort_order,
        is_archived: options.is_archived,
        created_at: format_timestamp(category.created_at()),
        updated_at: format_timestamp(category.updated_at()),
    }
}

fn export_category_kind(kind: CategoryKind) -> ExportCategoryKind {
    match kind {
        CategoryKind::Expense => ExportCategoryKind::Expense,
        CategoryKind::Income => ExportCategoryKind::Income,
    }
}

fn export_tag(tag: &LedgerTag) -> ExportTag {
    let options = tag.options();
    ExportTag {
        id: tag.id().to_string(),
        name: tag.name().to_string(),
        color_hex: tag.color_hex().to_string(),
        description: options.description.clone(),
        sort_order: options.sort_order,
        is_archived: options.is_archived,
        created_at: format_timestamp(tag.created_at()),
        updated_at: format_timestamp(tag.updated_at()),
    }
}

fn export_counterparty(counterparty: &Counterparty) -> ExportCounterparty {
    ExportCounterparty {
        id: counterparty.id().to_string(),
        kind: match counterparty.kind() {
            CounterpartyKind::HouseholdMember => ExportCounterpartyKind::HouseholdMember,
            CounterpartyKind::Person => ExportCounterpartyKind::Person,
            CounterpartyKind::Organization => ExportCounterpartyKind::Organization,
        },
        name: counterparty.name().to_string(),
        sort_order: counterparty.sort_order(),
        is_archived: counterparty.is_archived(),
        created_at: format_timestamp(counterparty.created_at()),
        updated_at: format_timestamp(counterparty.updated_at()),
    }
}

fn export_reconciliation_flag(flag: &ReconciliationFlag) -> ExportReconciliationFlag {
    let options = flag.options();
    ExportReconciliationFlag {
        id: flag.id().to_string(),
        name: flag.name().to_string(),
        color_hex: flag.color_hex().to_string(),
        system_kind: options.system_kind.map(|kind| match kind {
            ReconciliationFlagSystemKind::Cleared => ExportReconciliationFlagKind::Cleared,
            ReconciliationFlagSystemKind::NeedsReimbursement => {
                ExportReconciliationFlagKind::NeedsReimbursement
            }
            ReconciliationFlagSystemKind::Issue => ExportReconciliationFlagKind::Issue,
        }),
        locks_transactions: options.locks_transactions,
        shortcut_key: options.shortcut_key.clone(),
        sort_order: options.sort_order,
        is_archived: options.is_archived,
        created_at: format_timestamp(flag.created_at()),
        updated_at: format_timestamp(flag.updated_at()),
    }
}

fn export_note_preset(preset: &TransactionNotePreset) -> ExportNotePreset {
    ExportNotePreset {
        id: preset.id().to_string(),
        text: preset.text().to_string(),
        sort_order: preset.sort_order(),
        is_archived: preset.is_archived(),
        created_at: format_timestamp(preset.created_at()),
        updated_at: format_timestamp(preset.updated_at()),
    }
}

fn export_money(money: &Money) -> ExportMoney {
    ExportMoney {
        minor_units: money.minor_units(),
        currency: money.currency().code().to_string(),
        scale: money.currency().scale(),
    }
}

fn export_transaction(transaction: &Transaction) -> ExportTransaction {
    let fields = transaction.fields();
    ExportTransaction {
        id: transaction.id().to_string(),
        kind: match transaction.kind() {
            TransactionKind::OpeningBalance => ExportTransactionKind::OpeningBalance,
            TransactionKind::Expense => ExportTransactionKind::Expense,
            TransactionKind::Income => ExportTransactionKind::Income,
            TransactionKind::Transfer => ExportTransactionKind::Transfer,
        },
        amount: export_money(transaction.amount()),
        destination_amount: fields.destination_amount.as_ref().map(export_money),
        occurred_at: format_timestamp(transaction.occurred_at()),
        account_id: transaction.account_id().to_string(),
        destination_account_id: fields.destination_account_id.map(|id| id.to_string()),
        category_id: fields.category_id.map(|id| id.to_string()),
        reconciliation_flag_id: fields.reconciliation_flag_id.map(|id| id.to_string()),
        counterparty_id: fields.counterparty_id.map(|id| id.to_string()),
        note: fields.note.clone(),
        tag_ids: fields.tag_ids.iter().map(ToString::to_string).collect(),
        created_at: format_timestamp(transaction.created_at()),
        updated_at: format_timestamp(transaction.updated_at()),
        deleted_at: fields.deleted_at.map(format_timestamp),
    }
}

fn export_exchange_rate(rate: &ExchangeRate) -> ExportExchangeRate {
    let quote = rate.quote();
    ExportExchangeRate {
        id: rate.id().to_string(),
        source_currency: rate.source_currency().code().to_string(),
        source_currency_scale: rate.source_currency().scale(),
        reporting_currency: rate.reporting_currency().code().to_string(),
        reporting_currency_scale: rate.reporting_currency().scale(),
        quote_numerator: quote.numerator(),
        quote_denominator: quote.denominator(),
        effective_at: format_timestamp(rate.effective_at()),
        source: match rate.source() {
            ExchangeRateSource::Manual => ExportExchangeRateSource::Manual,
            ExchangeRateSource::EuropeanCentralBank => {
                ExportExchangeRateSource::EuropeanCentralBank
            }
        },
        superseded_at: rate.superseded_at().map(format_timestamp),
        created_at: format_timestamp(rate.created_at()),
        updated_at: format_timestamp(rate.updated_at()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_domain::{
        Account, AccountId, AccountOptions, AccountRepository, Category, CategoryId, CategoryKind,
        CategoryOptions, CategoryRepository, CurrencyScalePolicy, LedgerProfile, LedgerProfileId,
        LedgerProfileRepository, LedgerSettingsId, LedgerSettingsRepository, LedgerTag, Money,
        TagId, TagOptions, TagRepository, Transaction, TransactionFields, TransactionId,
        TransactionRepository,
    };
    use core_testkit::InMemoryLedger;

    use crate::test_support::FixedClock;

    const SCHEMA: &str = include_str!("../../../contracts/export/v1/export.schema.json");
    const FIXTURE: &str = include_str!("../../../contracts/export/v1/fixture.json");

    fn timestamp(milliseconds: i64) -> Timestamp {
        Timestamp::from_millisecond(milliseconds).unwrap()
    }

    fn settings() -> LedgerSettings {
        let currency = CurrencyScalePolicy::v1().currency("USD").unwrap();
        LedgerSettings::new(
            LedgerSettingsId::from_uuid(uuid::Uuid::from_u128(1)),
            currency,
            1,
            core_domain::LedgerCalendar::new("Asia/Singapore").unwrap(),
            timestamp(1_000),
            timestamp(1_000),
        )
    }

    fn profile() -> LedgerProfile {
        LedgerProfile::new(
            LedgerProfileId::from_uuid(uuid::Uuid::from_u128(2)),
            settings().id(),
            "Household",
            "🏠",
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn tag(id: u128, name: &str) -> LedgerTag {
        LedgerTag::new(
            TagId::from_uuid(uuid::Uuid::from_u128(id)),
            name,
            "#112233",
            TagOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn account() -> Account {
        Account::new(
            AccountId::from_uuid(uuid::Uuid::from_u128(20)),
            "Cash",
            AssetAccountType::Cash,
            CurrencyScalePolicy::v1().currency("USD").unwrap(),
            AccountOptions {
                tag_ids: [tag(4, "Food").id()].into_iter().collect(),
                note: "Everyday cash".to_string(),
                ..AccountOptions::default()
            },
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn category() -> Category {
        Category::new(
            CategoryId::from_uuid(uuid::Uuid::from_u128(40)),
            "Food",
            CategoryKind::Expense,
            CategoryOptions::default(),
            timestamp(1_000),
            timestamp(1_000),
        )
        .unwrap()
    }

    fn transaction() -> Transaction {
        Transaction::new(
            TransactionId::from_uuid(uuid::Uuid::from_u128(90)),
            TransactionKind::Expense,
            Money::new(12_345, CurrencyScalePolicy::v1().currency("USD").unwrap()),
            timestamp(2_000),
            account().id(),
            TransactionFields {
                category_id: Some(category().id()),
                note: Some("Lunch".to_string()),
                ..TransactionFields::default()
            },
            timestamp(2_000),
            timestamp(2_000),
        )
        .unwrap()
    }

    fn seeded_ledger() -> InMemoryLedger {
        let ledger = InMemoryLedger::new();
        ledger.save_settings(&settings()).unwrap();
        ledger.save_profile(&profile()).unwrap();
        ledger.save_tag(&tag(9, "Travel")).unwrap();
        ledger.save_tag(&tag(4, "Food")).unwrap();
        ledger.save_account(&account()).unwrap();
        ledger.save_category(&category()).unwrap();
        ledger.save_transaction(&transaction()).unwrap();
        ledger
    }

    fn validator() -> jsonschema::Validator {
        let schema: serde_json::Value = serde_json::from_str(SCHEMA).unwrap();
        jsonschema::validator_for(&schema).unwrap()
    }

    #[test]
    fn fixture_and_export_satisfy_the_v1_schema() {
        let fixture: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        validator().validate(&fixture).unwrap();
        let fixture_document: ExportDocument = serde_json::from_str(FIXTURE).unwrap();
        assert_eq!(fixture_document.export_version, 1);
        assert_eq!(fixture_document.transactions.len(), 1);

        let ledger = seeded_ledger();
        let clock = FixedClock::new(timestamp(1_000));
        let use_case = ExportLedgerUseCase::new(&ledger, &clock);
        let json = use_case.export_json("1.2.3").unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        validator().validate(&value).unwrap();

        let document = use_case.export("1.2.3").unwrap();
        let parsed: ExportDocument = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed, document);
        assert_eq!(document.export_version, EXPORT_VERSION);
        assert_eq!(document.ledger_id, settings().id().to_string());
        assert_eq!(document.exported_at, "1970-01-01T00:00:01.000Z");
        assert_eq!(document.app_version, "1.2.3");
        assert_eq!(document.transactions[0].amount.minor_units, 12_345);
        assert_eq!(document.transactions[0].amount.currency, "USD".to_string());
    }

    #[test]
    fn collections_are_sorted_by_identifier() {
        let tag_nine = tag(9, "Travel");
        let tag_four = tag(4, "Food");
        let snapshot = LedgerExportSnapshot {
            settings: settings(),
            profile: profile(),
            account_groups: Vec::new(),
            accounts: Vec::new(),
            category_groups: Vec::new(),
            categories: Vec::new(),
            tags: vec![tag_nine, tag_four],
            counterparties: Vec::new(),
            reconciliation_flags: Vec::new(),
            note_presets: Vec::new(),
            transactions: Vec::new(),
            exchange_rates: Vec::new(),
        };

        let document = build_document(&snapshot, timestamp(1_000), "1.0.0");
        assert_eq!(
            document.tags[0].id,
            TagId::from_uuid(uuid::Uuid::from_u128(4)).to_string()
        );
        assert_eq!(
            document.tags[1].id,
            TagId::from_uuid(uuid::Uuid::from_u128(9)).to_string()
        );
    }

    #[test]
    fn blank_app_version_is_rejected() {
        let ledger = seeded_ledger();
        let clock = FixedClock::new(timestamp(1_000));
        let use_case = ExportLedgerUseCase::new(&ledger, &clock);
        assert_eq!(
            use_case.export("   ").unwrap_err(),
            ApplicationError::EmptyAppVersion
        );
    }
}
