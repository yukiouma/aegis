use chrono::{DateTime, Utc};

use crate::domain::{Assignee, Mission};

/// Projection of `Mission` returned by the usecase to the facade.
/// The facade converts this into `apis::mission::MissionView` via
/// `From` impls in the facade module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionView {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: crate::domain::MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssigneeView {
    pub id: i64,
    pub user_code: String,
    pub role: crate::domain::MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Mission> for MissionView {
    fn from(m: Mission) -> Self {
        MissionView {
            id: m.id,
            project_code: m.project_code,
            mission_kind: m.mission_kind,
            mission_code: m.mission_code,
            assignees: m.assignees.into_iter().map(Into::into).collect(),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

impl From<Assignee> for AssigneeView {
    fn from(a: Assignee) -> Self {
        AssigneeView {
            id: a.id,
            user_code: a.user_code,
            role: a.role,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
