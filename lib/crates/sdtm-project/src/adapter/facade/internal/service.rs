//! `SdtmProjectService` adapter over the repository ports.
//!
//! Hosts `SdtmProjectServiceImpl<V, D, DO, Va, VO, P>` — the
//! implementation of `apis::sdtm_project::SdtmProjectService` that
//! adapts `sdtm_project::SdtmProjectUsecase` to the API contract.
//! Translation between `apis::sdtm_project::*` and
//! `sdtm_project::usecase::*` happens inline in each trait method.

use std::sync::Arc;

use async_trait::async_trait;

use apis::sdtm_project::{
    CreateSdtmProjectDomainRequest, CreateSdtmProjectVariableRequest,
    CreateSdtmProjectVersionRequest, SdtmProjectApiError, SdtmProjectDomainList,
    SdtmProjectDomainView, SdtmProjectService, SdtmProjectVariableList, SdtmProjectVariableView,
    SdtmProjectVersionList, SdtmProjectVersionView, UpdateSdtmProjectDomainRequest,
    UpdateSdtmProjectVariableRequest, UpdateSdtmProjectVersionRequest,
};

use crate::domain::{
    DomainError, ProjectLookup, SdtmProjectDomainOverrideRepository, SdtmProjectDomainRepository,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableRepository,
    SdtmProjectVersionRepository,
};
use crate::usecase::{
    CreateSdtmProjectDomain, CreateSdtmProjectVariable, CreateSdtmProjectVersion,
    SdtmProjectDomainView as InternalDomainView, SdtmProjectUsecase, SdtmProjectUsecaseConfig,
    SdtmProjectVariableView as InternalVariableView, SdtmProjectVersionView as InternalVersionView,
    UpdateSdtmProjectDomain, UpdateSdtmProjectVariable, UpdateSdtmProjectVersion, UsecaseError,
};

/// Adapter that implements [`SdtmProjectService`] on top of a
/// [`SdtmProjectUsecase`].
///
/// Generic over the five persistence ports plus the project lookup so
/// the adapter can be exercised against in-memory fakes in tests and
/// against the PostgreSQL-backed repositories in production.
pub struct SdtmProjectServiceImpl<
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
> {
    usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>,
}

impl<V, D, DO, Va, VO, P> SdtmProjectServiceImpl<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    /// Build a new `SdtmProjectServiceImpl` wrapping the supplied
    /// usecase.
    pub fn new(usecase: SdtmProjectUsecase<V, D, DO, Va, VO, P>) -> Self {
        Self { usecase }
    }

    /// Build a new `SdtmProjectServiceImpl` from the five repositories
    /// and the project lookup directly. Mirrors the
    /// [`SdtmProjectUsecase::new`] constructor shape so callers who
    /// already hold the repositories do not need to assemble a
    /// [`SdtmProjectUsecaseConfig`] first.
    pub fn from_repos(
        version_repo: V,
        domain_repo: D,
        domain_override_repo: DO,
        variable_repo: Va,
        variable_override_repo: VO,
        projects: Arc<P>,
    ) -> Self {
        Self::new(SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
            version_repo,
            domain_repo,
            domain_override_repo,
            variable_repo,
            variable_override_repo,
            projects,
        }))
    }
}

// ---- enum mappers (api <-> domain) ----
//
// The mappers for the promoted metadata (category, variable type / core /
// role, descriptions) land with the standalone model; until then the
// override surface is the only consumer and it is gone.

// ---- view mappers ----

fn version_view_from_internal(view: InternalVersionView) -> SdtmProjectVersionView {
    SdtmProjectVersionView {
        id: view.id,
        project_code: view.project_code,
        version_name: view.version_name,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn domain_view_from_internal(view: InternalDomainView) -> SdtmProjectDomainView {
    SdtmProjectDomainView {
        id: view.id,
        version_id: view.version_id,
        domain_code: view.domain_code,
        key_variables: view.key_variables,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

fn variable_view_from_internal(view: InternalVariableView) -> SdtmProjectVariableView {
    SdtmProjectVariableView {
        id: view.id,
        domain_id: view.domain_id,
        variable_code: view.variable_code,
        variable_sequence: view.variable_sequence,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}

// ---- error mapper ----

/// Translate a [`UsecaseError`] into the API's [`SdtmProjectApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants. The validation-only variants can still
/// reach the `Repository` branch when a stored row carries a value the
/// schema's CHECK constraint did not catch, so that branch maps them
/// to `Repository` rather than panicking.
impl From<UsecaseError> for SdtmProjectApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => SdtmProjectApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => SdtmProjectApiError::NotFound,
                DomainError::ProjectNotFound(code) => SdtmProjectApiError::ProjectNotFound(code),
                DomainError::SdtmProjectVersionNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectVersionNotFound(id)
                }
                DomainError::SdtmProjectDomainNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectDomainNotFound(id)
                }
                DomainError::SdtmProjectVariableNotFound(id) => {
                    SdtmProjectApiError::SdtmProjectVariableNotFound(id)
                }
                // The override sub-resource is no longer part of the
                // API surface, so these variants have no apis
                // counterpart. They are unreachable through this
                // facade — the override endpoints are gone — and the
                // crate's internal override machinery goes away with
                // the domain model itself.
                DomainError::SdtmProjectDomainOverrideNotFound(_)
                | DomainError::SdtmProjectVariableOverrideNotFound(_)
                | DomainError::DuplicateSdtmProjectDomainOverride { .. }
                | DomainError::DuplicateSdtmProjectVariableOverride { .. } => {
                    SdtmProjectApiError::Repository(domain.to_string())
                }
                DomainError::DuplicateSdtmProjectVersion {
                    project_code,
                    version_name,
                } => SdtmProjectApiError::DuplicateSdtmProjectVersion {
                    project_code,
                    version_name,
                },
                DomainError::DuplicateSdtmProjectDomain {
                    version_id,
                    domain_code,
                } => SdtmProjectApiError::DuplicateSdtmProjectDomain {
                    version_id,
                    domain_code,
                },
                DomainError::DuplicateSdtmProjectVariable {
                    domain_id,
                    variable_code,
                } => SdtmProjectApiError::DuplicateSdtmProjectVariable {
                    domain_id,
                    variable_code,
                },
                DomainError::FkSdtmProjectVersionNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectVersionNotFound(id)
                }
                DomainError::FkSdtmProjectDomainNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectDomainNotFound(id)
                }
                DomainError::FkSdtmProjectVariableNotFound(id) => {
                    SdtmProjectApiError::FkSdtmProjectVariableNotFound(id)
                }
                DomainError::EmptyProjectCode
                | DomainError::EmptyVersionName
                | DomainError::EmptyDomainCode
                | DomainError::EmptyVariableCode
                | DomainError::EmptyKeyVariable
                | DomainError::InvalidDomainCategory(_)
                | DomainError::InvalidVariableType(_)
                | DomainError::InvalidVariableCore(_)
                | DomainError::InvalidVariableRole(_) => {
                    SdtmProjectApiError::Repository(domain.to_string())
                }
                // A repository-level failure is already an
                // infrastructure error: pass the message straight
                // through.
                DomainError::Repository(msg) => SdtmProjectApiError::Repository(msg),
            },
        }
    }
}

#[async_trait]
impl<V, D, DO, Va, VO, P> SdtmProjectService for SdtmProjectServiceImpl<V, D, DO, Va, VO, P>
where
    V: SdtmProjectVersionRepository,
    D: SdtmProjectDomainRepository,
    DO: SdtmProjectDomainOverrideRepository,
    Va: SdtmProjectVariableRepository,
    VO: SdtmProjectVariableOverrideRepository,
    P: ProjectLookup,
{
    // ---- SdtmProjectVersion ----

    async fn create_version(
        &self,
        req: CreateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectVersion {
            project_code: req.project_code,
            version_name: req.version_name,
        };
        let view = self.usecase.create_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let view = self.usecase.get_version_by_id(id).await?;
        Ok(version_view_from_internal(view))
    }

    async fn list_versions_by_project(
        &self,
        project_code: &str,
    ) -> Result<SdtmProjectVersionList, SdtmProjectApiError> {
        let views = self.usecase.list_versions_by_project(project_code).await?;
        Ok(SdtmProjectVersionList {
            versions: views.into_iter().map(version_view_from_internal).collect(),
        })
    }

    async fn update_version(
        &self,
        req: UpdateSdtmProjectVersionRequest,
    ) -> Result<SdtmProjectVersionView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectVersion {
            id: req.id,
            version_name: req.version_name,
        };
        let view = self.usecase.update_version(cmd).await?;
        Ok(version_view_from_internal(view))
    }

    async fn delete_version(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_version(id).await?;
        Ok(())
    }

    // ---- SdtmProjectDomain ----

    async fn create_domain(
        &self,
        req: CreateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectDomain {
            version_id: req.version_id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        };
        let view = self.usecase.create_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn get_domain_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let view = self.usecase.get_domain_by_id(id).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<SdtmProjectDomainList, SdtmProjectApiError> {
        let views = self.usecase.list_domains_by_version(version_id).await?;
        Ok(SdtmProjectDomainList {
            domains: views.into_iter().map(domain_view_from_internal).collect(),
        })
    }

    async fn update_domain(
        &self,
        req: UpdateSdtmProjectDomainRequest,
    ) -> Result<SdtmProjectDomainView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectDomain {
            id: req.id,
            domain_code: req.domain_code,
            key_variables: req.key_variables,
        };
        let view = self.usecase.update_domain(cmd).await?;
        Ok(domain_view_from_internal(view))
    }

    async fn delete_domain(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_domain(id).await?;
        Ok(())
    }

    // ---- SdtmProjectVariable ----

    async fn create_variable(
        &self,
        req: CreateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let cmd = CreateSdtmProjectVariable {
            domain_id: req.domain_id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        };
        let view = self.usecase.create_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn get_variable_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let view = self.usecase.get_variable_by_id(id).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<SdtmProjectVariableList, SdtmProjectApiError> {
        let views = self.usecase.list_variables_by_domain(domain_id).await?;
        Ok(SdtmProjectVariableList {
            variables: views.into_iter().map(variable_view_from_internal).collect(),
        })
    }

    async fn update_variable(
        &self,
        req: UpdateSdtmProjectVariableRequest,
    ) -> Result<SdtmProjectVariableView, SdtmProjectApiError> {
        let cmd = UpdateSdtmProjectVariable {
            id: req.id,
            variable_code: req.variable_code,
            variable_sequence: req.variable_sequence,
        };
        let view = self.usecase.update_variable(cmd).await?;
        Ok(variable_view_from_internal(view))
    }

    async fn delete_variable(&self, id: i64) -> Result<(), SdtmProjectApiError> {
        self.usecase.delete_variable(id).await?;
        Ok(())
    }
}
