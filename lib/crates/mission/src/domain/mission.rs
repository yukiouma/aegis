use chrono::{DateTime, Utc};

use super::assignee::Assignee;
use super::error::DomainError;
use super::mission_kind::MissionKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<Assignee>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Mission {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs. The assignee list is not
    /// validated here — uniqueness of `(user_code, role)` within
    /// a mission is the usecase's job.
    pub fn new(
        id: i64,
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
        assignees: Vec<Assignee>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if mission_code.trim().is_empty() {
            return Err(DomainError::EmptyMissionCode);
        }
        Ok(Self {
            id,
            project_code,
            mission_kind,
            mission_code,
            assignees,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
        assignees: Vec<Assignee>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_code,
            mission_kind,
            mission_code,
            assignees,
            created_at,
            updated_at,
        }
    }
}
