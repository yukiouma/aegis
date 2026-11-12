use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Where a project variable's value comes from.
///
/// Adjacently tagged: the unit variants are flat strings, the two
/// payload variants carry an optional `from` object. The variant
/// spellings are pinned per-variant — no blanket `rename_all` can
/// produce both `CRF` and `eDT`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "from")]
pub enum VariableSource {
    /// The value is specified by the protocol.
    #[serde(rename = "Protocol")]
    Protocol,
    /// The value is computed from other variables.
    #[serde(rename = "Derived")]
    Derived,
    /// The value is assigned manually, optionally from a CRF form.
    #[serde(rename = "Assigned")]
    Assigned(Option<FromCrf>),
    /// The value is collected on a CRF form, optionally a specific one.
    #[serde(rename = "CRF")]
    Crf(Option<FromCrf>),
    /// The value comes from an eDiT.
    #[serde(rename = "eDT")]
    Edt,
}

/// The CRF form a source draws from.
///
/// `form_code` is the code field of the CRF form. It is **not**
/// validated against the `crf` crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromCrf {
    pub form_code: String,
}

/// One rule that turns a source into a variable's value.
///
/// `rule` is opaque text: this crate neither parses nor evaluates it.
///
/// The source is **flattened** into the rule, so a rule serialises as
/// `{"source": "CRF", "from": {…}, "rule": "…"}` — the source's own
/// fields sit beside the rule text. Without the flatten the source
/// would nest under a second `source` key.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionRule {
    #[serde(flatten)]
    pub source: VariableSource,
    pub rule: String,
}

impl ConversionRule {
    /// Validating constructor used by the domain layer. Rejects a
    /// blank `rule` — a rule with no expression carries no information.
    pub fn new(source: VariableSource, rule: String) -> Result<Self, DomainError> {
        if rule.trim().is_empty() {
            return Err(DomainError::EmptyConversionRule);
        }
        Ok(Self { source, rule })
    }
}
