use std::convert::TryFrom;

use super::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Root,
    Admin,
    General,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Root => "root",
            Role::Admin => "admin",
            Role::General => "general",
        }
    }
}

impl TryFrom<&str> for Role {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "root" => Ok(Role::Root),
            "admin" => Ok(Role::Admin),
            "general" => Ok(Role::General),
            other => Err(DomainError::InvalidRole(other.to_string())),
        }
    }
}
