use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueState {
    Opened,
    Closed,
}

impl IssueState {
    pub const fn as_str(self) -> &'static str {
        match self {
            IssueState::Opened => "opened",
            IssueState::Closed => "closed",
        }
    }
}

impl FromStr for IssueState {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "opened" => Ok(IssueState::Opened),
            "closed" => Ok(IssueState::Closed),
            other => Err(DomainError::UnknownMissionRole(other.to_string())),
        }
    }
}

impl TryFrom<&str> for IssueState {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
