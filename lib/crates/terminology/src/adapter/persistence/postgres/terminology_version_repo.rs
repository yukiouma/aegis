//! PostgreSQL-backed implementation of `TerminologyVersionRepository`.
//!
//! Uses the runtime SQLx API (`sqlx::query_as`, `QueryBuilder`)
//! rather than compile-time-checked macros; the workspace does
//! not currently provide a live `DATABASE_URL` or a checked-in
//! `sqlx-data.json` cache at build time.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::FromRow;
use sqlx::PgPool;

use crate::domain::{
    DomainError, TerminologyKind, TerminologyVersion, TerminologyVersionNew,
    TerminologyVersionRepository, TerminologyVersionUpdate,
};

/// PostgreSQL SQLSTATE codes.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

#[derive(FromRow)]
struct TerminologyVersionRow {
    id: i64,
    kind: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<TerminologyVersionRow> for TerminologyVersion {
    type Error = DomainError;

    fn try_from(row: TerminologyVersionRow) -> Result<Self, Self::Error> {
        let kind = TerminologyKind::try_from(row.kind.as_str())?;
        Ok(TerminologyVersion::for_repository(
            row.id,
            kind,
            row.name,
            row.created_at,
            row.updated_at,
        ))
    }
}

pub struct TerminologyVersionRepo {
    pool: PgPool,
}

impl TerminologyVersionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TerminologyVersionRepository for TerminologyVersionRepo {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow =
            sqlx::QueryBuilder::new("INSERT INTO terminology_versions (kind, name) VALUES (")
                .push_bind(input.kind.as_str())
                .push(", ")
                .push_bind(&input.name)
                .push(") RETURNING id, kind, name, created_at, updated_at")
                .build_query_as::<TerminologyVersionRow>()
                .fetch_one(&self.pool)
                .await
                .map_err(map_db_error)?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<TerminologyVersionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::VersionNotFound(id))?;
        row.try_into()
    }

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions WHERE kind = ",
        )
        .push_bind(kind.as_str())
        .push(" AND name = ")
        .push_bind(name)
        .build_query_as::<TerminologyVersionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError> {
        let rows: Vec<TerminologyVersionRow> = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions ORDER BY id",
        )
        .build_query_as::<TerminologyVersionRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(TerminologyVersion::try_from).collect()
    }

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE terminology_versions SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(kind) = input.kind {
            sep(&mut qb);
            qb.push("kind = ").push_bind(kind.as_str());
        }
        if let Some(ref name) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(name);
        }
        if first {
            // Nothing to update; short-circuit and return the
            // existing row, or `VersionNotFound` if the id is
            // unknown.
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, kind, name, created_at, updated_at");
        let row: TerminologyVersionRow = qb
            .build_query_as::<TerminologyVersionRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::VersionNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM terminology_versions WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::VersionNotFound(id));
        }
        Ok(())
    }
}

fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::Repository(format!(
                    "duplicate key violates unique constraint `{constraint}`"
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
