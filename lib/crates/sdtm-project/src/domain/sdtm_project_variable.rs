use chrono::{DateTime, Utc};

use super::conversion_rule::ConversionRule;
use super::error::DomainError;
use super::sdtm_variable_description::SdtmVariableDescription;
use super::variable_type::{SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType};

/// A variable a project declares inside one of its domains.
///
/// The aggregate is a **complete copy** of the variable: it carries the
/// type, the core, the role, the controlled terminology and the
/// descriptions itself, rather than pointing at a standard variable and
/// patching it.
///
/// `variable_sequence` is the column order within the parent domain.
/// `conversion_rules` says where the variable's value comes from and
/// how it is derived.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmProjectVariable {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmProjectVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmProjectVariable")
            .field("id", &self.id)
            .field("sdtm_project_domain_id", &self.sdtm_project_domain_id)
            .field("variable_code", &self.variable_code)
            .field("variable_controlled", &self.variable_controlled)
            .field("variable_type", &self.variable_type)
            .field("variable_core", &self.variable_core)
            .field("variable_role", &self.variable_role)
            .field("variable_sequence", &self.variable_sequence)
            .field("descriptions", &self.descriptions)
            .field("conversion_rules", &self.conversion_rules)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmProjectVariable {
    /// Validating constructor used by the domain layer. Rejects an
    /// empty / whitespace `variable_code` and a blank conversion rule.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sdtm_project_domain_id: i64,
        variable_code: String,
        variable_controlled: Option<SdtmVariableControlled>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        conversion_rules: Vec<ConversionRule>,
    ) -> Result<Self, DomainError> {
        if variable_code.trim().is_empty() {
            return Err(DomainError::EmptyVariableCode);
        }
        for rule in &conversion_rules {
            if rule.rule.trim().is_empty() {
                return Err(DomainError::EmptyConversionRule);
            }
        }
        Ok(Self {
            id: 0,
            sdtm_project_domain_id,
            variable_code,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            conversion_rules,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        sdtm_project_domain_id: i64,
        variable_code: String,
        variable_controlled: Option<SdtmVariableControlled>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        conversion_rules: Vec<ConversionRule>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            sdtm_project_domain_id,
            variable_code,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            conversion_rules,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmProjectVariableRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmProjectVariableNew {
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
}

/// Input DTO for `SdtmProjectVariableRepository::update`.
/// `sdtm_project_domain_id` is immutable — a variable always belongs
/// to the domain it was created under.
///
/// `variable_controlled` and `variable_role` use `Option<Option<T>>` so
/// the caller can distinguish "don't change" (outer `None`) from
/// "clear the field" (outer `Some(None)`); the other fields use flat
/// `Option<T>` where `None` means "don't change" and `Some(value)`
/// means "replace".
#[derive(Debug, Clone, Default)]
pub struct SdtmProjectVariableUpdate {
    pub id: i64,
    pub variable_code: Option<String>,
    pub variable_controlled: Option<Option<SdtmVariableControlled>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
    pub conversion_rules: Option<Vec<ConversionRule>>,
}
