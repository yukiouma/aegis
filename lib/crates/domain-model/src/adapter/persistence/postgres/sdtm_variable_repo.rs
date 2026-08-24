use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{AssertSqlSafe, FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmRole, SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableNew, SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate,
};

#[derive(FromRow)]
struct SdtmVariableRow {
    id: i64,
    domain_id: i64,
    name: String,
    variable_controlled: Option<String>,
    variable_type: String,
    variable_core: String,
    variable_role: Option<String>,
    variable_sequence: i64,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmVariableRow {
    fn into_var(self) -> Result<SdtmVariable, DomainError> {
        let variable_type = SdtmVariableType::try_from(self.variable_type.as_str())?;
        let variable_core = SdtmVariableCore::try_from(self.variable_core.as_str())?;
        let variable_role = match self.variable_role.as_deref() {
            None => None,
            Some(s) => Some(SdtmRole::try_from(s)?),
        };
        let descriptions: Vec<SdtmVariableDescription> = serde_json::from_value(self.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmVariable::for_repository(
            self.id,
            self.domain_id,
            self.name,
            self.variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            self.variable_sequence,
            descriptions,
            self.created_at,
            self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmVariableRepoPg {
    pool: PgPool,
}

impl SdtmVariableRepoPg {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl SdtmVariableRepository for SdtmVariableRepoPg {
    async fn create(&self, input: SdtmVariableNew) -> Result<SdtmVariable, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let type_str = input.variable_type.as_str();
        let core_str = input.variable_core.as_str();
        let role_str = input.variable_role.map(|r| r.as_str());
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "INSERT INTO sdtm_variables
                (domain_id, name, variable_controlled, variable_type,
                 variable_core, variable_role, variable_sequence,
                 descriptions)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, domain_id, name, variable_controlled,
                       variable_type, variable_core, variable_role,
                       variable_sequence, descriptions,
                       created_at, updated_at",
        )
        .bind(input.domain_id)
        .bind(&input.name)
        .bind(&input.variable_controlled)
        .bind(type_str)
        .bind(core_str)
        .bind(role_str)
        .bind(input.variable_sequence)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(map_variable_err)?;
        row.into_var()
    }

    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError> {
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_variable_err)?
        .ok_or(DomainError::SdtmVariableNotFound(id))?;
        row.into_var()
    }

    async fn list_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmVariable>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables
             WHERE domain_id = $1
             ORDER BY variable_sequence ASC, id ASC",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_variable_err)?;
        rows.into_iter().map(SdtmVariableRow::into_var).collect()
    }

    async fn update(&self, input: SdtmVariableUpdate) -> Result<SdtmVariable, DomainError> {
        // For nullable columns (variable_controlled, variable_role) the
        // three-state semantics are:
        //   - outer None       -> column unchanged
        //   - outer Some(None) -> column cleared to NULL
        // We translate that to SQL via dynamic fragments that select
        // between `column = $bind` (for "clear") and
        // `column = COALESCE($bind, column)` (for "don't change").

        let name = input.name.as_deref();
        let variable_type = input.variable_type.map(|t| t.as_str());
        let variable_core = input.variable_core.map(|c| c.as_str());
        let variable_sequence = input.variable_sequence;

        let variable_controlled_bound: Option<&str> = match &input.variable_controlled {
            None => None,
            Some(None) => None,
            Some(Some(s)) => Some(s.as_str()),
        };
        let clear_controlled = input.variable_controlled.is_some();

        let variable_role_bound: Option<&str> = match &input.variable_role {
            None => None,
            Some(None) => None,
            Some(Some(r)) => Some(r.as_str()),
        };
        let clear_role = input.variable_role.is_some();

        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => {
                Some(serde_json::to_value(v).map_err(|e| DomainError::Repository(e.to_string()))?)
            }
        };

        let ctrl_expr = if clear_controlled {
            "$7".to_string()
        } else {
            "COALESCE($7, variable_controlled)".to_string()
        };
        let role_expr = if clear_role {
            "$8".to_string()
        } else {
            "COALESCE($8, variable_role)".to_string()
        };

        let sql = format!(
            "UPDATE sdtm_variables SET
                name                = COALESCE($2, name),
                variable_type       = COALESCE($3, variable_type),
                variable_core       = COALESCE($4, variable_core),
                variable_sequence   = COALESCE($5, variable_sequence),
                descriptions        = COALESCE($6, descriptions),
                variable_controlled = {ctrl},
                variable_role       = {role}
             WHERE id = $1
             RETURNING id, domain_id, name, variable_controlled,
                       variable_type, variable_core, variable_role,
                       variable_sequence, descriptions,
                       created_at, updated_at",
            ctrl = ctrl_expr,
            role = role_expr,
        );

        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(AssertSqlSafe(sql))
            .bind(input.id)
            .bind(name)
            .bind(variable_type)
            .bind(variable_core)
            .bind(variable_sequence)
            .bind(descriptions_json)
            .bind(variable_controlled_bound)
            .bind(variable_role_bound)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_variable_err)?
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        row.into_var()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_variable_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

fn map_variable_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmVariable {
                    domain_id: 0,
                    name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                return DomainError::FkSdtmDomainNotFound(0);
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
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/0003_create_sdtm_variables.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_variables"));
        assert!(sql.contains("descriptions         JSONB"));
        assert!(sql.contains("sdtm_variables_type_check"));
        assert!(sql.contains("sdtm_variables_core_check"));
        assert!(sql.contains("sdtm_variables_role_check"));
    }
}
