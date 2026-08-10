use async_trait::async_trait;

use super::DomainError;
use super::credentials::UserCredentials;
use super::domain_identity::DomainIdentity;

/// Outbound port for persistence of `UserCredentials`.
#[async_trait]
pub trait UserCredentialsRepository: Send + Sync {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError>;

    async fn create(&self, credentials: UserCredentials) -> Result<UserCredentials, DomainError>;

    /// Atomically increments `token_version` for the user identified
    /// by `code` and returns the new value. Returns `DomainError::NotFound`
    /// if no row exists. The caller (the usecase) is responsible for
    /// writing the returned value into the
    /// [`TokenVersionCache`](super::TokenVersionCache) so subsequent
    /// `verify` / `refresh` calls in the same process reject tokens
    /// minted before the bump.
    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError>;

    /// Update `password_hash` for `code`. Returns the refreshed row,
    /// or `DomainError::NotFound` if no such credential exists. Does
    /// NOT bump `token_version` — callers that want the password
    /// change to invalidate outstanding tokens go through
    /// `bump_token_version` separately.
    async fn update_password_hash(
        &self,
        code: &str,
        password_hash: &str,
    ) -> Result<UserCredentials, DomainError>;

    /// Delete the credential row for `code`. Returns
    /// `DomainError::NotFound` if no such row exists.
    async fn delete_by_code(&self, code: &str) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `DomainIdentity`.
#[async_trait]
pub trait DomainIdentityRepository: Send + Sync {
    /// Find the row matching the supplied identity triple. Returns
    /// `DomainError::NotFound` if no row matches.
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError>;

    async fn create(&self, identity: DomainIdentity) -> Result<DomainIdentity, DomainError>;
}
