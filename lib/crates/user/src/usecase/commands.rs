use crate::domain::Role;

pub struct CreateUser {
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
}

#[derive(Default)]
pub struct UpdateUser {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}
