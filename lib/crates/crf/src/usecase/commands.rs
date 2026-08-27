//! Command DTOs accepted by `CrfUsecase`. Each `Create*` /
//! `Update*` mirrors a `apis::crf::Create*Request` /
//! `Update*Request` 1:1 so the facade can `From`-convert at
//! the boundary without losing data.

use crate::domain::{AnnotationOwner, CrfItemKind, CrfItemUpdate as DomainCrfItemUpdate};

// ---- CrfVersion ----

pub struct CreateCrfVersion {
    pub project_code: String,
    pub name: String,
}

#[derive(Default)]
pub struct UpdateCrfVersion {
    pub id: i64,
    pub name: Option<String>,
}

// ---- CrfForm ----

pub struct CreateCrfForm {
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Default)]
pub struct UpdateCrfForm {
    pub id: i64,
    pub code: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

// ---- CrfItem ----

pub struct CreateCrfItem {
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

#[derive(Default)]
pub struct UpdateCrfItem {
    pub id: i64,
    pub code: Option<String>,
    pub name: Option<String>,
    pub kind: Option<CrfItemKind>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

impl UpdateCrfItem {
    /// Convert to the domain `CrfItemUpdate` (which doesn't
    /// carry a `kind` because kind-shape validation re-reads
    /// the item from the repo).
    pub fn into_domain(self) -> DomainCrfItemUpdate {
        DomainCrfItemUpdate {
            id: self.id,
            code: self.code,
            name: self.name,
            order: self.order,
            not_submitted: self.not_submitted,
        }
    }
}

// ---- CrfOption ----

pub struct CreateCrfOption {
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Default)]
pub struct UpdateCrfOption {
    pub id: i64,
    pub value: Option<String>,
    pub not_submitted: Option<bool>,
}

// ---- CrfUnit ----

pub struct CreateCrfUnit {
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
}

#[derive(Default)]
pub struct UpdateCrfUnit {
    pub id: i64,
    pub value: Option<String>,
    pub not_submitted: Option<bool>,
}

// ---- DomainAnnotation ----

pub struct CreateDomainAnnotation {
    pub form_id: i64,
    pub name: String,
    pub description: String,
}

#[derive(Default)]
pub struct UpdateDomainAnnotation {
    pub id: i64,
    pub name: Option<String>,
    pub description: Option<String>,
}

// ---- Annotation ----

pub struct CreateAnnotation {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

#[derive(Default)]
pub struct UpdateAnnotation {
    pub id: i64,
    pub content: Option<String>,
    pub assign: Option<bool>,
}

// ---- Search ----

pub struct SearchCrfFormsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
pub struct SearchCrfItemsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
pub struct SearchCrfOptionsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
pub struct SearchCrfUnitsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
pub struct SearchDomainAnnotationsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
pub struct SearchAnnotationsByVersion {
    pub version_id: i64,
    pub fragment: String,
}
