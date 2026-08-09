use chrono::{DateTime, Utc};

use crate::domain::{Product, Project, UserSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

impl From<UserSummary> for UserSummaryView {
    fn from(s: UserSummary) -> Self {
        Self {
            code: s.code,
            name: s.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

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

impl From<Product> for ProductView {
    fn from(p: Product) -> Self {
        Self {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            active: p.active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

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

impl ProjectView {
    /// Build the parent Product + membership hydration around a domain
    /// `Project`. The product and user summaries are looked up via the
    /// supplied closures so the constructor stays testable without
    /// reaching for `ProductRepository` / `UserService` directly here.
    pub fn from_project(
        project: Project,
        product: Product,
        members: ProjectMemberView,
        unblind_members: ProjectMemberView,
    ) -> Self {
        Self {
            id: project.id,
            code: project.code,
            description: project.description,
            product: product.into(),
            members,
            unblind_members,
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

// Bring `ProjectMember` back into scope for any future use of the
// `Default` constructor in tests / mirrors.
#[allow(dead_code)]
fn _force_project_member_use(m: crate::domain::ProjectMember) -> crate::domain::ProjectMember {
    m
}
