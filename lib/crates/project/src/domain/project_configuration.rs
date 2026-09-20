use serde::{Deserialize, Serialize};

use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;

/// Composite of the project's optional locale and its tag list.
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
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream
    /// test code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
    ) -> Self {
        Self { language, tags }
    }
}