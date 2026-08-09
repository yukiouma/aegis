use async_trait::async_trait;

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub code: String,
    pub name: String,
}

/// Narrow user port that the project crate uses to hydrate membership
/// codes. Implementations adapt `apis::user::UserService` (the only
/// caller is `adapter::service::user::UserServiceImpl`).
#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError>;
}
