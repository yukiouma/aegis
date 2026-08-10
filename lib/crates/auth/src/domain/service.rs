//! Outbound port: look up a user's `active` state and `role` by code.
//!
//! `domain` defines this trait so the `usecase` layer never has to
//! reach into `apis::user::UserService` for these two facts. The
//! concrete adapter lives in
//! `crate::adapter::service::user::UserServiceImpl` and delegates
//! to `apis::user::UserService`.

use async_trait::async_trait;

use super::{DomainError, Role};

/// Minimal projection of a user — just the fields the auth usecase
/// needs to decide whether to mint tokens for `code`. The full user
/// record (id, name, timestamps, etc.) stays on the apis side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub code: String,
    pub active: bool,
    pub role: Role,
}

#[async_trait]
pub trait UserService: Send + Sync {
    async fn create(&self, code: &str, name: &str) -> Result<UserSummary, DomainError>;

    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
}
