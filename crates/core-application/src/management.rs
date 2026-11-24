//! Management use cases for categories, tags, counterparties, groups, and
//! accounts.

use core_domain::{
    Account, AccountGroup, AccountGroupId, AccountGroupRepository, AccountId, AccountOptions,
    AccountRepository, AssetAccountType, Category, CategoryGroup, CategoryGroupId,
    CategoryGroupRepository, CategoryId, CategoryKind, CategoryOptions, CategoryRepository,
    Counterparty, CounterpartyId, CounterpartyKind, CounterpartyRepository, LedgerTag, TagId,
    TagOptions, TagRepository,
};

use crate::error::ApplicationError;
use crate::ports::{Clock, IdSource};

fn next_id(ids: &dyn IdSource) -> uuid::Uuid {
    ids.next_uuid()
}

/// Creates a category group.
pub struct CreateCategoryGroupUseCase<'a> {
    repository: &'a dyn CategoryGroupRepository,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

/// The request to create a category group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategoryGroupRequest {
    /// The group name.
    pub name: String,
    /// The kind shared by the group's categories.
    pub kind: CategoryKind,
    /// The manual sort position.
    pub sort_order: i32,
}

impl<'a> CreateCategoryGroupUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn CategoryGroupRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }

    /// Creates and saves the group.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the entity is invalid or the
    /// repository fails.
    pub fn execute(
        &self,
        request: &CreateCategoryGroupRequest,
    ) -> Result<CategoryGroup, ApplicationError> {
        let now = self.clock.now();
        let group = CategoryGroup::new(
            CategoryGroupId::from_uuid(next_id(self.ids)),
            &request.name,
            request.kind,
            request.sort_order,
            false,
            now,
            now,
        )
        .map_err(ApplicationError::CategoryGroup)?;
        self.repository.save_category_group(&group)?;
        Ok(group)
    }
}

/// The request to update a category group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCategoryGroupRequest {
    /// The group to update.
    pub id: CategoryGroupId,
    /// The new name.
    pub name: String,
    /// The new sort position.
    pub sort_order: i32,
    /// The new archived state.
    pub is_archived: bool,
}

/// Updates a category group.
pub struct UpdateCategoryGroupUseCase<'a> {
    repository: &'a dyn CategoryGroupRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateCategoryGroupUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn CategoryGroupRepository, clock: &'a dyn Clock) -> Self {
        Self { repository, clock }
    }

    /// Updates and saves the group.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the group is missing, invalid, or
    /// the repository fails.
    pub fn execute(
        &self,
        request: &UpdateCategoryGroupRequest,
    ) -> Result<CategoryGroup, ApplicationError> {
        let existing = self
            .repository
            .category_group(request.id)?
            .ok_or(ApplicationError::CategoryGroupNotFound(request.id))?;
        let group = CategoryGroup::new(
            existing.id(),
            &request.name,
            existing.kind(),
            request.sort_order,
            request.is_archived,
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::CategoryGroup)?;
        self.repository.save_category_group(&group)?;
        Ok(group)
    }
}

/// The request to create a leaf category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCategoryRequest {
    /// The category name.
    pub name: String,
    /// The category kind.
    pub kind: CategoryKind,
    /// The optional containing group.
    pub group_id: Option<CategoryGroupId>,
    /// The manual sort position.
    pub sort_order: i32,
}

/// Creates a leaf category.
pub struct CreateCategoryUseCase<'a> {
    repository: &'a dyn CategoryRepository,
    groups: &'a dyn CategoryGroupRepository,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateCategoryUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn CategoryRepository,
        groups: &'a dyn CategoryGroupRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            groups,
            clock,
            ids,
        }
    }

    /// Creates and saves the category.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the group is missing, archived, or
    /// has a different kind, the entity is invalid, or the repository fails.
    pub fn execute(&self, request: &CreateCategoryRequest) -> Result<Category, ApplicationError> {
        self.validate_group(request.group_id, request.kind)?;
        let now = self.clock.now();
        let category = Category::new(
            CategoryId::from_uuid(next_id(self.ids)),
            &request.name,
            request.kind,
            CategoryOptions {
                group_id: request.group_id,
                sort_order: request.sort_order,
                is_archived: false,
            },
            now,
            now,
        )
        .map_err(ApplicationError::Category)?;
        self.repository.save_category(&category)?;
        Ok(category)
    }

    fn validate_group(
        &self,
        group_id: Option<CategoryGroupId>,
        kind: CategoryKind,
    ) -> Result<(), ApplicationError> {
        let Some(group_id) = group_id else {
            return Ok(());
        };
        let group = self
            .groups
            .category_group(group_id)?
            .ok_or(ApplicationError::CategoryGroupNotFound(group_id))?;
        if group.is_archived() {
            return Err(ApplicationError::CategoryGroupArchived(group_id));
        }
        if group.kind() != kind {
            return Err(ApplicationError::CategoryKindMismatch {
                expected: group.kind(),
                actual: kind,
            });
        }
        Ok(())
    }
}

/// The request to update a leaf category.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCategoryRequest {
    /// The category to update.
    pub id: CategoryId,
    /// The new name.
    pub name: String,
    /// The new containing group.
    pub group_id: Option<CategoryGroupId>,
    /// The new sort position.
    pub sort_order: i32,
    /// The new archived state.
    pub is_archived: bool,
}

/// Updates a leaf category.
pub struct UpdateCategoryUseCase<'a> {
    repository: &'a dyn CategoryRepository,
    groups: &'a dyn CategoryGroupRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateCategoryUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn CategoryRepository,
        groups: &'a dyn CategoryGroupRepository,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            repository,
            groups,
            clock,
        }
    }

    /// Updates and saves the category.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the category is missing, the group
    /// is invalid, the entity is invalid, or the repository fails.
    pub fn execute(&self, request: &UpdateCategoryRequest) -> Result<Category, ApplicationError> {
        let existing = self
            .repository
            .category(request.id)?
            .ok_or(ApplicationError::CategoryNotFound(request.id))?;
        if let Some(group_id) = request.group_id {
            let group = self
                .groups
                .category_group(group_id)?
                .ok_or(ApplicationError::CategoryGroupNotFound(group_id))?;
            if group.is_archived() {
                return Err(ApplicationError::CategoryGroupArchived(group_id));
            }
            if group.kind() != existing.kind() {
                return Err(ApplicationError::CategoryKindMismatch {
                    expected: group.kind(),
                    actual: existing.kind(),
                });
            }
        }
        let category = Category::new(
            existing.id(),
            &request.name,
            existing.kind(),
            CategoryOptions {
                group_id: request.group_id,
                sort_order: request.sort_order,
                is_archived: request.is_archived,
            },
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::Category)?;
        self.repository.save_category(&category)?;
        Ok(category)
    }
}

/// The request to create a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateTagRequest {
    /// The tag name.
    pub name: String,
    /// The `#RRGGBB` color.
    pub color_hex: String,
    /// The optional description.
    pub description: Option<String>,
    /// The manual sort position.
    pub sort_order: i32,
}

/// Creates a tag.
pub struct CreateTagUseCase<'a> {
    repository: &'a dyn TagRepository,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateTagUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn TagRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }

    /// Creates and saves the tag.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the entity is invalid or the
    /// repository fails.
    pub fn execute(&self, request: &CreateTagRequest) -> Result<LedgerTag, ApplicationError> {
        let now = self.clock.now();
        let tag = LedgerTag::new(
            TagId::from_uuid(next_id(self.ids)),
            &request.name,
            &request.color_hex,
            TagOptions {
                description: request.description.clone(),
                sort_order: request.sort_order,
                is_archived: false,
            },
            now,
            now,
        )
        .map_err(ApplicationError::Tag)?;
        self.repository.save_tag(&tag)?;
        Ok(tag)
    }
}

/// The request to update a tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateTagRequest {
    /// The tag to update.
    pub id: TagId,
    /// The new name.
    pub name: String,
    /// The new color.
    pub color_hex: String,
    /// The new description.
    pub description: Option<String>,
    /// The new sort position.
    pub sort_order: i32,
    /// The new archived state.
    pub is_archived: bool,
}

/// Updates a tag.
pub struct UpdateTagUseCase<'a> {
    repository: &'a dyn TagRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateTagUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn TagRepository, clock: &'a dyn Clock) -> Self {
        Self { repository, clock }
    }

    /// Updates and saves the tag.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the tag is missing, invalid, or
    /// the repository fails.
    pub fn execute(&self, request: &UpdateTagRequest) -> Result<LedgerTag, ApplicationError> {
        let existing = self
            .repository
            .tag(request.id)?
            .ok_or(ApplicationError::TagNotFound(request.id))?;
        let tag = LedgerTag::new(
            existing.id(),
            &request.name,
            &request.color_hex,
            TagOptions {
                description: request.description.clone(),
                sort_order: request.sort_order,
                is_archived: request.is_archived,
            },
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::Tag)?;
        self.repository.save_tag(&tag)?;
        Ok(tag)
    }
}

/// The request to create a counterparty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateCounterpartyRequest {
    /// The counterparty kind.
    pub kind: CounterpartyKind,
    /// The name.
    pub name: String,
    /// The manual sort position.
    pub sort_order: i32,
}

/// Creates a counterparty.
pub struct CreateCounterpartyUseCase<'a> {
    repository: &'a dyn CounterpartyRepository,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateCounterpartyUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn CounterpartyRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }

    /// Creates and saves the counterparty.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the entity is invalid or the
    /// repository fails.
    pub fn execute(
        &self,
        request: &CreateCounterpartyRequest,
    ) -> Result<Counterparty, ApplicationError> {
        let now = self.clock.now();
        let counterparty = Counterparty::new(
            CounterpartyId::from_uuid(next_id(self.ids)),
            request.kind,
            &request.name,
            request.sort_order,
            false,
            now,
            now,
        )
        .map_err(ApplicationError::Counterparty)?;
        self.repository.save_counterparty(&counterparty)?;
        Ok(counterparty)
    }
}

/// The request to update a counterparty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateCounterpartyRequest {
    /// The counterparty to update.
    pub id: CounterpartyId,
    /// The new kind.
    pub kind: CounterpartyKind,
    /// The new name.
    pub name: String,
    /// The new sort position.
    pub sort_order: i32,
    /// The new archived state.
    pub is_archived: bool,
}

/// Updates a counterparty.
pub struct UpdateCounterpartyUseCase<'a> {
    repository: &'a dyn CounterpartyRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateCounterpartyUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn CounterpartyRepository, clock: &'a dyn Clock) -> Self {
        Self { repository, clock }
    }

    /// Updates and saves the counterparty.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the counterparty is missing,
    /// invalid, or the repository fails.
    pub fn execute(
        &self,
        request: &UpdateCounterpartyRequest,
    ) -> Result<Counterparty, ApplicationError> {
        let existing = self
            .repository
            .counterparty(request.id)?
            .ok_or(ApplicationError::CounterpartyNotFound(request.id))?;
        let counterparty = Counterparty::new(
            existing.id(),
            request.kind,
            &request.name,
            request.sort_order,
            request.is_archived,
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::Counterparty)?;
        self.repository.save_counterparty(&counterparty)?;
        Ok(counterparty)
    }
}

/// The request to create an account group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateAccountGroupRequest {
    /// The group name.
    pub name: String,
    /// The manual sort position.
    pub sort_order: i32,
}

/// Creates an account group.
pub struct CreateAccountGroupUseCase<'a> {
    repository: &'a dyn AccountGroupRepository,
    clock: &'a dyn Clock,
    ids: &'a dyn IdSource,
}

impl<'a> CreateAccountGroupUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn AccountGroupRepository,
        clock: &'a dyn Clock,
        ids: &'a dyn IdSource,
    ) -> Self {
        Self {
            repository,
            clock,
            ids,
        }
    }

    /// Creates and saves the group.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the entity is invalid or the
    /// repository fails.
    pub fn execute(
        &self,
        request: &CreateAccountGroupRequest,
    ) -> Result<AccountGroup, ApplicationError> {
        let now = self.clock.now();
        let group = AccountGroup::new(
            AccountGroupId::from_uuid(next_id(self.ids)),
            &request.name,
            request.sort_order,
            false,
            now,
            now,
        )
        .map_err(ApplicationError::AccountGroup)?;
        self.repository.save_account_group(&group)?;
        Ok(group)
    }
}

/// The request to update an account group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccountGroupRequest {
    /// The group to update.
    pub id: AccountGroupId,
    /// The new name.
    pub name: String,
    /// The new sort position.
    pub sort_order: i32,
    /// The new archived state.
    pub is_archived: bool,
}

/// Updates an account group.
pub struct UpdateAccountGroupUseCase<'a> {
    repository: &'a dyn AccountGroupRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateAccountGroupUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(repository: &'a dyn AccountGroupRepository, clock: &'a dyn Clock) -> Self {
        Self { repository, clock }
    }

    /// Updates and saves the group.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the group is missing, invalid, or
    /// the repository fails.
    pub fn execute(
        &self,
        request: &UpdateAccountGroupRequest,
    ) -> Result<AccountGroup, ApplicationError> {
        let existing = self
            .repository
            .account_group(request.id)?
            .ok_or(ApplicationError::AccountGroupNotFound(request.id))?;
        let group = AccountGroup::new(
            existing.id(),
            &request.name,
            request.sort_order,
            request.is_archived,
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::AccountGroup)?;
        self.repository.save_account_group(&group)?;
        Ok(group)
    }
}

/// The request to update an account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpdateAccountRequest {
    /// The account to update.
    pub id: AccountId,
    /// The new name.
    pub name: String,
    /// The new asset type.
    pub asset_type: AssetAccountType,
    /// The new optional fields.
    pub options: AccountOptions,
}

/// Updates an account without changing its currency.
pub struct UpdateAccountUseCase<'a> {
    repository: &'a dyn AccountRepository,
    groups: &'a dyn AccountGroupRepository,
    clock: &'a dyn Clock,
}

impl<'a> UpdateAccountUseCase<'a> {
    /// Creates the use case.
    #[must_use]
    pub fn new(
        repository: &'a dyn AccountRepository,
        groups: &'a dyn AccountGroupRepository,
        clock: &'a dyn Clock,
    ) -> Self {
        Self {
            repository,
            groups,
            clock,
        }
    }

    /// Updates and saves the account.
    ///
    /// # Errors
    ///
    /// Returns an [`ApplicationError`] when the account is missing, a custom
    /// group is invalid, the entity is invalid, or the repository fails.
    pub fn execute(&self, request: &UpdateAccountRequest) -> Result<Account, ApplicationError> {
        let existing = self
            .repository
            .account(request.id)?
            .ok_or(ApplicationError::AccountNotFound(request.id))?;
        if let Some(group_id) = request.options.custom_group_id {
            let group = self
                .groups
                .account_group(group_id)?
                .ok_or(ApplicationError::AccountGroupNotFound(group_id))?;
            if group.is_archived() {
                return Err(ApplicationError::AccountGroupArchived(group_id));
            }
        }
        let account = Account::new(
            existing.id(),
            &request.name,
            request.asset_type,
            existing.currency().clone(),
            request.options.clone(),
            existing.created_at(),
            self.clock.now(),
        )
        .map_err(ApplicationError::Account)?;
        self.repository.save_account(&account)?;
        Ok(account)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{FixedClock, SequenceIds};
    use core_testkit::InMemoryLedger;
    use jiff::Timestamp;

    fn clock() -> FixedClock {
        FixedClock::new(Timestamp::from_second(1_000).unwrap())
    }

    #[test]
    fn category_group_kind_must_match_category_kind() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let create_group = CreateCategoryGroupUseCase::new(&ledger, &clock, &ids);
        let group = create_group
            .execute(&CreateCategoryGroupRequest {
                name: "Essentials".to_string(),
                kind: CategoryKind::Expense,
                sort_order: 0,
            })
            .unwrap();
        let create_category = CreateCategoryUseCase::new(&ledger, &ledger, &clock, &ids);

        assert_eq!(
            create_category
                .execute(&CreateCategoryRequest {
                    name: "Salary".to_string(),
                    kind: CategoryKind::Income,
                    group_id: Some(group.id()),
                    sort_order: 0,
                })
                .unwrap_err(),
            ApplicationError::CategoryKindMismatch {
                expected: CategoryKind::Expense,
                actual: CategoryKind::Income,
            }
        );
    }

    #[test]
    fn archived_group_cannot_accept_categories() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let group = CreateCategoryGroupUseCase::new(&ledger, &clock, &ids)
            .execute(&CreateCategoryGroupRequest {
                name: "Old".to_string(),
                kind: CategoryKind::Expense,
                sort_order: 0,
            })
            .unwrap();
        UpdateCategoryGroupUseCase::new(&ledger, &clock)
            .execute(&UpdateCategoryGroupRequest {
                id: group.id(),
                name: "Old".to_string(),
                sort_order: 0,
                is_archived: true,
            })
            .unwrap();
        let create_category = CreateCategoryUseCase::new(&ledger, &ledger, &clock, &ids);

        assert_eq!(
            create_category
                .execute(&CreateCategoryRequest {
                    name: "Food".to_string(),
                    kind: CategoryKind::Expense,
                    group_id: Some(group.id()),
                    sort_order: 0,
                })
                .unwrap_err(),
            ApplicationError::CategoryGroupArchived(group.id())
        );
    }

    #[test]
    fn tag_creation_and_update_preserve_identity() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let created = CreateTagUseCase::new(&ledger, &clock, &ids)
            .execute(&CreateTagRequest {
                name: "Travel".to_string(),
                color_hex: "#112233".to_string(),
                description: None,
                sort_order: 0,
            })
            .unwrap();
        let updated = UpdateTagUseCase::new(&ledger, &clock)
            .execute(&UpdateTagRequest {
                id: created.id(),
                name: "Trips".to_string(),
                color_hex: "#445566".to_string(),
                description: Some("All trips".to_string()),
                sort_order: 1,
                is_archived: false,
            })
            .unwrap();

        assert_eq!(updated.id(), created.id());
        assert_eq!(updated.created_at(), created.created_at());
        assert_eq!(updated.name(), "Trips");
        assert_eq!(updated.color_hex(), "#445566");
    }

    #[test]
    fn account_update_preserves_currency_and_can_archive() {
        let ledger = InMemoryLedger::new();
        let clock = clock();
        let ids = SequenceIds::new();
        let group = CreateAccountGroupUseCase::new(&ledger, &clock, &ids)
            .execute(&CreateAccountGroupRequest {
                name: "Bank".to_string(),
                sort_order: 0,
            })
            .unwrap();
        let account = Account::new(
            AccountId::from_uuid(uuid::Uuid::from_u128(1)),
            "Cash",
            AssetAccountType::Cash,
            core_domain::CurrencyScalePolicy::v1()
                .currency("USD")
                .unwrap(),
            AccountOptions::default(),
            Timestamp::from_second(1_000).unwrap(),
            Timestamp::from_second(1_000).unwrap(),
        )
        .unwrap();
        ledger.save_account(&account).unwrap();
        let update = UpdateAccountUseCase::new(&ledger, &ledger, &clock);

        let archived = update
            .execute(&UpdateAccountRequest {
                id: account.id(),
                name: "Cash".to_string(),
                asset_type: AssetAccountType::Cash,
                options: AccountOptions {
                    is_archived: true,
                    custom_group_id: Some(group.id()),
                    ..AccountOptions::default()
                },
            })
            .unwrap();

        assert!(archived.is_archived());
        assert_eq!(archived.currency().code(), "USD");
    }
}
