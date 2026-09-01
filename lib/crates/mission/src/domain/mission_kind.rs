use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl MissionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            MissionKind::Crf => "crf",
            MissionKind::Sdtm => "sdtm",
            MissionKind::Adam => "adam",
            MissionKind::Tfl => "tfl",
        }
    }
}

impl FromStr for MissionKind {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "crf" => Ok(MissionKind::Crf),
            "sdtm" => Ok(MissionKind::Sdtm),
            "adam" => Ok(MissionKind::Adam),
            "tfl" => Ok(MissionKind::Tfl),
            other => Err(DomainError::UnknownMissionKind(other.to_string())),
        }
    }
}

impl TryFrom<&str> for MissionKind {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
