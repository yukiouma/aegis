use async_trait::async_trait;

use super::assignee::Assignee;
use super::error::DomainError;
use super::mission::Mission;
use super::mission_kind::MissionKind;
use super::mission_role::MissionRole;

/// Persistence-input DTO for `MissionRepository::create`. Carries
/// the initial assignee list so the repo can insert both the
/// mission row and its assignee rows inside one transaction. The
/// DB CHECK + UNIQUE on `assignees` is the safety net for the
/// per-mission uniqueness invariant the usecase enforces up
/// front via [`assignees_within_mission_are_unique`].
#[derive(Debug, Clone)]
pub struct MissionNew {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeNew>,
}

/// Persistence-input DTO for `AssigneeRepository::add`
/// (single-row insert used by the standalone `add_assignee` flow).
#[derive(Debug, Clone)]
pub struct AssigneeNew {
    pub user_code: String,
    pub role: MissionRole,
}

/// Check the per-mission `(user_code, role)` uniqueness invariant.
/// Returns `Err(DomainError::DuplicateAssignee { mission_id: 0, … })`
/// on the first duplicate pair — `mission_id` is left at `0` here
/// because the caller has not yet assigned one; the usecase fills
/// it in if it needs a different value.
pub fn assignees_within_mission_are_unique(assignees: &[AssigneeNew]) -> Result<(), DomainError> {
    let mut seen: Vec<(String, MissionRole)> = Vec::with_capacity(assignees.len());
    for a in assignees {
        let pair = (a.user_code.clone(), a.role);
        if seen.contains(&pair) {
            return Err(DomainError::DuplicateAssignee {
                mission_id: 0,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        seen.push(pair);
    }
    Ok(())
}

#[async_trait]
pub trait MissionRepository: Send + Sync {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError>;

    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError>;

    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError>;

    /// Hard delete; cascades to `assignees` via `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

#[async_trait]
pub trait AssigneeRepository: Send + Sync {
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError>;

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError>;
}
