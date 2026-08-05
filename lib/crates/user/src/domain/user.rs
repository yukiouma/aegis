use chrono::{DateTime, Utc};

use super::DomainError;
use super::role::Role;

#[derive(Clone, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl User {
    /// Constructor used by the domain layer.
    ///
    /// Validates that `code` and `name` are non-empty. Kept
    /// `pub(crate)` because the production paths construct
    /// `User` from inbound SQLx rows (`for_repository`); the
    /// validating constructor is useful for in-crate tests.
    /// The `allow(dead_code)` silences the lib build (which does not
    /// see the test call sites); the test build sees the calls and
    /// would not warn even without the allow.
    #[allow(dead_code)]
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        name: String,
        role: Role,
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
            role,
            active,
            created_at,
            updated_at,
        })
    }

    /// Constructor reserved for the infrastructure layer when materialising
    /// rows from persistence. Skips domain validation because the data is
    /// assumed to have been validated on the way in.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        name: String,
        role: Role,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            name,
            role,
            active,
            created_at,
            updated_at,
        }
    }
}

/// Hand-rolled `Debug` impl that omits sensitive fields. Currently
/// every field on `User` is safe to log.
impl std::fmt::Debug for User {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("role", &self.role)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}