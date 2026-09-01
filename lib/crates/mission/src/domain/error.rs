use thiserror::Error;

use super::mission_kind::MissionKind;
use super::mission_role::MissionRole;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("mission code must not be empty")]
    EmptyMissionCode,

    #[error("user code must not be empty")]
    EmptyUserCode,

    #[error("unknown mission kind: {0}")]
    UnknownMissionKind(String),

    #[error("unknown mission role: {0}")]
    UnknownMissionRole(String),

    #[error("mission not found")]
    NotFound,

    #[error("assignee not found")]
    AssigneeNotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("mission already exists for {project_code}/{mission_kind:?}/{mission_code}")]
    DuplicateMission {
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
    },

    #[error("assignee already exists for mission {mission_id}/{user_code}/{role:?}")]
    DuplicateAssignee {
        mission_id: i64,
        user_code: String,
        role: MissionRole,
    },

    #[error("repository error: {0}")]
    Repository(String),
}
