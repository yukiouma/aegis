use crate::domain::ProjectMember;

#[derive(Debug, Clone)]
pub struct CreateProduct {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProduct {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProject {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}
