use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// One entry in `MissionIssue.comments`. Append-only — the API
/// surface never needs to address a single comment, only the
/// issue aggregate. `created_at` is set at the adapter boundary
/// from `NOW()` on insert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueComment {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl IssueComment {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs.
    pub fn new(
        user: String,
        content: String,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if user.trim().is_empty() {
            return Err(DomainError::EmptyUserCode);
        }
        if content.trim().is_empty() {
            return Err(DomainError::EmptyCommentContent);
        }
        Ok(Self {
            user,
            content,
            created_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(user: String, content: String, created_at: DateTime<Utc>) -> Self {
        Self {
            user,
            content,
            created_at,
        }
    }
}
