use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, TokenVersionCache, UserCredentials,
    UserCredentialsRepository, UserService,
};

use super::commands::{
    AccessTokenView, AuthClaimsView, CreateUserCredential, FindUserCredential,
    LoginWithDomainUserInfo, LoginWithPassword, Logout, LogoutAck, RefreshAccessToken,
    RegisterUser, RegisteredUserView, RemoveUserCredential, RemoveUserCredentialAck, Role,
    TokenPairView, UpdateUserCredential, UserCredentialView, VerifyAccessToken,
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
    /// Domains permitted for user registration. Compared
    /// case-insensitively after trimming whitespace. An empty list
    /// denies every registration.
    pub allow_domains: Vec<String>,
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
    allow_domains: std::collections::HashSet<String>,
}

impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthUsecase<R, D> {
    pub fn new(config: AuthUsecaseConfig<R, D>) -> Self {
        let allow_domains = config
            .allow_domains
            .into_iter()
            .map(|d| d.trim().to_ascii_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        Self {
            credentials: config.credentials,
            identities: config.identities,
            user_service: config.user_service,
            cache: config.cache,
            signing_key: config.signing_key,
            access_ttl: config.access_ttl,
            refresh_ttl: config.refresh_ttl,
            allow_domains,
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
        let summary = self.user_service.get_by_code(&cmd.code).await?;
        if !summary.active {
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

        let role = summary.role;
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
        let summary = self.user_service.get_by_code(&cmd.code).await?;
        if !summary.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }
        let creds = self.credentials.find_by_code(&cmd.code).await?;
        // Warm the cache so the freshly-minted tokens verify without a miss.
        self.cache.put(&cmd.code, creds.token_version).await;
        let role = summary.role;
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

        let summary = self.user_service.get_by_code(&claims.sub).await?;
        if !summary.active {
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

        let summary = self.user_service.get_by_code(&claims.sub).await?;
        if !summary.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }

        let role = summary.role;
        let access = self.mint_access_token(&claims.sub, role, current)?;
        Ok(AccessTokenView {
            access_token: access,
        })
    }

    pub async fn logout(&self, cmd: Logout) -> Result<LogoutAck, UsecaseError> {
        if cmd.refresh_token.trim().is_empty() {
            return Err(UsecaseError::Verification("empty refresh_token".into()));
        }
        // Decode the refresh token to extract the user code; signature
        // and expiry failures surface as Verification. A token whose
        // `ver` no longer matches the current `token_version` is
        // accepted here too — logout is meant to be idempotent, so a
        // second call with the same (stale) refresh token still
        // succeeds and bumps the version further.
        let code = self.extract_code_from_refresh_token(&cmd.refresh_token)?;
        let new_version = self.credentials.bump_token_version(&code).await?;
        self.cache.put(&code, new_version).await;
        Ok(LogoutAck {})
    }

    /// Decode + validate a refresh token and return the user code.
    /// Shared by `logout` (which only needs the `sub` claim).
    fn extract_code_from_refresh_token(&self, refresh_token: &str) -> Result<String, UsecaseError> {
        use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
        let mut validation = Validation::new(Algorithm::HS256);
        validation.leeway = 5;
        validation.required_spec_claims = std::collections::HashSet::new();
        let key = DecodingKey::from_secret(&self.signing_key);
        decode::<RefreshClaims>(refresh_token, &key, &validation)
            .map(|d| d.claims.sub)
            .map_err(|e| UsecaseError::Verification(format!("decode refresh: {e}")))
    }

    pub async fn find_user_credential(
        &self,
        cmd: FindUserCredential,
    ) -> Result<UserCredentialView, UsecaseError> {
        let creds = self.credentials.find_by_code(&cmd.code).await?;
        Ok(creds_to_view(&creds))
    }

    /// Register a new user, credential row, and domain identity.
    ///
    /// Validates inputs, enforces the configured `allow_domains`
    /// (case-insensitive after trimming whitespace; empty list rejects
    /// every registration), and creates any missing user, credential,
    /// or identity rows. Existing rows are reused instead of
    /// overwritten. The raw password is hashed with Argon2 before
    /// persistence; only the freshly seeded user is forced to
    /// `Role::General` and `active = false`.
    pub async fn register_user(
        &self,
        cmd: RegisterUser,
    ) -> Result<RegisteredUserView, UsecaseError> {
        if cmd.user_code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        if cmd.user_name.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        if cmd.hostname.trim().is_empty() || cmd.sid.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        if cmd.password.is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        let normalized_domain = cmd.domain_name.trim().to_ascii_lowercase();
        if normalized_domain.is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        if !self.allow_domains.contains(&normalized_domain) {
            return Err(UsecaseError::Repository(DomainError::DomainNotAllowed(
                cmd.domain_name,
            )));
        }

        // Idempotent user creation: reuse an existing record if the
        // user already exists, otherwise create one with the forced
        // `General`/inactive defaults.
        let user = match self.user_service.get_by_code(&cmd.user_code).await {
            Ok(existing) => existing,
            Err(DomainError::NotFound) => {
                self.user_service
                    .create(&cmd.user_code, &cmd.user_name)
                    .await?
            }
            Err(other) => return Err(other.into()),
        };

        // Idempotent credential creation: only create a credential row
        // if the user has none. The initial token_version is 0 so
        // verify/refresh treat the freshly-registered user identically
        // to the seeded default.
        if self.credentials.find_by_code(&cmd.user_code).await.is_err() {
            let password_hash = Self::hash_password(&cmd.password)?;
            let now = chrono::Utc::now();
            let creds =
                UserCredentials::for_repository(cmd.user_code.clone(), password_hash, 0, now, now);
            self.credentials.create(creds).await?;
        }

        // Idempotent identity creation: only insert if the exact
        // (user_code, domain_name, hostname, sid) tuple is missing.
        if self
            .identities
            .find(&cmd.user_code, &cmd.domain_name, &cmd.hostname, &cmd.sid)
            .await
            .is_err()
        {
            let identity = DomainIdentity::for_repository(
                cmd.user_code.clone(),
                cmd.domain_name.clone(),
                cmd.hostname.clone(),
                cmd.sid.clone(),
            );
            self.identities.create(identity).await?;
        }

        Ok(RegisteredUserView {
            user_code: cmd.user_code,
            user_name: cmd.user_name,
            role: user.role,
            active: user.active,
            domain_name: cmd.domain_name,
            hostname: cmd.hostname,
            sid: cmd.sid,
        })
    }

    pub async fn create_user_credential(
        &self,
        cmd: CreateUserCredential,
    ) -> Result<UserCredentialView, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyCode));
        }
        if cmd.password.is_empty() {
            return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
        }
        // The command carries the raw user-supplied password. Hash
        // it here so the repository never sees plaintext.
        let password_hash = Self::hash_password(&cmd.password)?;
        let now = chrono::Utc::now();
        let creds = UserCredentials::for_repository(
            cmd.code,
            password_hash,
            // Initial token_version — the spec / apis doc-comment notes
            // "typically 0". We pick 0 explicitly rather than letting
            // the schema default to 1 so callers can reason about the
            // first login's verify step.
            0,
            now,
            now,
        );
        let created = self.credentials.create(creds).await?;
        Ok(creds_to_view(&created))
    }

    pub async fn update_user_credential(
        &self,
        cmd: UpdateUserCredential,
    ) -> Result<UserCredentialView, UsecaseError> {
        // Surface NotFound early by looking up the credential before
        // any write. `update_password_hash` also returns NotFound, but
        // a no-op update (every optional field is None) needs the
        // lookup path to surface the error.
        let creds = self.credentials.find_by_code(&cmd.code).await?;
        let updated = if let Some(ref password) = cmd.password {
            if password.is_empty() {
                return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
            }
            // Hash the raw password before handing it to the repo so
            // the repository never stores plaintext.
            let password_hash = Self::hash_password(password)?;
            self.credentials
                .update_password_hash(&cmd.code, &password_hash)
                .await?
        } else {
            creds
        };
        Ok(creds_to_view(&updated))
    }

    /// Hash a raw password with Argon2 (default params, fresh random
    /// salt) and return the PHC-encoded string. Synchronous because
    /// Argon2 hashing is CPU-bound and must run on the executor
    /// thread — the operation is not I/O.
    ///
    /// Failures here mean the hashing primitive itself errored (very
    /// rare, e.g. RNG exhaustion); surfacing them through
    /// `UsecaseError::Repository` keeps the existing error envelope
    /// without introducing a new variant.
    fn hash_password(plain: &str) -> Result<String, UsecaseError> {
        use argon2::password_hash::{PasswordHasher, SaltString, rand_core::OsRng};
        let salt = SaltString::generate(&mut OsRng);
        argon2::Argon2::default()
            .hash_password(plain.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| {
                UsecaseError::Repository(DomainError::Repository(format!("argon2 hash: {e}")))
            })
    }

    pub async fn remove_user_credential(
        &self,
        cmd: RemoveUserCredential,
    ) -> Result<RemoveUserCredentialAck, UsecaseError> {
        self.credentials.delete_by_code(&cmd.code).await?;
        Ok(RemoveUserCredentialAck {})
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

fn role_from_str(s: &str) -> Result<Role, UsecaseError> {
    Role::try_from(s).map_err(UsecaseError::Repository)
}

/// Project a domain `UserCredentials` into the usecase-layer view.
/// The shape is identical to `apis::auth::UserCredentialView`; the
/// facade maps straight through.
fn creds_to_view(c: &UserCredentials) -> UserCredentialView {
    UserCredentialView {
        code: c.code.clone(),
        password_hash: c.password_hash.clone(),
        token_version: c.token_version,
    }
}
