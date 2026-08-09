//! Row -> domain conversion for the SQLx repositories.
//!
//! `ProductRow` and `ProjectRow` are the shapes returned by
//! `sqlx::query_as`. They are NOT re-exported at the crate root; only
//! the repositories use them.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, Product, Project, ProjectMember};

#[derive(Clone, FromRow)]
pub struct ProductRow {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProductRow> for Product {
    type Error = DomainError;

    fn try_from(row: ProductRow) -> Result<Self, Self::Error> {
        Ok(Product::for_repository(
            row.id,
            row.code,
            row.name,
            row.description,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Clone, FromRow)]
pub struct ProjectRow {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = DomainError;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project::for_repository(
            row.id,
            row.code,
            row.description,
            row.product_id,
            ProjectMember::default(),
            ProjectMember::default(),
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

/// One row from `project_members`.
#[derive(Clone, FromRow)]
#[allow(dead_code)]
pub struct ProjectMemberRow {
    pub project_id: i32,
    pub team_type: String,
    pub role_type: String,
    pub user_code: String,
}
