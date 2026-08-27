//! Domain layer.
//!
//! Houses value objects (`CrfItemKind`, `AnnotationOwner`),
//! the seven aggregates (`CrfVersion`, `CrfForm`, `CrfItem`,
//! `CrfOption`, `CrfUnit`, `DomainAnnotation`, `Annotation`)
//! along with their `*New` / `*Update` DTOs and persistence
//! ports, the cross-crate `ProjectLookup` port, and
//! `DomainError`. No I/O, no `sqlx`, no `tokio`.

mod annotation;
mod crf_bulk_form;
mod crf_form;
mod crf_item;
mod crf_item_kind;
mod crf_option;
mod crf_unit;
mod crf_version;
mod domain_annotation;
mod error;
mod project_lookup;

#[cfg(test)]
mod tests;

pub use annotation::{
    Annotation, AnnotationNew, AnnotationOwner, AnnotationRepository, AnnotationUpdate,
};
pub use crf_bulk_form::{
    CrfBulkCreateForm, CrfBulkCreateFormResult, CrfBulkCreateItem, CrfBulkFormRepository,
    validate_bulk_create,
};
pub use crf_form::{CrfForm, CrfFormNew, CrfFormRepository, CrfFormUpdate};
pub use crf_item::{CrfItem, CrfItemNew, CrfItemRepository, CrfItemUpdate};
pub use crf_item_kind::CrfItemKind;
pub use crf_option::{CrfOption, CrfOptionNew, CrfOptionRepository, CrfOptionUpdate};
pub use crf_unit::{CrfUnit, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate};
pub use crf_version::{CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate};
pub use domain_annotation::{
    DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository, DomainAnnotationUpdate,
};
pub use error::DomainError;
pub use project_lookup::ProjectLookup;
