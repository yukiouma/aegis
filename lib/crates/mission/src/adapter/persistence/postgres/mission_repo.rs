use std::collections::HashMap;
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    AssigneeNew, DomainError, Mission, MissionKind, MissionNew, MissionRepository,
};

use super::map_db_error;
use super::row::{AssigneeRow, MissionRow};

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct MissionRepo {
    pool: PgPool,
}

impl MissionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MissionRepository for MissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        let row: MissionRow = sqlx::QueryBuilder::new(
            "INSERT INTO missions (project_code, mission_kind, mission_code) VALUES (",
        )
        .push_bind(&input.project_code)
        .push(", ")
        .push_bind(input.mission_kind.as_str())
        .push(", ")
        .push_bind(&input.mission_code)
        .push(") RETURNING id, project_code, mission_kind, mission_code, created_at, updated_at")
        .build_query_as::<MissionRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db)
                if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) =>
            {
                DomainError::DuplicateMission {
                    project_code: input.project_code.clone(),
                    mission_kind: input.mission_kind,
                    mission_code: input.mission_code.clone(),
                }
            }
            other => map_db_error(other),
        })?;

        let mission_id = row.id;

        for assignee in &input.assignees {
            insert_assignee(&mut tx, mission_id, assignee).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        self.find_by_id(mission_id).await
    }

    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        let row: MissionRow = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<MissionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;

        let assignees = load_assignees(&self.pool, id).await?;
        Mission::try_from((row, assignees)).map_err(Into::into)
    }

    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE project_code = ",
        );
        qb.push_bind(project_code);
        if let Some(k) = kind {
            qb.push(" AND mission_kind = ").push_bind(k.as_str());
        }
        qb.push(" ORDER BY id ASC");
        let rows: Vec<MissionRow> = qb
            .build_query_as::<MissionRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        load_missions_with_assignees(&self.pool, rows).await
    }

    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        // First fetch the mission ids that have an assignee with
        // `user_code`, then fetch the missions + their assignees.
        let ids: Vec<i64> = sqlx::QueryBuilder::new(
            "SELECT DISTINCT mission_id FROM assignees WHERE user_code = ",
        )
        .push_bind(user_code)
        .build_query_as::<(i64,)>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|(id,)| id)
        .collect();

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in &ids {
            sep.push_bind(id);
        }
        qb.push(") ORDER BY id ASC");
        let rows: Vec<MissionRow> = qb
            .build_query_as::<MissionRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        load_missions_with_assignees(&self.pool, rows).await
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM missions WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }
}

async fn insert_assignee(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    mission_id: i64,
    assignee: &AssigneeNew,
) -> Result<(), DomainError> {
    let _: AssigneeRow = sqlx::QueryBuilder::new(
        "INSERT INTO assignees (mission_id, user_code, role) VALUES (",
    )
    .push_bind(mission_id)
    .push(", ")
    .push_bind(&assignee.user_code)
    .push(", ")
    .push_bind(assignee.role.as_str())
    .push(") RETURNING id, mission_id, user_code, role, created_at, updated_at")
    .build_query_as::<AssigneeRow>()
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db)
            if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) =>
        {
            DomainError::DuplicateAssignee {
                mission_id,
                user_code: assignee.user_code.clone(),
                role: assignee.role,
            }
        }
        other => map_db_error(other),
    })?;
    Ok(())
}

async fn load_assignees(pool: &PgPool, mission_id: i64) -> Result<Vec<AssigneeRow>, DomainError> {
    let rows: Vec<AssigneeRow> = sqlx::QueryBuilder::new(
        "SELECT id, mission_id, user_code, role, created_at, updated_at \
         FROM assignees WHERE mission_id = ",
    )
    .push_bind(mission_id)
    .push(" ORDER BY id ASC")
    .build_query_as::<AssigneeRow>()
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    Ok(rows)
}

async fn load_missions_with_assignees(
    pool: &PgPool,
    rows: Vec<MissionRow>,
) -> Result<Vec<Mission>, DomainError> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    let mut qb = sqlx::QueryBuilder::new(
        "SELECT id, mission_id, user_code, role, created_at, updated_at \
         FROM assignees WHERE mission_id IN (",
    );
    let mut sep = qb.separated(", ");
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    for id in &ids {
        sep.push_bind(id);
    }
    qb.push(") ORDER BY mission_id ASC, id ASC");
    let assignee_rows: Vec<AssigneeRow> = qb
        .build_query_as::<AssigneeRow>()
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;

    let mut by_mission: HashMap<i64, Vec<AssigneeRow>> = HashMap::new();
    for a in assignee_rows {
        by_mission.entry(a.mission_id).or_default().push(a);
    }

    rows.into_iter()
        .map(|row| {
            let assignees = by_mission.remove(&row.id).unwrap_or_default();
            Mission::try_from((row, assignees)).map_err(Into::into)
        })
        .collect()
}