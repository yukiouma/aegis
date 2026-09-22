use chrono::{DateTime, Utc};

use crate::domain::{ModelVersion, Project, ProjectConfiguration, ProjectTag, UserSummary};

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

/// Server-side projection of a single `ModelVersion` pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelVersionView {
    pub version_id: i64,
    pub version_name: String,
}

impl From<ModelVersion> for ModelVersionView {
    fn from(v: ModelVersion) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Server-side projection of the project's configuration: an
/// optional locale, the tag list, and the optional SDTM-IG version
/// pointer. Mirrors the apis `ProjectConfigurationView` so the
/// facade `From` impl is a straight rename.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectConfigurationView {
    pub language: Option<crate::domain::ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
}

impl From<ProjectConfiguration> for ProjectConfigurationView {
    fn from(c: ProjectConfiguration) -> Self {
        Self {
            language: c.language,
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
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
    pub configurations: ProjectConfigurationView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectView {
    /// Build the view around a domain `Project`. Membership lists
    /// must already be hydrated to `ProjectMemberView` (look up user
    /// summaries before calling). Configuration passes through as
    /// `ProjectConfigurationView`.
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
            configurations: project.configurations.into(),
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}