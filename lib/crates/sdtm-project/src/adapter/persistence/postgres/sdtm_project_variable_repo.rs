// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
//
// The variable is a complete copy: it carries its own type, core,
// role, controlled terminology and descriptions, so there is no join
// and no separate override row to hydrate.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    ConversionRule, DomainError, SdtmProjectVariable, SdtmProjectVariableNew,
    SdtmProjectVariableRepository, SdtmProjectVariableUpdate, SdtmRole, SdtmVariableControlled,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableType,
};

#[derive(FromRow)]
struct SdtmProjectVariableRow {
    id: i64,
    sdtm_project_domain_id: i64,
    variable_code: String,
    variable_controlled: Option<JsonValue>,
    variable_type: String,
    variable_core: String,
    variable_role: Option<String>,
    variable_sequence: i64,
    descriptions: JsonValue,
    conversion_rules: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmProjectVariableRow {
    fn into_variable(self) -> Result<SdtmProjectVariable, DomainError> {
        let variable_type = SdtmVariableType::try_from(self.variable_type.as_str())?;
        let variable_core = SdtmVariableCore::try_from(self.variable_core.as_str())?;
        let variable_role = match self.variable_role.as_deref() {
            None => None,
            Some(s) => Some(SdtmRole::try_from(s)?),
        };
        // A SQL NULL arrives as `None`, so "no controlled terminology"
        // stays `None` rather than becoming an empty structure.
        let variable_controlled: Option<SdtmVariableControlled> = match self.variable_controlled {
            None => None,
            Some(v) => Some(
                serde_json::from_value(v).map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };
        let descriptions: Vec<SdtmVariableDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let conversion_rules: Vec<ConversionRule> =
            serde_json::from_value(self.conversion_rules)
                .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmProjectVariable::for_repository(
            self.id,
            self.sdtm_project_domain_id,
            self.variable_code,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            self.variable_sequence,
            descriptions,
            conversion_rules,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmProjectVariableRepoPg {
    pool: PgPool,
}

impl SdtmProjectVariableRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmProjectVariableRepository for SdtmProjectVariableRepoPg {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError> {
        let controlled_json = match &input.variable_controlled {
            None => None,
            Some(c) => {
                Some(serde_json::to_value(c).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let conversion_rules_json = serde_json::to_value(&input.conversion_rules)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "INSERT INTO sdtm_project_variables
                (sdtm_project_domain_id, variable_code, variable_controlled, variable_type,
                 variable_core, variable_role, variable_sequence, descriptions, conversion_rules)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             RETURNING id, sdtm_project_domain_id, variable_code, variable_controlled,
                       variable_type, variable_core, variable_role, variable_sequence,
                       descriptions, conversion_rules, created_at, updated_at",
        )
        .bind(input.sdtm_project_domain_id)
        .bind(&input.variable_code)
        .bind(controlled_json)
        .bind(input.variable_type.as_str())
        .bind(input.variable_core.as_str())
        .bind(input.variable_role.map(|r| r.as_str()))
        .bind(input.variable_sequence)
        .bind(descriptions_json)
        .bind(conversion_rules_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, input.sdtm_project_domain_id))?;
        row.into_variable()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError> {
        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "SELECT id, sdtm_project_domain_id, variable_code, variable_controlled,
                    variable_type, variable_core, variable_role, variable_sequence,
                    descriptions, conversion_rules, created_at, updated_at
             FROM sdtm_project_variables
             WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?
        .ok_or(DomainError::SdtmProjectVariableNotFound(id))?;
        row.into_variable()
    }

    async fn list_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmProjectVariable>, DomainError> {
        let rows: Vec<SdtmProjectVariableRow> = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "SELECT id, sdtm_project_domain_id, variable_code, variable_controlled,
                    variable_type, variable_core, variable_role, variable_sequence,
                    descriptions, conversion_rules, created_at, updated_at
             FROM sdtm_project_variables
             WHERE sdtm_project_domain_id = $1
             ORDER BY variable_sequence ASC, id ASC",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?;
        rows.into_iter()
            .map(SdtmProjectVariableRow::into_variable)
            .collect()
    }

    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError> {
        // COALESCE: a NULL argument leaves the column unchanged.
        //
        // For the two nullable columns the three-state semantics are:
        //   - outer None       -> column unchanged
        //   - outer Some(None) -> column cleared to NULL
        //   - outer Some(Some) -> column replaced
        // A `CASE WHEN` with a bound flag distinguishes the three,
        // because `COALESCE` cannot tell "clear" from "leave alone".
        let controlled_json = match &input.variable_controlled {
            // Unchanged: the CASE expression never reads this
            // parameter, so binding NULL here is harmless.
            None => None,
            // Clear: bind SQL NULL. The JSON literal `null` would be
            // stored as a JSONB value, and the row would then fail to
            // deserialise on the way back.
            Some(None) => None,
            // Replace: bind the structure.
            Some(Some(c)) => {
                Some(serde_json::to_value(c).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };
        let conversion_rules_json = match &input.conversion_rules {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };

        let row: SdtmProjectVariableRow = sqlx::query_as::<_, SdtmProjectVariableRow>(
            "UPDATE sdtm_project_variables
             SET variable_code      = COALESCE($2, variable_code),
                 variable_sequence  = COALESCE($3, variable_sequence),
                 variable_type      = COALESCE($4, variable_type),
                 variable_core      = COALESCE($5, variable_core),
                 descriptions       = COALESCE($6, descriptions),
                 conversion_rules   = COALESCE($7, conversion_rules),
                 variable_role      = CASE WHEN $8 THEN $9::text ELSE variable_role END,
                 variable_controlled = CASE WHEN $10 THEN $11::jsonb ELSE variable_controlled END
             WHERE id = $1
             RETURNING id, sdtm_project_domain_id, variable_code, variable_controlled,
                       variable_type, variable_core, variable_role, variable_sequence,
                       descriptions, conversion_rules, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.variable_code.as_deref())
        .bind(input.variable_sequence)
        .bind(input.variable_type.map(|t| t.as_str()))
        .bind(input.variable_core.map(|c| c.as_str()))
        .bind(descriptions_json)
        .bind(conversion_rules_json)
        .bind(input.variable_role.is_some())
        .bind(input.variable_role.flatten().map(|r| r.as_str()))
        .bind(input.variable_controlled.is_some())
        .bind(controlled_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| map_variable_err(e, 0))?
        .ok_or(DomainError::SdtmProjectVariableNotFound(input.id))?;
        row.into_variable()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_project_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| map_variable_err(e, 0))?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmProjectVariableNotFound(id));
        }
        Ok(())
    }
}

/// Map a driver error to a domain error.
///
/// `parent_domain_id` is the domain the statement referenced. A
/// foreign-key violation can only arise from an INSERT, so the read
/// and update paths pass 0; the create path passes the id the caller
/// actually used.
fn map_variable_err(err: sqlx::Error, parent_domain_id: i64) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // Unique violation on (sdtm_project_domain_id,
                // variable_code). The driver does not surface the
                // natural key.
                return DomainError::DuplicateSdtmProjectVariable {
                    domain_id: 0,
                    variable_code: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: the parent domain is gone. This is
                // the backstop for the race where the domain is
                // deleted between the usecase's check and the insert.
                return DomainError::FkSdtmProjectDomainNotFound(parent_domain_id);
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
    fn migration_file_declares_variable_table() {
        let sql =
            include_str!("../../../../migrations/20261103135414_create_sdtm_project_variables.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_project_variables"));
        assert!(sql.contains("sdtm_project_domain_id  BIGINT"));
        assert!(sql.contains(
            "CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (sdtm_project_domain_id, variable_code)"
        ));
        assert!(sql.contains("variable_controlled      JSONB"));
        assert!(sql.contains("conversion_rules         JSONB"));
        assert!(sql.contains("sdtm_project_variables_updated_at"));
        // The DDL is rewritten in place, never amended.
        assert!(!sql.contains("ALTER TABLE"));
    }
}
