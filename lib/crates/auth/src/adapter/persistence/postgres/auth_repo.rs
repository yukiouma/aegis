use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, UserCredentials,
    UserCredentialsRepository,
};

use super::row::{CredentialRow, DomainIdentityRow};

/// PostgreSQL SQLSTATE for a unique-violation error.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct UserCredentialsRepo {
    pool: PgPool,
}

impl UserCredentialsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserCredentialsRepository for UserCredentialsRepo {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError> {
        let row: Option<CredentialRow> = sqlx::QueryBuilder::new(
            "SELECT code, password_hash, token_version, created_at, updated_at \
             FROM auth_user_credentials WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<CredentialRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        let row = row.ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn create(&self, credentials: UserCredentials) -> Result<UserCredentials, DomainError> {
        let row: CredentialRow = sqlx::QueryBuilder::new(
            "INSERT INTO auth_user_credentials (code, password_hash, token_version) VALUES (",
        )
        .push_bind(credentials.code.clone())
        .push(", ")
        .push_bind(credentials.password_hash.clone())
        .push(", ")
        .push_bind(credentials.token_version as i32)
        .push(") RETURNING code, password_hash, token_version, created_at, updated_at")
        .build_query_as::<CredentialRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        row.try_into()
    }

    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError> {
        let row: (i32,) = sqlx::QueryBuilder::new(
            "UPDATE auth_user_credentials SET token_version = token_version + 1 \
             WHERE code = ",
        )
        .push_bind(code)
        .push(" RETURNING token_version")
        .build_query_as::<(i32,)>()
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::RowNotFound => DomainError::NotFound,
            other => map_db_error(other),
        })?;
        if row.0 < 0 {
            return Err(DomainError::Repository(format!(
                "negative token_version after bump: {}",
                row.0
            )));
        }
        Ok(row.0 as u32)
    }

    async fn update_password_hash(
        &self,
        code: &str,
        password_hash: &str,
    ) -> Result<UserCredentials, DomainError> {
        let row: Option<CredentialRow> =
            sqlx::QueryBuilder::new("UPDATE auth_user_credentials SET password_hash = ")
                .push_bind(password_hash)
                .push(" WHERE code = ")
                .push_bind(code)
                .push(" RETURNING code, password_hash, token_version, created_at, updated_at")
                .build_query_as::<CredentialRow>()
                .fetch_optional(&self.pool)
                .await
                .map_err(map_db_error)?;
        let row = row.ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn delete_by_code(&self, code: &str) -> Result<(), DomainError> {
        let rows = sqlx::QueryBuilder::new("DELETE FROM auth_user_credentials WHERE code = ")
            .push_bind(code)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?
            .rows_affected();
        if rows == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }
}

pub struct DomainIdentityRepo {
    pool: PgPool,
}

impl DomainIdentityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DomainIdentityRepository for DomainIdentityRepo {
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError> {
        let row: Option<DomainIdentityRow> = sqlx::QueryBuilder::new(
            "SELECT user_code, domain_name, hostname, sid \
             FROM auth_user_domain_identities \
             WHERE user_code = ",
        )
        .push_bind(user_code)
        .push(" AND domain_name = ")
        .push_bind(domain_name)
        .push(" AND hostname = ")
        .push_bind(hostname)
        .push(" AND sid = ")
        .push_bind(sid)
        .build_query_as::<DomainIdentityRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        let row = row.ok_or(DomainError::NotFound)?;
        row.try_into()
    }
}

fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::DuplicateCode(format!("(constraint {constraint})"))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
