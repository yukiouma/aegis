use chrono::{DateTime, Utc};

use crate::domain::{Project, ProjectTag, UserSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

impl From<UserSummary> for UserSummaryView {
    fn from(s: UserSummary) -> Self {
        Self {
            code: s.code,
            name: s.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

impl From<ProjectTag> for TagView {
    fn from(t: ProjectTag) -> Self {
        Self {
            key: t.key,
            value: t.value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub tags: Vec<TagView>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectView {
    /// Build the view around a domain `Project`. Membership lists
    /// must already be hydrated to `ProjectMemberView` (look up user
    /// summaries before calling). Tags pass straight through.
    pub fn from_project(
        project: Project,
        members: ProjectMemberView,
        unblind_members: ProjectMemberView,
    ) -> Self {
        Self {
            id: project.id,
            code: project.code,
            description: project.description,
            members,
            unblind_members,
            tags: project.tags.into_iter().map(Into::into).collect(),
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}