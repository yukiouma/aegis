use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(DomainError),

    #[error("repository error: {0}")]
    Repository(DomainError),

    #[error("password hashing failed: {0}")]
    Hashing(String),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        UsecaseError::Repository(err)
    }
}
