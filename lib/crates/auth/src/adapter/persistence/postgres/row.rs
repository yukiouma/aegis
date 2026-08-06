use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, DomainIdentity, UserCredentials};

#[derive(Clone, FromRow)]
pub struct CredentialRow {
    pub code: String,
    pub password_hash: String,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<CredentialRow> for UserCredentials {
    type Error = DomainError;

    fn try_from(row: CredentialRow) -> Result<Self, Self::Error> {
        if row.token_version < 0 {
            return Err(DomainError::Repository(format!(
                "negative token_version: {}",
                row.token_version
            )));
        }
        Ok(UserCredentials::for_repository(
            row.code,
            row.password_hash,
            row.token_version as u32,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Clone, FromRow)]
pub struct DomainIdentityRow {
    pub user_code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl TryFrom<DomainIdentityRow> for DomainIdentity {
    type Error = DomainError;

    fn try_from(row: DomainIdentityRow) -> Result<Self, Self::Error> {
        Ok(DomainIdentity::for_repository(
            row.user_code,
            row.domain_name,
            row.hostname,
            row.sid,
        ))
    }
}
