//! Outbound port for authentication.
//!
//! See [`AuthService`] for the trait surface. All supporting types
//! (`TokenPair`, `AuthClaims`, the request / view / response DTOs,
//! and `AuthApiError`) are defined alongside the trait so a single
//! `use apis::auth::*;` brings the whole contract into scope.

use thiserror::Error;

use crate::user::Role;

/// Access + refresh token pair returned by the login methods.
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
    pub refresh_token: String,
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

/// Input DTO for [`AuthService::create_user_credential`].
///
/// `token_version` is intentionally absent: the implementation picks
/// the initial value (typically `0`).
#[derive(Debug, Clone)]
pub struct CreateUserCredentialRequest {
    pub user_code: String,
    pub password_hash: String,
}

/// Input DTO for [`AuthService::update_user_credential`].
///
/// Only `password_hash` is mutable through this DTO. To change
/// `token_version` callers go through a future admin-facing API
/// (out of scope here).
#[derive(Debug, Clone, Default)]
pub struct UpdateUserCredentialRequest {
    pub user_code: String,
    pub password_hash: Option<String>,
}

/// Response DTO for [`AuthService::logout`].
///
/// Empty by design — a successful logout carries no payload. Kept
/// as a named type (rather than `()`) so the response shape is
/// explicit at the API boundary and can be extended later
/// without a breaking trait change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogoutResponse {}

/// Response DTO for [`AuthService::refresh`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshResponse {
    pub access_token: String,
}

/// Safe projection of a user's credential.
///
/// `password_hash` is always a hashed representation (Argon2 in the
/// canonical backend); the trait does not constrain the hashing
/// algorithm. `token_version` is read-only through this trait
/// surface — see [`CreateUserCredentialRequest`] and
/// [`UpdateUserCredentialRequest`] for what callers may set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCredentialView {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

/// Response DTO for [`AuthService::remove_user_credential`].
///
/// Empty by design — a successful removal carries no payload. Kept
/// as a named type (rather than `()`) so the response shape is
/// explicit at the API boundary and can be extended later
/// without a breaking trait change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoveUserCredentialResponse {}

/// Error surface returned by every [`AuthService`] method.
///
/// Adapters map backend-specific errors into this type at the
/// implementation boundary. The shape intentionally combines
/// validation, lookup, credential, and token concerns into a
/// single type so handlers can match exhaustively.
#[derive(Debug, Clone, Error)]
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

    #[error("user credential already exists: {0}")]
    DuplicateCode(String),
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

    /// Verify an access token and recover the identity it was minted for.
    ///
    /// Returns `AuthClaims` on success. Token-format, signature,
    /// and expiry failures all surface as
    /// `AuthApiError::Verification`.
    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError>;

    /// Exchange a still-valid refresh token for a brand-new access token.
    ///
    /// Returns `RefreshResponse { access_token }` on success.
    /// Expired or tampered-with refresh tokens surface as
    /// `AuthApiError::Verification`. The refresh token itself is
    /// not rotated — callers keep using the same refresh token
    /// until it expires.
    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError>;

    // -- credential management -----------------------------------------

    /// Look up the credential row attached to `code`. Returns
    /// `NotFound` if no credential exists for that code.
    async fn find_user_credential_by_code(
        &self,
        code: &str,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Persist a new credential row. The implementation picks the
    /// initial `token_version`. Returns `DuplicateCode(code)` if a
    /// credential already exists for that `user_code`.
    async fn create_user_credential(
        &self,
        req: CreateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Apply the optional fields on `req` to the credential
    /// identified by `req.user_code`. Returns `NotFound` if no such
    /// credential exists. A `req` whose only set field is
    /// `user_code` (every other field is `None`) is permitted and
    /// returns the unchanged credential view.
    async fn update_user_credential(
        &self,
        req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Delete the credential row for `code`. Returns `NotFound` if
    /// no such credential exists.
    async fn remove_user_credential(
        &self,
        code: &str,
    ) -> Result<RemoveUserCredentialResponse, AuthApiError>;

    // -- session lifecycle --------------------------------------------

    /// Invalidate the session identified by `req.refresh_token`.
    ///
    /// The implementation looks up the token, removes any stored
    /// refresh-token entry, and returns `Ok(LogoutResponse::default())`.
    /// Returns `Ok(...)` even when the token had no active session
    /// (idempotent). A malformed or already-revoked refresh token
    /// surfaces as `AuthApiError::Verification`. Storage failures
    /// surface as `AuthApiError::Repository`.
    async fn logout(
        &self,
        req: LogoutRequest,
    ) -> Result<LogoutResponse, AuthApiError>;
}
