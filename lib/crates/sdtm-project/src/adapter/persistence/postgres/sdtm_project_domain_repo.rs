// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
// `sqlx::query_as` + `sqlx::query` + `FromRow` are sufficient and
// keep the crate reproducible.
//
// The domain is a complete copy: it carries its own category and
// descriptions, so there is no join and no separate override row to
// hydrate.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainCategory, DomainError, SdtmDomainDescription, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate,
};

#[derive(FromRow)]
struct SdtmProjectDomainRow {
    id: i64,
    sdtm_project_version_id: i64,
    domain_code: String,
    category: String,
    descriptions: JsonValue,
    key_variables: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmProjectDomainRow {
    fn into_domain(self) -> Result<SdtmProjectDomain, DomainError> {
        let category = DomainCategory::try_from(self.category.as_str())?;
        let descriptions: Vec<SdtmDomainDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let key_variables: Vec<String> = serde_json::from_value(self.key_variables)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmProjectDomain::for_repository(
            self.id,
            self.sdtm_project_version_id,
            self.domain_code,
            category,
            descriptions,
            key_variables,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmProjectDomainRepoPg {
    pool: PgPool,
}

impl SdtmProjectDomainRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectDomainRepository for SdtmProjectDomainRepoPg {
    async fn create(&self, input: SdtmProjectDomainNew) -> Result<SdtmProjectDomain, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let key_variables_json = serde_json::to_value(&input.key_variables)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        // The caller's order is preserved verbatim: the vector is
        // serialised as-is and never re-sorted.
        let row: SdtmProjectDomainRow = sqlx::query_as::<_, SdtmProjectDomainRow>(
            "INSERT INTO sdtm_project_domains
                (sdtm_project_version_id, domain_code, category, descriptions, key_variables)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, sdtm_project_version_id, domain_code, category, descriptions,
                       key_variables, created_at, updated_at",
        )
        .bind(input.sdtm_project_version_id)
        .bind(&input.domain_code)
        .bind(input.category.as_str())
        .bind(descriptions_json)
        .bind(key_variables_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_domain_err(e, input.sdtm_project_version_id))?;
        row.into_domain()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectDomain, DomainError> {
        let row: SdtmProjectDomainRow = sqlx::query_as::<_, SdtmProjectDomainRow>(
            "SELECT id, sdtm_project_version_id, domain_code, category, descriptions,
                    key_variables, created_at, updated_at
             FROM sdtm_project_domains
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_domain_err(e, 0))?
        .ok_or(DomainError::SdtmProjectDomainNotFound(id))?;
        row.into_domain()
    }

    async fn list_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmProjectDomain>, DomainError> {
        let rows: Vec<SdtmProjectDomainRow> = sqlx::query_as::<_, SdtmProjectDomainRow>(
            "SELECT id, sdtm_project_version_id, domain_code, category, descriptions,
                    key_variables, created_at, updated_at
             FROM sdtm_project_domains
             WHERE sdtm_project_version_id = $1
             ORDER BY id ASC",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_domain_err(e, 0))?;
        rows.into_iter()
            .map(SdtmProjectDomainRow::into_domain)
            .collect()
    }

    async fn update(
        &self,
        input: SdtmProjectDomainUpdate,
    ) -> Result<SdtmProjectDomain, DomainError> {
        // COALESCE: a NULL argument leaves the column unchanged. A
        // `Some(vec![])` binds an empty JSONB array (not NULL), so the
        // column is cleared rather than left untouched.
        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let key_variables_json = match &input.key_variables {
            None => None,
            Some(kv) => {
                Some(serde_json::to_value(kv).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let row: SdtmProjectDomainRow = sqlx::query_as::<_, SdtmProjectDomainRow>(
            "UPDATE sdtm_project_domains
             SET domain_code   = COALESCE($2, domain_code),
                 category      = COALESCE($3, category),
                 descriptions  = COALESCE($4, descriptions),
                 key_variables = COALESCE($5, key_variables)
             WHERE id = $1
             RETURNING id, sdtm_project_version_id, domain_code, category, descriptions,
                       key_variables, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.domain_code.as_deref())
        .bind(input.category.map(|c| c.as_str()))
        .bind(descriptions_json)
        .bind(key_variables_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_domain_err(e, 0))?
        .ok_or(DomainError::SdtmProjectDomainNotFound(input.id))?;
        row.into_domain()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        // The FK cascades: deleting a domain removes its variables.
        let res = sqlx::query("DELETE FROM sdtm_project_domains WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| map_domain_err(e, 0))?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectDomainNotFound(id));
        }
        Ok(())
    }
}

/// Map a driver error to a domain error.
///
/// `parent_version_id` is the version the statement referenced. A
/// foreign-key violation can only arise from an INSERT, so the read
/// and update paths — which cannot violate the FK — pass 0; the
/// create path passes the id the caller actually used, so the error
/// names the version that was missing.
fn map_domain_err(err: sqlx::Error, parent_version_id: i64) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // Unique violation on (sdtm_project_version_id,
                // domain_code). The driver does not surface the
                // natural key.
                return DomainError::DuplicateSdtmProjectDomain {
                    version_id: 0,
                    domain_code: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: the parent version is gone. This is
                // the backstop for the race where the version is
                // deleted between the usecase's check and the insert.
                return DomainError::FkSdtmProjectVersionNotFound(parent_version_id);
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migration_file_declares_domain_tables() {
        let sql =
            include_str!("../../../../migrations/20261103135412_create_sdtm_project_domains.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_domains"));
        assert!(sql.contains("sdtm_project_version_id  BIGINT"));
        assert!(sql.contains(
            "CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (sdtm_project_version_id, domain_code)"
        ));
        assert!(sql.contains("category                 TEXT        NOT NULL"));
        assert!(sql.contains("descriptions             JSONB"));
        assert!(sql.contains("key_variables            JSONB"));
        assert!(sql.contains("sdtm_project_domains_updated_at"));
        // The DDL is rewritten in place, never amended.
        assert!(!sql.contains("ALTER TABLE"));
    }
}
