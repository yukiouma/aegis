//! In-memory `UserService` adapter.
//!
//! Hosts `UserServiceImpl<R>`, the implementation of
//! `apis::user::UserService` that adapts `user::UserUsecase` to the
//! API contract. Behaviour is exercised by `tests`, which wires the
//! adapter on top of an in-memory `UserRepository` so no live
//! PostgreSQL connection is required.

use async_trait::async_trait;

use apis::user::{
    CreateUserRequest, UpdateUserRequest, UserApiError, UserService, UserView,
};
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, Role, UserRepository};
use crate::usecase::{CreateUser, UpdateUser, UsecaseError, UserUsecase};

/// Adapter that implements [`UserService`] on top of a
/// [`UserUsecase`].
///
/// Generic over the persistence port (`R: UserRepository`) so the
/// adapter can be exercised against in-memory fakes in tests and
/// against the PostgreSQL-backed [`UserRepo`](crate::UserRepo) in
/// production. Translation between `apis::user::*` and
/// `user::usecase::*` happens inline in each trait method.
pub struct UserServiceImpl<R: UserRepository> {
    usecase: UserUsecase<R>,
}

impl<R: UserRepository> UserServiceImpl<R> {
    /// Build a new `UserServiceImpl` wrapping the supplied usecase.
    pub fn new(usecase: UserUsecase<R>) -> Self {
        Self { usecase }
    }
}

/// Map the API's `Role` into the domain's `Role`. The two enums
/// share the same three variants; the match is exhaustive and the
/// compiler enforces it on either side.
fn to_internal_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

/// Inverse of [`to_internal_role`].
fn from_internal_role(r: Role) -> ApiRole {
    match r {
        Role::Root => ApiRole::Root,
        Role::Admin => ApiRole::Admin,
        Role::General => ApiRole::General,
    }
}

/// Translate a [`UsecaseError`] into the API's [`UserApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants; the `unreachable!` arm in the
/// `Repository` branch documents that fact and would fire if a
/// future change ever broke the invariant.
impl From<UsecaseError> for UserApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => UserApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => UserApiError::NotFound,
                DomainError::DuplicateCode(code) => UserApiError::DuplicateCode(code),
                DomainError::Repository(msg) => UserApiError::Repository(msg),
                DomainError::EmptyCode
                | DomainError::EmptyName
                | DomainError::InvalidRole(_) => unreachable!(
                    "domain validation errors are only produced as UsecaseError::Validation"
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests;

#[async_trait]
impl<R: UserRepository> UserService for UserServiceImpl<R> {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError> {
        let cmd = CreateUser {
            code: req.code,
            name: req.name,
            role: to_internal_role(req.role),
        };
        let view = self.usecase.create(cmd).await?;
        Ok(UserView {
            id: view.id,
            code: view.code,
            name: view.name,
            role: from_internal_role(view.role),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        })
    }

    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        todo!("implemented in task 5")
    }

    async fn get_by_code(&self, _code: &str) -> Result<UserView, UserApiError> {
        todo!("implemented in task 6")
    }

    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        todo!("implemented in task 7")
    }

    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        todo!("implemented in task 8")
    }
}