use std::collections::HashMap;

use crate::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, UserService, UserSummary,
};

use super::commands::{CreateProduct, CreateProject, UpdateProduct, UpdateProject};
use super::error::UsecaseError;
use super::views::{ProductView, ProjectMemberView, ProjectView};

pub struct ProjectUsecaseConfig<P: ProductRepository, R: ProjectRepository, U: UserService> {
    pub product_repo: P,
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<P: ProductRepository, R: ProjectRepository, U: UserService> {
    product_repo: P,
    project_repo: R,
    users: U,
}

impl<P: ProductRepository, R: ProjectRepository, U: UserService> ProjectUsecase<P, R, U> {
    pub fn new(cfg: ProjectUsecaseConfig<P, R, U>) -> Self {
        Self {
            product_repo: cfg.product_repo,
            project_repo: cfg.project_repo,
            users: cfg.users,
        }
    }

    // -------- Products --------

    pub async fn create_product(
        &self,
        cmd: CreateProduct,
    ) -> Result<ProductView, UsecaseError> {
        validate_create_product(&cmd)?;
        let product = self
            .product_repo
            .create(ProductNew {
                code: cmd.code,
                name: cmd.name,
                description: cmd.description,
            })
            .await?;
        Ok(product.into())
    }

    pub async fn get_product_by_id(&self, id: i32) -> Result<ProductView, UsecaseError> {
        let product = self.product_repo.find_by_id(id).await?;
        Ok(product.into())
    }

    pub async fn get_product_by_code(&self, code: &str) -> Result<ProductView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let product = self.product_repo.find_by_code(code).await?;
        Ok(product.into())
    }

    pub async fn list_products(&self) -> Result<Vec<ProductView>, UsecaseError> {
        let products = self.product_repo.list().await?;
        Ok(products.into_iter().map(ProductView::from).collect())
    }

    pub async fn update_product(
        &self,
        cmd: UpdateProduct,
    ) -> Result<ProductView, UsecaseError> {
        validate_update_product(&cmd)?;
        let product = self
            .product_repo
            .update(ProductUpdate {
                id: cmd.id,
                code: cmd.code,
                name: cmd.name,
                description: cmd.description,
                active: cmd.active,
            })
            .await?;
        Ok(product.into())
    }

    // -------- Projects --------

    pub async fn create_project(
        &self,
        cmd: CreateProject,
    ) -> Result<ProjectView, UsecaseError> {
        validate_create_project(&cmd)?;
        // Surface `ProductNotFound` early; the FK would catch it later
        // but failing here gives a clearer error path.
        let product = self
            .product_repo
            .find_by_id(cmd.product_id)
            .await
            .map_err(|err| match err {
                DomainError::NotFound => UsecaseError::Repository(DomainError::ProductNotFound(
                    cmd.product_id.to_string(),
                )),
                other => UsecaseError::Repository(other),
            })?;

        let new_project = self
            .project_repo
            .create(ProjectNew {
                code: cmd.code,
                description: cmd.description,
                product_id: cmd.product_id,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
            })
            .await?;

        self.hydrate_project_view(new_project, Some(product)).await
    }

    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError> {
        let project = self.project_repo.find_by_id(id).await?;
        self.hydrate_project_view(project, None).await
    }

    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let project = self.project_repo.find_by_code(code).await?;
        self.hydrate_project_view(project, None).await
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError> {
        let projects = self.project_repo.list().await?;
        // One user-service round-trip per call; bucket the codes into
        // each project's two teams afterwards.
        let all_users = self.users.list().await?;
        let mut out = Vec::with_capacity(projects.len());
        for project in projects {
            let product = self.product_repo.find_by_id(project.product_id).await?;
            let view = hydrate_with(&all_users, project, product)?;
            out.push(view);
        }
        Ok(out)
    }

    pub async fn update_project(
        &self,
        cmd: UpdateProject,
    ) -> Result<ProjectView, UsecaseError> {
        validate_update_project(&cmd)?;
        let updated = self
            .project_repo
            .update(ProjectUpdate {
                id: cmd.id,
                code: cmd.code,
                description: cmd.description,
                product_id: cmd.product_id,
                active: cmd.active,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
            })
            .await?;
        self.hydrate_project_view(updated, None).await
    }

    // -------- helpers --------

    async fn hydrate_project_view(
        &self,
        project: Project,
        product: Option<Product>,
    ) -> Result<ProjectView, UsecaseError> {
        let product = match product {
            Some(p) => p,
            None => self.product_repo.find_by_id(project.product_id).await?,
        };
        let all_users = self.users.list().await?;
        hydrate_with(&all_users, project, product)
    }
}

/// Bucket the supplied user summaries into a project's two teams and
/// produce a `ProjectView`. Pure (no I/O) so tests can exercise it
/// directly through the usecase.
fn hydrate_with(
    users: &[UserSummary],
    project: Project,
    product: Product,
) -> Result<ProjectView, UsecaseError> {
    let by_code: HashMap<&str, &UserSummary> =
        users.iter().map(|u| (u.code.as_str(), u)).collect();
    let members = project.members.clone();
    let unblind_members = project.unblind_members.clone();

    let leaders: Vec<UserSummary> = lookup_set(&by_code, &members.leaders)?;
    let workers: Vec<UserSummary> = lookup_set(&by_code, &members.workers)?;
    let members_view = ProjectMemberView {
        leaders: leaders.into_iter().map(Into::into).collect(),
        workers: workers.into_iter().map(Into::into).collect(),
    };

    let unblind_leaders: Vec<UserSummary> =
        lookup_set(&by_code, &unblind_members.leaders)?;
    let unblind_workers: Vec<UserSummary> =
        lookup_set(&by_code, &unblind_members.workers)?;
    let unblind_view = ProjectMemberView {
        leaders: unblind_leaders.into_iter().map(Into::into).collect(),
        workers: unblind_workers.into_iter().map(Into::into).collect(),
    };

    Ok(ProjectView::from_project(
        project,
        product,
        members_view,
        unblind_view,
    ))
}

fn lookup_set<'a>(
    by_code: &HashMap<&'a str, &'a UserSummary>,
    codes: &[String],
) -> Result<Vec<UserSummary>, UsecaseError> {
    let mut out = Vec::with_capacity(codes.len());
    for code in codes {
        match by_code.get(code.as_str()) {
            Some(summary) => out.push((*summary).clone()),
            None => {
                return Err(UsecaseError::Repository(DomainError::UserNotFound(
                    code.clone(),
                )));
            }
        }
    }
    Ok(out)
}

fn validate_create_product(cmd: &CreateProduct) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_product(cmd: &UpdateProduct) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref n) = cmd.name
        && n.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_project(cmd: &CreateProject) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.product_id == 0 {
        return Err(UsecaseError::Validation(DomainError::ZeroProductId));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    Ok(())
}

fn validate_update_project(cmd: &UpdateProject) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(pid) = cmd.product_id
        && pid == 0
    {
        return Err(UsecaseError::Validation(DomainError::ZeroProductId));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    Ok(())
}
