use serde::{Deserialize, Serialize};

use super::model_version::ModelVersion;
use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;
use super::terminology_version::TerminologyVersionData;

/// Composite of the project's optional locale, its tag list, the
/// SDTM-IG version it targets, and the SDTM terminology release it
/// pins. The struct owns no domain rule beyond composition: the
/// inner `ProjectTag` already enforces non-empty key / value when
/// tags arrive via the validating constructor on the wire, so no
/// separate `ProjectConfiguration::new` is needed. Constructors skip
/// validation because the data is either trusted (adapter
/// materialising from JSONB) or already validated (usecase after
/// `ProjectTag::new` / `ModelVersion::new` /
/// `TerminologyVersionData::new`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectConfiguration {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<ProjectTag>,
    /// Optional pointer to the SDTM-IG version this project targets.
    /// `None` means "no SDTM-IG version configured". The domain
    /// layer does not verify the referenced version exists in the
    /// domain-model crate's `sdtm_versions` table; whatever the
    /// caller sends is saved verbatim.
    pub sdtmig: Option<ModelVersion>,
    /// Optional pointer to the SDTM controlled-terminology release
    /// this project pins. Same trust contract as `sdtmig` — the
    /// domain does not verify the referenced version exists in
    /// `terminology::terminology_versions` at write time.
    pub sdtm_terminology: Option<TerminologyVersionData>,
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream test
    /// code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
        sdtmig: Option<ModelVersion>,
        sdtm_terminology: Option<TerminologyVersionData>,
    ) -> Self {
        Self {
            language,
            tags,
            sdtmig,
            sdtm_terminology,
        }
    }
}
