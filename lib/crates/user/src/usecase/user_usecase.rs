use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHasher};

use crate::domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};

use super::commands::{CreateUser, UpdateUser};
use super::error::UsecaseError;

/// Safe projection of `User` that omits the password hash. This is what
/// the usecase layer returns so the hash never escapes the `user` crate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
}

impl From<User> for UserView {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            code: user.code,
            name: user.name,
            role: user.role,
            active: user.active,
        }
    }
}

/// Async orchestration for user lifecycle operations.
///
/// The usecase owns the password hashing policy: callers hand it
/// plaintext passwords and the usecase hands the repository only
/// argon2 PHC strings. It also projects every `User` it returns into a
/// [`UserView`] so the password hash never leaves the persistence
/// boundary.
pub struct UserUsecase<R: UserRepository> {
    repository: R,
}

impl<R: UserRepository> UserUsecase<R> {
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    pub async fn create(&self, cmd: CreateUser) -> Result<UserView, UsecaseError> {
        validate_create(&cmd)?;

        let password_hash = hash_password(&cmd.password)?;

        let input = UserNew {
            code: cmd.code,
            name: cmd.name,
            role: cmd.role,
            password_hash,
            active: true,
        };

        let user = self.repository.create(input).await?;
        Ok(user.into())
    }

    pub async fn get_by_id(&self, id: i32) -> Result<UserView, UsecaseError> {
        let user = self.repository.find_by_id(id).await?;
        Ok(user.into())
    }

    pub async fn get_by_code(&self, code: &str) -> Result<UserView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let user = self.repository.find_by_code(code).await?;
        Ok(user.into())
    }

    pub async fn list(&self) -> Result<Vec<UserView>, UsecaseError> {
        let users = self.repository.list().await?;
        Ok(users.into_iter().map(UserView::from).collect())
    }

    pub async fn update(&self, cmd: UpdateUser) -> Result<UserView, UsecaseError> {
        validate_update(&cmd)?;

        let password_hash = match cmd.password.as_deref() {
            Some(plain) => Some(hash_password(plain)?),
            None => None,
        };

        let input = UserUpdate {
            id: cmd.id,
            code: cmd.code,
            name: cmd.name,
            role: cmd.role,
            active: cmd.active,
            password_hash,
        };

        let user = self.repository.update(input).await?;
        Ok(user.into())
    }

    pub async fn deactivate(&self, id: i32) -> Result<UserView, UsecaseError> {
        let user = self.repository.deactivate(id).await?;
        Ok(user.into())
    }
}

/// Reject empty / whitespace-only `code`, `name`, `password` before any
/// hashing work or repository call.
fn validate_create(cmd: &CreateUser) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    if cmd.password.is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyPassword));
    }
    Ok(())
}

fn validate_update(cmd: &UpdateUser) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
    }
    if let Some(ref name) = cmd.name {
        if name.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyName));
        }
    }
    if let Some(ref password) = cmd.password {
        if password.is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyPassword));
        }
    }
    Ok(())
}

/// Hash a plaintext password with argon2id and a random salt, returning
/// the canonical PHC string (`$argon2id$v=19$...`).
fn hash_password(plain: &str) -> Result<String, UsecaseError> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    argon2
        .hash_password(plain.as_bytes(), &salt)
        .map(|h| h.to_string())
        .map_err(|e| UsecaseError::Hashing(e.to_string()))
}