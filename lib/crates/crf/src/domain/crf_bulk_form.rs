//! Bulk form creation port.
//!
//! [`CrfBulkFormRepository::bulk_create`] atomically inserts a form,
//! every item, and each item's options + units. The transaction is
//! owned entirely inside the adapter (the Postgres impl opens
//! `pool.begin()`; the in-memory impl uses a sequential loop) so the
//! domain port stays free of `sqlx::Transaction` types.
//!
//! Validation lives in [`validate_bulk_create`] and runs up-front at
//! the usecase, BEFORE the port call. A validation failure therefore
//! never leaves partial state.
//!
//! `form_id` / `item_id` are stamped inside the port — the caller
//! passes placeholder `0`s in the `*New` DTOs and the port fills in
//! the freshly-generated surrogate ids as it walks the tree.

use async_trait::async_trait;

use super::crf_form::{CrfForm, CrfFormNew};
use super::crf_item::{CrfItem, CrfItemNew};
use super::crf_item_kind::CrfItemKind;
use super::crf_option::CrfOptionNew;
use super::crf_unit::CrfUnitNew;
use super::error::DomainError;

/// Full bulk input. The form is the root; each item carries its own
/// options and units. `form_id` / `item_id` fields on the `*New`
/// DTOs MUST be `0` — the port stamps the real surrogate id on each
/// row as it is inserted.
#[derive(Debug, Clone)]
pub struct CrfBulkCreateForm {
    pub form: CrfFormNew,
    pub items: Vec<CrfBulkCreateItem>,
}

#[derive(Debug, Clone)]
pub struct CrfBulkCreateItem {
    pub item: CrfItemNew,
    pub options: Vec<CrfOptionNew>,
    pub units: Vec<CrfUnitNew>,
}

#[derive(Debug, Clone)]
pub struct CrfBulkCreateFormResult {
    pub form: CrfForm,
    pub items: Vec<CrfItem>,
}

/// Persistence port for the bulk form + items + options + units
/// transaction. Object-safe: no `Self`, no generics beyond the
/// `&self` receiver.
#[async_trait]
pub trait CrfBulkFormRepository: Send + Sync {
    async fn bulk_create(
        &self,
        input: CrfBulkCreateForm,
    ) -> Result<CrfBulkCreateFormResult, DomainError>;
}

/// Up-front validation. Mirrors the single-row validators
/// (`validate_create_form` / `validate_create_item` /
/// `validate_create_option` / `validate_create_unit`) plus the
/// kind-shape rules. Runs BEFORE the port call so a violation never
/// leaves partial state.
pub fn validate_bulk_create(input: &CrfBulkCreateForm) -> Result<(), DomainError> {
    // Form-level: empty code / name.
    if input.form.code.trim().is_empty() {
        return Err(DomainError::EmptyCode);
    }
    if input.form.name.trim().is_empty() {
        return Err(DomainError::EmptyName);
    }

    for bi in &input.items {
        // Item-level: empty code / name.
        if bi.item.code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if bi.item.name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        // Per-item option / unit: empty value.
        for o in &bi.options {
            if o.value.trim().is_empty() {
                return Err(DomainError::EmptyValue);
            }
        }
        for u in &bi.units {
            if u.value.trim().is_empty() {
                return Err(DomainError::EmptyValue);
            }
        }

        // Kind-shape: Selection / Checkbox require ≥1 option;
        // Text / Datetime / Label require 0 options.
        let kind = bi.item.kind;
        let needs_options = matches!(kind, CrfItemKind::Selection | CrfItemKind::Checkbox);
        if needs_options == bi.options.is_empty() {
            return Err(DomainError::KindShapeViolation {
                kind,
                field: "options".to_string(),
            });
        }
    }
    Ok(())
}
