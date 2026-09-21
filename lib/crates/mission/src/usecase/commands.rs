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

/// Input to `MissionIssueUsecase::create_issue`. The
/// `description` is the only mutable text field after creation;
/// `target_item` is caller-supplied at creation and immutable
/// afterwards.
#[derive(Debug, Clone)]
pub struct CreateIssue {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub description: String,
}
