use chrono::{DateTime, Utc};

use crate::domain::{CodeItem, CodeList, TerminologyKind, TerminologyVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyVersionView {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TerminologyVersion> for TerminologyVersionView {
    fn from(v: TerminologyVersion) -> Self {
        Self {
            id: v.id,
            kind: v.kind,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeListView {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CodeList> for CodeListView {
    fn from(c: CodeList) -> Self {
        Self {
            id: c.id,
            version_id: c.version_id,
            code: c.code,
            extensible: c.extensible,
            name: c.name,
            submission_value: c.submission_value,
            synonym: c.synonym,
            definition: c.definition,
            nci_preferred_term: c.nci_preferred_term,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeItemView {
    pub id: i64,
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CodeItem> for CodeItemView {
    fn from(i: CodeItem) -> Self {
        Self {
            id: i.id,
            codelist_id: i.codelist_id,
            code: i.code,
            submission_value: i.submission_value,
            synonym: i.synonym,
            definition: i.definition,
            nci_preferred_term: i.nci_preferred_term,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

// Re-export the search-hit views so the usecase surface is one
// `use terminology::*` away.
pub use crate::domain::{CodeItemSearchHit, CodeListSearchHit};
