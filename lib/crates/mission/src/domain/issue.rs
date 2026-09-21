use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::issue_comment::IssueComment;
use super::issue_state::IssueState;

/// Mission-scoped issue with an append-only comment thread. The
/// `state` and `comments` fields are not editable through the
/// update path; only `description` is mutable after creation
/// (see `MissionIssueUsecase::update_issue_description`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionIssue {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueComment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MissionIssue {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs. Empty `description` and empty
    /// `issuer` are rejected here.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        mission_id: i64,
        target_item: Option<String>,
        issuer: String,
        description: String,
        state: IssueState,
        comments: Vec<IssueComment>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if description.trim().is_empty() {
            return Err(DomainError::EmptyIssueDescription);
        }
        if issuer.trim().is_empty() {
            return Err(DomainError::EmptyIssueIssuer);
        }
        Ok(Self {
            id,
            mission_id,
            target_item,
            issuer,
            description,
            state,
            comments,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        mission_id: i64,
        target_item: Option<String>,
        issuer: String,
        description: String,
        state: IssueState,
        comments: Vec<IssueComment>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            mission_id,
            target_item,
            issuer,
            description,
            state,
            comments,
            created_at,
            updated_at,
        }
    }
}
