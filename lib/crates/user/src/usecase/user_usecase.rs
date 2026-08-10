use chrono::{DateTime, Utc};

use crate::domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};

use super::commands::{CreateUser, UpdateUser};
use super::error::UsecaseError;

/// Safe projection of `User` returned by the usecase layer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            code: user.code,
            name: user.name,
            role: user.role,
            active: user.active,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

/// Async orchestration for user lifecycle operations.
///
/// The usecase projects every `User` it returns into a [`UserView`]
/// so the persistence boundary stays consistent.
pub struct UserUsecase<R: UserRepository> {
    repository: R,
}

impl<R: UserRepository> UserUsecase<R> {
    /// Build a new `UserUsecase` wrapping the supplied `repository`.
    ///
    /// `repository` is the persistence port; pass
    /// [`UserRepo`](crate::UserRepo) for the PostgreSQL-backed
    /// implementation.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Validate the inputs and persist a new user. Returns the
    /// resulting user as a [`UserView`].
    pub async fn create(&self, cmd: CreateUser) -> Result<UserView, UsecaseError> {
        validate_create(&cmd)?;

        let input = UserNew {
            code: cmd.code,
            name: cmd.name,
            role: cmd.role,
            active: cmd.active,
        };

        let user = self.repository.create(input).await?;
        Ok(user.into())
    }

    /// Look up a user by numeric id and project the result into a
    /// [`UserView`]. Returns [`UsecaseError::Repository`] wrapping
    /// [`DomainError::NotFound`] if the id is unknown.
    pub async fn get_by_id(&self, id: i32) -> Result<UserView, UsecaseError> {
        let user = self.repository.find_by_id(id).await?;
        Ok(user.into())
    }

    /// Look up a user by their unique `code`. The code is validated
    /// for non-emptiness before the repository is touched.
    pub async fn get_by_code(&self, code: &str) -> Result<UserView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let user = self.repository.find_by_code(code).await?;
        Ok(user.into())
    }

    /// List every user as a `Vec<UserView>`. There is no pagination
    /// yet; the full collection is returned.
    pub async fn list(&self) -> Result<Vec<UserView>, UsecaseError> {
        let users = self.repository.list().await?;
        Ok(users.into_iter().map(UserView::from).collect())
    }

    /// Apply the optional fields on `cmd` to the user identified by
    /// `cmd.id`.
    pub async fn update(&self, cmd: UpdateUser) -> Result<UserView, UsecaseError> {
        validate_update(&cmd)?;

        let input = UserUpdate {
            id: cmd.id,
            code: cmd.code,
            name: cmd.name,
            role: cmd.role,
            active: cmd.active,
        };

        let user = self.repository.update(input).await?;
        Ok(user.into())
    }
}

/// Reject empty / whitespace-only `code` and `name` before any
/// repository call.
fn validate_create(cmd: &CreateUser) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update(cmd: &UpdateUser) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}
