use async_trait::async_trait;

use apis::project::{
    CreateProductRequest, CreateProjectRequest, ProductView as ApiProductView, ProjectApiError,
    ProjectMemberData, ProjectMemberView as ApiProjectMemberView, ProjectService, ProjectView,
    UpdateProductRequest, UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{ProductRepository, ProjectMember, ProjectRepository, UserService};
use crate::usecase::{
    CreateProduct, CreateProject, ProjectUsecase, UpdateProduct, UpdateProject,
    UserSummaryView as DomainUserSummaryView,
};

/// Facade adapting `ProjectUsecase<P, R, U>` to
/// `apis::project::ProjectService`. The construction is the same
/// regardless of the underlying storage: the generic `P / R / U`
/// arguments stay concrete in the caller.
pub struct ProjectServiceImpl<P, R, U>
where
    P: ProductRepository,
    R: ProjectRepository,
    U: UserService,
{
    usecase: ProjectUsecase<P, R, U>,
}

impl<P, R, U> ProjectServiceImpl<P, R, U>
where
    P: ProductRepository,
    R: ProjectRepository,
    U: UserService,
{
    pub fn new(usecase: ProjectUsecase<P, R, U>) -> Self {
        Self { usecase }
    }
}

#[async_trait]
impl<P, R, U> ProjectService for ProjectServiceImpl<P, R, U>
where
    P: ProductRepository + 'static,
    R: ProjectRepository + 'static,
    U: UserService + 'static,
{
    async fn create_product(
        &self,
        req: CreateProductRequest,
    ) -> Result<ApiProductView, ProjectApiError> {
        let view = self
            .usecase
            .create_product(CreateProduct {
                code: req.code,
                name: req.name,
                description: req.description,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_product_by_id(&self, id: i32) -> Result<ApiProductView, ProjectApiError> {
        let view = self
            .usecase
            .get_product_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_product_by_code(&self, code: &str) -> Result<ApiProductView, ProjectApiError> {
        let view = self
            .usecase
            .get_product_by_code(code)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_products(&self) -> Result<Vec<ApiProductView>, ProjectApiError> {
        let views = self.usecase.list_products().await.map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_product(
        &self,
        req: UpdateProductRequest,
    ) -> Result<ApiProductView, ProjectApiError> {
        let view = self
            .usecase
            .update_product(UpdateProduct {
                id: req.id,
                code: req.code,
                name: req.name,
                description: req.description,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .create_project(CreateProject {
                code: req.code,
                description: req.description,
                product_id: req.product_id,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_id(id)
            .await
            .map_err(map_error)?;
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
                product_id: req.product_id,
                active: req.active,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn member_data_to_domain(d: ProjectMemberData) -> ProjectMember {
    ProjectMember::for_repository(d.leaders, d.workers)
}

fn map_error(err: crate::usecase::UsecaseError) -> ProjectApiError {
    use crate::domain::DomainError;
    use crate::usecase::UsecaseError;
    match err {
        UsecaseError::Validation(d) => ProjectApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => ProjectApiError::NotFound,
            DomainError::ProductNotFound(id) => ProjectApiError::ProductNotFound(id),
            DomainError::UserNotFound(code) => ProjectApiError::UserNotFound(code),
            DomainError::DuplicateCode(code) => ProjectApiError::DuplicateCode(code),
            other => ProjectApiError::Repository(other.to_string()),
        },
    }
}

// ---- From impls: domain usecase views -> apis views ----

impl From<crate::usecase::ProductView> for ApiProductView {
    fn from(v: crate::usecase::ProductView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            name: v.name,
            description: v.description,
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            product: v.product.into(),
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
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
