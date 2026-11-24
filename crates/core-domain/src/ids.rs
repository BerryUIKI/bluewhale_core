//! Typed identifiers for domain entities.
//!
//! Identifiers wrap a UUID. The clients generate UUIDv7 values; the domain
//! only validates and carries them, so this crate does not depend on an
//! entropy source.

use std::fmt;
use std::str::FromStr;

use uuid::Uuid;

macro_rules! define_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(Uuid);

        impl $name {
            /// Wraps an existing UUID.
            #[must_use]
            pub const fn from_uuid(value: Uuid) -> Self {
                Self(value)
            }

            /// Returns the wrapped UUID.
            #[must_use]
            pub const fn as_uuid(&self) -> Uuid {
                self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(formatter, "{}", self.0)
            }
        }

        impl FromStr for $name {
            type Err = uuid::Error;

            fn from_str(value: &str) -> Result<Self, Self::Err> {
                Uuid::parse_str(value).map(Self)
            }
        }

        impl From<Uuid> for $name {
            fn from(value: Uuid) -> Self {
                Self(value)
            }
        }

        impl From<$name> for Uuid {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

define_id!(
    LedgerId,
    "Identifies a ledger in the device-local registry."
);
define_id!(LedgerSettingsId, "Identifies a ledger's settings record.");
define_id!(LedgerProfileId, "Identifies a ledger's profile record.");
define_id!(AccountId, "Identifies an asset account.");
define_id!(AccountGroupId, "Identifies a custom account group.");
define_id!(CategoryId, "Identifies a leaf category.");
define_id!(CategoryGroupId, "Identifies a category group.");
define_id!(TagId, "Identifies a tag.");
define_id!(CounterpartyId, "Identifies a counterparty.");
define_id!(TransactionId, "Identifies a transaction or transfer event.");
define_id!(ExchangeRateId, "Identifies an exchange-rate revision.");
define_id!(ReconciliationFlagId, "Identifies a reconciliation flag.");
define_id!(
    TransactionNotePresetId,
    "Identifies a transaction note preset."
);

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "018f4f5c-7d7a-7cc1-8f8a-3c2b1a0d9e8f";

    #[test]
    fn identifiers_parse_and_display_round_trip() {
        let account: AccountId = SAMPLE.parse().unwrap();
        assert_eq!(account.to_string(), SAMPLE);
        assert_eq!(
            account,
            AccountId::from_uuid(Uuid::parse_str(SAMPLE).unwrap())
        );
        assert_eq!(Uuid::from(account).to_string(), SAMPLE);
    }

    #[test]
    fn identifiers_reject_invalid_values() {
        assert!("not-a-uuid".parse::<TransactionId>().is_err());
    }

    #[test]
    fn identifiers_are_ordered_by_uuid() {
        let lower = TagId::from_uuid(Uuid::from_u128(1));
        let higher = TagId::from_uuid(Uuid::from_u128(2));
        assert!(lower < higher);
    }
}
