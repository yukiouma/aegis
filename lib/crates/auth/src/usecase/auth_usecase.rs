use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use apis::user::{UserApiError, UserService};
use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, DomainIdentityRepository, UserCredentials, UserCredentialsRepository,
};

use super::commands::{
    AccessTokenView, AuthClaimsView, Logout, LogoutAck, LoginWithDomainUserInfo,
    LoginWithPassword, RefreshAccessToken, Role, TokenPairView, VerifyAccessToken,
};
use super::error::UsecaseError;

/// Configuration passed to [`AuthUsecase::new`]. Plain pub-field struct;
/// no builder ceremony. Generic over the same two repository types so
/// field types stay concrete.
pub struct AuthUsecaseConfig<
    R: UserCredentialsRepository,
    D: DomainIdentityRepository,
> {
    pub credentials: R,
    pub identities: D,
    pub user_service: Arc<dyn UserService>,
    /// HS256 secret bytes. The caller owns the entropy; we never log it.
    pub signing_key: Vec<u8>,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Internal JWT claim payload for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    role: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Internal JWT claim payload for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshClaims {
    sub: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Async orchestration for the auth flow.
///
/// Holds an `Arc<RwLock<HashMap<String, u32>>>` cache of token versions
/// keyed by user code. `verify` and `refresh` consult the cache; on miss
/// they fall back to `credentials.find_by_code` and populate the cache.
/// `login_with_*` and `logout` write to the cache directly.
pub struct AuthUsecase<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    credentials: R,
    identities: D,
    user_service: Arc<dyn UserService>,
    signing_key: Vec<u8>,
    access_ttl: Duration,
    refresh_ttl: Duration,
    token_versions: Arc<RwLock<HashMap<String, u32>>>,
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
            token_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Internal helper: read the cached `token_version` for `code`,
    /// falling back to the repo on miss.
    async fn current_token_version(&self, code: &str) -> Result<u32, UsecaseError> {
        if let Some(v) = self.token_versions.read().unwrap().get(code).copied() {
            return Ok(v);
        }
        let creds = self.credentials.find_by_code(code).await?;
        let v = creds.token_version;
        self.token_versions
            .write()
            .unwrap()
            .insert(code.to_string(), v);
        Ok(v)
    }

    /// Placeholder method bodies; full implementations land in Tasks 4 and 5.
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
            UsecaseError::Repository(DomainError::Repository(format!(
                "argon2 parse: {e}"
            )))
        })?;
        use argon2::PasswordVerifier;
        if argon2::Argon2::default()
            .verify_password(cmd.password.as_bytes(), &parsed_hash)
            .is_err()
        {
            return Err(UsecaseError::Repository(DomainError::InvalidCredentials));
        }

        // Populate the cache so the freshly-minted tokens verify without a miss.
        self.token_versions
            .write()
            .unwrap()
            .insert(cmd.code.clone(), creds.token_version);

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
        self.token_versions
            .write()
            .unwrap()
            .insert(cmd.code.clone(), creds.token_version);
        let role = role_from_api(user.role);
        let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
        let refresh = self.mint_refresh_token(&cmd.code, creds.token_version)?;
        Ok(TokenPairView {
            access_token: access,
            refresh_token: refresh,
        })
    }

    pub async fn verify(
        &self,
        _cmd: VerifyAccessToken,
    ) -> Result<AuthClaimsView, UsecaseError> {
        unimplemented!("filled in by Task 5")
    }

    pub async fn refresh(
        &self,
        _cmd: RefreshAccessToken,
    ) -> Result<AccessTokenView, UsecaseError> {
        unimplemented!("filled in by Task 5")
    }

    pub async fn logout(&self, cmd: Logout) -> Result<LogoutAck, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        let new_version = self.credentials.bump_token_version(&cmd.code).await?;
        self.token_versions
            .write()
            .unwrap()
            .insert(cmd.code.clone(), new_version);
        Ok(LogoutAck { code: cmd.code })
    }

    // `UserCredentials` is referenced in tests later; keep the import
    // so the compiler warns if a later task accidentally removes it.
    #[allow(dead_code)]
    fn _phantom(&self, _: UserCredentials) {}

    fn mint_access_token(
        &self,
        code: &str,
        role: Role,
        version: u32,
    ) -> Result<String, UsecaseError> {
        use jsonwebtoken::{encode, EncodingKey, Header};
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
        use jsonwebtoken::{encode, EncodingKey, Header};
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