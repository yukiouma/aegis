//! Usecase layer.
//!
//! `CrfUsecase<V, F, I, O, U, Da, A, P>` orchestrates the seven
//! aggregates and the cross-crate project lookup, projects
//! domain aggregates into view DTOs, and surfaces
//! `UsecaseError`. The in-memory facade in
//! `adapter::facade::in_memory::CrfServiceImpl` adapts this
//! type to `apis::crf::CrfService`.

mod commands;
mod crf_usecase;
mod error;
mod views;

#[cfg(test)]
pub(crate) mod tests;

pub use commands::{
    CreateAnnotation, CreateCrfBulkForm, CreateCrfBulkItem, CreateCrfForm, CreateCrfItem,
    CreateCrfOption, CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation,
    SearchAnnotationsByVersion, SearchCrfFormsByVersion, SearchCrfItemsByVersion,
    SearchCrfOptionsByVersion, SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion,
    UpdateAnnotation, UpdateCrfForm, UpdateCrfItem, UpdateCrfOption, UpdateCrfUnit,
    UpdateCrfVersion, UpdateDomainAnnotation,
};
pub use crf_usecase::{CrfUsecase, CrfUsecaseConfig};
pub use error::UsecaseError;
pub use views::{
    AnnotationView, CrfBulkFormResult, CrfFormView, CrfItemView, CrfOptionView, CrfUnitView,
    CrfVersionView, DomainAnnotationView,
};
