use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Locale hint attached to a project configuration. Currently just
/// English (`"en"`) and Simplified Chinese (`"zh-CN"`); new locales
/// land here as variants. Stored as an enum so the wire code is
/// one of two well-known strings; an `Option<ProjectLanguage>` is
/// the way to model "no language configured".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    English,
    SimplifiedChinese,
}

impl ProjectLanguage {
    /// Wire-level code. Stable; the consumer UI maps this to the
    /// `Locale` value it already knows.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::English => "en",
            ProjectLanguage::SimplifiedChinese => "zh-CN",
        }
    }
}

impl TryFrom<&str> for ProjectLanguage {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "en" => Ok(ProjectLanguage::English),
            "zh-CN" => Ok(ProjectLanguage::SimplifiedChinese),
            other => Err(DomainError::UnknownLanguage(other.to_string())),
        }
    }
}