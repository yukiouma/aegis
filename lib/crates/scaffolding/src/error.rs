/// Why a scaffold could not be built at all.
///
/// Every *name*-level problem — an unresolved field, a malformed
/// template, an invalid resolved name — is a skip, not an error. The
/// only hard failure is an empty `rootpath`: there is nothing to
/// scaffold relative to.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ScaffoldError {
    #[error("the structure has no root path")]
    EmptyRootpath,
}
