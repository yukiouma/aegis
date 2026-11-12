use thiserror::Error;

/// Exhaustive error surface for the sdtm-project domain. Used by
/// `UsecaseError` (as the inner of `Validation` / `Repository`) and as
/// the source of every per-aggregate `*NotFound` mapping in
/// `SdtmProjectApiError`.
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty project code")]
    EmptyProjectCode,
    #[error("empty version name")]
    EmptyVersionName,
    #[error("empty domain code")]
    EmptyDomainCode,
    #[error("empty variable code")]
    EmptyVariableCode,
    #[error("empty key variable code")]
    EmptyKeyVariable,
    #[error("invalid domain category: {0}")]
    InvalidDomainCategory(String),
    #[error("invalid variable type: {0}")]
    InvalidVariableType(String),
    #[error("invalid variable core: {0}")]
    InvalidVariableCore(String),
    #[error("invalid variable role: {0}")]
    InvalidVariableRole(String),
    #[error("empty conversion rule")]
    EmptyConversionRule,

    // ---- existence / FK / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("sdtm project version not found: {0}")]
    SdtmProjectVersionNotFound(i64),
    #[error("sdtm project domain not found: {0}")]
    SdtmProjectDomainNotFound(i64),
    #[error("sdtm project variable not found: {0}")]
    SdtmProjectVariableNotFound(i64),

    #[error("key variable not found in domain: {variable_code}")]
    KeyVariableNotFound { variable_code: String },

    #[error("variable is referenced by key variables: {variable_code}")]
    VariableReferencedByKeyVariables { variable_code: String },

    #[error("sdtm project version already exists: {project_code} / {version_name}")]
    DuplicateSdtmProjectVersion {
        project_code: String,
        version_name: String,
    },
    #[error("sdtm project domain already exists: version {version_id} / {domain_code}")]
    DuplicateSdtmProjectDomain {
        version_id: i64,
        domain_code: String,
    },
    #[error("sdtm project variable already exists: domain {domain_id} / {variable_code}")]
    DuplicateSdtmProjectVariable {
        domain_id: i64,
        variable_code: String,
    },

    #[error("referenced sdtm project version not found: {0}")]
    FkSdtmProjectVersionNotFound(i64),
    #[error("referenced sdtm project domain not found: {0}")]
    FkSdtmProjectDomainNotFound(i64),
    #[error("referenced sdtm project variable not found: {0}")]
    FkSdtmProjectVariableNotFound(i64),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
