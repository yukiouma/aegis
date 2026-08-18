//! PostgreSQL-backed implementation of `CodeListRepository`,
//! including the `tsvector` / GIN-backed search.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CodeList, CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery,
    CodeListUpdate, DomainError,
};

const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
const SQLSTATE_FK_VIOLATION: &str = "23503";

#[derive(FromRow)]
struct CodeListRow {
    id: i64,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CodeListRow> for CodeList {
    type Error = DomainError;

    fn try_from(row: CodeListRow) -> Result<Self, Self::Error> {
        Ok(CodeList::for_repository(
            row.id,
            row.version_id,
            row.code,
            row.extensible,
            row.name,
            row.submission_value,
            row.synonym,
            row.definition,
            row.nci_preferred_term,
            row.created_at,
            row.updated_at,
        ))
    }
}

pub struct CodeListRepo {
    pool: PgPool,
}

impl CodeListRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodeListRepository for CodeListRepo {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError> {
        let row: CodeListRow = sqlx::QueryBuilder::new(
            "INSERT INTO code_lists \
             (version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term) \
             VALUES (",
        )
        .push_bind(input.version_id)
        .push(", ")
        .push_bind(&input.code)
        .push(", ")
        .push_bind(input.extensible)
        .push(", ")
        .push_bind(&input.name)
        .push(", ")
        .push_bind(&input.submission_value)
        .push(", ")
        .push_bind(&input.synonym)
        .push(", ")
        .push_bind(&input.definition)
        .push(", ")
        .push_bind(&input.nci_preferred_term)
        .push(") RETURNING id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at")
        .build_query_as::<CodeListRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|err| map_db_error(err, Some(input.version_id)))?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError> {
        let row: CodeListRow = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_lists WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<CodeListRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error_simple)?
        .ok_or(DomainError::CodeListNotFound(id))?;
        row.try_into()
    }

    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError> {
        let rows: Vec<CodeListRow> = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_lists WHERE version_id = ",
        )
        .push_bind(version_id)
        .push(" ORDER BY id")
        .build_query_as::<CodeListRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter().map(CodeList::try_from).collect()
    }

    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE code_lists SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref code) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(code);
        }
        if let Some(ext) = input.extensible {
            sep(&mut qb);
            qb.push("extensible = ").push_bind(ext);
        }
        if let Some(ref name) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(name);
        }
        if let Some(ref sv) = input.submission_value {
            sep(&mut qb);
            qb.push("submission_value = ").push_bind(sv);
        }
        if let Some(ref syn) = input.synonym {
            sep(&mut qb);
            qb.push("synonym = ").push_bind(syn);
        }
        if let Some(ref def) = input.definition {
            sep(&mut qb);
            qb.push("definition = ").push_bind(def);
        }
        if let Some(ref pt) = input.nci_preferred_term {
            sep(&mut qb);
            qb.push("nci_preferred_term = ").push_bind(pt);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at");
        let row: CodeListRow = qb
            .build_query_as::<CodeListRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error_simple)?
            .ok_or(DomainError::CodeListNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM code_lists WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CodeListNotFound(id));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError> {
        // Full-text search via the generated `tsv` tsvector +
        // GIN(tsv) index from migration 0002. `to_tsquery`
        // parameterises the entire query string, so no SQL
        // injection surface. The query plan uses the GIN index for
        // the `@@` predicate; ordering by `ts_rank` (Postgres
        // computes it lazily over the candidate rows) plus `LIMIT`
        // keeps the response bounded.
        let limit = query.limit;
        let rows: Vec<CodeListRow> = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_lists \
             WHERE version_id = ",
        )
        .push_bind(query.version_id)
        .push(" AND tsv @@ to_tsquery('english', ")
        .push_bind(format!("{}:*", query.fragment))
        .push(") \
             ORDER BY ts_rank(tsv, to_tsquery('english', ")
        .push_bind(format!("{}:*", query.fragment))
        .push(")) DESC \
             LIMIT ")
        .push_bind(limit as i64)
        .build_query_as::<CodeListRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter()
            .map(|row| {
                Ok(CodeListSearchHit {
                    codelist: row.try_into()?,
                })
            })
            .collect()
    }
}

/// Map any `sqlx::Error` from a non-create call on this repo to a
/// `DomainError`. These calls never produce SQLSTATE `23503`
/// (the FK is satisfied before the call) so the simpler mapper
/// is correct.
fn map_db_error_simple(err: sqlx::Error) -> DomainError {
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

/// `create` mapper: knows about the version_id it just inserted
/// with, so SQLSTATE `23503` becomes `FkVersionNotFound(version_id)`.
fn map_db_error(err: sqlx::Error, version_id_hint: Option<i64>) -> DomainError {
    if let sqlx::Error::Database(db_err) = &err
        && db_err.code().as_deref() == Some(SQLSTATE_FK_VIOLATION)
    {
        return DomainError::FkVersionNotFound(version_id_hint.unwrap_or(0));
    }
    map_db_error_simple(err)
}
