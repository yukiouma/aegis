use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("code must not be empty")]
    EmptyCode,

    #[error("tag key must not be empty")]
    EmptyTagKey,

    #[error("tag value must not be empty")]
    EmptyTagValue,

    #[error("duplicate code in leaders: {0}")]
    DuplicateLeader(String),

    #[error("duplicate code in workers: {0}")]
    DuplicateWorker(String),

    #[error("unknown team type: {0}")]
    UnknownTeamType(String),

    #[error("unknown role type: {0}")]
    UnknownRoleType(String),

    #[error("not found")]
    NotFound,

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}