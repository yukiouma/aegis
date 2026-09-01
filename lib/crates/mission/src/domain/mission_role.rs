use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionRole {
    Dev,
    Qc,
}

impl MissionRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            MissionRole::Dev => "dev",
            MissionRole::Qc => "qc",
        }
    }
}

impl FromStr for MissionRole {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dev" => Ok(MissionRole::Dev),
            "qc" => Ok(MissionRole::Qc),
            other => Err(DomainError::UnknownMissionRole(other.to_string())),
        }
    }
}

impl TryFrom<&str> for MissionRole {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
