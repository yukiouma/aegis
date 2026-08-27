use thiserror::Error;

use crate::domain::DomainError;

/// Two-variant error enum returned by every `CrfUsecase`
/// method. Adapters can exhaustively match.
///
/// - `Validation(DomainError)` — a domain invariant was
///   violated before the call hit persistence.
/// - `Repository(DomainError)` — the persistence / adapter
///   layer surfaced a failure (including "contract broken
///   upstream" cases).
#[derive(Debug, Clone, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),
    #[error("repository failed: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    /// Maps `DomainError` into `UsecaseError` using the
    /// convention that "contract broken upstream" (i.e. a
    /// validating constructor was bypassed) is `Repository`.
    fn from(e: DomainError) -> Self {
        match e {
            DomainError::EmptyProjectCode
            | DomainError::EmptyName
            | DomainError::EmptyCode
            | DomainError::EmptyValue
            | DomainError::EmptyContent
            | DomainError::InvalidCrfItemKind(_)
            | DomainError::KindShapeViolation { .. } => Self::Validation(e),
            other => Self::Repository(other),
        }
    }
}
