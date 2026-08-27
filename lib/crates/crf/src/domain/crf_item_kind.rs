use std::fmt;
use std::str::FromStr;

use super::error::DomainError;

/// `CrfItemKind` discriminant — a CRF item can collect text, a
/// selection from options, multi-select checkboxes, a datetime
/// stamp, or a static label.
///
/// Mirrors `apis::crf::CrfItemKind`. The wire form is the
/// PascalCase enum variant name (matches the SQL CHECK).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

impl CrfItemKind {
    /// Wire form (matches the SQL CHECK values).
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "Text",
            Self::Selection => "Selection",
            Self::Checkbox => "Checkbox",
            Self::Datetime => "Datetime",
            Self::Label => "Label",
        }
    }

    /// Parse the wire form. Returns
    /// `DomainError::InvalidCrfItemKind(s)` for unknown inputs.
    pub fn try_from_str(s: &str) -> Result<Self, DomainError> {
        match s {
            "Text" => Ok(Self::Text),
            "Selection" => Ok(Self::Selection),
            "Checkbox" => Ok(Self::Checkbox),
            "Datetime" => Ok(Self::Datetime),
            "Label" => Ok(Self::Label),
            other => Err(DomainError::InvalidCrfItemKind(other.to_string())),
        }
    }

    /// True if this kind requires at least one option to exist
    /// on the item (Selection / Checkbox).
    pub fn requires_options(&self) -> bool {
        matches!(self, Self::Selection | Self::Checkbox)
    }
}

impl fmt::Display for CrfItemKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for CrfItemKind {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::try_from_str(s)
    }
}
