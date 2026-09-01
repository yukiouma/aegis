use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{Assignee, AssigneeNew, AssigneeRepository, DomainError};

use super::map_db_error;
use super::row::AssigneeRow;

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct AssigneeRepo {
    pool: PgPool,
}

impl AssigneeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssigneeRepository for AssigneeRepo {
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError> {
        // Mission existence is enforced via FK — the row will fail
        // to insert if `mission_id` does not exist. Map that
        // generic FK violation to `DomainError::NotFound` so the
        // facade surfaces a 404 instead of a 500.
        let row: AssigneeRow =
            sqlx::QueryBuilder::new("INSERT INTO assignees (mission_id, user_code, role) VALUES (")
                .push_bind(mission_id)
                .push(", ")
                .push_bind(&input.user_code)
                .push(", ")
                .push_bind(input.role.as_str())
                .push(") RETURNING id, mission_id, user_code, role, created_at, updated_at")
                .build_query_as::<AssigneeRow>()
                .fetch_one(&self.pool)
                .await
                .map_err(|e| match e {
                    sqlx::Error::Database(ref db)
                        if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) =>
                    {
                        DomainError::DuplicateAssignee {
                            mission_id,
                            user_code: input.user_code.clone(),
                            role: input.role,
                        }
                    }
                    other => map_db_error(other),
                })?;
        Assignee::try_from(row)
    }

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM assignees WHERE mission_id = ")
            .push_bind(mission_id)
            .push(" AND id = ")
            .push_bind(assignee_id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::AssigneeNotFound);
        }
        Ok(())
    }
}
