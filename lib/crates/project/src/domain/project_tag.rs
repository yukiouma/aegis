use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Wire-shape value object persisted inside `projects.tags`.
///
/// Two string fields, both required and non-empty after trim.
/// Duplicate keys within the same project are intentionally allowed —
/// the same key may carry multiple distinct values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTag {
    pub key: String,
    pub value: String,
}

impl ProjectTag {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects empty / whitespace `key` and `value`.
    pub fn new(key: String, value: String) -> Result<Self, DomainError> {
        if key.trim().is_empty() {
            return Err(DomainError::EmptyTagKey);
        }
        if value.trim().is_empty() {
            return Err(DomainError::EmptyTagValue);
        }
        Ok(Self { key, value })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column.
    #[allow(dead_code)]
    pub(crate) fn for_repository(key: String, value: String) -> Self {
        Self { key, value }
    }
}