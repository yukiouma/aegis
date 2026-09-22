use serde::{Deserialize, Serialize};

use super::model_version::ModelVersion;
use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;

/// Composite of the project's optional locale, its tag list, and
/// the SDTM-IG version it targets.
///
/// The struct owns no domain rule beyond composition: the inner
/// `ProjectTag` already enforces non-empty key / value when tags
/// arrive via the validating constructor on the wire, so no
/// separate `ProjectConfiguration::new` is needed. Constructors
/// skip validation because the data is either trusted (adapter
/// materialising from JSONB) or already validated (usecase after
/// `ProjectTag::new`).
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
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream
    /// test code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
        sdtmig: Option<ModelVersion>,
    ) -> Self {
        Self {
            language,
            tags,
            sdtmig,
        }
    }
}
