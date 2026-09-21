use chrono::{DateTime, Utc};

use crate::domain::{Assignee, IssueComment, IssueState, Mission, MissionIssue};

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

/// Projection of `MissionIssue` returned by the issue usecase to
/// the facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueCommentView {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl From<MissionIssue> for IssueView {
    fn from(i: MissionIssue) -> Self {
        IssueView {
            id: i.id,
            mission_id: i.mission_id,
            target_item: i.target_item,
            issuer: i.issuer,
            description: i.description,
            state: i.state,
            comments: i.comments.into_iter().map(Into::into).collect(),
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

impl From<IssueComment> for IssueCommentView {
    fn from(c: IssueComment) -> Self {
        IssueCommentView {
            user: c.user,
            content: c.content,
            created_at: c.created_at,
        }
    }
}
