//! View DTOs returned by `CrfUsecase`. Each mirrors a
//! `apis::crf::*View` 1:1 so the facade can `From`-convert at
//! the boundary without losing data.

use chrono::{DateTime, Utc};

use crate::domain::{
    Annotation, AnnotationOwner, CrfForm, CrfItem, CrfItemKind, CrfOption, CrfUnit, CrfVersion,
    DomainAnnotation,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfVersionView {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrfVersion> for CrfVersionView {
    fn from(v: CrfVersion) -> Self {
        Self {
            id: v.id,
            project_code: v.project_code,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfFormView {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrfForm> for CrfFormView {
    fn from(f: CrfForm) -> Self {
        Self {
            id: f.id,
            version_id: f.version_id,
            code: f.code,
            name: f.name,
            order: f.order,
            not_submitted: f.not_submitted,
            created_at: f.created_at,
            updated_at: f.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfItemView {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrfItem> for CrfItemView {
    fn from(i: CrfItem) -> Self {
        Self {
            id: i.id,
            form_id: i.form_id,
            code: i.code,
            name: i.name,
            kind: i.kind,
            order: i.order,
            not_submitted: i.not_submitted,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfOptionView {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrfOption> for CrfOptionView {
    fn from(o: CrfOption) -> Self {
        Self {
            id: o.id,
            item_id: o.item_id,
            value: o.value,
            not_submitted: o.not_submitted,
            created_at: o.created_at,
            updated_at: o.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfUnitView {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CrfUnit> for CrfUnitView {
    fn from(u: CrfUnit) -> Self {
        Self {
            id: u.id,
            item_id: u.item_id,
            value: u.value,
            not_submitted: u.not_submitted,
            created_at: u.created_at,
            updated_at: u.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAnnotationView {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<DomainAnnotation> for DomainAnnotationView {
    fn from(d: DomainAnnotation) -> Self {
        Self {
            id: d.id,
            form_id: d.form_id,
            name: d.name,
            description: d.description,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationView {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Annotation> for AnnotationView {
    fn from(a: Annotation) -> Self {
        Self {
            id: a.id,
            domain_annotation_id: a.domain_annotation_id,
            content: a.content,
            assign: a.assign,
            owner: a.owner,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}

/// Return shape for `CrfUsecase::create_bulk_form`. Mirrors
/// `apis::crf::BulkCreateCrfFormResult`. Caller can fetch options
/// and units through the existing per-item list endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfBulkFormResult {
    pub form: CrfFormView,
    pub items: Vec<CrfItemView>,
}
