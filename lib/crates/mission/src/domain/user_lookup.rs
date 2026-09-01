use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port for user existence checks. Adapted to
/// `apis::user::UserService` by `adapter::service::user::UserLookupImpl`.
#[async_trait]
pub trait UserLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
