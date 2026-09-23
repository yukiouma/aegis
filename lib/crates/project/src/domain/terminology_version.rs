use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Pointer to a `TerminologyVersion` a project targets. Mirrors
/// `ModelVersion` shape-wise but carries a distinct Rust type so the
/// wire field name (`sdtm_terminology`) can diverge from `sdtmig`
/// without alias gymnastics, and so the two pointer types can evolve
/// validation independently.
///
/// `version_id` is the surrogate key from
/// `terminology::TerminologyVersion::id`; `version_name` is the
/// human-readable workbook suffix (e.g. "2024-03-29"). The domain
/// layer does NOT verify the row still exists at write time — the
/// trust contract matches `ModelVersion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionData {
    pub version_id: i64,
    pub version_name: String,
}

impl TerminologyVersionData {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs). Rejects
    /// empty / whitespace `version_name`.
    pub fn new(version_id: i64, version_name: String) -> Result<Self, DomainError> {
        if version_name.trim().is_empty() {
            return Err(DomainError::EmptySdtmTerminologyName);
        }
        Ok(Self {
            version_id,
            version_name,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column, and for downstream
    /// test / integration code that needs to construct a pointer
    /// from a trusted source.
    #[allow(dead_code)]
    pub fn for_repository(version_id: i64, version_name: String) -> Self {
        Self {
            version_id,
            version_name,
        }
    }
}
