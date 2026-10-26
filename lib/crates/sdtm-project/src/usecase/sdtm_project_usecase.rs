use std::sync::Arc;

use crate::domain::{
    DomainError, ProjectLookup, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideRepository,
    SdtmProjectDomainOverrideUpdate, SdtmProjectDomainRepository, SdtmProjectDomainUpdate,
    SdtmProjectVariableNew, SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideRepository,
    SdtmProjectVariableOverrideUpdate, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersionNew, SdtmProjectVersionRepository, SdtmProjectVersionUpdate,
};

use super::commands::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, UpdateSdtmProjectDomain,
    UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable, UpdateSdtmProjectVariableOverride,
    UpdateSdtmProjectVersion,
};
use super::error::UsecaseError;
use super::views::{
    SdtmProjectDomainOverrideView, SdtmProjectDomainView, SdtmProjectVariableOverrideView,
    SdtmProjectVariableView, SdtmProjectVersionView,
};

/// Configuration for `SdtmProjectUsecase::new`. Wraps the five
/// repositories plus the project lookup so the constructor stays
/// readable.
pub struct SdtmProjectUsecaseConfig<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
> {
    pub version_repo: V,
    pub domain_repo: D,
    pub domain_override_repo: DO,
    pub variable_repo: Va,
    pub variable_override_repo: VO,
    /// `Arc<P>` rather than `P`, so the call site can hand over an
    /// `Arc<ProjectLookupImpl>` directly. Mirrors
    /// `crf::CrfUsecaseConfig`.
    pub projects: Arc<P>,
}

/// Async orchestration for the per-project SDTM model tree. Generic
/// over the five repository ports plus the project lookup so tests can
/// inject in-memory fakes. Domain → view projection runs through the
/// `From` impls in `super::views`.
pub struct SdtmProjectUsecase<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
> {
    version_repo: V,
    domain_repo: D,
    domain_override_repo: DO,
    variable_repo: Va,
    variable_override_repo: VO,
    projects: Arc<P>,
}

impl<V, D, DO, Va, VO, P> SdtmProjectUsecase<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    pub fn new(cfg: SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            domain_repo: cfg.domain_repo,
            domain_override_repo: cfg.domain_override_repo,
            variable_repo: cfg.variable_repo,
            variable_override_repo: cfg.variable_override_repo,
            projects: cfg.projects,
        }
    }

    // ---- SdtmProjectVersion ----

    pub async fn create_version(
        &self,
        cmd: CreateSdtmProjectVersion,
    ) -> Result<SdtmProjectVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        // The project must exist before the version is created. A
        // missing project is a 404, not a 400 — the client named a
        // resource that is not there.
        self.projects.get_by_code(&cmd.project_code).await?;
        let v = self
            .version_repo
            .create(SdtmProjectVersionNew {
                project_code: cmd.project_code,
                version_name: cmd.version_name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn get_version_by_id(&self, id: i64) -> Result<SdtmProjectVersionView, UsecaseError> {
        let v = self.version_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_versions_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<SdtmProjectVersionView>, UsecaseError> {
        let vs = self.version_repo.list_by_project(project_code).await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateSdtmProjectVersion,
    ) -> Result<SdtmProjectVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(SdtmProjectVersionUpdate {
                id: cmd.id,
                version_name: cmd.version_name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmProjectDomain ----

    pub async fn create_domain(
        &self,
        cmd: CreateSdtmProjectDomain,
    ) -> Result<SdtmProjectDomainView, UsecaseError> {
        validate_create_domain(&cmd)?;
        // Normalise here so the repository always receives the
        // caller's order with duplicates collapsed, and a blank entry
        // is rejected before it reaches the database.
        let key_variables = SdtmProjectDomain::normalize_key_variables(&cmd.key_variables)
            .map_err(UsecaseError::Validation)?;
        ensure_version_exists(&self.version_repo, cmd.version_id).await?;
        let d = self
            .domain_repo
            .create(SdtmProjectDomainNew {
                version_id: cmd.version_id,
                domain_code: cmd.domain_code,
                key_variables,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn get_domain_by_id(&self, id: i64) -> Result<SdtmProjectDomainView, UsecaseError> {
        let d = self.domain_repo.find_by_id(id).await?;
        Ok(d.into())
    }

    pub async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmProjectDomainView>, UsecaseError> {
        let ds = self.domain_repo.list_by_version(version_id).await?;
        Ok(ds.into_iter().map(Into::into).collect())
    }

    pub async fn update_domain(
        &self,
        cmd: UpdateSdtmProjectDomain,
    ) -> Result<SdtmProjectDomainView, UsecaseError> {
        validate_update_domain(&cmd)?;
        let key_variables = match &cmd.key_variables {
            None => None,
            Some(kv) => Some(
                SdtmProjectDomain::normalize_key_variables(kv).map_err(UsecaseError::Validation)?,
            ),
        };
        let d = self
            .domain_repo
            .update(SdtmProjectDomainUpdate {
                id: cmd.id,
                domain_code: cmd.domain_code,
                key_variables,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError> {
        self.domain_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmProjectDomainOverride ----

    pub async fn create_domain_override(
        &self,
        cmd: CreateSdtmProjectDomainOverride,
    ) -> Result<SdtmProjectDomainOverrideView, UsecaseError> {
        ensure_domain_exists(&self.domain_repo, cmd.domain_id).await?;
        let o = self
            .domain_override_repo
            .create(SdtmProjectDomainOverrideNew {
                domain_id: cmd.domain_id,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(o.into())
    }

    /// The domain's override, or `None` when it has none.
    pub async fn get_domain_override(
        &self,
        domain_id: i64,
    ) -> Result<Option<SdtmProjectDomainOverrideView>, UsecaseError> {
        match self.domain_override_repo.find_by_domain_id(domain_id).await {
            Ok(o) => Ok(Some(o.into())),
            // The override is optional: a domain without one is a
            // normal state, not a failure. The repository still
            // reports a missing row as `SdtmProjectDomainOverrideNotFound`; the usecase is
            // where that becomes "no override".
            Err(DomainError::SdtmProjectDomainOverrideNotFound(_)) => Ok(None),
            Err(e) => Err(UsecaseError::Repository(e)),
        }
    }

    pub async fn update_domain_override(
        &self,
        cmd: UpdateSdtmProjectDomainOverride,
    ) -> Result<SdtmProjectDomainOverrideView, UsecaseError> {
        let o = self
            .domain_override_repo
            .update(SdtmProjectDomainOverrideUpdate {
                domain_id: cmd.domain_id,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(o.into())
    }

    pub async fn delete_domain_override(&self, domain_id: i64) -> Result<(), UsecaseError> {
        self.domain_override_repo.delete(domain_id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariable ----

    pub async fn create_variable(
        &self,
        cmd: CreateSdtmProjectVariable,
    ) -> Result<SdtmProjectVariableView, UsecaseError> {
        validate_create_variable(&cmd)?;
        ensure_domain_exists(&self.domain_repo, cmd.domain_id).await?;
        let v = self
            .variable_repo
            .create(SdtmProjectVariableNew {
                domain_id: cmd.domain_id,
                variable_code: cmd.variable_code,
                variable_sequence: cmd.variable_sequence,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn get_variable_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVariableView, UsecaseError> {
        let v = self.variable_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmProjectVariableView>, UsecaseError> {
        let vs = self.variable_repo.list_by_domain(domain_id).await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_variable(
        &self,
        cmd: UpdateSdtmProjectVariable,
    ) -> Result<SdtmProjectVariableView, UsecaseError> {
        validate_update_variable(&cmd)?;
        let v = self
            .variable_repo
            .update(SdtmProjectVariableUpdate {
                id: cmd.id,
                variable_code: cmd.variable_code,
                variable_sequence: cmd.variable_sequence,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError> {
        self.variable_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariableOverride ----

    pub async fn create_variable_override(
        &self,
        cmd: CreateSdtmProjectVariableOverride,
    ) -> Result<SdtmProjectVariableOverrideView, UsecaseError> {
        ensure_variable_exists(&self.variable_repo, cmd.variable_id).await?;
        let o = self
            .variable_override_repo
            .create(SdtmProjectVariableOverrideNew {
                variable_id: cmd.variable_id,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(o.into())
    }

    /// The variable's override, or `None` when it has none.
    pub async fn get_variable_override(
        &self,
        variable_id: i64,
    ) -> Result<Option<SdtmProjectVariableOverrideView>, UsecaseError> {
        match self
            .variable_override_repo
            .find_by_variable_id(variable_id)
            .await
        {
            Ok(o) => Ok(Some(o.into())),
            // The override is optional: a variable without one is a
            // normal state, not a failure. The repository still
            // reports a missing row as `SdtmProjectVariableOverrideNotFound`; the usecase is
            // where that becomes "no override".
            Err(DomainError::SdtmProjectVariableOverrideNotFound(_)) => Ok(None),
            Err(e) => Err(UsecaseError::Repository(e)),
        }
    }

    pub async fn update_variable_override(
        &self,
        cmd: UpdateSdtmProjectVariableOverride,
    ) -> Result<SdtmProjectVariableOverrideView, UsecaseError> {
        let o = self
            .variable_override_repo
            .update(SdtmProjectVariableOverrideUpdate {
                variable_id: cmd.variable_id,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(o.into())
    }

    pub async fn delete_variable_override(&self, variable_id: i64) -> Result<(), UsecaseError> {
        self.variable_override_repo.delete(variable_id).await?;
        Ok(())
    }
}

// ---- parent existence checks ----
//
// Every create checks its parent before the insert. A missing parent
// is a 404 (the client named a resource that is not there), and the
// database foreign key remains the backstop for the race where the
// parent is deleted between the check and the insert.

async fn ensure_version_exists<V>(repo: &V, version_id: i64) -> Result<(), UsecaseError>
where
    V: SdtmProjectVersionRepository,
{
    match repo.find_by_id(version_id).await {
        Ok(_) => Ok(()),
        Err(DomainError::SdtmProjectVersionNotFound(_)) => Err(UsecaseError::Repository(
            DomainError::FkSdtmProjectVersionNotFound(version_id),
        )),
        Err(e) => Err(UsecaseError::Repository(e)),
    }
}

async fn ensure_domain_exists<D>(repo: &D, domain_id: i64) -> Result<(), UsecaseError>
where
    D: SdtmProjectDomainRepository,
{
    match repo.find_by_id(domain_id).await {
        Ok(_) => Ok(()),
        Err(DomainError::SdtmProjectDomainNotFound(_)) => Err(UsecaseError::Repository(
            DomainError::FkSdtmProjectDomainNotFound(domain_id),
        )),
        Err(e) => Err(UsecaseError::Repository(e)),
    }
}

async fn ensure_variable_exists<Va>(repo: &Va, variable_id: i64) -> Result<(), UsecaseError>
where
    Va: SdtmProjectVariableRepository,
{
    match repo.find_by_id(variable_id).await {
        Ok(_) => Ok(()),
        Err(DomainError::SdtmProjectVariableNotFound(_)) => Err(UsecaseError::Repository(
            DomainError::FkSdtmProjectVariableNotFound(variable_id),
        )),
        Err(e) => Err(UsecaseError::Repository(e)),
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateSdtmProjectVersion) -> Result<(), UsecaseError> {
    if cmd.project_code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyProjectCode));
    }
    if cmd.version_name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyVersionName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateSdtmProjectVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.version_name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyVersionName));
    }
    Ok(())
}

fn validate_create_domain(cmd: &CreateSdtmProjectDomain) -> Result<(), UsecaseError> {
    if cmd.domain_code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyDomainCode));
    }
    Ok(())
}

fn validate_update_domain(cmd: &UpdateSdtmProjectDomain) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.domain_code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyDomainCode));
    }
    if let Some(ref kv) = cmd.key_variables {
        SdtmProjectDomain::normalize_key_variables(kv).map_err(UsecaseError::Validation)?;
    }
    Ok(())
}

fn validate_create_variable(cmd: &CreateSdtmProjectVariable) -> Result<(), UsecaseError> {
    if cmd.variable_code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyVariableCode));
    }
    Ok(())
}

fn validate_update_variable(cmd: &UpdateSdtmProjectVariable) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.variable_code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyVariableCode));
    }
    Ok(())
}
