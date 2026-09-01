use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{ProjectApiError, ProjectService};

use crate::domain::{DomainError, ProjectLookup};

/// Adapter that maps the apis `ProjectService` port onto the
/// narrow domain `ProjectLookup` port. The mission crate never
/// reaches apis `project` types directly; everything flows
/// through this struct so the domain layer stays free of `apis`
/// references.
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

    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError> {
        let view = self
            .projects
            .get_project_by_code(project_code)
            .await
            .map_err(|e| match e {
                ProjectApiError::NotFound => DomainError::ProjectNotFound(project_code.to_string()),
                other => DomainError::Repository(other.to_string()),
            })?;
        Ok(view.members.leaders.iter().any(|u| u.code == user_code))
    }
}

#[cfg(test)]
mod tests;
