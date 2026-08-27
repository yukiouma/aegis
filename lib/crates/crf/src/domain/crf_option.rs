use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A selectable value attached to a `CrfItem` of kind
/// `Selection` or `Checkbox`. Items may carry any number of
/// options; the DB schema carries no per-item uniqueness.
#[derive(Clone, PartialEq, Eq)]
pub struct CrfOption {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CrfOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CrfOption")
            .field("id", &self.id)
            .field("item_id", &self.item_id)
            .field("value", &self.value)
            .field("not_submitted", &self.not_submitted)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CrfOption {
    /// Validating constructor used by the domain layer.
    /// Rejects empty / whitespace `value`.
    pub fn new(item_id: i64, value: String, not_submitted: bool) -> Result<Self, DomainError> {
        if value.trim().is_empty() {
            return Err(DomainError::EmptyValue);
        }
        Ok(Self {
            id: 0,
            item_id,
            value,
            not_submitted,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer
    /// when materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        item_id: i64,
        value: String,
        not_submitted: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            item_id,
            value,
            not_submitted,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CrfOptionRepository::create`.
#[derive(Debug, Clone)]
pub struct CrfOptionNew {
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
}

/// Input DTO for `CrfOptionRepository::update`.
#[derive(Debug, Clone, Default)]
pub struct CrfOptionUpdate {
    pub id: i64,
    pub value: Option<String>,
    pub not_submitted: Option<bool>,
}

/// Persistence port for the `CrfOption` aggregate.
#[async_trait]
pub trait CrfOptionRepository: Send + Sync {
    async fn create(&self, input: CrfOptionNew) -> Result<CrfOption, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CrfOption, DomainError>;
    async fn list_by_item(&self, item_id: i64) -> Result<Vec<CrfOption>, DomainError>;
    /// Batch-fetch every option whose `item_id` is in
    /// `item_ids`. Returns `Ok(Vec::new())` for empty input
    /// without hitting the DB. Used by the form-detail usecase
    /// to hydrate the items subtree in one round-trip.
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<CrfOption>, DomainError>;
    async fn update(&self, input: CrfOptionUpdate) -> Result<CrfOption, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    /// Count options on an item. Used by the kind-shape
    /// validation in the usecase.
    async fn count_by_item(&self, item_id: i64) -> Result<i64, DomainError>;
    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfOption>, DomainError>;
}
