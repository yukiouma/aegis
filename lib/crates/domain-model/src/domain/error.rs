use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("repository error: {0}")]
    Repository(String),
}
