use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::domain::{DomainError, IssueComment, IssueState, MissionIssue, MissionIssueNew};

use super::map_db_error;
use super::row::IssueRow;

pub struct IssueRepo {
    pool: PgPool,
}

impl IssueRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::MissionIssueRepository for IssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "INSERT INTO mission_issues \
             (mission_id, target_item, issuer, description, state, comments) \
             VALUES (",
        )
        .push_bind(input.mission_id)
        .push(", ")
        .push_bind(&input.target_item)
        .push(", ")
        .push_bind(&input.issuer)
        .push(", ")
        .push_bind(&input.description)
        .push(
            ", 'opened', '[]'::jsonb) \
             RETURNING id, mission_id, target_item, issuer, description, state, \
                       comments, created_at, updated_at",
        )
        .build_query_as::<IssueRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        MissionIssue::try_from(row)
    }

    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "SELECT id, mission_id, target_item, issuer, description, state, \
                    comments, created_at, updated_at \
             FROM mission_issues WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, mission_id, target_item, issuer, description, state, \
                    comments, created_at, updated_at \
             FROM mission_issues WHERE mission_id = ",
        );
        qb.push_bind(mission_id);
        if let Some(s) = state {
            qb.push(" AND state = ").push_bind(s.as_str());
        }
        qb.push(" ORDER BY id ASC");
        let rows: Vec<IssueRow> = qb
            .build_query_as::<IssueRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        rows.into_iter().map(MissionIssue::try_from).collect()
    }

    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET state = 'closed', updated_at = NOW() \
             WHERE id = ",
        )
        .push_bind(id)
        .push(
            " RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at",
        )
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET state = 'opened', updated_at = NOW() \
             WHERE id = ",
        )
        .push_bind(id)
        .push(
            " RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at",
        )
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new("UPDATE mission_issues SET description = ")
            .push_bind(&description)
            .push(", updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(
                " RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at",
            )
            .build_query_as::<IssueRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError> {
        // Read-modify-write: load the existing row, append the
        // new comment to its `comments: Vec<_>` in Rust, and
        // write the whole array back as a single jsonb column.
        // We do NOT use Postgres's `jsonb || jsonb` operator
        // because that performs a shallow merge and would
        // silently drop entries under index drift.
        let existing = self.find_by_id(id).await?;
        let mut comments = existing.comments;
        comments.push(comment);
        let row: IssueRow = sqlx::QueryBuilder::new("UPDATE mission_issues SET comments = ")
            .push_bind(Json(&comments))
            .push("::jsonb, updated_at = NOW() WHERE id = ")
            .push_bind(id)
            .push(
                " RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at",
            )
            .build_query_as::<IssueRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }
}
