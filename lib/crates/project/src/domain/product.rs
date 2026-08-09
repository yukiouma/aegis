use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct Product {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        name: String,
        description: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            code,
            name,
            description,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        name: String,
        description: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            name,
            description,
            active,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for Product {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Product")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input DTO for `ProductRepository::create`.
#[derive(Debug, Clone)]
pub struct ProductNew {
    pub code: String,
    pub name: String,
    pub description: String,
}

/// Input DTO for `ProductRepository::update`. Every field is optional
/// so the usecase can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct ProductUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

/// Outbound port for persistence of `Product` aggregates.
#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError>;
    async fn list(&self) -> Result<Vec<Product>, DomainError>;
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError>;
}

