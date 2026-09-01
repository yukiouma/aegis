use crate::domain::{MissionKind, MissionRole};

#[derive(Debug, Clone)]
pub struct CreateMission {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

#[derive(Debug, Clone)]
pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}
