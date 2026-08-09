//! Outbound port for product / project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`ProjectApiError`, `ProductView`, `ProjectView`,
//! `ProjectMemberView`, `UserSummaryView`, `*Request`) are defined
//! alongside the trait so a single `use apis::project::*;` brings the
//! whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Error surface returned by every [`ProjectService`] method.
///
/// Adapters map backend-specific errors (e.g. `project::UsecaseError`)
/// into this type at the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum ProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("product not found: {0}")]
    ProductNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Safe projection of a product — every field is safe to log today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a project: the parent `ProductView` is
/// denormalised in, and the membership lists are hydrated to
/// `Vec<UserSummaryView>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductView,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

/// Wire-shaped membership data. `leaders` and `workers` are user codes
/// (not full user records); the backend hydrates them to
/// `UserSummaryView` on read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMemberData {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProductRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. Omit (or pass an empty `ProjectMemberData`) to create
    /// the project with no membership rows; the shell can be filled in
    /// via a later `update_project` call.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
}

/// Outbound port for product / project lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn ProjectService>` can be shared state in
/// an async server (axum, tarpc, etc.).
#[async_trait]
pub trait ProjectService: Send + Sync {
    // Products
    async fn create_product(
        &self,
        req: CreateProductRequest,
    ) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_id(&self, id: i32) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_code(&self, code: &str) -> Result<ProductView, ProjectApiError>;
    async fn list_products(&self) -> Result<Vec<ProductView>, ProjectApiError>;
    async fn update_product(
        &self,
        req: UpdateProductRequest,
    ) -> Result<ProductView, ProjectApiError>;

    // Projects
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError>;
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError>;
    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
}
