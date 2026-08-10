use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("user code must not be empty")]
    EmptyCode,

    #[error("user name must not be empty")]
    EmptyName,

    #[error("invalid role: {0}")]
    InvalidRole(String),

    #[error("user not found")]
    NotFound,

    #[error("user code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}
