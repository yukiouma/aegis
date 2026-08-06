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

/// Input for `AuthUsecase::logout`. The implementation decodes the
/// refresh token to extract the user code and bumps the token_version.
#[derive(Debug, Clone)]
pub struct Logout {
    pub refresh_token: String,
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

/// Output of `logout`. Empty by design — a successful logout carries
/// no payload.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogoutAck {}

/// Input for `AuthUsecase::find_user_credential`.
#[derive(Debug, Clone)]
pub struct FindUserCredential {
    pub code: String,
}

/// Input for `AuthUsecase::create_user_credential`. The implementation
/// picks the initial `token_version`.
#[derive(Debug, Clone)]
pub struct CreateUserCredential {
    pub code: String,
    pub password_hash: String,
}

/// Input for `AuthUsecase::update_user_credential`. Only `password_hash`
/// is mutable through this command.
#[derive(Debug, Clone, Default)]
pub struct UpdateUserCredential {
    pub code: String,
    pub password_hash: Option<String>,
}

/// Input for `AuthUsecase::remove_user_credential`.
#[derive(Debug, Clone)]
pub struct RemoveUserCredential {
    pub code: String,
}

/// Output of `find_user_credential` / `create_user_credential` /
/// `update_user_credential`. Same shape as the apis `UserCredentialView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCredentialView {
    pub code: String,
    pub password_hash: String,
    pub token_version: u32,
}

/// Output of `remove_user_credential`. Empty by design.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoveUserCredentialAck {}

// `Role` re-exported for `AuthClaimsView`'s public surface.
pub use crate::domain::Role;
