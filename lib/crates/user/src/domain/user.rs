use std::fmt;

use super::DomainError;
use super::role::Role;

#[derive(Clone, PartialEq, Eq)]
pub struct User {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub(crate) password: String,
}

impl User {
    /// Strict constructor used by the domain layer.
    ///
    /// Validates that `code`, `name`, and `password` are non-empty.
    pub fn new(
        id: i32,
        code: String,
        name: String,
        role: Role,
        active: bool,
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
            password,
        })
    }

    /// Constructor reserved for the infrastructure layer when materialising
    /// rows from persistence. Skips domain validation because the data is
    /// assumed to have been validated on the way in.
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        name: String,
        role: Role,
        active: bool,
        password: String,
    ) -> Self {
        Self {
            id,
            code,
            name,
            role,
            active,
            password,
        }
    }

    /// Returns the argon2 password hash. Only available inside the crate
    /// so the hash never leaves the `user` crate's boundary.
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
            .field("password", &"<redacted>")
            .finish()
    }
}
