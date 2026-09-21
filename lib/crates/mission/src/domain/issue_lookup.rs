use async_trait::async_trait;

use super::error::DomainError;
use super::issue::MissionIssue;
use super::issue_comment::IssueComment;
use super::issue_state::IssueState;

/// Persistence-input DTO for [`MissionIssueRepository::create`].
/// `mission_id` is a foreign key resolved at the usecase via
/// `MissionRepository::find_by_id` before the repo is called.
#[derive(Debug, Clone)]
pub struct MissionIssueNew {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
}

#[async_trait]
pub trait MissionIssueRepository: Send + Sync {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError>;

    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError>;

    /// Idempotent — succeeds regardless of the prior state.
    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError>;

    /// Idempotent — succeeds regardless of the prior state.
    /// Mirror of [`close`](Self::close).
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError>;

    /// Update the issue's `description`. Allowed regardless of
    /// the current state (a closed issue's description is still
    /// mutable).
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError>;

    /// Append `comment` to `MissionIssue.comments`. The
    /// implementation reads the existing row, appends in Rust,
    /// and writes the whole array back — so the jsonb merge
    /// never goes through the `||` operator (which does a
    /// shallow merge and can drop entries).
    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError>;
}
