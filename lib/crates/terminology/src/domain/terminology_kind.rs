use std::convert::TryFrom;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminologyKind {
    Sdtm,
    Adam,
}

impl TerminologyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TerminologyKind::Sdtm => "sdtm",
            TerminologyKind::Adam => "adam",
        }
    }
}

impl TryFrom<&str> for TerminologyKind {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "sdtm" => Ok(TerminologyKind::Sdtm),
            "adam" => Ok(TerminologyKind::Adam),
            other => Err(DomainError::InvalidKind(other.to_string())),
        }
    }
}
