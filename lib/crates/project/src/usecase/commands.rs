use crate::domain::{ProjectMember, ProjectTag};

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    pub tags: Option<Vec<ProjectTag>>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProject {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// `None` = leave tags unchanged; `Some(vec)` = whole-list replace.
    pub tags: Option<Vec<ProjectTag>>,
}