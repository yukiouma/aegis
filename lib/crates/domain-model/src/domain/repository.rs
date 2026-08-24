use async_trait::async_trait;

use super::error::DomainError;
use super::sdtm_domain::{SdtmDomain, SdtmDomainNew, SdtmDomainUpdate};
use super::sdtm_variable::{SdtmVariable, SdtmVariableNew, SdtmVariableUpdate};
use super::sdtm_version::{SdtmVersion, SdtmVersionNew, SdtmVersionUpdate};

/// Outbound port for persistence of `SdtmVersion` aggregates.
/// Implementations live in the adapter layer.
///
/// No `find_by_id` / `find_by_name`: `update` returns the
/// updated aggregate via `UPDATE … RETURNING *` and `delete`
/// runs `DELETE FROM … WHERE id = $1` directly.
#[async_trait]
pub trait SdtmVersionRepository: Send + Sync {
    async fn create(&self, input: SdtmVersionNew) -> Result<SdtmVersion, DomainError>;
    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError>;
    async fn update(&self, input: SdtmVersionUpdate) -> Result<SdtmVersion, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmDomain` aggregates.
/// The only list path is scoped to a version (no bare
/// `list()`).
#[async_trait]
pub trait SdtmDomainRepository: Send + Sync {
    async fn create(&self, input: SdtmDomainNew) -> Result<SdtmDomain, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError>;
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<SdtmDomain>, DomainError>;
    async fn update(&self, input: SdtmDomainUpdate) -> Result<SdtmDomain, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmVariable` aggregates.
/// The only list path is scoped to a domain (no bare
/// `list()`).
#[async_trait]
pub trait SdtmVariableRepository: Send + Sync {
    async fn create(&self, input: SdtmVariableNew) -> Result<SdtmVariable, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError>;
    async fn list_by_domain(&self, domain_id: i64) -> Result<Vec<SdtmVariable>, DomainError>;
    async fn update(&self, input: SdtmVariableUpdate) -> Result<SdtmVariable, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}
