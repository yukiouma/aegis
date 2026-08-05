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

/// Input DTO for [`AuthService::logout`].
#[derive(Debug, Clone)]
pub struct LogoutRequest {
    pub code: String,
}

/// Input DTO for [`AuthService::verify`].
#[derive(Debug, Clone)]
pub struct VerifyRequest {
    pub access_token: String,
}

/// Input DTO for [`AuthService::refresh`].
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    pub refresh_token: String,
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

/// Outbound port for authentication.
///
/// `Send + Sync` so a `Box<dyn AuthService>` can be shared state in
/// an async server (axum, tarpc, etc.). Object-safe: no generic
/// methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer into this
/// contract, translating between backend-specific DTOs / errors
/// and the `apis` types defined above.
#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    /// Authenticate with a user code + password.
    ///
    /// On success mints a fresh access token and refresh token and
    /// returns them. Implementations check the password against
    /// the persisted hash and surface `InvalidCredentials` (not
    /// `NotFound`) for a code that exists with the wrong password.
    async fn login_with_password(
        &self,
        req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError>;

    /// Authenticate with Windows-domain user info (AD / NTLM style).
    ///
    /// `domain_name`, `hostname`, and `sid` identify the domain
    /// account. On success mints a fresh access token and refresh
    /// token and returns them. Implementations surface `NotFound`
    /// when no user maps to the supplied domain-identity triple.
    async fn login_with_domain_user_info(
        &self,
        req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError>;

    /// Invalidate any server-side session state for `code`.
    ///
    /// Returns `Ok(())` even if the user had no active session.
    /// Storage failures surface as `AuthApiError::Repository`.
    async fn logout(&self, req: LogoutRequest) -> Result<(), AuthApiError>;

    /// Verify an access token and recover the identity it was minted for.
    ///
    /// Returns `AuthClaims` on success. Token-format, signature,
    /// and expiry failures all surface as
    /// `AuthApiError::Verification`.
    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError>;

    /// Exchange a still-valid refresh token for a brand-new access token.
    ///
    /// Returns the freshly minted access token as a `String`.
    /// Expired or tampered-with refresh tokens surface as
    /// `AuthApiError::Verification`. The refresh token itself is
    /// not rotated — callers keep using the same refresh token
    /// until it expires.
    async fn refresh(&self, req: RefreshRequest) -> Result<String, AuthApiError>;
}
