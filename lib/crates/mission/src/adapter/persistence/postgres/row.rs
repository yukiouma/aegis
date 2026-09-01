use std::convert::TryFrom;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{Assignee, DomainError, Mission, MissionKind, MissionRole};

/// Raw row from `missions`. `mission_kind` is read as TEXT and
/// parsed via `MissionKind::from_str` so the DB CHECK is the
/// belt-and-braces against out-of-band inserts.
#[derive(FromRow)]
pub(crate) struct MissionRow {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: String,
    pub mission_code: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw row from `assignees`. `role` is read as TEXT and parsed
/// via `MissionRole::from_str`.
#[derive(FromRow)]
pub(crate) struct AssigneeRow {
    pub id: i64,
    pub mission_id: i64,
    pub user_code: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<(MissionRow, Vec<AssigneeRow>)> for Mission {
    type Error = DomainError;
    fn try_from((row, assignees): (MissionRow, Vec<AssigneeRow>)) -> Result<Self, Self::Error> {
        let mission_kind = MissionKind::from_str(&row.mission_kind)?;
        let assignees = assignees
            .into_iter()
            .map(Assignee::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Mission::for_repository(
            row.id,
            row.project_code,
            mission_kind,
            row.mission_code,
            assignees,
            row.created_at,
            row.updated_at,
        ))
    }
}

impl TryFrom<AssigneeRow> for Assignee {
    type Error = DomainError;
    fn try_from(row: AssigneeRow) -> Result<Self, Self::Error> {
        let role = MissionRole::from_str(&row.role)?;
        Ok(Assignee::for_repository(
            row.id,
            row.user_code,
            role,
            row.created_at,
            row.updated_at,
        ))
    }
}
