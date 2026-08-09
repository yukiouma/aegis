use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{UserApiError, UserService as ApiUserService, UserView};

use crate::domain::{DomainError, UserService, UserSummary};

/// Adapter that maps the apis `UserService` port onto the narrow
/// domain `UserService` port. The project crate never reaches apis
/// `user` types directly; everything flows through this struct so the
/// domain layer stays free of `apis` references.
pub struct UserServiceImpl {
    inner: Arc<dyn ApiUserService>,
}

impl UserServiceImpl {
    pub fn new(inner: Arc<dyn ApiUserService>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        let view = self.inner.get_by_code(code).await.map_err(map_error)?;
        Ok(to_summary(view))
    }

    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        let views = self.inner.list().await.map_err(map_error)?;
        Ok(views.into_iter().map(to_summary).collect())
    }
}

fn to_summary(v: UserView) -> UserSummary {
    UserSummary {
        code: v.code,
        name: v.name,
    }
}

fn map_error(err: UserApiError) -> DomainError {
    match err {
        UserApiError::NotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}
