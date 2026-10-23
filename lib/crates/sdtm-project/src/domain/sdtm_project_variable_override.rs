use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::sdtm_variable_description::SdtmVariableDescription;
use super::variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};

/// A project's own override for one of its variables. Each variable
/// holds at most one override, enforced by `UNIQUE (variable_id)` on
/// `sdtm_project_variable_overrides`.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectVariableOverride {
    pub id: i64,
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectVariableOverride {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectVariableOverride")
            .field("id", &self.id)
            .field("variable_id", &self.variable_id)
            .field("variable_type", &self.variable_type)
            .field("variable_core", &self.variable_core)
            .field("variable_role", &self.variable_role)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectVariableOverride {
    /// Validating constructor used by the domain layer. There are no
    /// free-form string fields to reject, so this only builds the
    /// aggregate.
    pub fn new(
        variable_id: i64,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        descriptions: Vec<SdtmVariableDescription>,
    ) -> Result<Self, DomainError> {
        Ok(Self {
            id: 0,
            variable_id,
            variable_type,
            variable_core,
            variable_role,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    // Used by the adapter layer and the in-memory test fakes; both land
    // in later tasks.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        variable_id: i64,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        descriptions: Vec<SdtmVariableDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            variable_id,
            variable_type,
            variable_core,
            variable_role,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectVariableOverrideRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectVariableOverrideNew {
    pub variable_id: i64,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Input DTO for `SdtmProjectVariableOverrideRepository::update`. The
/// override is keyed on its **parent** id — a 1:1 sub-resource's
/// natural key — so the caller never has to know the surrogate id.
///
/// `variable_role` is three-state: outer `None` = leave unchanged,
/// `Some(None)` = clear the column to NULL, `Some(Some(role))` =
/// replace.
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectVariableOverrideUpdate {
    pub variable_id: i64,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}
