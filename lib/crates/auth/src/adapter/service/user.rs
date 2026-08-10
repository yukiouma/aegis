use std::sync::Arc;

use async_trait::async_trait;

use crate::domain::{DomainError, Role, UserService, UserSummary};
use apis::user::{CreateUserRequest, Role as ApiRole, UserApiError, UserService as ApiUserService};

/// Adapter that implements the domain `UserService` port on top of
/// the apis `UserService`. Delegates `get_by_code` to the inner apis
/// implementation and translates the apis types into the domain
/// equivalents.
pub struct UserServiceImpl {
    inner: Arc<dyn ApiUserService>,
}

impl UserServiceImpl {
    pub fn new(inner: Arc<dyn ApiUserService>) -> Self {
        Self { inner }
    }
}

fn map_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

fn map_error(err: UserApiError) -> DomainError {
    match err {
        UserApiError::NotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn create(&self, code: &str, name: &str) -> Result<UserSummary, DomainError> {
        let view = self
            .inner
            .create(CreateUserRequest {
                code: code.to_owned(),
                name: name.to_owned(),
                role: ApiRole::General,
                active: false,
            })
            .await
            .map_err(map_error)?;
        Ok(UserSummary {
            code: view.code,
            active: view.active,
            role: map_role(view.role),
        })
    }

    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        let view = self.inner.get_by_code(code).await.map_err(map_error)?;
        Ok(UserSummary {
            code: view.code,
            active: view.active,
            role: map_role(view.role),
        })
    }
}
