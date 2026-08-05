use chrono::{DateTime, Utc};

use super::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct UserCredentials {
    pub code: String,
    pub password_hash: String,
    pub token_version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserCredentials {
    /// Validating constructor used by the domain / usecase layers.
    #[allow(dead_code)]
    pub(crate) fn new(
        code: String,
        password_hash: String,
        token_version: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if password_hash.is_empty() {
            return Err(DomainError::EmptyPasswordHash);
        }
        Ok(Self {
            code,
            password_hash,
            token_version,
            created_at,
            updated_at,
        })
    }

    /// Repository-bound constructor. Skips validation because the row
    /// is assumed to have been validated on the way in.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        code: String,
        password_hash: String,
        token_version: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            code,
            password_hash,
            token_version,
            created_at,
            updated_at,
        }
    }
}

/// Hand-rolled `Debug` that omits the password hash.
impl std::fmt::Debug for UserCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserCredentials")
            .field("code", &self.code)
            .field("token_version", &self.token_version)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish_non_exhaustive()
    }
}