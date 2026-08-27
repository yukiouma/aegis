use thiserror::Error;

use super::crf_item_kind::CrfItemKind;

/// Exhaustive error surface for the crf domain. Used by
/// `UsecaseError` (as the inner of `Validation` / `Repository`)
/// and as the source of every per-aggregate `*NotFound`
/// mapping in `CrfApiError`.
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty project code")]
    EmptyProjectCode,
    #[error("empty name")]
    EmptyName,
    #[error("empty code")]
    EmptyCode,
    #[error("empty value")]
    EmptyValue,
    #[error("empty content")]
    EmptyContent,
    #[error("invalid crf item kind: {0}")]
    InvalidCrfItemKind(String),
    #[error("kind-shape violation: {kind:?} cannot carry {field}")]
    KindShapeViolation { kind: CrfItemKind, field: String },

    // ---- existence / FK / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("crf version not found: {0}")]
    CrfVersionNotFound(i32),
    #[error("crf form not found: {0}")]
    CrfFormNotFound(i32),
    #[error("crf item not found: {0}")]
    CrfItemNotFound(i32),
    #[error("crf option not found: {0}")]
    CrfOptionNotFound(i32),
    #[error("crf unit not found: {0}")]
    CrfUnitNotFound(i32),
    #[error("domain annotation not found: {0}")]
    DomainAnnotationNotFound(i32),
    #[error("annotation not found: {0}")]
    AnnotationNotFound(i32),

    #[error("crf version already exists: {project_code} / {name}")]
    DuplicateCrfVersion { project_code: String, name: String },
    #[error("crf form already exists: version {version_id} / {code}")]
    DuplicateCrfForm { version_id: i32, code: String },
    #[error("crf item already exists: form {form_id} / {code}")]
    DuplicateCrfItem { form_id: i32, code: String },
    #[error("domain annotation already exists: form {form_id} / {name}")]
    DuplicateDomainAnnotation { form_id: i32, name: String },

    #[error("referenced crf version not found: {0}")]
    FkCrfVersionNotFound(i32),
    #[error("referenced crf form not found: {0}")]
    FkCrfFormNotFound(i32),
    #[error("referenced crf item not found: {0}")]
    FkCrfItemNotFound(i32),
    #[error("referenced crf option not found: {0}")]
    FkCrfOptionNotFound(i32),
    #[error("referenced crf unit not found: {0}")]
    FkCrfUnitNotFound(i32),
    #[error("referenced domain annotation not found: {0}")]
    FkDomainAnnotationNotFound(i32),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
