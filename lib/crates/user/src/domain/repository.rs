use async_trait::async_trait;

use super::DomainError;
use super::role::Role;
use super::user::User;

/// Input DTO for `UserRepository::create`.
#[derive(Debug, Clone)]
pub struct UserNew {
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
}

/// Input DTO for `UserRepository::update`. Every field is optional so the
/// use case can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct UserUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

/// Outbound port for persistence of `User` aggregates.
///
/// Implementations live in the infrastructure layer (e.g. PostgreSQL via
/// `sqlx`). Domain code depends on this trait only; never on concrete
/// repositories.
#[async_trait]
pub trait UserRepository: Send + Sync {
    /// Persists a new user. Returns `DomainError::DuplicateCode` if a
    /// user with the same code already exists.
    async fn create(&self, input: UserNew) -> Result<User, DomainError>;

    async fn find_by_id(&self, id: i32) -> Result<User, DomainError>;

    async fn find_by_code(&self, code: &str) -> Result<User, DomainError>;

    /// Returns every user. Paginated iteration should be added in a later
    /// task; for now the repository returns the full collection.
    async fn list(&self) -> Result<Vec<User>, DomainError>;

    /// Applies the fields set on `input` to the user identified by `input.id`.
    /// Returns `DomainError::NotFound` if no such user exists.
    async fn update(&self, input: UserUpdate) -> Result<User, DomainError>;
}