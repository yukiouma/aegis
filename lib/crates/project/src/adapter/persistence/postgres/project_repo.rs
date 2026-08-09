use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectUpdate, RoleType,
    TeamType,
};

use super::row::{ProjectMemberRow, ProjectRow};

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct ProjectRepo {
    pool: PgPool,
}

impl ProjectRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepository for ProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        let row: ProjectRow = sqlx::QueryBuilder::new(
            "INSERT INTO projects (code, description, product_id, active) VALUES ",
        )
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.description)
        .push(", ")
        .push_bind(input.product_id)
        .push(", ")
        .push_bind(true)
        .push(" RETURNING id, code, description, product_id, active, created_at, updated_at")
        .build_query_as::<ProjectRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let project_id = row.id;

        if let Some(ref members) = input.members {
            insert_membership(&mut tx, project_id, TeamType::Members, members).await?;
        }
        if let Some(ref members) = input.unblind_members {
            insert_membership(&mut tx, project_id, TeamType::UnblindMembers, members).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        // Reload so the membership rows are read back into the
        // returned `Project`.
        self.find_by_id(project_id).await
    }

    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        let row: ProjectRow = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProjectRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        let mut project: Project = row.try_into()?;
        let (members, unblind) = load_membership(&self.pool, id).await?;
        project.members = members;
        project.unblind_members = unblind;
        Ok(project)
    }

    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        let row: ProjectRow = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<ProjectRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        let mut project: Project = row.try_into()?;
        let project_id = project.id;
        let (members, unblind) = load_membership(&self.pool, project_id).await?;
        project.members = members;
        project.unblind_members = unblind;
        Ok(project)
    }

    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        let rows: Vec<ProjectRow> = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects ORDER BY id",
        )
        .build_query_as::<ProjectRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut project: Project = row.try_into()?;
            let (members, unblind) = load_membership(&self.pool, project.id).await?;
            project.members = members;
            project.unblind_members = unblind;
            out.push(project);
        }
        Ok(out)
    }

    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // Apply metadata first. If the metadata update fails we never
        // touch membership.
        let mut qb = sqlx::QueryBuilder::new("UPDATE projects SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref c) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(c);
        }
        if let Some(ref d) = input.description {
            sep(&mut qb);
            qb.push("description = ").push_bind(d);
        }
        if let Some(pid) = input.product_id {
            sep(&mut qb);
            qb.push("product_id = ").push_bind(pid);
        }
        if let Some(a) = input.active {
            sep(&mut qb);
            qb.push("active = ").push_bind(a);
        }
        if !first {
            qb.push(" WHERE id = ").push_bind(input.id);
            qb.push(
                " RETURNING id, code, description, product_id, active, created_at, updated_at",
            );
            let row: ProjectRow = qb
                .build_query_as::<ProjectRow>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
                .ok_or(DomainError::NotFound)?;
            let _: Project = row.try_into()?;
        }

        // Replace membership per supplied team. We always delete-then-
        // reinsert so the operation is atomic; `None` leaves that team
        // alone.
        if input.members.is_some() || input.unblind_members.is_some() {
            // Ensure the project exists before we touch membership,
            // otherwise `DELETE` on an unknown id silently succeeds.
            let exists: Option<(i32,)> = sqlx::QueryBuilder::new("SELECT id FROM projects WHERE id = ")
                .push_bind(input.id)
                .build_query_as::<(i32,)>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?;
            if exists.is_none() {
                return Err(DomainError::NotFound);
            }
        }
        if let Some(ref members) = input.members {
            replace_team(&mut tx, input.id, TeamType::Members, members).await?;
        }
        if let Some(ref members) = input.unblind_members {
            replace_team(&mut tx, input.id, TeamType::UnblindMembers, members).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        self.find_by_id(input.id).await
    }
}

async fn insert_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: i32,
    team: TeamType,
    members: &ProjectMember,
) -> Result<(), DomainError> {
    for code in &members.leaders {
        sqlx::query(
            "INSERT INTO project_members (project_id, team_type, role_type, user_code) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(project_id)
        .bind(team.as_str())
        .bind(RoleType::Leader.as_str())
        .bind(code)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    for code in &members.workers {
        sqlx::query(
            "INSERT INTO project_members (project_id, team_type, role_type, user_code) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(project_id)
        .bind(team.as_str())
        .bind(RoleType::Worker.as_str())
        .bind(code)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn replace_team(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: i32,
    team: TeamType,
    members: &ProjectMember,
) -> Result<(), DomainError> {
    sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND team_type = $2")
        .bind(project_id)
        .bind(team.as_str())
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    insert_membership(tx, project_id, team, members).await
}

async fn load_membership(
    pool: &PgPool,
    project_id: i32,
) -> Result<(ProjectMember, ProjectMember), DomainError> {
    let rows: Vec<ProjectMemberRow> = sqlx::QueryBuilder::new(
        "SELECT project_id, team_type, role_type, user_code \
         FROM project_members WHERE project_id = ",
    )
    .push_bind(project_id)
    .build_query_as::<ProjectMemberRow>()
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;

    let mut members = ProjectMember::default();
    let mut unblind = ProjectMember::default();
    for row in rows {
        let team = TeamType::try_from(row.team_type.as_str())?;
        let role = RoleType::try_from(row.role_type.as_str())?;
        let target = match team {
            TeamType::Members => &mut members,
            TeamType::UnblindMembers => &mut unblind,
        };
        match role {
            RoleType::Leader => target.leaders.push(row.user_code),
            RoleType::Worker => target.workers.push(row.user_code),
        }
    }
    // Stable ordering so the returned `Project` matches what the
    // usecase tests expect.
    members.leaders.sort();
    members.workers.sort();
    unblind.leaders.sort();
    unblind.workers.sort();
    Ok((members, unblind))
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
