use crate::domain::{
    DomainError, SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate, SdtmVariableNew,
    SdtmVariableRepository, SdtmVariableUpdate, SdtmVersionNew, SdtmVersionRepository,
    SdtmVersionUpdate,
};

use super::commands::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, UpdateSdtmDomain, UpdateSdtmVariable,
    UpdateSdtmVersion,
};
use super::error::UsecaseError;
use super::views::{SdtmDomainView, SdtmVariableView, SdtmVersionView};

/// Configuration for `DomainModelUsecase::new`. Wraps the three
/// concrete (or fake) repositories so the constructor stays
/// readable.
pub struct DomainModelUsecaseConfig<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    pub version_repo: V,
    pub domain_repo: D,
    pub variable_repo: Va,
}

/// Async orchestration for SDTM domain-model lifecycle
/// operations. Generic over the three repository ports so tests
/// can inject in-memory fakes. Domain → view projection runs
/// through the `From` impls in `super::views`.
pub struct DomainModelUsecase<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    version_repo: V,
    domain_repo: D,
    variable_repo: Va,
}

impl<V, D, Va> DomainModelUsecase<V, D, Va>
where
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
{
    pub fn new(cfg: DomainModelUsecaseConfig<V, D, Va>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            domain_repo: cfg.domain_repo,
            variable_repo: cfg.variable_repo,
        }
    }

    // ---- SdtmVersion ----

    pub async fn create_version(
        &self,
        cmd: CreateSdtmVersion,
    ) -> Result<SdtmVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        let v = self
            .version_repo
            .create(SdtmVersionNew { name: cmd.name })
            .await?;
        Ok(v.into())
    }

    pub async fn list_versions(&self) -> Result<Vec<SdtmVersionView>, UsecaseError> {
        let vs = self.version_repo.list().await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateSdtmVersion,
    ) -> Result<SdtmVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(SdtmVersionUpdate {
                id: cmd.id,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmDomain ----

    pub async fn create_domain(
        &self,
        cmd: CreateSdtmDomain,
    ) -> Result<SdtmDomainView, UsecaseError> {
        validate_create_domain(&cmd)?;
        let d = self
            .domain_repo
            .create(SdtmDomainNew {
                version_id: cmd.version_id,
                name: cmd.name,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn get_domain_by_id(&self, id: i64) -> Result<SdtmDomainView, UsecaseError> {
        let d = self.domain_repo.find_by_id(id).await?;
        Ok(d.into())
    }

    pub async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmDomainView>, UsecaseError> {
        let ds = self.domain_repo.list_by_version(version_id).await?;
        Ok(ds.into_iter().map(Into::into).collect())
    }

    pub async fn update_domain(
        &self,
        cmd: UpdateSdtmDomain,
    ) -> Result<SdtmDomainView, UsecaseError> {
        validate_update_domain(&cmd)?;
        let d = self
            .domain_repo
            .update(SdtmDomainUpdate {
                id: cmd.id,
                name: cmd.name,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError> {
        self.domain_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmVariable ----

    pub async fn create_variable(
        &self,
        cmd: CreateSdtmVariable,
    ) -> Result<SdtmVariableView, UsecaseError> {
        validate_create_variable(&cmd)?;
        let v = self
            .variable_repo
            .create(SdtmVariableNew {
                domain_id: cmd.domain_id,
                name: cmd.name,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn get_variable_by_id(&self, id: i64) -> Result<SdtmVariableView, UsecaseError> {
        let v = self.variable_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmVariableView>, UsecaseError> {
        let vs = self.variable_repo.list_by_domain(domain_id).await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_variable(
        &self,
        cmd: UpdateSdtmVariable,
    ) -> Result<SdtmVariableView, UsecaseError> {
        validate_update_variable(&cmd)?;
        let v = self
            .variable_repo
            .update(SdtmVariableUpdate {
                id: cmd.id,
                name: cmd.name,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError> {
        self.variable_repo.delete(id).await?;
        Ok(())
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateSdtmVersion) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateSdtmVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_domain(cmd: &CreateSdtmDomain) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_domain(cmd: &UpdateSdtmDomain) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_variable(cmd: &CreateSdtmVariable) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_variable(cmd: &UpdateSdtmVariable) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}
