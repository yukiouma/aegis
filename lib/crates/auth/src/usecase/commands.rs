//! Command / view DTOs for the auth usecase.

/// Input for `AuthUsecase::login_with_password`.
#[derive(Debug, Clone)]
pub struct LoginWithPassword {
    pub code: String,
    pub password: String,
}

/// Input for `AuthUsecase::login_with_domain_user_info`.
#[derive(Debug, Clone)]
pub struct LoginWithDomainUserInfo {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

/// Input for `AuthUsecase::verify`.
#[derive(Debug, Clone)]
pub struct VerifyAccessToken {
    pub access_token: String,
}

/// Input for `AuthUsecase::refresh`.
#[derive(Debug, Clone)]
pub struct RefreshAccessToken {
    pub refresh_token: String,
}

/// Input for `AuthUsecase::logout`.
#[derive(Debug, Clone)]
pub struct Logout {
    pub code: String,
}

/// Output of `login_with_*` — opaque JWT strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPairView {
    pub access_token: String,
    pub refresh_token: String,
}

/// Output of `verify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthClaimsView {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

/// Output of `refresh` — a freshly-minted access JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenView {
    pub access_token: String,
}

/// Output of `logout`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutAck {
    pub code: String,
}

// `Role` re-exported for `AuthClaimsView`'s public surface.
pub use crate::domain::Role;
