//! Outbound port for user lifecycle operations.
//!
//! See [`UserService`] for the trait surface. All supporting
//! types (`Role`, `UserApiError`, `UserView`, `CreateUserRequest`,
//! `UpdateUserRequest`) are defined alongside the trait so a
//! single `use apis::user::*;` brings the whole contract into
//! scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Role of a user within the system.
///
/// Mirrors `user::Role` so adapters between the two crates can
/// convert losslessly. Kept independent here so `apis` does not
/// depend on the `user` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Root,
    Admin,
    General,
}

/// Error surface returned by every [`UserService`] method.
///
/// Adapters map backend-specific errors (e.g. `user::UsecaseError`)
/// into this type at the implementation boundary. The shape
/// intentionally combines validation, lookup, and infrastructure
/// concerns into a single type so handlers can match exhaustively.
#[derive(Debug, Error)]
pub enum UserApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("user not found")]
    NotFound,

    #[error("user code already exists: {0}")]
    DuplicateCode(String),

    #[error("password hashing failed: {0}")]
    Hashing(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Safe projection of a user — no password / hash field, by
/// construction. This is what adapters hand back to whatever
/// consumes the API.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input DTO for creating a user.
///
/// Deliberately omits `password` — the password-hashing policy
/// lives in the backend's usecase layer. Adapters receive this
/// shape from outside and translate it into a backend-specific
/// create DTO that includes the password.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

/// Input DTO for updating a user.
///
/// Every field except `id` is optional; only the fields that
/// actually changed need to be supplied. Same rationale as
/// [`CreateUserRequest`] for the omission of `password`.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

/// Outbound port for user lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn UserService>` can be shared state
/// in an async server (axum, tarpc, etc.). Object-safe: no generic
/// methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `user::UserUsecase`) into this contract, translating between
/// backend-specific DTOs / errors and the `apis` types defined
/// above. The `password` field never appears on this trait's
/// surface.
#[async_trait]
pub trait UserService: Send + Sync {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError>;

    async fn get_by_id(&self, id: i32) -> Result<UserView, UserApiError>;

    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError>;

    async fn list(&self) -> Result<Vec<UserView>, UserApiError>;

    async fn update(&self, req: UpdateUserRequest) -> Result<UserView, UserApiError>;
}