//! PostgreSQL-backed implementation of `CodeItemRepository`,
//! including the `tsvector` / GIN-backed search.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, DomainError,
};

const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
const SQLSTATE_FK_VIOLATION: &str = "23503";

#[derive(FromRow)]
struct CodeItemRow {
    id: i64,
    codelist_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CodeItemRow> for CodeItem {
    type Error = DomainError;

    fn try_from(row: CodeItemRow) -> Result<Self, Self::Error> {
        Ok(CodeItem::for_repository(
            row.id,
            row.codelist_id,
            row.code,
            row.submission_value,
            row.synonym,
            row.definition,
            row.nci_preferred_term,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(FromRow)]
struct CodeItemSearchRow {
    id: i64,
    codelist_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    score: f32,
}

pub struct CodeItemRepo {
    pool: PgPool,
}

impl CodeItemRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodeItemRepository for CodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "INSERT INTO code_items \
             (codelist_id, code, submission_value, synonym, definition, nci_preferred_term) \
             VALUES (",
        )
        .push_bind(input.codelist_id)
        .push(", ")
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.submission_value)
        .push(", ")
        .push_bind(&input.synonym)
        .push(", ")
        .push_bind(&input.definition)
        .push(", ")
        .push_bind(&input.nci_preferred_term)
        .push(") RETURNING id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at")
        .build_query_as::<CodeItemRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|err| map_db_error(err, Some(input.codelist_id)))?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_items WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<CodeItemRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error_simple)?
        .ok_or(DomainError::CodeItemNotFound(id))?;
        row.try_into()
    }

    async fn list_by_codelist(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItem>, DomainError> {
        let rows: Vec<CodeItemRow> = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_items WHERE codelist_id = ",
        )
        .push_bind(codelist_id)
        .push(" ORDER BY id")
        .build_query_as::<CodeItemRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter().map(CodeItem::try_from).collect()
    }

    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE code_items SET ");
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
        qb.push(" RETURNING id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at");
        let row: CodeItemRow = qb
            .build_query_as::<CodeItemRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error_simple)?
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM code_items WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError> {
        let text = query.text.clone();
        let kind = query.kind;
        let version_name = query.version_name.clone();
        let limit = query.limit as i64;
        let rows: Vec<CodeItemSearchRow> = sqlx::QueryBuilder::new(
            "SELECT ci.id, ci.codelist_id, ci.code, ci.submission_value, \
                    ci.synonym, ci.definition, ci.nci_preferred_term, \
                    ci.created_at, ci.updated_at, \
                    ts_rank_cd(ci.tsv, websearch_to_tsquery('english', ",
        )
        .push_bind(&text)
        .push(
            ")) AS score \
             FROM code_items ci \
             JOIN code_lists cl ON cl.id = ci.codelist_id \
             JOIN terminology_versions v ON v.id = cl.version_id \
             WHERE v.kind = ",
        )
        .push_bind(kind.as_str())
        .push(" AND v.name = ")
        .push_bind(&version_name)
        .push(" AND ci.tsv @@ websearch_to_tsquery('english', ")
        .push_bind(&text)
        .push(") ORDER BY score DESC LIMIT ")
        .push_bind(limit)
        .build_query_as::<CodeItemSearchRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter()
            .map(|row| {
                let codelist_id = row.codelist_id;
                let item = CodeItem::try_from(CodeItemRow {
                    id: row.id,
                    codelist_id,
                    code: row.code,
                    submission_value: row.submission_value,
                    synonym: row.synonym,
                    definition: row.definition,
                    nci_preferred_term: row.nci_preferred_term,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })?;
                Ok(CodeItemSearchHit {
                    item,
                    score: row.score,
                    codelist_id,
                })
            })
            .collect()
    }
}

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

/// `create` mapper: knows about the codelist_id it just inserted
/// with, so SQLSTATE `23503` becomes `FkCodeListNotFound(codelist_id)`.
fn map_db_error(err: sqlx::Error, codelist_id_hint: Option<i64>) -> DomainError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some(SQLSTATE_FK_VIOLATION) {
            return DomainError::FkCodeListNotFound(codelist_id_hint.unwrap_or(0));
        }
    }
    map_db_error_simple(err)
}