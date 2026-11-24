-- Bluewhale local schema v1.
-- Timestamps are Unix milliseconds in UTC. Money is integer minor units plus
-- the currency code and scale. Identifiers are lowercase hyphenated UUIDs.

CREATE TABLE settings (
    id TEXT PRIMARY KEY,
    currency_code TEXT NOT NULL,
    currency_scale INTEGER NOT NULL CHECK (currency_scale BETWEEN 0 AND 9),
    currency_policy_version INTEGER NOT NULL,
    calendar_identifier TEXT NOT NULL,
    time_zone_identifier TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE profile (
    id TEXT PRIMARY KEY,
    settings_id TEXT NOT NULL REFERENCES settings (id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    emoji TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE account_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE tags (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    color_hex TEXT NOT NULL,
    description TEXT,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE counterparties (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE accounts (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    asset_type TEXT NOT NULL,
    currency_code TEXT NOT NULL,
    currency_scale INTEGER NOT NULL CHECK (currency_scale BETWEEN 0 AND 9),
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    owner_id TEXT REFERENCES counterparties (id),
    custom_group_id TEXT REFERENCES account_groups (id),
    note TEXT NOT NULL,
    is_hidden INTEGER NOT NULL CHECK (is_hidden IN (0, 1)),
    is_included_in_net_worth INTEGER NOT NULL CHECK (is_included_in_net_worth IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE account_tags (
    account_id TEXT NOT NULL REFERENCES accounts (id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (account_id, tag_id)
);

CREATE TABLE category_groups (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE categories (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    kind TEXT NOT NULL,
    group_id TEXT REFERENCES category_groups (id),
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE reconciliation_flags (
    id TEXT PRIMARY KEY,
    system_kind TEXT,
    name TEXT NOT NULL,
    color_hex TEXT NOT NULL,
    locks_transactions INTEGER NOT NULL CHECK (locks_transactions IN (0, 1)),
    shortcut_key TEXT,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE note_presets (
    id TEXT PRIMARY KEY,
    text TEXT NOT NULL,
    sort_order INTEGER NOT NULL,
    is_archived INTEGER NOT NULL CHECK (is_archived IN (0, 1)),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE TABLE transactions (
    id TEXT PRIMARY KEY,
    kind TEXT NOT NULL,
    amount_minor INTEGER NOT NULL,
    amount_currency_code TEXT NOT NULL,
    amount_currency_scale INTEGER NOT NULL CHECK (amount_currency_scale BETWEEN 0 AND 9),
    destination_minor INTEGER,
    destination_currency_code TEXT,
    destination_currency_scale INTEGER CHECK (destination_currency_scale BETWEEN 0 AND 9),
    occurred_at INTEGER NOT NULL,
    account_id TEXT NOT NULL REFERENCES accounts (id),
    destination_account_id TEXT REFERENCES accounts (id),
    category_id TEXT REFERENCES categories (id),
    reconciliation_flag_id TEXT REFERENCES reconciliation_flags (id),
    counterparty_id TEXT REFERENCES counterparties (id),
    note TEXT,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    deleted_at INTEGER,
    CHECK (
        (destination_minor IS NULL AND destination_currency_code IS NULL AND destination_currency_scale IS NULL)
        OR (destination_minor IS NOT NULL AND destination_currency_code IS NOT NULL AND destination_currency_scale IS NOT NULL)
    )
);

CREATE TABLE transaction_tags (
    transaction_id TEXT NOT NULL REFERENCES transactions (id) ON DELETE CASCADE,
    tag_id TEXT NOT NULL REFERENCES tags (id) ON DELETE CASCADE,
    PRIMARY KEY (transaction_id, tag_id)
);

CREATE TABLE exchange_rates (
    id TEXT PRIMARY KEY,
    source_currency_code TEXT NOT NULL,
    source_currency_scale INTEGER NOT NULL CHECK (source_currency_scale BETWEEN 0 AND 9),
    reporting_currency_code TEXT NOT NULL,
    reporting_currency_scale INTEGER NOT NULL CHECK (reporting_currency_scale BETWEEN 0 AND 9),
    quote_numerator INTEGER NOT NULL CHECK (quote_numerator > 0),
    quote_denominator INTEGER NOT NULL CHECK (quote_denominator > 0),
    effective_at INTEGER NOT NULL,
    source TEXT NOT NULL,
    superseded_at INTEGER,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX accounts_archived_idx ON accounts (is_archived);
CREATE INDEX accounts_group_idx ON accounts (custom_group_id);
CREATE INDEX account_tags_tag_idx ON account_tags (tag_id);
CREATE INDEX categories_group_idx ON categories (group_id);
CREATE INDEX categories_kind_idx ON categories (kind);
CREATE INDEX transactions_occurred_idx ON transactions (occurred_at);
CREATE INDEX transactions_account_idx ON transactions (account_id);
CREATE INDEX transactions_destination_idx ON transactions (destination_account_id);
CREATE INDEX transactions_deleted_idx ON transactions (deleted_at);
CREATE INDEX transaction_tags_tag_idx ON transaction_tags (tag_id);
CREATE INDEX exchange_rates_pair_idx
    ON exchange_rates (source_currency_code, reporting_currency_code, effective_at);
