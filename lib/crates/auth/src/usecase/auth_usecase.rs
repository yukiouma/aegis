use std::sync::Arc;
use std::time::Duration;

use apis::user::{UserApiError, UserService};
use serde::{Deserialize, Serialize};

use crate::domain::{DomainError, DomainIdentityRepository, UserCredentialsRepository};

use super::commands::{
    AccessTokenView, AuthClaimsView, LoginWithDomainUserInfo, LoginWithPassword, Logout, LogoutAck,
    RefreshAccessToken, Role, TokenPairView, VerifyAccessToken,
};
use super::error::UsecaseError;

/// Configuration passed to [`AuthUsecase::new`]. Plain pub-field struct;
/// no builder ceremony. Generic over the same two repository types so
/// field types stay concrete.
pub struct AuthUsecaseConfig<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    pub credentials: R,
    pub identities: D,
    pub user_service: Arc<dyn UserService>,
    /// HS256 secret bytes. The caller owns the entropy; we never log it.
    pub signing_key: Vec<u8>,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Internal JWT claim payload for access tokens.
///
/// `deny_unknown_fields` makes the decode reject refresh tokens (which
/// lack `role`) and any future field additions explicit. The check is
/// what gives `verify` and `refresh` their structural type-rejection
/// without needing a `typ` discriminator.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct AccessClaims {
    sub: String,
    role: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Internal JWT claim payload for refresh tokens. See [`AccessClaims`]
/// for why `deny_unknown_fields` matters.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct RefreshClaims {
    sub: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Async orchestration for the auth flow.
///
/// The token-version cache lives inside the
/// [`UserCredentialsRepository`](crate::domain::UserCredentialsRepository)
/// implementation — see the `current_token_version` port method. The
/// usecase calls it directly; no cache state of its own.
pub struct AuthUsecase<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    credentials: R,
    identities: D,
    user_service: Arc<dyn UserService>,
    signing_key: Vec<u8>,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthUsecase<R, D> {
    pub fn new(config: AuthUsecaseConfig<R, D>) -> Self {
        Self {
            credentials: config.credentials,
            identities: config.identities,
            user_service: config.user_service,
            signing_key: config.signing_key,
            access_ttl: config.access_ttl,
            refresh_ttl: config.refresh_ttl,
        }
    }

    pub async fn login_with_password(
        &self,
        cmd: LoginWithPassword,
    ) -> Result<TokenPairView, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        if cmd.password.is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        let creds = self.credentials.find_by_code(&cmd.code).await?;
        let user = self
            .user_service
            .get_by_code(&cmd.code)
            .await
            .map_err(map_user_service_error)?;
        if !user.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }
        let parsed_hash = argon2::PasswordHash::new(&creds.password_hash).map_err(|e| {
            UsecaseError::Repository(DomainError::Repository(format!("argon2 parse: {e}")))
        })?;
        use argon2::PasswordVerifier;
        if argon2::Argon2::default()
            .verify_password(cmd.password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(UsecaseError::Repository(DomainError::InvalidCredentials));
        }

        let role = role_from_api(user.role);
        let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
        let refresh = self.mint_refresh_token(&cmd.code, creds.token_version)?;

        Ok(TokenPairView {
            access_token: access,
            refresh_token: refresh,
        })
    }

    pub async fn login_with_domain_user_info(
        &self,
        cmd: LoginWithDomainUserInfo,
    ) -> Result<TokenPairView, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        if cmd.domain_name.trim().is_empty()
            || cmd.hostname.trim().is_empty()
            || cmd.sid.trim().is_empty()
        {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        self.identities
            .find(&cmd.code, &cmd.domain_name, &cmd.hostname, &cmd.sid)
            .await?;
        let user = self
            .user_service
            .get_by_code(&cmd.code)
            .await
            .map_err(map_user_service_error)?;
        if !user.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }
        let creds = self.credentials.find_by_code(&cmd.code).await?;
        let role = role_from_api(user.role);
        let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
        let refresh = self.mint_refresh_token(&cmd.code, creds.token_version)?;
        Ok(TokenPairView {
            access_token: access,
            refresh_token: refresh,
        })
    }

    pub async fn verify(&self, cmd: VerifyAccessToken) -> Result<AuthClaimsView, UsecaseError> {
        use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 5;
        validation.required_spec_claims = std::collections::HashSet::new();
        let key = DecodingKey::from_secret(&self.signing_key);
        let data = decode::<AccessClaims>(&cmd.access_token, &key, &validation)
            .map_err(|e| UsecaseError::Verification(format!("decode access: {e}")))?;
        let claims = data.claims;

        let current = self.credentials.current_token_version(&claims.sub).await?;
        if current != claims.ver {
            return Err(UsecaseError::Verification(format!(
                "token_version mismatch (current = {current}, jwt.ver = {})",
                claims.ver
            )));
        }

        let user = self
            .user_service
            .get_by_code(&claims.sub)
            .await
            .map_err(map_user_service_error)?;
        if !user.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }

        let role = role_from_str(&claims.role)?;
        Ok(AuthClaimsView {
            code: claims.sub,
            role,
            token_version: claims.ver,
        })
    }

    pub async fn refresh(&self, cmd: RefreshAccessToken) -> Result<AccessTokenView, UsecaseError> {
        use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 5;
        validation.required_spec_claims = std::collections::HashSet::new();
        let key = DecodingKey::from_secret(&self.signing_key);
        let data = decode::<RefreshClaims>(&cmd.refresh_token, &key, &validation)
            .map_err(|e| UsecaseError::Verification(format!("decode refresh: {e}")))?;
        let claims = data.claims;

        let current = self.credentials.current_token_version(&claims.sub).await?;
        if current != claims.ver {
            return Err(UsecaseError::Verification(format!(
                "token_version mismatch (current = {current}, jwt.ver = {})",
                claims.ver
            )));
        }

        let user = self
            .user_service
            .get_by_code(&claims.sub)
            .await
            .map_err(map_user_service_error)?;
        if !user.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }

        let role = role_from_api(user.role);
        let access = self.mint_access_token(&claims.sub, role, current)?;
        Ok(AccessTokenView {
            access_token: access,
        })
    }

    pub async fn logout(&self, cmd: Logout) -> Result<LogoutAck, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        // The repository atomically bumps the database and writes the new
        // version into its in-memory cache, so subsequent verify / refresh
        // calls in this process reject tokens minted before the bump.
        self.credentials.bump_token_version(&cmd.code).await?;
        Ok(LogoutAck { code: cmd.code })
    }

    fn mint_access_token(
        &self,
        code: &str,
        role: Role,
        version: u32,
    ) -> Result<String, UsecaseError> {
        use jsonwebtoken::{EncodingKey, Header, encode};
        let now = chrono::Utc::now().timestamp();
        let claims = AccessClaims {
            sub: code.to_string(),
            role: role.as_str().to_string(),
            ver: version,
            iat: now,
            exp: now + self.access_ttl.as_secs() as i64,
        };
        let enc = EncodingKey::from_secret(&self.signing_key);
        encode(&Header::new(jsonwebtoken::Algorithm::HS256), &claims, &enc)
            .map_err(|e| UsecaseError::Verification(format!("encode access: {e}")))
    }

    fn mint_refresh_token(&self, code: &str, version: u32) -> Result<String, UsecaseError> {
        use jsonwebtoken::{EncodingKey, Header, encode};
        let now = chrono::Utc::now().timestamp();
        let claims = RefreshClaims {
            sub: code.to_string(),
            ver: version,
            iat: now,
            exp: now + self.refresh_ttl.as_secs() as i64,
        };
        let enc = EncodingKey::from_secret(&self.signing_key);
        encode(&Header::new(jsonwebtoken::Algorithm::HS256), &claims, &enc)
            .map_err(|e| UsecaseError::Verification(format!("encode refresh: {e}")))
    }
}

fn map_user_service_error(err: UserApiError) -> UsecaseError {
    match err {
        UserApiError::NotFound => UsecaseError::Repository(DomainError::NotFound),
        other => UsecaseError::Repository(DomainError::Repository(other.to_string())),
    }
}

fn role_from_api(r: apis::user::Role) -> Role {
    match r {
        apis::user::Role::Root => Role::Root,
        apis::user::Role::Admin => Role::Admin,
        apis::user::Role::General => Role::General,
    }
}

fn role_from_str(s: &str) -> Result<Role, UsecaseError> {
    Role::try_from(s).map_err(UsecaseError::Repository)
}
