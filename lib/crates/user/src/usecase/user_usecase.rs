use argon2::password_hash::{SaltString, rand_core::OsRng};
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
    /// Build a new `UserUsecase` wrapping the supplied `repository`.
    ///
    /// The usecase owns the password hashing policy and the
    /// `User` -> `UserView` projection. `repository` is the
    /// persistence port; pass [`UserRepo`](crate::UserRepo) for the
    /// PostgreSQL-backed implementation.
    pub fn new(repository: R) -> Self {
        Self { repository }
    }

    /// Validate the inputs, hash the plaintext password with argon2id,
    /// and persist a new user. Returns the resulting user as a
    /// [`UserView`] (the password hash is not exposed).
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
    /// `cmd.id`. A supplied `password` is re-hashed before it reaches
    /// the repository.
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

    /// Soft-remove the user by id (sets `active = false`). There is
    /// no hard `delete` operation by design.
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
    if let Some(ref password) = cmd.password
        && password.is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyPassword));
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
