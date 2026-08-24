use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("name must not be empty")]
    EmptyName,

    #[error("invalid domain category: {0}")]
    InvalidDomainCategory(String),
    #[error("invalid variable type: {0}")]
    InvalidVariableType(String),
    #[error("invalid variable core: {0}")]
    InvalidVariableCore(String),
    #[error("invalid variable role: {0}")]
    InvalidVariableRole(String),

    #[error("not found")]
    NotFound,
    #[error("sdtm version not found: {0}")]
    SdtmVersionNotFound(i64),
    #[error("sdtm domain not found: {0}")]
    SdtmDomainNotFound(i64),
    #[error("sdtm variable not found: {0}")]
    SdtmVariableNotFound(i64),

    #[error("sdtm version already exists: {name}")]
    DuplicateSdtmVersion { name: String },
    #[error("sdtm domain already exists for version {version_id} / {name}")]
    DuplicateSdtmDomain { version_id: i64, name: String },
    #[error("sdtm variable already exists for domain {domain_id} / {name}")]
    DuplicateSdtmVariable { domain_id: i64, name: String },

    #[error("referenced sdtm version not found: {0}")]
    FkSdtmVersionNotFound(i64),
    #[error("referenced sdtm domain not found: {0}")]
    FkSdtmDomainNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}
