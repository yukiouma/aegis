use std::sync::Arc;

use crate::domain::{
    ConversionRule, DomainError, ProjectLookup, SdtmProjectDomain, SdtmProjectDomainNew,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectVariable,
    SdtmProjectVariableNew, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersionNew, SdtmProjectVersionRepository, SdtmProjectVersionUpdate,
};

use super::commands::{
    CreateSdtmProjectDomain, CreateSdtmProjectVariable, CreateSdtmProjectVersion,
    UpdateSdtmProjectDomain, UpdateSdtmProjectVariable, UpdateSdtmProjectVersion,
};
use super::error::UsecaseError;
use super::views::{SdtmProjectDomainView, SdtmProjectVariableView, SdtmProjectVersionView};

/// Configuration for `SdtmProjectUsecase::new`. Wraps the three
/// repositories plus the project lookup so the constructor stays
/// readable.
pub struct SdtmProjectUsecaseConfig<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    Va: SdtmProjectVariableRepository,
    P: ProjectLookup,
> {
    pub version_repo: V,
    pub domain_repo: D,
    pub variable_repo: Va,
    /// `Arc<P>` rather than `P`, so the call site can hand over an
    /// `Arc<ProjectLookupImpl>` directly. Mirrors
    /// `crf::CrfUsecaseConfig`.
    pub projects: Arc<P>,
}

/// Async orchestration for the per-project SDTM model tree. Generic
/// over the three persistence ports plus the project lookup so tests
/// can inject in-memory fakes. Domain → view projection runs through the
/// `From` impls in `super::views`.
pub struct SdtmProjectUsecase<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    Va: SdtmProjectVariableRepository,
    P: ProjectLookup,
> {
    version_repo: V,
    domain_repo: D,
    variable_repo: Va,
    projects: Arc<P>,
}

impl<V, D, Va, P> SdtmProjectUsecase<V, D, Va, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    Va: SdtmProjectVariableRepository,
    P: ProjectLookup,
{
    pub fn new(cfg: SdtmProjectUsecaseConfig<V, D, Va, P>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            domain_repo: cfg.domain_repo,
            variable_repo: cfg.variable_repo,
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
        ensure_version_exists(&self.version_repo, cmd.sdtm_project_version_id).await?;
        // A brand-new domain has no variables yet, so any key variable
        // it names is missing by definition. Checked here rather than
        // through `ensure_key_variables_exist` because there is no
        // domain id to scope the lookup to.
        if let Some(code) = key_variables.first() {
            return Err(UsecaseError::Repository(DomainError::KeyVariableNotFound {
                variable_code: code.clone(),
            }));
        }
        let d = self
            .domain_repo
            .create(SdtmProjectDomainNew {
                sdtm_project_version_id: cmd.sdtm_project_version_id,
                domain_code: cmd.domain_code,
                category: cmd.category,
                descriptions: cmd.descriptions,
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
            Some(kv) => {
                let kv = SdtmProjectDomain::normalize_key_variables(kv)
                    .map_err(UsecaseError::Validation)?;
                // Read the domain before checking the codes: a missing
                // domain is a 404, and `list_by_domain` cannot tell an
                // empty domain from a nonexistent one.
                self.domain_repo.find_by_id(cmd.id).await?;
                ensure_key_variables_exist(&self.variable_repo, cmd.id, &kv).await?;
                Some(kv)
            }
        };
        let d = self
            .domain_repo
            .update(SdtmProjectDomainUpdate {
                id: cmd.id,
                domain_code: cmd.domain_code,
                category: cmd.category,
                descriptions: cmd.descriptions,
                key_variables,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError> {
        self.domain_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariable ----

    pub async fn create_variable(
        &self,
        cmd: CreateSdtmProjectVariable,
    ) -> Result<SdtmProjectVariableView, UsecaseError> {
        validate_create_variable(&cmd)?;
        ensure_domain_exists(&self.domain_repo, cmd.sdtm_project_domain_id).await?;
        let v = self
            .variable_repo
            .create(SdtmProjectVariableNew {
                sdtm_project_domain_id: cmd.sdtm_project_domain_id,
                variable_code: cmd.variable_code,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
                conversion_rules: cmd.conversion_rules,
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
        // A variable a domain names as a key variable cannot be
        // renamed out from under it. Re-sending the *same* code is not
        // a rename, so an ordinary metadata edit stays possible.
        if let Some(ref new_code) = cmd.variable_code {
            let current = self.variable_repo.find_by_id(cmd.id).await?;
            if new_code != &current.variable_code {
                ensure_not_a_key_variable(&self.domain_repo, &current).await?;
            }
        }
        let v = self
            .variable_repo
            .update(SdtmProjectVariableUpdate {
                id: cmd.id,
                variable_code: cmd.variable_code,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
                conversion_rules: cmd.conversion_rules,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError> {
        let current = self.variable_repo.find_by_id(id).await?;
        ensure_not_a_key_variable(&self.domain_repo, &current).await?;
        self.variable_repo.delete(id).await?;
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

/// Every code in `key_variables` must name a variable of `domain_id`.
///
/// The check is scoped to the domain being written — a code that
/// exists in a *sibling* domain is still missing here.
async fn ensure_key_variables_exist<Va>(
    repo: &Va,
    domain_id: i64,
    key_variables: &[String],
) -> Result<(), UsecaseError>
where
    Va: SdtmProjectVariableRepository,
{
    if key_variables.is_empty() {
        return Ok(());
    }
    let vars = repo.list_by_domain(domain_id).await?;
    for code in key_variables {
        if !vars.iter().any(|v| v.variable_code == *code) {
            return Err(UsecaseError::Repository(DomainError::KeyVariableNotFound {
                variable_code: code.clone(),
            }));
        }
    }
    Ok(())
}

/// A variable that a domain names as a key variable cannot be renamed
/// or deleted out from under it. The caller must drop the code from
/// `key_variables` first.
async fn ensure_not_a_key_variable<D>(
    domain_repo: &D,
    variable: &SdtmProjectVariable,
) -> Result<(), UsecaseError>
where
    D: SdtmProjectDomainRepository,
{
    let domain = domain_repo
        .find_by_id(variable.sdtm_project_domain_id)
        .await?;
    if domain
        .key_variables
        .iter()
        .any(|code| code == &variable.variable_code)
    {
        return Err(UsecaseError::Repository(
            DomainError::VariableReferencedByKeyVariables {
                variable_code: variable.variable_code.clone(),
            },
        ));
    }
    Ok(())
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
    validate_conversion_rules(&cmd.conversion_rules)?;
    Ok(())
}

fn validate_update_variable(cmd: &UpdateSdtmProjectVariable) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.variable_code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyVariableCode));
    }
    if let Some(ref rules) = cmd.conversion_rules {
        validate_conversion_rules(rules)?;
    }
    Ok(())
}

/// A rule with no expression carries no information.
fn validate_conversion_rules(rules: &[ConversionRule]) -> Result<(), UsecaseError> {
    for rule in rules {
        if rule.rule.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyConversionRule));
        }
    }
    Ok(())
}
