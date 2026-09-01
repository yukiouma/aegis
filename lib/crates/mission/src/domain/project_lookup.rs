use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port for project existence + leadership
/// checks. Adapted to `apis::project::ProjectService` by
/// `adapter::service::project::ProjectLookupImpl`.
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;

    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError>;
}
