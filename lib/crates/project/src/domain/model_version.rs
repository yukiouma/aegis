use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Pointer to the SDTM Implementation Guide (SDTM-IG) version a
/// project targets. `version_id` is the surrogate key from
/// `domain_model::SdtmVersion::id`; `version_name` is the
/// human-readable workbook suffix (e.g. `"2024-03-29"`). The two
/// fields travel together so callers can render the version name
/// without re-querying the domain-model crate.
///
/// The domain layer does NOT verify that `(version_id,
/// version_name)` actually exists in the `sdtm_versions` table — the
/// project crate trusts the caller (matching how `ProjectTag::new`
/// only enforces non-empty strings, not cross-record references).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersion {
    pub version_id: i64,
    pub version_name: String,
}

impl ModelVersion {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects empty / whitespace `version_name`. `version_id` is
    /// accepted as-is because no cross-table check is performed.
    pub fn new(version_id: i64, version_name: String) -> Result<Self, DomainError> {
        if version_name.trim().is_empty() {
            return Err(DomainError::EmptySdtmigName);
        }
        Ok(Self {
            version_id,
            version_name,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column, and for downstream
    /// test / integration code that needs to construct a version
    /// pointer from a trusted source.
    #[allow(dead_code)]
    pub fn for_repository(version_id: i64, version_name: String) -> Self {
        Self {
            version_id,
            version_name,
        }
    }
}
