use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{Role as ApiRole, UserApiError, UserService, UserView};

use crate::domain::{DomainError, UserLookup};

use super::UserLookupImpl;

#[derive(Clone)]
struct FakeUser;

#[async_trait]
impl UserService for FakeUser {
    async fn create(
        &self,
        _: apis::user::CreateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        Ok(UserView {
            id: 1,
            code: code.into(),
            name: code.into(),
            role: ApiRole::General,
            active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: apis::user::UpdateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

struct MissingUser;

#[async_trait]
impl UserService for MissingUser {
    async fn create(
        &self,
        _: apis::user::CreateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, _: &str) -> Result<UserView, UserApiError> {
        Err(UserApiError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: apis::user::UpdateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn user_lookup_get_by_code_ok() {
    let lookup = UserLookupImpl::new(Arc::new(FakeUser));
    lookup.get_by_code("u1").await.unwrap();
}

#[tokio::test]
async fn user_lookup_get_by_code_missing_maps_error() {
    let lookup = UserLookupImpl::new(Arc::new(MissingUser));
    let err = lookup.get_by_code("ghost").await.unwrap_err();
    assert!(matches!(err, DomainError::UserNotFound(ref c) if c == "ghost"));
}