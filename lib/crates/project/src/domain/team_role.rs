use std::convert::TryFrom;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamType {
    Members,
    UnblindMembers,
}

impl TeamType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TeamType::Members => "members",
            TeamType::UnblindMembers => "unblind_members",
        }
    }
}

impl TryFrom<&str> for TeamType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "members" => Ok(TeamType::Members),
            "unblind_members" => Ok(TeamType::UnblindMembers),
            other => Err(DomainError::UnknownTeamType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleType {
    Leader,
    Worker,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleType::Leader => "leader",
            RoleType::Worker => "worker",
        }
    }
}

impl TryFrom<&str> for RoleType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "leader" => Ok(RoleType::Leader),
            "worker" => Ok(RoleType::Worker),
            other => Err(DomainError::UnknownRoleType(other.to_string())),
        }
    }
}
