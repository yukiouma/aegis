use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("user code must not be empty")]
    EmptyCode,

    #[error("password hash must not be empty")]
    EmptyPasswordHash,

    #[error("invalid role: {0}")]
    InvalidRole(String),

    #[error("not found")]
    NotFound,

    #[error("user code already exists: {0}")]
    DuplicateCode(String),

    #[error("user is inactive")]
    Inactive,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("domain is not allowed: {0}")]
    DomainNotAllowed(String),

    #[error("repository error: {0}")]
    Repository(String),
}
