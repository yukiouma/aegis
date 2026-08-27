use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{ProjectApiError, ProjectService};

use crate::domain::{DomainError, ProjectLookup};

/// Adapter that implements the domain [`ProjectLookup`] port on
/// top of `apis::project::ProjectService`. Maps
/// `ProjectApiError::NotFound` to `DomainError::ProjectNotFound`;
/// every other error collapses to `DomainError::Repository`.
pub struct ProjectLookupImpl {
    projects: Arc<dyn ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(projects: Arc<dyn ProjectService>) -> Self {
        Self { projects }
    }
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.projects.get_project_by_code(code).await {
            Ok(_) => Ok(()),
            Err(ProjectApiError::NotFound) => Err(DomainError::ProjectNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}
