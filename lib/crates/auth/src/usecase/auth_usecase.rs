use std::sync::Arc;
use std::time::Duration;

use apis::user::{UserApiError, UserService};
use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, DomainIdentityRepository, TokenVersionCache, UserCredentialsRepository,
};

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
    /// Token-version cache. The in-memory backend
    /// ([`crate::InMemoryTokenVersionCache`]) is the default; a future
    /// Redis backend will swap in here without touching the rest of
    /// the layer.
    pub cache: Arc<dyn TokenVersionCache>,
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
/// The token-version cache is a separate port
/// ([`TokenVersionCache`]) reached via `Arc<dyn TokenVersionCache>`.
/// `verify` and `refresh` go through `cache.get` with
/// `credentials.find_by_code` as the fallback; `logout` updates the
/// cache after `credentials.bump_token_version`.
pub struct AuthUsecase<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    credentials: R,
    identities: D,
    user_service: Arc<dyn UserService>,
    cache: Arc<dyn TokenVersionCache>,
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
            cache: config.cache,
            signing_key: config.signing_key,
            access_ttl: config.access_ttl,
            refresh_ttl: config.refresh_ttl,
        }
    }

    /// Resolve the current `token_version` for `code` via the cache,
    /// falling back to the repository on miss. The fallback populates
    /// the cache with the value just read.
    async fn current_token_version(&self, code: &str) -> Result<u32, UsecaseError> {
        if let Some(v) = self.cache.get(code).await {
            return Ok(v);
        }
        let creds = self.credentials.find_by_code(code).await?;
        self.cache.put(code, creds.token_version).await;
        Ok(creds.token_version)
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

        // Warm the cache so the freshly-minted tokens verify without a miss.
        self.cache.put(&cmd.code, creds.token_version).await;

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
        // Warm the cache so the freshly-minted tokens verify without a miss.
        self.cache.put(&cmd.code, creds.token_version).await;
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

        let current = self.current_token_version(&claims.sub).await?;
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

        let current = self.current_token_version(&claims.sub).await?;
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
        // The repository bumps the database; the cache update is best-effort
        // and does not block the response. Subsequent verify / refresh calls
        // in this process see the new version via the cache; a missing cache
        // entry falls back to the DB and re-warms the cache.
        let new_version = self.credentials.bump_token_version(&cmd.code).await?;
        self.cache.put(&cmd.code, new_version).await;
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
