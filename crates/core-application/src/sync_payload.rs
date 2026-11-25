//! Encoding and decoding of entity aggregates inside encrypted sync payloads.
//!
//! The decrypted payload is the export v1 entity object for the envelope's
//! entity type, serialized as compact JSON. This module converts between the
//! domain aggregates and those documents. It is the client-side semantic
//! validation step that the server cannot perform because it never sees the
//! plaintext.

use std::collections::BTreeSet;
use std::fmt;
use std::str::FromStr;

use core_domain::{
    Account, AccountGroup, AccountOptions, AssetAccountType, Category, CategoryGroup, CategoryKind,
    CategoryOptions, Counterparty, CounterpartyKind, Currency, CurrencyScalePolicy, ExactRatio,
    ExchangeRate, ExchangeRateSource, LedgerCalendar, LedgerProfile, LedgerSettings, LedgerTag,
    Money, ReconciliationFlag, ReconciliationFlagOptions, ReconciliationFlagSystemKind, SyncEntity,
    SyncEntityType, TagId, TagOptions, Transaction, TransactionFields, TransactionKind,
    TransactionNotePreset,
};
use jiff::Timestamp;

use crate::export::{
    ExportAccount, ExportAccountGroup, ExportAssetType, ExportCategory, ExportCategoryGroup,
    ExportCategoryKind, ExportCounterparty, ExportCounterpartyKind, ExportDocument,
    ExportExchangeRate, ExportExchangeRateSource, ExportMoney, ExportNotePreset, ExportProfile,
    ExportReconciliationFlag, ExportReconciliationFlagKind, ExportSettings, ExportTag,
    ExportTransaction, ExportTransactionKind,
};

/// A payload document could not be encoded or decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPayloadError {
    message: String,
}

impl SyncPayloadError {
    /// Creates a payload error with a diagnostic message.
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Returns the message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for SyncPayloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for SyncPayloadError {}

/// Encodes an entity aggregate as a compact sync payload.
///
/// # Errors
///
/// Returns [`SyncPayloadError`] when serialization fails.
pub fn encode_entity(entity: &SyncEntity) -> Result<Vec<u8>, SyncPayloadError> {
    let value = match entity {
        SyncEntity::Account(entity) => to_value(crate::export::export_account(entity))?,
        SyncEntity::AccountGroup(entity) => to_value(crate::export::export_account_group(entity))?,
        SyncEntity::Category(entity) => to_value(crate::export::export_category(entity))?,
        SyncEntity::CategoryGroup(entity) => {
            to_value(crate::export::export_category_group(entity))?
        }
        SyncEntity::Counterparty(entity) => to_value(crate::export::export_counterparty(entity))?,
        SyncEntity::ExchangeRate(entity) => to_value(crate::export::export_exchange_rate(entity))?,
        SyncEntity::LedgerProfile(entity) => to_value(crate::export::export_profile(entity))?,
        SyncEntity::LedgerSettings(entity) => to_value(crate::export::export_settings(entity))?,
        SyncEntity::NotePreset(entity) => to_value(crate::export::export_note_preset(entity))?,
        SyncEntity::ReconciliationFlag(entity) => {
            to_value(crate::export::export_reconciliation_flag(entity))?
        }
        SyncEntity::Tag(entity) => to_value(crate::export::export_tag(entity))?,
        SyncEntity::Transaction(entity) => to_value(crate::export::export_transaction(entity))?,
    };
    serde_json::to_vec(&value)
        .map_err(|error| SyncPayloadError::new(format!("serialize payload: {error}")))
}

fn to_value<T: serde::Serialize>(value: T) -> Result<serde_json::Value, SyncPayloadError> {
    serde_json::to_value(value)
        .map_err(|error| SyncPayloadError::new(format!("serialize payload: {error}")))
}

/// Decodes a sync payload into the entity aggregate for its entity type.
///
/// # Errors
///
/// Returns [`SyncPayloadError`] when the JSON is malformed or the aggregate
/// fails domain validation.
pub fn decode_entity(
    entity_type: SyncEntityType,
    payload: &[u8],
) -> Result<SyncEntity, SyncPayloadError> {
    match entity_type {
        SyncEntityType::Account => {
            let document: ExportAccount = from_slice(payload)?;
            Ok(SyncEntity::Account(account_from_document(document)?))
        }
        SyncEntityType::AccountGroup => {
            let document: ExportAccountGroup = from_slice(payload)?;
            Ok(SyncEntity::AccountGroup(account_group_from_document(
                document,
            )?))
        }
        SyncEntityType::Category => {
            let document: ExportCategory = from_slice(payload)?;
            Ok(SyncEntity::Category(category_from_document(document)?))
        }
        SyncEntityType::CategoryGroup => {
            let document: ExportCategoryGroup = from_slice(payload)?;
            Ok(SyncEntity::CategoryGroup(category_group_from_document(
                document,
            )?))
        }
        SyncEntityType::Counterparty => {
            let document: ExportCounterparty = from_slice(payload)?;
            Ok(SyncEntity::Counterparty(counterparty_from_document(
                document,
            )?))
        }
        SyncEntityType::ExchangeRate => {
            let document: ExportExchangeRate = from_slice(payload)?;
            Ok(SyncEntity::ExchangeRate(exchange_rate_from_document(
                document,
            )?))
        }
        SyncEntityType::LedgerProfile => {
            let document: ExportProfile = from_slice(payload)?;
            Ok(SyncEntity::LedgerProfile(profile_from_document(document)?))
        }
        SyncEntityType::LedgerSettings => {
            let document: ExportSettings = from_slice(payload)?;
            Ok(SyncEntity::LedgerSettings(settings_from_document(
                document,
            )?))
        }
        SyncEntityType::NotePreset => {
            let document: ExportNotePreset = from_slice(payload)?;
            Ok(SyncEntity::NotePreset(note_preset_from_document(document)?))
        }
        SyncEntityType::ReconciliationFlag => {
            let document: ExportReconciliationFlag = from_slice(payload)?;
            Ok(SyncEntity::ReconciliationFlag(
                reconciliation_flag_from_document(document)?,
            ))
        }
        SyncEntityType::Tag => {
            let document: ExportTag = from_slice(payload)?;
            Ok(SyncEntity::Tag(tag_from_document(document)?))
        }
        SyncEntityType::Transaction => {
            let document: ExportTransaction = from_slice(payload)?;
            Ok(SyncEntity::Transaction(transaction_from_document(
                document,
            )?))
        }
    }
}

/// Converts every entity in an export document to a domain aggregate in
/// dependency order: settings and profile, master data, accounts and
/// categories, then transactions and exchange rates.
///
/// # Errors
///
/// Returns [`SyncPayloadError`] for the first invalid aggregate.
pub fn entities_from_document(
    document: &ExportDocument,
) -> Result<Vec<SyncEntity>, SyncPayloadError> {
    let mut entities = Vec::new();
    entities.push(SyncEntity::LedgerSettings(settings_from_document(
        document.settings.clone(),
    )?));
    entities.push(SyncEntity::LedgerProfile(profile_from_document(
        document.profile.clone(),
    )?));
    for item in &document.account_groups {
        entities.push(SyncEntity::AccountGroup(account_group_from_document(
            item.clone(),
        )?));
    }
    for item in &document.category_groups {
        entities.push(SyncEntity::CategoryGroup(category_group_from_document(
            item.clone(),
        )?));
    }
    for item in &document.tags {
        entities.push(SyncEntity::Tag(tag_from_document(item.clone())?));
    }
    for item in &document.counterparties {
        entities.push(SyncEntity::Counterparty(counterparty_from_document(
            item.clone(),
        )?));
    }
    for item in &document.reconciliation_flags {
        entities.push(SyncEntity::ReconciliationFlag(
            reconciliation_flag_from_document(item.clone())?,
        ));
    }
    for item in &document.note_presets {
        entities.push(SyncEntity::NotePreset(note_preset_from_document(
            item.clone(),
        )?));
    }
    for item in &document.accounts {
        entities.push(SyncEntity::Account(account_from_document(item.clone())?));
    }
    for item in &document.categories {
        entities.push(SyncEntity::Category(category_from_document(item.clone())?));
    }
    for item in &document.transactions {
        entities.push(SyncEntity::Transaction(transaction_from_document(
            item.clone(),
        )?));
    }
    for item in &document.exchange_rates {
        entities.push(SyncEntity::ExchangeRate(exchange_rate_from_document(
            item.clone(),
        )?));
    }
    Ok(entities)
}

fn from_slice<T: serde::de::DeserializeOwned>(payload: &[u8]) -> Result<T, SyncPayloadError> {
    serde_json::from_slice(payload)
        .map_err(|error| SyncPayloadError::new(format!("parse payload: {error}")))
}

fn settings_from_document(document: ExportSettings) -> Result<LedgerSettings, SyncPayloadError> {
    if document.currency_policy_version != CurrencyScalePolicy::v1().version() {
        return Err(SyncPayloadError::new("unsupported currency policy version"));
    }
    Ok(LedgerSettings::new(
        parse_id(&document.id, "settings id")?,
        currency(&document.currency, document.currency_scale)?,
        document.currency_policy_version,
        LedgerCalendar::new(&document.time_zone).map_err(invalid)?,
        parse_timestamp(&document.created_at, "settings.created_at")?,
        parse_timestamp(&document.updated_at, "settings.updated_at")?,
    ))
}

fn profile_from_document(document: ExportProfile) -> Result<LedgerProfile, SyncPayloadError> {
    LedgerProfile::new(
        parse_id(&document.id, "profile id")?,
        parse_id(&document.settings_id, "profile settings id")?,
        &document.name,
        &document.emoji,
        parse_timestamp(&document.created_at, "profile.created_at")?,
        parse_timestamp(&document.updated_at, "profile.updated_at")?,
    )
    .map_err(invalid)
}

fn account_group_from_document(
    document: ExportAccountGroup,
) -> Result<AccountGroup, SyncPayloadError> {
    AccountGroup::new(
        parse_id(&document.id, "account group id")?,
        &document.name,
        document.sort_order,
        document.is_archived,
        parse_timestamp(&document.created_at, "account_group.created_at")?,
        parse_timestamp(&document.updated_at, "account_group.updated_at")?,
    )
    .map_err(invalid)
}

fn account_from_document(document: ExportAccount) -> Result<Account, SyncPayloadError> {
    let tag_ids: BTreeSet<TagId> = document
        .tag_ids
        .iter()
        .map(|value| parse_id(value, "account tag id"))
        .collect::<Result<_, _>>()?;
    Account::new(
        parse_id(&document.id, "account id")?,
        &document.name,
        asset_type(document.asset_type),
        currency(&document.currency, document.currency_scale)?,
        AccountOptions {
            sort_order: document.sort_order,
            is_archived: document.is_archived,
            tag_ids,
            owner_id: optional_id(document.owner_id.as_deref(), "account owner id")?,
            custom_group_id: optional_id(document.account_group_id.as_deref(), "account group id")?,
            note: document.note,
            is_hidden: document.is_hidden,
            is_included_in_net_worth: document.is_included_in_net_worth,
        },
        parse_timestamp(&document.created_at, "account.created_at")?,
        parse_timestamp(&document.updated_at, "account.updated_at")?,
    )
    .map_err(invalid)
}

fn category_group_from_document(
    document: ExportCategoryGroup,
) -> Result<CategoryGroup, SyncPayloadError> {
    CategoryGroup::new(
        parse_id(&document.id, "category group id")?,
        &document.name,
        category_kind(document.kind),
        document.sort_order,
        document.is_archived,
        parse_timestamp(&document.created_at, "category_group.created_at")?,
        parse_timestamp(&document.updated_at, "category_group.updated_at")?,
    )
    .map_err(invalid)
}

fn category_from_document(document: ExportCategory) -> Result<Category, SyncPayloadError> {
    Category::new(
        parse_id(&document.id, "category id")?,
        &document.name,
        category_kind(document.kind),
        CategoryOptions {
            group_id: optional_id(document.group_id.as_deref(), "category group id")?,
            sort_order: document.sort_order,
            is_archived: document.is_archived,
        },
        parse_timestamp(&document.created_at, "category.created_at")?,
        parse_timestamp(&document.updated_at, "category.updated_at")?,
    )
    .map_err(invalid)
}

fn tag_from_document(document: ExportTag) -> Result<LedgerTag, SyncPayloadError> {
    LedgerTag::new(
        parse_id(&document.id, "tag id")?,
        &document.name,
        &document.color_hex,
        TagOptions {
            description: document.description,
            sort_order: document.sort_order,
            is_archived: document.is_archived,
        },
        parse_timestamp(&document.created_at, "tag.created_at")?,
        parse_timestamp(&document.updated_at, "tag.updated_at")?,
    )
    .map_err(invalid)
}

fn counterparty_from_document(
    document: ExportCounterparty,
) -> Result<Counterparty, SyncPayloadError> {
    Counterparty::new(
        parse_id(&document.id, "counterparty id")?,
        counterparty_kind(document.kind),
        &document.name,
        document.sort_order,
        document.is_archived,
        parse_timestamp(&document.created_at, "counterparty.created_at")?,
        parse_timestamp(&document.updated_at, "counterparty.updated_at")?,
    )
    .map_err(invalid)
}

fn reconciliation_flag_from_document(
    document: ExportReconciliationFlag,
) -> Result<ReconciliationFlag, SyncPayloadError> {
    ReconciliationFlag::new(
        parse_id(&document.id, "reconciliation flag id")?,
        &document.name,
        &document.color_hex,
        ReconciliationFlagOptions {
            system_kind: document.system_kind.map(reconciliation_flag_kind),
            locks_transactions: document.locks_transactions,
            shortcut_key: document.shortcut_key,
            sort_order: document.sort_order,
            is_archived: document.is_archived,
        },
        parse_timestamp(&document.created_at, "reconciliation_flag.created_at")?,
        parse_timestamp(&document.updated_at, "reconciliation_flag.updated_at")?,
    )
    .map_err(invalid)
}

fn note_preset_from_document(
    document: ExportNotePreset,
) -> Result<TransactionNotePreset, SyncPayloadError> {
    TransactionNotePreset::new(
        parse_id(&document.id, "note preset id")?,
        &document.text,
        document.sort_order,
        document.is_archived,
        parse_timestamp(&document.created_at, "note_preset.created_at")?,
        parse_timestamp(&document.updated_at, "note_preset.updated_at")?,
    )
    .map_err(invalid)
}

fn transaction_from_document(document: ExportTransaction) -> Result<Transaction, SyncPayloadError> {
    let tag_ids: BTreeSet<TagId> = document
        .tag_ids
        .iter()
        .map(|value| parse_id(value, "transaction tag id"))
        .collect::<Result<_, _>>()?;
    Transaction::new(
        parse_id(&document.id, "transaction id")?,
        transaction_kind(document.kind),
        money(document.amount)?,
        parse_timestamp(&document.occurred_at, "transaction.occurred_at")?,
        parse_id(&document.account_id, "transaction account id")?,
        TransactionFields {
            destination_amount: document.destination_amount.map(money).transpose()?,
            destination_account_id: optional_id(
                document.destination_account_id.as_deref(),
                "transaction destination account id",
            )?,
            category_id: optional_id(document.category_id.as_deref(), "transaction category id")?,
            tag_ids,
            reconciliation_flag_id: optional_id(
                document.reconciliation_flag_id.as_deref(),
                "transaction reconciliation flag id",
            )?,
            counterparty_id: optional_id(
                document.counterparty_id.as_deref(),
                "transaction counterparty id",
            )?,
            note: document.note,
            deleted_at: document
                .deleted_at
                .as_deref()
                .map(|value| parse_timestamp(value, "transaction.deleted_at"))
                .transpose()?,
        },
        parse_timestamp(&document.created_at, "transaction.created_at")?,
        parse_timestamp(&document.updated_at, "transaction.updated_at")?,
    )
    .map_err(invalid)
}

fn exchange_rate_from_document(
    document: ExportExchangeRate,
) -> Result<ExchangeRate, SyncPayloadError> {
    ExchangeRate::new(
        parse_id(&document.id, "exchange rate id")?,
        currency(&document.source_currency, document.source_currency_scale)?,
        currency(
            &document.reporting_currency,
            document.reporting_currency_scale,
        )?,
        ExactRatio::new(document.quote_numerator, document.quote_denominator).map_err(invalid)?,
        parse_timestamp(&document.effective_at, "exchange_rate.effective_at")?,
        match document.source {
            ExportExchangeRateSource::Manual => ExchangeRateSource::Manual,
            ExportExchangeRateSource::EuropeanCentralBank => {
                ExchangeRateSource::EuropeanCentralBank
            }
        },
        document
            .superseded_at
            .as_deref()
            .map(|value| parse_timestamp(value, "exchange_rate.superseded_at"))
            .transpose()?,
        parse_timestamp(&document.created_at, "exchange_rate.created_at")?,
        parse_timestamp(&document.updated_at, "exchange_rate.updated_at")?,
    )
    .map_err(invalid)
}

fn parse_id<T: FromStr>(value: &str, field: &str) -> Result<T, SyncPayloadError>
where
    T::Err: fmt::Display,
{
    value
        .parse()
        .map_err(|error| SyncPayloadError::new(format!("invalid {field}: {error}")))
}

fn optional_id<T: FromStr>(value: Option<&str>, field: &str) -> Result<Option<T>, SyncPayloadError>
where
    T::Err: fmt::Display,
{
    value.map(|value| parse_id(value, field)).transpose()
}

fn parse_timestamp(value: &str, field: &str) -> Result<Timestamp, SyncPayloadError> {
    value
        .parse()
        .map_err(|error| SyncPayloadError::new(format!("invalid {field}: {error}")))
}

fn currency(code: &str, scale: u8) -> Result<Currency, SyncPayloadError> {
    let currency = CurrencyScalePolicy::v1().currency(code).map_err(invalid)?;
    if currency.scale() != scale {
        return Err(SyncPayloadError::new(format!(
            "currency {code} has scale {} in the policy, not {scale}",
            currency.scale()
        )));
    }
    Ok(currency)
}

fn money(value: ExportMoney) -> Result<Money, SyncPayloadError> {
    Ok(Money::new(
        value.minor_units,
        currency(&value.currency, value.scale)?,
    ))
}

fn asset_type(value: ExportAssetType) -> AssetAccountType {
    match value {
        ExportAssetType::Cash => AssetAccountType::Cash,
        ExportAssetType::Checking => AssetAccountType::Checking,
        ExportAssetType::Savings => AssetAccountType::Savings,
        ExportAssetType::StoredValue => AssetAccountType::StoredValue,
    }
}

fn category_kind(value: ExportCategoryKind) -> CategoryKind {
    match value {
        ExportCategoryKind::Expense => CategoryKind::Expense,
        ExportCategoryKind::Income => CategoryKind::Income,
    }
}

fn counterparty_kind(value: ExportCounterpartyKind) -> CounterpartyKind {
    match value {
        ExportCounterpartyKind::HouseholdMember => CounterpartyKind::HouseholdMember,
        ExportCounterpartyKind::Person => CounterpartyKind::Person,
        ExportCounterpartyKind::Organization => CounterpartyKind::Organization,
    }
}

fn reconciliation_flag_kind(value: ExportReconciliationFlagKind) -> ReconciliationFlagSystemKind {
    match value {
        ExportReconciliationFlagKind::Cleared => ReconciliationFlagSystemKind::Cleared,
        ExportReconciliationFlagKind::NeedsReimbursement => {
            ReconciliationFlagSystemKind::NeedsReimbursement
        }
        ExportReconciliationFlagKind::Issue => ReconciliationFlagSystemKind::Issue,
    }
}

fn transaction_kind(value: ExportTransactionKind) -> TransactionKind {
    match value {
        ExportTransactionKind::OpeningBalance => TransactionKind::OpeningBalance,
        ExportTransactionKind::Expense => TransactionKind::Expense,
        ExportTransactionKind::Income => TransactionKind::Income,
        ExportTransactionKind::Transfer => TransactionKind::Transfer,
    }
}

fn invalid(error: impl fmt::Display) -> SyncPayloadError {
    SyncPayloadError::new(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../../contracts/export/v1/fixture.json");

    #[test]
    fn every_fixture_entity_round_trips() {
        let document: ExportDocument = serde_json::from_str(FIXTURE).unwrap();
        let entities = entities_from_document(&document).unwrap();
        assert!(entities.len() >= 12);
        for entity in entities {
            let payload = encode_entity(&entity).unwrap();
            let decoded = decode_entity(entity.entity_type(), &payload).unwrap();
            assert_eq!(decoded, entity);
        }
    }

    #[test]
    fn malformed_payloads_are_rejected() {
        assert!(decode_entity(SyncEntityType::Tag, b"not json").is_err());
        assert!(decode_entity(SyncEntityType::Account, br#"{"id":"x"}"#).is_err());
        assert!(decode_entity(SyncEntityType::Tag, br#"{"unexpected":true}"#).is_err());

        let document: ExportDocument = serde_json::from_str(FIXTURE).unwrap();
        let mut settings = serde_json::to_value(&document.settings).unwrap();
        settings["currency_policy_version"] = serde_json::json!(2);
        assert!(
            decode_entity(
                SyncEntityType::LedgerSettings,
                &serde_json::to_vec(&settings).unwrap()
            )
            .is_err()
        );

        let mut account = serde_json::to_value(&document.accounts[0]).unwrap();
        account["currency_scale"] = serde_json::json!(3);
        assert!(
            decode_entity(
                SyncEntityType::Account,
                &serde_json::to_vec(&account).unwrap()
            )
            .is_err()
        );
    }
}
