use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::DomainError;
use super::variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};

/// Localised description of an SDTM variable. Carried on the
/// `SdtmVariable` aggregate and persisted as a single JSONB
/// column on `sdtm_variables`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

/// A single SDTM variable (e.g. `AETERM`, `AESEV`) attached
/// to a `SdtmDomain`. `variable_sequence` is the column order
/// within the parent domain (1-based; the domain decides what
/// makes sense).
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmVariable {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmVariable")
            .field("id", &self.id)
            .field("domain_id", &self.domain_id)
            .field("name", &self.name)
            .field("variable_controlled", &self.variable_controlled)
            .field("variable_type", &self.variable_type)
            .field("variable_core", &self.variable_core)
            .field("variable_role", &self.variable_role)
            .field("variable_sequence", &self.variable_sequence)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmVariable {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain_id: i64,
        name: String,
        variable_controlled: Option<String>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            domain_id,
            name,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        domain_id: i64,
        name: String,
        variable_controlled: Option<String>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            domain_id,
            name,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmVariableRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmVariableNew {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Input DTO for `SdtmVariableRepository::update`. Every field
/// except `id` is optional so the usecase can pass only what
/// actually changed. `variable_controlled` and `variable_role`
/// use `Option<Option<T>>` so the caller can distinguish
/// "don't change" (outer `None`) from "clear the field" (outer
/// `Some(None)`); the other fields use flat `Option<T>` where
/// `None` means "don't change" and `Some(value)` means "replace".
#[derive(Debug, Clone, Default)]
pub struct SdtmVariableUpdate {
    pub id: i64,
    pub name: Option<String>,
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}
