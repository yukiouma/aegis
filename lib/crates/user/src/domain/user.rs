use std::fmt;

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
    pub(crate) password: String,
}

impl User {
    /// Strict constructor used by the domain layer.
    ///
    /// Validates that `code`, `name`, and `password` are non-empty.
    /// Kept `pub(crate)` because the production paths construct
    /// `User` from inbound SQLx rows (`for_repository`) or from
    /// `UserView` (`From`); the validating constructor is only useful
    /// for in-crate tests and the future password-verification path.
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
        password: String,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        if password.is_empty() {
            return Err(DomainError::EmptyPassword);
        }
        Ok(Self {
            id,
            code,
            name,
            role,
            active,
            created_at,
            updated_at,
            password,
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
        password: String,
    ) -> Self {
        Self {
            id,
            code,
            name,
            role,
            active,
            created_at,
            updated_at,
            password,
        }
    }

    /// Returns the argon2 password hash. Only available inside the crate
    /// so the hash never leaves the `user` crate's boundary.
    ///
    /// The accessor is reserved for the future password-verification
    /// entry point in the infrastructure layer. Until that lands, the
    /// only consumer is the infrastructure test suite (which calls
    /// `password_hash()` to assert that `for_repository` round-trips
    /// the `password` field). The `#[allow(dead_code)]` silences the
    /// lib build, which does not see test usage; the test build sees
    /// the call site and would not warn even without the allow.
    #[allow(dead_code)]
    pub(crate) fn password_hash(&self) -> &str {
        &self.password
    }
}

/// Hand-rolled `Debug` impl that intentionally redacts the `password`
/// field. The hash must never appear in logs or error messages.
impl fmt::Debug for User {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("User")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("role", &self.role)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .field("password", &"<redacted>")
            .finish()
    }
}
