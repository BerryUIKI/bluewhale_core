//! SQLite/SQLCipher repository implementations.

use std::collections::BTreeSet;
use std::path::Path;
use std::str::FromStr;

use core_domain::{
    Account, AccountGroup, AccountGroupId, AccountGroupRepository, AccountId, AccountOptions,
    AccountRepository, AccountSetupRepository, AssetAccountType, Category, CategoryGroup,
    CategoryGroupId, CategoryGroupRepository, CategoryId, CategoryKind, CategoryOptions,
    CategoryRepository, Counterparty, CounterpartyId, CounterpartyKind, CounterpartyRepository,
    Currency, ExchangeRate, ExchangeRateId, ExchangeRateRepository, ExchangeRateSource,
    LedgerBootstrapRepository, LedgerCalendar, LedgerExportRepository, LedgerExportSnapshot,
    LedgerProfile, LedgerProfileId, LedgerProfileRepository, LedgerReadSnapshot,
    LedgerReadSnapshotRepository, LedgerSettings, LedgerSettingsId, LedgerSettingsRepository,
    LedgerTag, Money, ReconciliationFlag, ReconciliationFlagId, ReconciliationFlagOptions,
    ReconciliationFlagRepository, ReconciliationFlagSystemKind, RepositoryError, SyncConflict,
    SyncConflictResolution, SyncEntity, SyncEntityType, SyncOperation, SyncOperationType,
    SyncRepository, TagId, TagOptions, TagRepository, Transaction, TransactionFields,
    TransactionId, TransactionKind, TransactionNotePreset, TransactionNotePresetId,
    TransactionNotePresetRepository, TransactionNotePresetValidationError, TransactionRepository,
    TransactionValidationError,
};
use jiff::Timestamp;
use rusqlite::types::FromSql;
use rusqlite::{Connection, Row, RowIndex, params};
use uuid::Uuid;

use crate::database::{LedgerDatabase, to_repository_error};

trait RowExt {
    fn get_value<I: RowIndex, T: FromSql>(&self, index: I) -> Result<T, RepositoryError>;
}

impl RowExt for Row<'_> {
    fn get_value<I: RowIndex, T: FromSql>(&self, index: I) -> Result<T, RepositoryError> {
        self.get(index).map_err(to_repository_error)
    }
}

/// A ledger stored in SQLCipher through rusqlite.
#[derive(Debug)]
pub struct SqliteLedger {
    database: LedgerDatabase,
}

impl SqliteLedger {
    /// Opens or creates an encrypted ledger database file.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when the database cannot be opened,
    /// decrypted, or migrated.
    pub fn open(path: &Path, key: &str) -> Result<Self, RepositoryError> {
        Ok(Self {
            database: LedgerDatabase::open(path, key)?,
        })
    }

    /// Opens an in-memory encrypted ledger database.
    ///
    /// # Errors
    ///
    /// Returns a [`RepositoryError`] when initialization fails.
    pub fn open_in_memory(key: &str) -> Result<Self, RepositoryError> {
        Ok(Self {
            database: LedgerDatabase::open_in_memory(key)?,
        })
    }

    /// Returns the database, for schema checks in tests and tooling.
    #[must_use]
    pub fn database(&self) -> &LedgerDatabase {
        &self.database
    }
}

fn parse_uuid<T>(value: &str) -> Result<T, RepositoryError>
where
    T: FromStr<Err = uuid::Error>,
{
    value
        .parse()
        .map_err(|error: uuid::Error| RepositoryError::new(error.to_string()))
}

fn parse_optional_uuid<T>(value: Option<String>) -> Result<Option<T>, RepositoryError>
where
    T: FromStr<Err = uuid::Error>,
{
    value.map(|value| parse_uuid(&value)).transpose()
}

fn timestamp_milliseconds(instant: Timestamp) -> i64 {
    instant.as_millisecond()
}

fn timestamp_from_milliseconds(value: i64) -> Result<Timestamp, RepositoryError> {
    Timestamp::from_millisecond(value).map_err(|error| RepositoryError::new(error.to_string()))
}

fn optional_timestamp(value: Option<i64>) -> Result<Option<Timestamp>, RepositoryError> {
    value.map(timestamp_from_milliseconds).transpose()
}

fn currency_from_parts(code: &str, scale: i64) -> Result<Currency, RepositoryError> {
    let scale = u8::try_from(scale)
        .map_err(|_| RepositoryError::new(format!("invalid currency scale: {scale}")))?;
    Currency::new(code, scale).map_err(|error| RepositoryError::new(error.to_string()))
}

fn currency_parts(currency: &Currency) -> (String, i64) {
    (currency.code().to_string(), i64::from(currency.scale()))
}

fn bool_value(value: i64) -> bool {
    value != 0
}

fn int_value(value: bool) -> i64 {
    i64::from(value)
}

fn asset_type_name(asset_type: AssetAccountType) -> &'static str {
    match asset_type {
        AssetAccountType::Cash => "cash",
        AssetAccountType::Checking => "checking",
        AssetAccountType::Savings => "savings",
        AssetAccountType::StoredValue => "stored_value",
    }
}

fn asset_type_from_name(name: &str) -> Result<AssetAccountType, RepositoryError> {
    match name {
        "cash" => Ok(AssetAccountType::Cash),
        "checking" => Ok(AssetAccountType::Checking),
        "savings" => Ok(AssetAccountType::Savings),
        "stored_value" => Ok(AssetAccountType::StoredValue),
        other => Err(RepositoryError::new(format!(
            "unknown asset account type: {other}"
        ))),
    }
}

fn category_kind_name(kind: CategoryKind) -> &'static str {
    match kind {
        CategoryKind::Expense => "expense",
        CategoryKind::Income => "income",
    }
}

fn category_kind_from_name(name: &str) -> Result<CategoryKind, RepositoryError> {
    match name {
        "expense" => Ok(CategoryKind::Expense),
        "income" => Ok(CategoryKind::Income),
        other => Err(RepositoryError::new(format!(
            "unknown category kind: {other}"
        ))),
    }
}

fn counterparty_kind_name(kind: CounterpartyKind) -> &'static str {
    match kind {
        CounterpartyKind::HouseholdMember => "household_member",
        CounterpartyKind::Person => "person",
        CounterpartyKind::Organization => "organization",
    }
}

fn counterparty_kind_from_name(name: &str) -> Result<CounterpartyKind, RepositoryError> {
    match name {
        "household_member" => Ok(CounterpartyKind::HouseholdMember),
        "person" => Ok(CounterpartyKind::Person),
        "organization" => Ok(CounterpartyKind::Organization),
        other => Err(RepositoryError::new(format!(
            "unknown counterparty kind: {other}"
        ))),
    }
}

fn transaction_kind_name(kind: TransactionKind) -> &'static str {
    match kind {
        TransactionKind::OpeningBalance => "opening_balance",
        TransactionKind::Expense => "expense",
        TransactionKind::Income => "income",
        TransactionKind::Transfer => "transfer",
    }
}

fn transaction_kind_from_name(name: &str) -> Result<TransactionKind, RepositoryError> {
    match name {
        "opening_balance" => Ok(TransactionKind::OpeningBalance),
        "expense" => Ok(TransactionKind::Expense),
        "income" => Ok(TransactionKind::Income),
        "transfer" => Ok(TransactionKind::Transfer),
        other => Err(RepositoryError::new(format!(
            "unknown transaction kind: {other}"
        ))),
    }
}

fn exchange_rate_source_name(source: ExchangeRateSource) -> &'static str {
    match source {
        ExchangeRateSource::Manual => "manual",
        ExchangeRateSource::EuropeanCentralBank => "european_central_bank",
    }
}

fn exchange_rate_source_from_name(name: &str) -> Result<ExchangeRateSource, RepositoryError> {
    match name {
        "manual" => Ok(ExchangeRateSource::Manual),
        "european_central_bank" => Ok(ExchangeRateSource::EuropeanCentralBank),
        other => Err(RepositoryError::new(format!(
            "unknown exchange rate source: {other}"
        ))),
    }
}

fn system_flag_kind_name(kind: ReconciliationFlagSystemKind) -> &'static str {
    match kind {
        ReconciliationFlagSystemKind::Cleared => "cleared",
        ReconciliationFlagSystemKind::NeedsReimbursement => "needs_reimbursement",
        ReconciliationFlagSystemKind::Issue => "issue",
    }
}

fn system_flag_kind_from_name(name: &str) -> Result<ReconciliationFlagSystemKind, RepositoryError> {
    match name {
        "cleared" => Ok(ReconciliationFlagSystemKind::Cleared),
        "needs_reimbursement" => Ok(ReconciliationFlagSystemKind::NeedsReimbursement),
        "issue" => Ok(ReconciliationFlagSystemKind::Issue),
        other => Err(RepositoryError::new(format!(
            "unknown reconciliation flag kind: {other}"
        ))),
    }
}

fn collect_rows<T>(
    connection: &Connection,
    sql: &str,
    mut map: impl FnMut(&Row<'_>) -> Result<T, RepositoryError>,
) -> Result<Vec<T>, RepositoryError> {
    let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
    let mut rows = statement.query([]).map_err(to_repository_error)?;
    let mut items = Vec::new();
    while let Some(row) = rows.next().map_err(to_repository_error)? {
        items.push(map(row)?);
    }
    Ok(items)
}

fn map_settings(row: &Row<'_>) -> Result<LedgerSettings, RepositoryError> {
    let id: LedgerSettingsId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let currency = currency_from_parts(
        &row.get_value::<_, String>("currency_code")?,
        row.get_value::<_, i64>("currency_scale")?,
    )?;
    let policy_version = u32::try_from(row.get_value::<_, i64>("currency_policy_version")?)
        .map_err(|_| RepositoryError::new("invalid currency policy version"))?;
    let calendar_identifier = row.get_value::<_, String>("calendar_identifier")?;
    if calendar_identifier != "gregorian" {
        return Err(RepositoryError::new(format!(
            "unknown calendar identifier: {calendar_identifier}"
        )));
    }
    let calendar = LedgerCalendar::new(&row.get_value::<_, String>("time_zone_identifier")?)
        .map_err(|error| RepositoryError::new(error.to_string()))?;
    Ok(LedgerSettings::new(
        id,
        currency,
        policy_version,
        calendar,
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    ))
}

fn map_profile(row: &Row<'_>) -> Result<LedgerProfile, RepositoryError> {
    let id: LedgerProfileId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let settings_id: LedgerSettingsId = parse_uuid(&row.get_value::<_, String>("settings_id")?)?;
    LedgerProfile::new(
        id,
        settings_id,
        &row.get_value::<_, String>("name")?,
        &row.get_value::<_, String>("emoji")?,
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_account_group(row: &Row<'_>) -> Result<AccountGroup, RepositoryError> {
    let id: AccountGroupId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    AccountGroup::new(
        id,
        &row.get_value::<_, String>("name")?,
        i32::try_from(row.get_value::<_, i64>("sort_order")?)
            .map_err(|_| RepositoryError::new("invalid sort order"))?,
        bool_value(row.get_value("is_archived")?),
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_tag(row: &Row<'_>) -> Result<LedgerTag, RepositoryError> {
    let id: TagId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    LedgerTag::new(
        id,
        &row.get_value::<_, String>("name")?,
        &row.get_value::<_, String>("color_hex")?,
        TagOptions {
            description: row.get_value("description")?,
            sort_order: i32::try_from(row.get_value::<_, i64>("sort_order")?)
                .map_err(|_| RepositoryError::new("invalid sort order"))?,
            is_archived: bool_value(row.get_value("is_archived")?),
        },
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_counterparty(row: &Row<'_>) -> Result<Counterparty, RepositoryError> {
    let id: CounterpartyId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    Counterparty::new(
        id,
        counterparty_kind_from_name(&row.get_value::<_, String>("kind")?)?,
        &row.get_value::<_, String>("name")?,
        i32::try_from(row.get_value::<_, i64>("sort_order")?)
            .map_err(|_| RepositoryError::new("invalid sort order"))?,
        bool_value(row.get_value("is_archived")?),
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_category_group(row: &Row<'_>) -> Result<CategoryGroup, RepositoryError> {
    let id: CategoryGroupId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    CategoryGroup::new(
        id,
        &row.get_value::<_, String>("name")?,
        category_kind_from_name(&row.get_value::<_, String>("kind")?)?,
        i32::try_from(row.get_value::<_, i64>("sort_order")?)
            .map_err(|_| RepositoryError::new("invalid sort order"))?,
        bool_value(row.get_value("is_archived")?),
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_category(row: &Row<'_>) -> Result<Category, RepositoryError> {
    let id: CategoryId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let group_id: Option<CategoryGroupId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("group_id")?)?;
    Category::new(
        id,
        &row.get_value::<_, String>("name")?,
        category_kind_from_name(&row.get_value::<_, String>("kind")?)?,
        CategoryOptions {
            group_id,
            sort_order: i32::try_from(row.get_value::<_, i64>("sort_order")?)
                .map_err(|_| RepositoryError::new("invalid sort order"))?,
            is_archived: bool_value(row.get_value("is_archived")?),
        },
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_reconciliation_flag(row: &Row<'_>) -> Result<ReconciliationFlag, RepositoryError> {
    let id: ReconciliationFlagId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let system_kind = row
        .get_value::<_, Option<String>>("system_kind")?
        .map(|name| system_flag_kind_from_name(&name))
        .transpose()?;
    ReconciliationFlag::new(
        id,
        &row.get_value::<_, String>("name")?,
        &row.get_value::<_, String>("color_hex")?,
        ReconciliationFlagOptions {
            system_kind,
            locks_transactions: bool_value(row.get_value("locks_transactions")?),
            shortcut_key: row.get_value("shortcut_key")?,
            sort_order: i32::try_from(row.get_value::<_, i64>("sort_order")?)
                .map_err(|_| RepositoryError::new("invalid sort order"))?,
            is_archived: bool_value(row.get_value("is_archived")?),
        },
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_note_preset(row: &Row<'_>) -> Result<TransactionNotePreset, RepositoryError> {
    let id: TransactionNotePresetId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    TransactionNotePreset::new(
        id,
        &row.get_value::<_, String>("text")?,
        i32::try_from(row.get_value::<_, i64>("sort_order")?)
            .map_err(|_| RepositoryError::new("invalid sort order"))?,
        bool_value(row.get_value("is_archived")?),
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error: TransactionNotePresetValidationError| RepositoryError::new(error.to_string()))
}

fn load_account_tags(
    connection: &Connection,
    account_id: AccountId,
) -> Result<BTreeSet<TagId>, RepositoryError> {
    let mut statement = connection
        .prepare("SELECT tag_id FROM account_tags WHERE account_id = ?1")
        .map_err(to_repository_error)?;
    let rows = statement
        .query_map([account_id.to_string()], |row| row.get::<_, String>(0))
        .map_err(to_repository_error)?;
    let mut tags = BTreeSet::new();
    for row in rows {
        let value = row.map_err(to_repository_error)?;
        tags.insert(parse_uuid(&value)?);
    }
    Ok(tags)
}

fn load_transaction_tags(
    connection: &Connection,
    transaction_id: TransactionId,
) -> Result<BTreeSet<TagId>, RepositoryError> {
    let mut statement = connection
        .prepare("SELECT tag_id FROM transaction_tags WHERE transaction_id = ?1")
        .map_err(to_repository_error)?;
    let rows = statement
        .query_map([transaction_id.to_string()], |row| row.get::<_, String>(0))
        .map_err(to_repository_error)?;
    let mut tags = BTreeSet::new();
    for row in rows {
        let value = row.map_err(to_repository_error)?;
        tags.insert(parse_uuid(&value)?);
    }
    Ok(tags)
}

fn map_account(connection: &Connection, row: &Row<'_>) -> Result<Account, RepositoryError> {
    let id: AccountId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let currency = currency_from_parts(
        &row.get_value::<_, String>("currency_code")?,
        row.get_value::<_, i64>("currency_scale")?,
    )?;
    let owner_id: Option<CounterpartyId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("owner_id")?)?;
    let custom_group_id: Option<AccountGroupId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("custom_group_id")?)?;
    let tags = load_account_tags(connection, id)?;
    Account::new(
        id,
        &row.get_value::<_, String>("name")?,
        asset_type_from_name(&row.get_value::<_, String>("asset_type")?)?,
        currency,
        AccountOptions {
            sort_order: i32::try_from(row.get_value::<_, i64>("sort_order")?)
                .map_err(|_| RepositoryError::new("invalid sort order"))?,
            is_archived: bool_value(row.get_value("is_archived")?),
            tag_ids: tags,
            owner_id,
            custom_group_id,
            note: row.get_value::<_, String>("note")?,
            is_hidden: bool_value(row.get_value("is_hidden")?),
            is_included_in_net_worth: bool_value(row.get_value("is_included_in_net_worth")?),
        },
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn map_transaction(connection: &Connection, row: &Row<'_>) -> Result<Transaction, RepositoryError> {
    let id: TransactionId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let amount_currency = currency_from_parts(
        &row.get_value::<_, String>("amount_currency_code")?,
        row.get_value::<_, i64>("amount_currency_scale")?,
    )?;
    let amount = Money::new(row.get_value::<_, i64>("amount_minor")?, amount_currency);
    let destination_minor: Option<i64> = row.get_value("destination_minor")?;
    let destination_amount = match destination_minor {
        Some(minor) => Some(Money::new(
            minor,
            currency_from_parts(
                &row.get_value::<_, String>("destination_currency_code")?,
                row.get_value::<_, i64>("destination_currency_scale")?,
            )?,
        )),
        None => None,
    };
    let account_id: AccountId = parse_uuid(&row.get_value::<_, String>("account_id")?)?;
    let destination_account_id: Option<AccountId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("destination_account_id")?)?;
    let category_id: Option<CategoryId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("category_id")?)?;
    let reconciliation_flag_id: Option<ReconciliationFlagId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("reconciliation_flag_id")?)?;
    let counterparty_id: Option<CounterpartyId> =
        parse_optional_uuid(row.get_value::<_, Option<String>>("counterparty_id")?)?;
    let tag_ids = load_transaction_tags(connection, id)?;

    Transaction::new(
        id,
        transaction_kind_from_name(&row.get_value::<_, String>("kind")?)?,
        amount,
        timestamp_from_milliseconds(row.get_value("occurred_at")?)?,
        account_id,
        TransactionFields {
            destination_amount,
            destination_account_id,
            category_id,
            tag_ids,
            reconciliation_flag_id,
            counterparty_id,
            note: row.get_value("note")?,
            deleted_at: optional_timestamp(row.get_value("deleted_at")?)?,
        },
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error: TransactionValidationError| RepositoryError::new(error.to_string()))
}

fn map_exchange_rate(row: &Row<'_>) -> Result<ExchangeRate, RepositoryError> {
    let id: ExchangeRateId = parse_uuid(&row.get_value::<_, String>("id")?)?;
    let source_currency = currency_from_parts(
        &row.get_value::<_, String>("source_currency_code")?,
        row.get_value::<_, i64>("source_currency_scale")?,
    )?;
    let reporting_currency = currency_from_parts(
        &row.get_value::<_, String>("reporting_currency_code")?,
        row.get_value::<_, i64>("reporting_currency_scale")?,
    )?;
    let quote = core_domain::ExactRatio::new(
        row.get_value::<_, i64>("quote_numerator")?,
        row.get_value::<_, i64>("quote_denominator")?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))?;
    ExchangeRate::new(
        id,
        source_currency,
        reporting_currency,
        quote,
        timestamp_from_milliseconds(row.get_value("effective_at")?)?,
        exchange_rate_source_from_name(&row.get_value::<_, String>("source")?)?,
        optional_timestamp(row.get_value("superseded_at")?)?,
        timestamp_from_milliseconds(row.get_value("created_at")?)?,
        timestamp_from_milliseconds(row.get_value("updated_at")?)?,
    )
    .map_err(|error| RepositoryError::new(error.to_string()))
}

fn upsert_settings(
    connection: &Connection,
    settings: &LedgerSettings,
) -> Result<(), RepositoryError> {
    let (code, scale) = currency_parts(settings.currency());
    connection
        .execute(
            "INSERT INTO settings (id, currency_code, currency_scale, currency_policy_version, calendar_identifier, time_zone_identifier, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, 'gregorian', ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 currency_code = excluded.currency_code,
                 currency_scale = excluded.currency_scale,
                 currency_policy_version = excluded.currency_policy_version,
                 calendar_identifier = excluded.calendar_identifier,
                 time_zone_identifier = excluded.time_zone_identifier,
                 updated_at = excluded.updated_at",
            params![
                settings.id().to_string(),
                code,
                scale,
                i64::from(settings.currency_policy_version()),
                settings.calendar().identifier(),
                timestamp_milliseconds(settings.created_at()),
                timestamp_milliseconds(settings.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_profile(connection: &Connection, profile: &LedgerProfile) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO profile (id, settings_id, name, emoji, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 settings_id = excluded.settings_id,
                 name = excluded.name,
                 emoji = excluded.emoji,
                 updated_at = excluded.updated_at",
            params![
                profile.id().to_string(),
                profile.settings_id().to_string(),
                profile.name(),
                profile.emoji(),
                timestamp_milliseconds(profile.created_at()),
                timestamp_milliseconds(profile.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_account(connection: &Connection, account: &Account) -> Result<(), RepositoryError> {
    let (code, scale) = currency_parts(account.currency());
    connection
        .execute(
            "INSERT INTO accounts (id, name, asset_type, currency_code, currency_scale, sort_order, is_archived, owner_id, custom_group_id, note, is_hidden, is_included_in_net_worth, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 asset_type = excluded.asset_type,
                 currency_code = excluded.currency_code,
                 currency_scale = excluded.currency_scale,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 owner_id = excluded.owner_id,
                 custom_group_id = excluded.custom_group_id,
                 note = excluded.note,
                 is_hidden = excluded.is_hidden,
                 is_included_in_net_worth = excluded.is_included_in_net_worth,
                 updated_at = excluded.updated_at",
            params![
                account.id().to_string(),
                account.name(),
                asset_type_name(account.asset_type()),
                code,
                scale,
                i64::from(account.options().sort_order),
                int_value(account.options().is_archived),
                account.options().owner_id.map(|id| id.to_string()),
                account.options().custom_group_id.map(|id| id.to_string()),
                account.options().note,
                int_value(account.options().is_hidden),
                int_value(account.options().is_included_in_net_worth),
                timestamp_milliseconds(account.created_at()),
                timestamp_milliseconds(account.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    connection
        .execute(
            "DELETE FROM account_tags WHERE account_id = ?1",
            [account.id().to_string()],
        )
        .map_err(to_repository_error)?;
    for tag_id in &account.options().tag_ids {
        connection
            .execute(
                "INSERT INTO account_tags (account_id, tag_id) VALUES (?1, ?2)",
                params![account.id().to_string(), tag_id.to_string()],
            )
            .map_err(to_repository_error)?;
    }
    Ok(())
}

fn upsert_account_group(
    connection: &Connection,
    group: &AccountGroup,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO account_groups (id, name, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                group.id().to_string(),
                group.name(),
                i64::from(group.sort_order()),
                int_value(group.is_archived()),
                timestamp_milliseconds(group.created_at()),
                timestamp_milliseconds(group.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_category(connection: &Connection, category: &Category) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO categories (id, name, kind, group_id, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 kind = excluded.kind,
                 group_id = excluded.group_id,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                category.id().to_string(),
                category.name(),
                category_kind_name(category.kind()),
                category.options().group_id.map(|id| id.to_string()),
                i64::from(category.options().sort_order),
                int_value(category.options().is_archived),
                timestamp_milliseconds(category.created_at()),
                timestamp_milliseconds(category.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_category_group(
    connection: &Connection,
    group: &CategoryGroup,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO category_groups (id, name, kind, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 kind = excluded.kind,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                group.id().to_string(),
                group.name(),
                category_kind_name(group.kind()),
                i64::from(group.sort_order()),
                int_value(group.is_archived()),
                timestamp_milliseconds(group.created_at()),
                timestamp_milliseconds(group.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_tag(connection: &Connection, tag: &LedgerTag) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO tags (id, name, color_hex, description, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET
                 name = excluded.name,
                 color_hex = excluded.color_hex,
                 description = excluded.description,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                tag.id().to_string(),
                tag.name(),
                tag.color_hex(),
                tag.options().description,
                i64::from(tag.options().sort_order),
                int_value(tag.options().is_archived),
                timestamp_milliseconds(tag.created_at()),
                timestamp_milliseconds(tag.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_counterparty(
    connection: &Connection,
    counterparty: &Counterparty,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO counterparties (id, kind, name, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET
                 kind = excluded.kind,
                 name = excluded.name,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                counterparty.id().to_string(),
                counterparty_kind_name(counterparty.kind()),
                counterparty.name(),
                i64::from(counterparty.sort_order()),
                int_value(counterparty.is_archived()),
                timestamp_milliseconds(counterparty.created_at()),
                timestamp_milliseconds(counterparty.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_reconciliation_flag(
    connection: &Connection,
    flag: &ReconciliationFlag,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO reconciliation_flags (id, system_kind, name, color_hex, locks_transactions, shortcut_key, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET
                 system_kind = excluded.system_kind,
                 name = excluded.name,
                 color_hex = excluded.color_hex,
                 locks_transactions = excluded.locks_transactions,
                 shortcut_key = excluded.shortcut_key,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                flag.id().to_string(),
                flag.options().system_kind.map(system_flag_kind_name),
                flag.name(),
                flag.color_hex(),
                int_value(flag.locks_transactions()),
                flag.options().shortcut_key,
                i64::from(flag.options().sort_order),
                int_value(flag.is_archived()),
                timestamp_milliseconds(flag.created_at()),
                timestamp_milliseconds(flag.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_note_preset(
    connection: &Connection,
    preset: &TransactionNotePreset,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO note_presets (id, text, sort_order, is_archived, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)
             ON CONFLICT(id) DO UPDATE SET
                 text = excluded.text,
                 sort_order = excluded.sort_order,
                 is_archived = excluded.is_archived,
                 updated_at = excluded.updated_at",
            params![
                preset.id().to_string(),
                preset.text(),
                i64::from(preset.sort_order()),
                int_value(preset.is_archived()),
                timestamp_milliseconds(preset.created_at()),
                timestamp_milliseconds(preset.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn upsert_transaction(
    connection: &Connection,
    transaction: &Transaction,
) -> Result<(), RepositoryError> {
    let (amount_code, amount_scale) = currency_parts(transaction.amount().currency());
    let destination_parts = transaction
        .fields()
        .destination_amount
        .as_ref()
        .map(|money| {
            let (code, scale) = currency_parts(money.currency());
            (money.minor_units(), code, scale)
        });
    connection
        .execute(
            "INSERT INTO transactions (id, kind, amount_minor, amount_currency_code, amount_currency_scale,
                 destination_minor, destination_currency_code, destination_currency_scale,
                 occurred_at, account_id, destination_account_id, category_id, reconciliation_flag_id,
                 counterparty_id, note, created_at, updated_at, deleted_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)
             ON CONFLICT(id) DO UPDATE SET
                 kind = excluded.kind,
                 amount_minor = excluded.amount_minor,
                 amount_currency_code = excluded.amount_currency_code,
                 amount_currency_scale = excluded.amount_currency_scale,
                 destination_minor = excluded.destination_minor,
                 destination_currency_code = excluded.destination_currency_code,
                 destination_currency_scale = excluded.destination_currency_scale,
                 occurred_at = excluded.occurred_at,
                 account_id = excluded.account_id,
                 destination_account_id = excluded.destination_account_id,
                 category_id = excluded.category_id,
                 reconciliation_flag_id = excluded.reconciliation_flag_id,
                 counterparty_id = excluded.counterparty_id,
                 note = excluded.note,
                 updated_at = excluded.updated_at,
                 deleted_at = excluded.deleted_at",
            params![
                transaction.id().to_string(),
                transaction_kind_name(transaction.kind()),
                transaction.amount().minor_units(),
                amount_code,
                amount_scale,
                destination_parts.as_ref().map(|(minor, _, _)| *minor),
                destination_parts.as_ref().map(|(_, code, _)| code.clone()),
                destination_parts.as_ref().map(|(_, _, scale)| *scale),
                timestamp_milliseconds(transaction.occurred_at()),
                transaction.account_id().to_string(),
                transaction
                    .fields()
                    .destination_account_id
                    .map(|id| id.to_string()),
                transaction.fields().category_id.map(|id| id.to_string()),
                transaction
                    .fields()
                    .reconciliation_flag_id
                    .map(|id| id.to_string()),
                transaction
                    .fields()
                    .counterparty_id
                    .map(|id| id.to_string()),
                transaction.fields().note,
                timestamp_milliseconds(transaction.created_at()),
                timestamp_milliseconds(transaction.updated_at()),
                transaction.fields().deleted_at.map(timestamp_milliseconds),
            ],
        )
        .map_err(to_repository_error)?;
    connection
        .execute(
            "DELETE FROM transaction_tags WHERE transaction_id = ?1",
            [transaction.id().to_string()],
        )
        .map_err(to_repository_error)?;
    for tag_id in &transaction.fields().tag_ids {
        connection
            .execute(
                "INSERT INTO transaction_tags (transaction_id, tag_id) VALUES (?1, ?2)",
                params![transaction.id().to_string(), tag_id.to_string()],
            )
            .map_err(to_repository_error)?;
    }
    Ok(())
}

fn upsert_exchange_rate(
    connection: &Connection,
    rate: &ExchangeRate,
) -> Result<(), RepositoryError> {
    let (source_code, source_scale) = currency_parts(rate.source_currency());
    let (reporting_code, reporting_scale) = currency_parts(rate.reporting_currency());
    connection
        .execute(
            "INSERT INTO exchange_rates (id, source_currency_code, source_currency_scale,
                 reporting_currency_code, reporting_currency_scale, quote_numerator, quote_denominator,
                 effective_at, source, superseded_at, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
             ON CONFLICT(id) DO UPDATE SET
                 source_currency_code = excluded.source_currency_code,
                 source_currency_scale = excluded.source_currency_scale,
                 reporting_currency_code = excluded.reporting_currency_code,
                 reporting_currency_scale = excluded.reporting_currency_scale,
                 quote_numerator = excluded.quote_numerator,
                 quote_denominator = excluded.quote_denominator,
                 effective_at = excluded.effective_at,
                 source = excluded.source,
                 superseded_at = excluded.superseded_at,
                 updated_at = excluded.updated_at",
            params![
                rate.id().to_string(),
                source_code,
                source_scale,
                reporting_code,
                reporting_scale,
                rate.quote().numerator(),
                rate.quote().denominator(),
                timestamp_milliseconds(rate.effective_at()),
                exchange_rate_source_name(rate.source()),
                rate.superseded_at().map(timestamp_milliseconds),
                timestamp_milliseconds(rate.created_at()),
                timestamp_milliseconds(rate.updated_at()),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

impl LedgerSettingsRepository for SqliteLedger {
    fn settings(&self) -> Result<Option<LedgerSettings>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, currency_code, currency_scale, currency_policy_version,
                        calendar_identifier, time_zone_identifier, created_at, updated_at
                 FROM settings LIMIT 1",
            )
            .map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_settings(row)?)),
            None => Ok(None),
        }
    }

    fn save_settings(&self, settings: &LedgerSettings) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_settings(&connection, settings)
    }
}

impl LedgerProfileRepository for SqliteLedger {
    fn profile(&self) -> Result<Option<LedgerProfile>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, settings_id, name, emoji, created_at, updated_at FROM profile LIMIT 1",
            )
            .map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_profile(row)?)),
            None => Ok(None),
        }
    }

    fn save_profile(&self, profile: &LedgerProfile) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_profile(&connection, profile)
    }
}

impl AccountRepository for SqliteLedger {
    fn account(&self, id: AccountId) -> Result<Option<Account>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM accounts WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_account(&connection, row)?)),
            None => Ok(None),
        }
    }

    fn accounts(&self, include_archived: bool) -> Result<Vec<Account>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM accounts ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM accounts WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut accounts = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            accounts.push(map_account(&connection, row)?);
        }
        Ok(accounts)
    }

    fn save_account(&self, account: &Account) -> Result<(), RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        upsert_account(&transaction, account)?;
        transaction.commit().map_err(to_repository_error)
    }
}

impl AccountGroupRepository for SqliteLedger {
    fn account_group(&self, id: AccountGroupId) -> Result<Option<AccountGroup>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM account_groups WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_account_group(row)?)),
            None => Ok(None),
        }
    }

    fn account_groups(&self, include_archived: bool) -> Result<Vec<AccountGroup>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM account_groups ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM account_groups WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut groups = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            groups.push(map_account_group(row)?);
        }
        Ok(groups)
    }

    fn save_account_group(&self, group: &AccountGroup) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_account_group(&connection, group)
    }
}

impl CategoryRepository for SqliteLedger {
    fn category(&self, id: CategoryId) -> Result<Option<Category>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM categories WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_category(row)?)),
            None => Ok(None),
        }
    }

    fn categories(&self, include_archived: bool) -> Result<Vec<Category>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM categories ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM categories WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut categories = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            categories.push(map_category(row)?);
        }
        Ok(categories)
    }

    fn save_category(&self, category: &Category) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_category(&connection, category)
    }
}

impl CategoryGroupRepository for SqliteLedger {
    fn category_group(
        &self,
        id: CategoryGroupId,
    ) -> Result<Option<CategoryGroup>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM category_groups WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_category_group(row)?)),
            None => Ok(None),
        }
    }

    fn category_groups(
        &self,
        include_archived: bool,
    ) -> Result<Vec<CategoryGroup>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM category_groups ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM category_groups WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut groups = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            groups.push(map_category_group(row)?);
        }
        Ok(groups)
    }

    fn save_category_group(&self, group: &CategoryGroup) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_category_group(&connection, group)
    }
}

impl TagRepository for SqliteLedger {
    fn tag(&self, id: TagId) -> Result<Option<LedgerTag>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM tags WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_tag(row)?)),
            None => Ok(None),
        }
    }

    fn tags(&self, include_archived: bool) -> Result<Vec<LedgerTag>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM tags ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM tags WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut tags = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            tags.push(map_tag(row)?);
        }
        Ok(tags)
    }

    fn save_tag(&self, tag: &LedgerTag) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_tag(&connection, tag)
    }
}

impl CounterpartyRepository for SqliteLedger {
    fn counterparty(&self, id: CounterpartyId) -> Result<Option<Counterparty>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM counterparties WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_counterparty(row)?)),
            None => Ok(None),
        }
    }

    fn counterparties(&self, include_archived: bool) -> Result<Vec<Counterparty>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM counterparties ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM counterparties WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut counterparties = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            counterparties.push(map_counterparty(row)?);
        }
        Ok(counterparties)
    }

    fn save_counterparty(&self, counterparty: &Counterparty) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_counterparty(&connection, counterparty)
    }
}

impl ReconciliationFlagRepository for SqliteLedger {
    fn reconciliation_flag(
        &self,
        id: ReconciliationFlagId,
    ) -> Result<Option<ReconciliationFlag>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM reconciliation_flags WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_reconciliation_flag(row)?)),
            None => Ok(None),
        }
    }

    fn reconciliation_flags(
        &self,
        include_archived: bool,
    ) -> Result<Vec<ReconciliationFlag>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM reconciliation_flags ORDER BY sort_order ASC, name ASC, id ASC"
        } else {
            "SELECT * FROM reconciliation_flags WHERE is_archived = 0 ORDER BY sort_order ASC, name ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut flags = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            flags.push(map_reconciliation_flag(row)?);
        }
        Ok(flags)
    }

    fn save_reconciliation_flag(&self, flag: &ReconciliationFlag) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_reconciliation_flag(&connection, flag)
    }
}

impl TransactionNotePresetRepository for SqliteLedger {
    fn note_preset(
        &self,
        id: TransactionNotePresetId,
    ) -> Result<Option<TransactionNotePreset>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM note_presets WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_note_preset(row)?)),
            None => Ok(None),
        }
    }

    fn note_presets(
        &self,
        include_archived: bool,
    ) -> Result<Vec<TransactionNotePreset>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_archived {
            "SELECT * FROM note_presets ORDER BY sort_order ASC, text ASC, id ASC"
        } else {
            "SELECT * FROM note_presets WHERE is_archived = 0 ORDER BY sort_order ASC, text ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut presets = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            presets.push(map_note_preset(row)?);
        }
        Ok(presets)
    }

    fn save_note_preset(&self, preset: &TransactionNotePreset) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_note_preset(&connection, preset)
    }
}

impl TransactionRepository for SqliteLedger {
    fn transaction(&self, id: TransactionId) -> Result<Option<Transaction>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare("SELECT * FROM transactions WHERE id = ?1")
            .map_err(to_repository_error)?;
        let mut rows = statement
            .query([id.to_string()])
            .map_err(to_repository_error)?;
        match rows.next().map_err(to_repository_error)? {
            Some(row) => Ok(Some(map_transaction(&connection, row)?)),
            None => Ok(None),
        }
    }

    fn transactions(&self, include_deleted: bool) -> Result<Vec<Transaction>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_deleted {
            "SELECT * FROM transactions ORDER BY occurred_at ASC, id ASC"
        } else {
            "SELECT * FROM transactions WHERE deleted_at IS NULL ORDER BY occurred_at ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut transactions = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            transactions.push(map_transaction(&connection, row)?);
        }
        Ok(transactions)
    }

    fn save_transaction(&self, transaction: &Transaction) -> Result<(), RepositoryError> {
        let mut connection = self.database.connection()?;
        let tx = connection.transaction().map_err(to_repository_error)?;
        upsert_transaction(&tx, transaction)?;
        tx.commit().map_err(to_repository_error)
    }
}

impl ExchangeRateRepository for SqliteLedger {
    fn exchange_rates(
        &self,
        include_superseded: bool,
    ) -> Result<Vec<ExchangeRate>, RepositoryError> {
        let connection = self.database.connection()?;
        let sql = if include_superseded {
            "SELECT * FROM exchange_rates ORDER BY effective_at ASC, id ASC"
        } else {
            "SELECT * FROM exchange_rates WHERE superseded_at IS NULL ORDER BY effective_at ASC, id ASC"
        };
        let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut rates = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            rates.push(map_exchange_rate(row)?);
        }
        Ok(rates)
    }

    fn save_exchange_rate(&self, rate: &ExchangeRate) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        upsert_exchange_rate(&connection, rate)
    }
}

impl LedgerBootstrapRepository for SqliteLedger {
    fn initialize(
        &self,
        settings: &LedgerSettings,
        profile: &LedgerProfile,
        default_account: &Account,
        default_categories: &[Category],
        opening_balance: Option<&Transaction>,
    ) -> Result<bool, RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        let count: i64 = transaction
            .query_row("SELECT COUNT(*) FROM settings", [], |row| row.get(0))
            .map_err(to_repository_error)?;
        if count > 0 {
            return Ok(false);
        }
        upsert_settings(&transaction, settings)?;
        upsert_profile(&transaction, profile)?;
        upsert_account(&transaction, default_account)?;
        for category in default_categories {
            upsert_category(&transaction, category)?;
        }
        if let Some(opening_balance) = opening_balance {
            upsert_transaction(&transaction, opening_balance)?;
        }
        transaction.commit().map_err(to_repository_error)?;
        Ok(true)
    }
}

impl AccountSetupRepository for SqliteLedger {
    fn create_account_with_opening_balance(
        &self,
        account: &Account,
        opening_balance: Option<&Transaction>,
    ) -> Result<(), RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        upsert_account(&transaction, account)?;
        if let Some(opening_balance) = opening_balance {
            upsert_transaction(&transaction, opening_balance)?;
        }
        transaction.commit().map_err(to_repository_error)
    }
}

impl LedgerExportRepository for SqliteLedger {
    fn export_snapshot(&self) -> Result<LedgerExportSnapshot, RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        let settings = collect_rows(&transaction, "SELECT * FROM settings LIMIT 1", map_settings)?
            .into_iter()
            .next()
            .ok_or_else(|| RepositoryError::new("ledger is not initialized"))?;
        let profile = collect_rows(&transaction, "SELECT * FROM profile LIMIT 1", map_profile)?
            .into_iter()
            .next()
            .ok_or_else(|| RepositoryError::new("ledger profile is not initialized"))?;
        let snapshot = LedgerExportSnapshot {
            settings,
            profile,
            account_groups: collect_rows(
                &transaction,
                "SELECT * FROM account_groups ORDER BY sort_order ASC, id ASC",
                map_account_group,
            )?,
            accounts: collect_rows(
                &transaction,
                "SELECT * FROM accounts ORDER BY sort_order ASC, id ASC",
                |row| map_account(&transaction, row),
            )?,
            category_groups: collect_rows(
                &transaction,
                "SELECT * FROM category_groups ORDER BY sort_order ASC, id ASC",
                map_category_group,
            )?,
            categories: collect_rows(
                &transaction,
                "SELECT * FROM categories ORDER BY sort_order ASC, id ASC",
                map_category,
            )?,
            tags: collect_rows(
                &transaction,
                "SELECT * FROM tags ORDER BY sort_order ASC, id ASC",
                map_tag,
            )?,
            counterparties: collect_rows(
                &transaction,
                "SELECT * FROM counterparties ORDER BY sort_order ASC, id ASC",
                map_counterparty,
            )?,
            reconciliation_flags: collect_rows(
                &transaction,
                "SELECT * FROM reconciliation_flags ORDER BY sort_order ASC, id ASC",
                map_reconciliation_flag,
            )?,
            note_presets: collect_rows(
                &transaction,
                "SELECT * FROM note_presets ORDER BY sort_order ASC, id ASC",
                map_note_preset,
            )?,
            transactions: collect_rows(
                &transaction,
                "SELECT * FROM transactions ORDER BY occurred_at ASC, id ASC",
                |row| map_transaction(&transaction, row),
            )?,
            exchange_rates: collect_rows(
                &transaction,
                "SELECT * FROM exchange_rates ORDER BY effective_at ASC, id ASC",
                map_exchange_rate,
            )?,
        };
        transaction.commit().map_err(to_repository_error)?;
        Ok(snapshot)
    }
}

impl LedgerReadSnapshotRepository for SqliteLedger {
    fn snapshot(&self) -> Result<LedgerReadSnapshot, RepositoryError> {
        let settings = self
            .settings()?
            .ok_or_else(|| RepositoryError::new("ledger is not initialized"))?;
        let active_accounts = self.accounts(false)?;
        let active_transactions = self.transactions(false)?;
        let exchange_rates = self.exchange_rates(false)?;
        Ok(LedgerReadSnapshot {
            settings,
            active_accounts,
            active_transactions,
            exchange_rates,
        })
    }
}

impl SyncRepository for SqliteLedger {
    fn cursor(&self) -> Result<u64, RepositoryError> {
        let connection = self.database.connection()?;
        let value: i64 = connection
            .query_row(
                "SELECT COALESCE((SELECT applied_seq FROM sync_cursor WHERE singleton = 1), 0)",
                [],
                |row| row.get(0),
            )
            .map_err(to_repository_error)?;
        u64_from_i64(value, "applied_seq")
    }

    fn save_cursor(&self, seq: u64) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        connection
            .execute(
                "INSERT INTO sync_cursor (singleton, applied_seq) VALUES (1, ?1)
                 ON CONFLICT(singleton) DO UPDATE SET
                     applied_seq = MAX(sync_cursor.applied_seq, excluded.applied_seq)",
                params![i64_from_u64(seq, "applied_seq")?],
            )
            .map_err(to_repository_error)?;
        Ok(())
    }

    fn pending_operations(&self, limit: usize) -> Result<Vec<SyncOperation>, RepositoryError> {
        let connection = self.database.connection()?;
        let limit =
            i64::try_from(limit).map_err(|_| RepositoryError::new("outbox limit exceeds i64"))?;
        let mut statement = connection
            .prepare(
                "SELECT op_id, ledger_id, device_id, entity_type, entity_id, op_type,
                        base_revision, client_timestamp, idempotency_key, key_version, nonce,
                        ciphertext, payload_version, signature
                 FROM outbox_operations
                 WHERE server_seq IS NULL
                 ORDER BY created_at ASC, op_id ASC
                 LIMIT ?1",
            )
            .map_err(to_repository_error)?;
        let mut rows = statement.query([limit]).map_err(to_repository_error)?;
        let mut operations = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            operations.push(sync_operation_from_row(row)?);
        }
        Ok(operations)
    }

    fn mark_operations_sent(&self, accepted: &[(Uuid, u64)]) -> Result<(), RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        for (op_id, seq) in accepted {
            let updated = transaction
                .execute(
                    "UPDATE outbox_operations SET server_seq = ?2
                     WHERE op_id = ?1 AND (server_seq IS NULL OR server_seq = ?2)",
                    params![op_id.to_string(), i64_from_u64(*seq, "server_seq")?],
                )
                .map_err(to_repository_error)?;
            if updated != 1 {
                return Err(RepositoryError::new(
                    "outbox operation is missing or has a conflicting sequence",
                ));
            }
            let (entity_type, entity_id): (String, String) = transaction
                .query_row(
                    "SELECT entity_type, entity_id FROM outbox_operations WHERE op_id = ?1",
                    [op_id.to_string()],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )
                .map_err(to_repository_error)?;
            let entity_type = SyncEntityType::parse(&entity_type)
                .ok_or_else(|| RepositoryError::new("unknown sync entity type"))?;
            let entity_id = parse_sync_uuid(entity_id, "entity_id")?;
            set_sync_server_seq(&transaction, entity_type, entity_id, *seq)?;
        }
        transaction.commit().map_err(to_repository_error)
    }

    fn is_operation_known(&self, op_id: Uuid) -> Result<bool, RepositoryError> {
        let connection = self.database.connection()?;
        let known: i64 = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM outbox_operations WHERE op_id = ?1)
                     OR EXISTS(SELECT 1 FROM sync_applied_operations WHERE op_id = ?1)",
                [op_id.to_string()],
                |row| row.get(0),
            )
            .map_err(to_repository_error)?;
        Ok(known != 0)
    }

    fn entity_revision(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError> {
        let connection = self.database.connection()?;
        let value: i64 = connection
            .query_row(
                "SELECT COALESCE((SELECT revision FROM sync_entity_state
                                  WHERE entity_type = ?1 AND entity_id = ?2), 0)",
                params![entity_type.as_str(), entity_id.to_string()],
                |row| row.get(0),
            )
            .map_err(to_repository_error)?;
        u64_from_i64(value, "revision")
    }

    fn entity_server_seq(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<u64, RepositoryError> {
        let connection = self.database.connection()?;
        let value: i64 = connection
            .query_row(
                "SELECT COALESCE((SELECT last_server_seq FROM sync_entity_state
                                  WHERE entity_type = ?1 AND entity_id = ?2), 0)",
                params![entity_type.as_str(), entity_id.to_string()],
                |row| row.get(0),
            )
            .map_err(to_repository_error)?;
        u64_from_i64(value, "last_server_seq")
    }

    fn load_entity(
        &self,
        entity_type: SyncEntityType,
        entity_id: Uuid,
    ) -> Result<Option<SyncEntity>, RepositoryError> {
        let connection = self.database.connection()?;
        let id = entity_id.to_string();
        match entity_type {
            SyncEntityType::Account => load_single(
                &connection,
                "SELECT * FROM accounts WHERE id = ?1",
                &id,
                |row| map_account(&connection, row).map(SyncEntity::Account),
            ),
            SyncEntityType::AccountGroup => load_single(
                &connection,
                "SELECT * FROM account_groups WHERE id = ?1",
                &id,
                |row| map_account_group(row).map(SyncEntity::AccountGroup),
            ),
            SyncEntityType::Category => load_single(
                &connection,
                "SELECT * FROM categories WHERE id = ?1",
                &id,
                |row| map_category(row).map(SyncEntity::Category),
            ),
            SyncEntityType::CategoryGroup => load_single(
                &connection,
                "SELECT * FROM category_groups WHERE id = ?1",
                &id,
                |row| map_category_group(row).map(SyncEntity::CategoryGroup),
            ),
            SyncEntityType::Counterparty => load_single(
                &connection,
                "SELECT * FROM counterparties WHERE id = ?1",
                &id,
                |row| map_counterparty(row).map(SyncEntity::Counterparty),
            ),
            SyncEntityType::ExchangeRate => load_single(
                &connection,
                "SELECT * FROM exchange_rates WHERE id = ?1",
                &id,
                |row| map_exchange_rate(row).map(SyncEntity::ExchangeRate),
            ),
            SyncEntityType::LedgerProfile => load_single(
                &connection,
                "SELECT * FROM profile WHERE id = ?1",
                &id,
                |row| map_profile(row).map(SyncEntity::LedgerProfile),
            ),
            SyncEntityType::LedgerSettings => load_single(
                &connection,
                "SELECT * FROM settings WHERE id = ?1",
                &id,
                |row| map_settings(row).map(SyncEntity::LedgerSettings),
            ),
            SyncEntityType::NotePreset => load_single(
                &connection,
                "SELECT * FROM note_presets WHERE id = ?1",
                &id,
                |row| map_note_preset(row).map(SyncEntity::NotePreset),
            ),
            SyncEntityType::ReconciliationFlag => load_single(
                &connection,
                "SELECT * FROM reconciliation_flags WHERE id = ?1",
                &id,
                |row| map_reconciliation_flag(row).map(SyncEntity::ReconciliationFlag),
            ),
            SyncEntityType::Tag => load_single(
                &connection,
                "SELECT * FROM tags WHERE id = ?1",
                &id,
                |row| map_tag(row).map(SyncEntity::Tag),
            ),
            SyncEntityType::Transaction => load_single(
                &connection,
                "SELECT * FROM transactions WHERE id = ?1",
                &id,
                |row| map_transaction(&connection, row).map(SyncEntity::Transaction),
            ),
        }
    }

    fn record_local_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
    ) -> Result<(), RepositoryError> {
        require_operation_matches_entity(operation, entity)?;
        if operation.server_seq.is_some() {
            return Err(RepositoryError::new(
                "a local change cannot already have a server sequence",
            ));
        }
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        upsert_sync_entity(&transaction, entity)?;
        insert_outbox_operation(&transaction, operation)?;
        set_sync_revision(
            &transaction,
            operation.entity_type,
            operation.entity_id,
            revision,
        )?;
        transaction.commit().map_err(to_repository_error)
    }

    fn apply_remote_change(
        &self,
        entity: &SyncEntity,
        operation: &SyncOperation,
        revision: u64,
        conflict: Option<&SyncConflict>,
    ) -> Result<(), RepositoryError> {
        require_operation_matches_entity(operation, entity)?;
        let Some(seq) = operation.server_seq else {
            return Err(RepositoryError::new(
                "a remote operation must have a server sequence",
            ));
        };
        if let Some(conflict) = conflict
            && (conflict.entity_type != operation.entity_type
                || conflict.entity_id != operation.entity_id
                || conflict.remote_op_id != operation.op_id)
        {
            return Err(RepositoryError::new(
                "conflict does not match the remote operation",
            ));
        }
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        upsert_sync_entity(&transaction, entity)?;
        set_sync_revision(
            &transaction,
            operation.entity_type,
            operation.entity_id,
            revision,
        )?;
        set_sync_server_seq(
            &transaction,
            operation.entity_type,
            operation.entity_id,
            seq,
        )?;
        transaction
            .execute(
                "INSERT INTO sync_applied_operations (op_id, server_seq) VALUES (?1, ?2)",
                params![
                    operation.op_id.to_string(),
                    i64_from_u64(seq, "server_seq")?
                ],
            )
            .map_err(to_repository_error)?;
        if let Some(conflict) = conflict {
            insert_sync_conflict(&transaction, conflict)?;
        }
        transaction.commit().map_err(to_repository_error)
    }

    fn restore_entity(
        &self,
        entity: &SyncEntity,
        revision: u64,
        server_seq: u64,
    ) -> Result<(), RepositoryError> {
        let mut connection = self.database.connection()?;
        let transaction = connection.transaction().map_err(to_repository_error)?;
        upsert_sync_entity(&transaction, entity)?;
        set_sync_revision(
            &transaction,
            entity.entity_type(),
            entity.entity_id(),
            revision,
        )?;
        set_sync_server_seq(
            &transaction,
            entity.entity_type(),
            entity.entity_id(),
            server_seq,
        )?;
        transaction.commit().map_err(to_repository_error)
    }

    fn entity_revisions(&self) -> Result<Vec<(SyncEntityType, Uuid, u64)>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT entity_type, entity_id, revision FROM sync_entity_state
                 ORDER BY entity_type ASC, entity_id ASC",
            )
            .map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut revisions = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            let entity_type = SyncEntityType::parse(&row.get_value::<_, String>(0)?)
                .ok_or_else(|| RepositoryError::new("unknown sync entity type"))?;
            revisions.push((
                entity_type,
                parse_sync_uuid(row.get_value::<_, String>(1)?, "entity_id")?,
                u64_from_i64(row.get_value(2)?, "revision")?,
            ));
        }
        Ok(revisions)
    }

    fn conflicts(&self) -> Result<Vec<SyncConflict>, RepositoryError> {
        let connection = self.database.connection()?;
        let mut statement = connection
            .prepare(
                "SELECT id, entity_type, entity_id, local_payload, remote_op_id,
                        local_revision, remote_base_revision, detected_at, resolution, resolved_at
                 FROM sync_conflicts
                 ORDER BY detected_at ASC, id ASC",
            )
            .map_err(to_repository_error)?;
        let mut rows = statement.query([]).map_err(to_repository_error)?;
        let mut conflicts = Vec::new();
        while let Some(row) = rows.next().map_err(to_repository_error)? {
            conflicts.push(sync_conflict_from_row(row)?);
        }
        Ok(conflicts)
    }

    fn resolve_conflict(
        &self,
        conflict_id: Uuid,
        resolution: SyncConflictResolution,
        at: Timestamp,
    ) -> Result<(), RepositoryError> {
        let connection = self.database.connection()?;
        let updated = connection
            .execute(
                "UPDATE sync_conflicts SET resolution = ?2, resolved_at = ?3
                 WHERE id = ?1 AND resolution IS NULL",
                params![
                    conflict_id.to_string(),
                    resolution.as_str(),
                    timestamp_milliseconds(at)
                ],
            )
            .map_err(to_repository_error)?;
        if updated == 1 {
            Ok(())
        } else {
            Err(RepositoryError::new(
                "conflict not found or already resolved",
            ))
        }
    }
}

fn load_single<T>(
    connection: &Connection,
    sql: &str,
    id: &str,
    map: impl Fn(&Row<'_>) -> Result<T, RepositoryError>,
) -> Result<Option<T>, RepositoryError> {
    let mut statement = connection.prepare(sql).map_err(to_repository_error)?;
    let mut rows = statement.query([id]).map_err(to_repository_error)?;
    match rows.next().map_err(to_repository_error)? {
        Some(row) => Ok(Some(map(row)?)),
        None => Ok(None),
    }
}

fn upsert_sync_entity(connection: &Connection, entity: &SyncEntity) -> Result<(), RepositoryError> {
    match entity {
        SyncEntity::Account(entity) => upsert_account(connection, entity),
        SyncEntity::AccountGroup(entity) => upsert_account_group(connection, entity),
        SyncEntity::Category(entity) => upsert_category(connection, entity),
        SyncEntity::CategoryGroup(entity) => upsert_category_group(connection, entity),
        SyncEntity::Counterparty(entity) => upsert_counterparty(connection, entity),
        SyncEntity::ExchangeRate(entity) => upsert_exchange_rate(connection, entity),
        SyncEntity::LedgerProfile(entity) => upsert_profile(connection, entity),
        SyncEntity::LedgerSettings(entity) => upsert_settings(connection, entity),
        SyncEntity::NotePreset(entity) => upsert_note_preset(connection, entity),
        SyncEntity::ReconciliationFlag(entity) => upsert_reconciliation_flag(connection, entity),
        SyncEntity::Tag(entity) => upsert_tag(connection, entity),
        SyncEntity::Transaction(entity) => upsert_transaction(connection, entity),
    }
}

fn require_operation_matches_entity(
    operation: &SyncOperation,
    entity: &SyncEntity,
) -> Result<(), RepositoryError> {
    if operation.entity_type != entity.entity_type() || operation.entity_id != entity.entity_id() {
        return Err(RepositoryError::new(
            "operation does not match its entity aggregate",
        ));
    }
    Ok(())
}

fn insert_outbox_operation(
    connection: &Connection,
    operation: &SyncOperation,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO outbox_operations (
                 op_id, ledger_id, device_id, entity_type, entity_id, op_type, base_revision,
                 client_timestamp, idempotency_key, key_version, nonce, ciphertext,
                 payload_version, signature, server_seq, created_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                operation.op_id.to_string(),
                operation.ledger_id.to_string(),
                operation.device_id.to_string(),
                operation.entity_type.as_str(),
                operation.entity_id.to_string(),
                operation.op_type.as_str(),
                i64_from_u64(operation.base_revision, "base_revision")?,
                operation.client_timestamp,
                operation.idempotency_key.to_string(),
                i64::from(operation.key_version),
                operation.nonce,
                operation.ciphertext,
                i64::from(operation.payload_version),
                operation.signature,
                operation
                    .server_seq
                    .map(|seq| i64_from_u64(seq, "server_seq"))
                    .transpose()?,
                operation.client_timestamp,
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn set_sync_revision(
    connection: &Connection,
    entity_type: SyncEntityType,
    entity_id: Uuid,
    revision: u64,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO sync_entity_state (entity_type, entity_id, revision)
             VALUES (?1, ?2, ?3)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET revision = excluded.revision",
            params![
                entity_type.as_str(),
                entity_id.to_string(),
                i64_from_u64(revision, "revision")?
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn set_sync_server_seq(
    connection: &Connection,
    entity_type: SyncEntityType,
    entity_id: Uuid,
    server_seq: u64,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO sync_entity_state (entity_type, entity_id, revision, last_server_seq)
             VALUES (?1, ?2, 0, ?3)
             ON CONFLICT(entity_type, entity_id) DO UPDATE SET
                 last_server_seq = MAX(sync_entity_state.last_server_seq, excluded.last_server_seq)",
            params![
                entity_type.as_str(),
                entity_id.to_string(),
                i64_from_u64(server_seq, "last_server_seq")?
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn insert_sync_conflict(
    connection: &Connection,
    conflict: &SyncConflict,
) -> Result<(), RepositoryError> {
    connection
        .execute(
            "INSERT INTO sync_conflicts (
                 id, entity_type, entity_id, local_payload, remote_op_id, local_revision,
                 remote_base_revision, detected_at, resolution, resolved_at
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                conflict.id.to_string(),
                conflict.entity_type.as_str(),
                conflict.entity_id.to_string(),
                conflict.local_payload,
                conflict.remote_op_id.to_string(),
                i64_from_u64(conflict.local_revision, "local_revision")?,
                i64_from_u64(conflict.remote_base_revision, "remote_base_revision")?,
                timestamp_milliseconds(conflict.detected_at),
                conflict.resolution.map(|resolution| resolution.as_str()),
                conflict.resolved_at.map(timestamp_milliseconds),
            ],
        )
        .map_err(to_repository_error)?;
    Ok(())
}

fn sync_operation_from_row(row: &Row<'_>) -> Result<SyncOperation, RepositoryError> {
    Ok(SyncOperation {
        op_id: parse_sync_uuid(row.get_value::<_, String>(0)?, "op_id")?,
        ledger_id: parse_sync_uuid(row.get_value::<_, String>(1)?, "ledger_id")?,
        device_id: parse_sync_uuid(row.get_value::<_, String>(2)?, "device_id")?,
        entity_type: SyncEntityType::parse(&row.get_value::<_, String>(3)?)
            .ok_or_else(|| RepositoryError::new("unknown sync entity type"))?,
        entity_id: parse_sync_uuid(row.get_value::<_, String>(4)?, "entity_id")?,
        op_type: SyncOperationType::parse(&row.get_value::<_, String>(5)?)
            .ok_or_else(|| RepositoryError::new("unknown sync operation type"))?,
        base_revision: u64_from_i64(row.get_value(6)?, "base_revision")?,
        client_timestamp: row.get_value(7)?,
        idempotency_key: parse_sync_uuid(row.get_value::<_, String>(8)?, "idempotency_key")?,
        key_version: u32::try_from(row.get_value::<_, i64>(9)?)
            .map_err(|_| RepositoryError::new("invalid key_version"))?,
        nonce: row.get_value(10)?,
        ciphertext: row.get_value(11)?,
        payload_version: u32::try_from(row.get_value::<_, i64>(12)?)
            .map_err(|_| RepositoryError::new("invalid payload_version"))?,
        signature: row.get_value(13)?,
        server_seq: None,
    })
}

fn sync_conflict_from_row(row: &Row<'_>) -> Result<SyncConflict, RepositoryError> {
    let resolution = row
        .get_value::<_, Option<String>>(8)?
        .map(|value| {
            SyncConflictResolution::parse(&value)
                .ok_or_else(|| RepositoryError::new("unknown conflict resolution"))
        })
        .transpose()?;
    Ok(SyncConflict {
        id: parse_sync_uuid(row.get_value::<_, String>(0)?, "conflict id")?,
        entity_type: SyncEntityType::parse(&row.get_value::<_, String>(1)?)
            .ok_or_else(|| RepositoryError::new("unknown sync entity type"))?,
        entity_id: parse_sync_uuid(row.get_value::<_, String>(2)?, "entity_id")?,
        local_payload: row.get_value(3)?,
        remote_op_id: parse_sync_uuid(row.get_value::<_, String>(4)?, "remote_op_id")?,
        local_revision: u64_from_i64(row.get_value(5)?, "local_revision")?,
        remote_base_revision: u64_from_i64(row.get_value(6)?, "remote_base_revision")?,
        detected_at: timestamp_from_milliseconds(row.get_value(7)?)?,
        resolution,
        resolved_at: row
            .get_value::<_, Option<i64>>(9)?
            .map(timestamp_from_milliseconds)
            .transpose()?,
    })
}

fn parse_sync_uuid(value: String, field: &str) -> Result<Uuid, RepositoryError> {
    Uuid::parse_str(&value)
        .map_err(|error| RepositoryError::new(format!("invalid {field}: {error}")))
}

fn u64_from_i64(value: i64, field: &str) -> Result<u64, RepositoryError> {
    u64::try_from(value).map_err(|_| RepositoryError::new(format!("invalid {field}: {value}")))
}

fn i64_from_u64(value: u64, field: &str) -> Result<i64, RepositoryError> {
    i64::try_from(value).map_err(|_| RepositoryError::new(format!("{field} exceeds i64")))
}
