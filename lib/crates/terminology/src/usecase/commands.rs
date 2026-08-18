use crate::domain::TerminologyKind;

// TerminologyVersion

pub struct CreateTerminologyVersion {
    pub kind: TerminologyKind,
    pub name: String,
}

#[derive(Default)]
pub struct UpdateTerminologyVersion {
    pub id: i64,
    pub kind: Option<TerminologyKind>,
    pub name: Option<String>,
}

// CodeList

pub struct CreateCodeList {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Default)]
pub struct UpdateCodeList {
    pub id: i64,
    pub code: Option<String>,
    pub extensible: Option<bool>,
    pub name: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

// CodeItem

pub struct CreateCodeItem {
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Default)]
pub struct UpdateCodeItem {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}