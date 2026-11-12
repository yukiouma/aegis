use async_trait::async_trait;

use super::error::DomainError;
use super::sdtm_project_domain::{
    SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainUpdate,
};
use super::sdtm_project_variable::{
    SdtmProjectVariable, SdtmProjectVariableNew, SdtmProjectVariableUpdate,
};
use super::sdtm_project_version::{
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionUpdate,
};

/// Outbound port for persistence of `SdtmProjectVersion` aggregates.
/// The only list path is scoped to a project (no bare `list()`).
#[async_trait]
pub trait SdtmProjectVersionRepository: Send + Sync {
    async fn create(&self, input: SdtmProjectVersionNew)
    -> Result<SdtmProjectVersion, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVersion, DomainError>;
    async fn list_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersion>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectVersionUpdate,
    ) -> Result<SdtmProjectVersion, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectDomain` aggregates.
/// The only list path is scoped to a version (no bare `list()`).
#[async_trait]
pub trait SdtmProjectDomainRepository: Send + Sync {
    async fn create(&self, input: SdtmProjectDomainNew) -> Result<SdtmProjectDomain, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectDomain, DomainError>;
    async fn list_by_version(&self, version_id: i64)
    -> Result<Vec<SdtmProjectDomain>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectDomainUpdate,
    ) -> Result<SdtmProjectDomain, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmProjectVariable` aggregates.
/// The only list path is scoped to a domain (no bare `list()`).
#[async_trait]
pub trait SdtmProjectVariableRepository: Send + Sync {
    async fn create(
        &self,
        input: SdtmProjectVariableNew,
    ) -> Result<SdtmProjectVariable, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmProjectVariable, DomainError>;
    async fn list_by_domain(&self, domain_id: i64)
    -> Result<Vec<SdtmProjectVariable>, DomainError>;
    async fn update(
        &self,
        input: SdtmProjectVariableUpdate,
    ) -> Result<SdtmProjectVariable, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}
