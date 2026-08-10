use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        UsecaseError::Repository(err)
    }
}
