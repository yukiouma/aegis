use async_trait::async_trait;

use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectConfigurationData, ProjectLanguage as ApiProjectLanguage,
    ProjectMemberData, ProjectMemberView as ApiProjectMemberView, ProjectService, ProjectView,
    TagData, UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{
    ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectRepository, ProjectTag,
    UserService,
};
use crate::usecase::{
    CreateProject, ProjectConfigurationView, ProjectUsecase, UpdateProject,
    UserSummaryView as DomainUserSummaryView,
};

/// Facade adapting `ProjectUsecase<R, U>` to
/// `apis::project::ProjectService`. The construction is the same
/// regardless of the underlying storage: the generic `R / U`
/// arguments stay concrete in the caller.
pub struct ProjectServiceImpl<R, U>
where
    R: ProjectRepository,
    U: UserService,
{
    usecase: ProjectUsecase<R, U>,
}

impl<R, U> ProjectServiceImpl<R, U>
where
    R: ProjectRepository,
    U: UserService,
{
    pub fn new(usecase: ProjectUsecase<R, U>) -> Self {
        Self { usecase }
    }
}

#[async_trait]
impl<R, U> ProjectService for ProjectServiceImpl<R, U>
where
    R: ProjectRepository + 'static,
    U: UserService + 'static,
{
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .create_project(CreateProject {
                code: req.code,
                description: req.description,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                configuration: req.configurations.map(configuration_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError> {
        let view = self.usecase.get_project_by_id(id).await.map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_code(code)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
        let views = self.usecase.list_projects().await.map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .update_project(UpdateProject {
                id: req.id,
                code: req.code,
                description: req.description,
                active: req.active,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                configuration: req.configurations.map(configuration_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn member_data_to_domain(d: ProjectMemberData) -> ProjectMember {
    ProjectMember::for_repository(d.leaders, d.workers)
}

/// Bridge for the request-side `ProjectConfigurationData` so the
/// apis port doesn't need to reach into the domain types. The
/// usecase / domain layer re-validates the inner tags via
/// `ProjectTag::new`; if the wire payload violated the non-empty
/// contract, that re-validation surfaces as
/// `UsecaseError::Validation(EmptyTagKey | EmptyTagValue)`.
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(
        d.language.map(api_language_to_domain),
        tag_data_vec_to_domain(d.tags),
    )
}

fn api_language_to_domain(l: ApiProjectLanguage) -> ProjectLanguage {
    match l {
        ApiProjectLanguage::English => ProjectLanguage::English,
        ApiProjectLanguage::SimplifiedChinese => ProjectLanguage::SimplifiedChinese,
    }
}

fn tag_data_vec_to_domain(tags: Vec<TagData>) -> Vec<ProjectTag> {
    tags.into_iter()
        .map(|t| ProjectTag::for_repository(t.key, t.value))
        .collect()
}

fn map_error(err: crate::usecase::UsecaseError) -> ProjectApiError {
    use crate::domain::DomainError;
    use crate::usecase::UsecaseError;
    match err {
        UsecaseError::Validation(d) => ProjectApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => ProjectApiError::NotFound,
            DomainError::UserNotFound(code) => ProjectApiError::UserNotFound(code),
            DomainError::DuplicateCode(code) => ProjectApiError::DuplicateCode(code),
            other => ProjectApiError::Repository(other.to_string()),
        },
    }
}

// ---- From impls: domain usecase views -> apis views ----

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            configurations: v.configurations.into(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView {
    fn from(v: ProjectConfigurationView) -> Self {
        Self {
            language: v.language.map(domain_language_to_api),
            tags: v.tags.into_iter().map(Into::into).collect(),
        }
    }
}

fn domain_language_to_api(l: ProjectLanguage) -> ApiProjectLanguage {
    match l {
        ProjectLanguage::English => ApiProjectLanguage::English,
        ProjectLanguage::SimplifiedChinese => ApiProjectLanguage::SimplifiedChinese,
    }
}

impl From<crate::usecase::TagView> for apis::project::TagView {
    fn from(v: crate::usecase::TagView) -> Self {
        Self {
            key: v.key,
            value: v.value,
        }
    }
}

impl From<crate::usecase::ProjectMemberView> for ApiProjectMemberView {
    fn from(v: crate::usecase::ProjectMemberView) -> Self {
        Self {
            leaders: v.leaders.into_iter().map(Into::into).collect(),
            workers: v.workers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DomainUserSummaryView> for ApiUserSummaryView {
    fn from(v: DomainUserSummaryView) -> Self {
        Self {
            code: v.code,
            name: v.name,
        }
    }
}