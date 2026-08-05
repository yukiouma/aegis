//! Outbound port for authentication.
//!
//! See [`AuthService`] for the trait surface. All supporting types
//! (`TokenPair`, `AuthClaims`, the login request DTOs, and
//! `AuthApiError`) are defined alongside the trait so a single
//! `use apis::auth::*;` brings the whole contract into scope.

use thiserror::Error;

use crate::user::Role;

/// Access + refresh token pair returned by the login methods.
///
/// `refresh` does not use `TokenPair` — it mints a new access token
/// only, returning the bare `String`. The login methods return both
/// freshly minted tokens so callers can hand them out together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Authenticated identity recovered from a verified access token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthClaims {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

/// Input DTO for [`AuthService::login_with_password`].
#[derive(Debug, Clone)]
pub struct LoginWithPasswordRequest {
    pub code: String,
    pub password: String,
}

/// Input DTO for [`AuthService::login_with_domain_user_info`].
#[derive(Debug, Clone)]
pub struct LoginWithDomainUserInfoRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

/// Error surface returned by every [`AuthService`] method.
///
/// Adapters map backend-specific errors into this type at the
/// implementation boundary. The shape intentionally combines
/// validation, lookup, credential, and token concerns into a
/// single type so handlers can match exhaustively.
#[derive(Debug, Error)]
pub enum AuthApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("user not found")]
    NotFound,

    #[error("user is inactive")]
    Inactive,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("token signing failed: {0}")]
    Signing(String),

    #[error("token verification failed: {0}")]
    Verification(String),

    #[error("repository error: {0}")]
    Repository(String),
}
