use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("{0}")]
    Domain(#[source] DomainError),

    #[error("forbidden: user {user_code} is not a leader of project {project_code}")]
    Forbidden {
        user_code: String,
        project_code: String,
    },
}

impl From<DomainError> for UsecaseError {
    fn from(e: DomainError) -> Self {
        UsecaseError::Domain(e)
    }
}