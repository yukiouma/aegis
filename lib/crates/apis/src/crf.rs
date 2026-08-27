//! Outbound port for the Case Report Form (CRF) service.
//!
//! Mirrors the surface of `crf::usecase::CrfUsecase` so adapters
//! in any backend (in-memory, PostgreSQL, …) can adapt their
//! own types to the shared contract defined here. All supporting
//! DTOs (request shapes, view projections, enum re-declarations,
//! and [`CrfApiError`]) live alongside the trait so a single
//! `use apis::crf::*;` brings the whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// `CrfItemKind` discriminant — a CRF item can collect text, a
/// selection from options, multi-select checkboxes, a datetime
/// stamp, or a static label.
///
/// Mirrors `crf::domain::CrfItemKind`. The two enums are kept in
/// sync layer by layer — adapter implementations convert
/// losslessly via the matching variant constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CrfItemKind {
    Text,
    Selection,
    Checkbox,
    Datetime,
    Label,
}

/// Polymorphic owner of an `Annotation`. Exactly one variant is
/// set per row — the DB CHECK constraint enforces this at the
/// storage layer; the type enforces it at the type layer.
///
/// Mirrors `crf::domain::AnnotationOwner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnnotationOwner {
    Form(i32),
    Item(i32),
    Option(i32),
    Unit(i32),
}

// ---- view projections ----

/// Safe projection of a `CrfVersion` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfVersionView {
    pub id: i32,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CrfForm` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfFormView {
    pub id: i32,
    pub version_id: i32,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CrfItem` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfItemView {
    pub id: i32,
    pub form_id: i32,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CrfOption` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfOptionView {
    pub id: i32,
    pub item_id: i32,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `CrfUnit` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CrfUnitView {
    pub id: i32,
    pub item_id: i32,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `DomainAnnotation` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainAnnotationView {
    pub id: i32,
    pub form_id: i32,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `Annotation` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnnotationView {
    pub id: i32,
    pub domain_annotation_id: i32,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// Input DTO for [`CrfService::create_version`].
#[derive(Debug, Clone)]
pub struct CreateCrfVersionRequest {
    pub project_code: String,
    pub name: String,
}

/// Input DTO for [`CrfService::update_version`].
#[derive(Debug, Default, Clone)]
pub struct UpdateCrfVersionRequest {
    pub id: i32,
    pub name: Option<String>,
}

/// Input DTO for [`CrfService::create_form`].
#[derive(Debug, Clone)]
pub struct CreateCrfFormRequest {
    pub version_id: i32,
    pub code: String,
    pub name: String,
    pub order: i32,
    pub not_submitted: bool,
}

/// Input DTO for [`CrfService::update_form`].
#[derive(Debug, Default, Clone)]
pub struct UpdateCrfFormRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

/// Input DTO for [`CrfService::create_item`].
#[derive(Debug, Clone)]
pub struct CreateCrfItemRequest {
    pub form_id: i32,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i32,
    pub not_submitted: bool,
}

/// Input DTO for [`CrfService::update_item`].
#[derive(Debug, Default, Clone)]
pub struct UpdateCrfItemRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub kind: Option<CrfItemKind>,
    pub order: Option<i32>,
    pub not_submitted: Option<bool>,
}

/// Input DTO for [`CrfService::create_option`].
#[derive(Debug, Clone)]
pub struct CreateCrfOptionRequest {
    pub item_id: i32,
    pub value: String,
    pub not_submitted: bool,
}

/// Input DTO for [`CrfService::update_option`].
#[derive(Debug, Default, Clone)]
pub struct UpdateCrfOptionRequest {
    pub id: i32,
    pub value: Option<String>,
    pub not_submitted: Option<bool>,
}

/// Input DTO for [`CrfService::create_unit`].
#[derive(Debug, Clone)]
pub struct CreateCrfUnitRequest {
    pub item_id: i32,
    pub value: String,
    pub not_submitted: bool,
}

/// Input DTO for [`CrfService::update_unit`].
#[derive(Debug, Default, Clone)]
pub struct UpdateCrfUnitRequest {
    pub id: i32,
    pub value: Option<String>,
    pub not_submitted: Option<bool>,
}

/// Input DTO for [`CrfService::create_domain_annotation`].
#[derive(Debug, Clone)]
pub struct CreateDomainAnnotationRequest {
    pub form_id: i32,
    pub name: String,
    pub description: String,
}

/// Input DTO for [`CrfService::update_domain_annotation`].
#[derive(Debug, Default, Clone)]
pub struct UpdateDomainAnnotationRequest {
    pub id: i32,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// Input DTO for [`CrfService::create_annotation`].
#[derive(Debug, Clone)]
pub struct CreateAnnotationRequest {
    pub domain_annotation_id: i32,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

/// Input DTO for [`CrfService::update_annotation`].
#[derive(Debug, Default, Clone)]
pub struct UpdateAnnotationRequest {
    pub id: i32,
    pub content: Option<String>,
    pub assign: Option<bool>,
}

/// Input DTO for [`CrfService::get_*_by_id`] (id-only requests).
#[derive(Debug, Clone)]
pub struct GetCrfVersionByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetCrfFormByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetCrfItemByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetCrfOptionByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetCrfUnitByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetDomainAnnotationByIdRequest {
    pub id: i32,
}
#[derive(Debug, Clone)]
pub struct GetAnnotationByIdRequest {
    pub id: i32,
}

/// Input DTO for [`CrfService::list_versions_by_project`].
#[derive(Debug, Clone)]
pub struct ListCrfVersionsByProjectRequest {
    pub project_code: String,
}

/// Input DTO for [`CrfService::list_forms_by_version`].
#[derive(Debug, Clone)]
pub struct ListCrfFormsByVersionRequest {
    pub version_id: i32,
}

/// Input DTO for [`CrfService::list_items_by_form`].
#[derive(Debug, Clone)]
pub struct ListCrfItemsByFormRequest {
    pub form_id: i32,
}

/// Input DTO for [`CrfService::list_options_by_item`].
#[derive(Debug, Clone)]
pub struct ListCrfOptionsByItemRequest {
    pub item_id: i32,
}

/// Input DTO for [`CrfService::list_units_by_item`].
#[derive(Debug, Clone)]
pub struct ListCrfUnitsByItemRequest {
    pub item_id: i32,
}

/// Input DTO for [`CrfService::list_domain_annotations_by_form`].
#[derive(Debug, Clone)]
pub struct ListDomainAnnotationsByFormRequest {
    pub form_id: i32,
}

/// Input DTO for [`CrfService::list_annotations_by_form`].
#[derive(Debug, Clone)]
pub struct ListAnnotationsByFormRequest {
    pub form_id: i32,
}

/// Input DTO for [`CrfService::list_annotations_by_item`].
#[derive(Debug, Clone)]
pub struct ListAnnotationsByItemRequest {
    pub item_id: i32,
}

/// Input DTO for [`CrfService::list_annotations_by_option`].
#[derive(Debug, Clone)]
pub struct ListAnnotationsByOptionRequest {
    pub option_id: i32,
}

/// Input DTO for [`CrfService::list_annotations_by_unit`].
#[derive(Debug, Clone)]
pub struct ListAnnotationsByUnitRequest {
    pub unit_id: i32,
}

/// Input DTOs for the search endpoints. The fragment is
/// required and non-empty.
#[derive(Debug, Clone)]
pub struct SearchCrfFormsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}
#[derive(Debug, Clone)]
pub struct SearchCrfItemsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}
#[derive(Debug, Clone)]
pub struct SearchCrfOptionsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}
#[derive(Debug, Clone)]
pub struct SearchCrfUnitsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}
#[derive(Debug, Clone)]
pub struct SearchDomainAnnotationsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}
#[derive(Debug, Clone)]
pub struct SearchAnnotationsByVersionRequest {
    pub version_id: i32,
    pub fragment: String,
}

// ---- error ----

/// Error surface returned by every [`CrfService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `crf::UsecaseError` / `crf::DomainError`) into this type at
/// the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum CrfApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("crf version not found: {0}")]
    CrfVersionNotFound(i32),
    #[error("crf form not found: {0}")]
    CrfFormNotFound(i32),
    #[error("crf item not found: {0}")]
    CrfItemNotFound(i32),
    #[error("crf option not found: {0}")]
    CrfOptionNotFound(i32),
    #[error("crf unit not found: {0}")]
    CrfUnitNotFound(i32),
    #[error("domain annotation not found: {0}")]
    DomainAnnotationNotFound(i32),
    #[error("annotation not found: {0}")]
    AnnotationNotFound(i32),

    #[error("crf version already exists: {project_code} / {name}")]
    DuplicateCrfVersion { project_code: String, name: String },
    #[error("crf form already exists: version {version_id} / {code}")]
    DuplicateCrfForm { version_id: i32, code: String },
    #[error("crf item already exists: form {form_id} / {code}")]
    DuplicateCrfItem { form_id: i32, code: String },
    #[error("domain annotation already exists: form {form_id} / {name}")]
    DuplicateDomainAnnotation { form_id: i32, name: String },

    #[error("referenced crf version not found: {0}")]
    FkCrfVersionNotFound(i32),
    #[error("referenced crf form not found: {0}")]
    FkCrfFormNotFound(i32),
    #[error("referenced crf item not found: {0}")]
    FkCrfItemNotFound(i32),
    #[error("referenced crf option not found: {0}")]
    FkCrfOptionNotFound(i32),
    #[error("referenced crf unit not found: {0}")]
    FkCrfUnitNotFound(i32),
    #[error("referenced domain annotation not found: {0}")]
    FkDomainAnnotationNotFound(i32),

    #[error("search fragment cannot be empty")]
    EmptySearchFragment,

    #[error("kind-shape violation: {kind:?} cannot carry {field}")]
    KindShapeViolation { kind: CrfItemKind, field: String },

    #[error("repository error: {0}")]
    Repository(String),
}

// ---- port trait ----

/// Outbound port for Case Report Form lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn CrfService>` can be shared state
/// in an async server (axum, tarpc, etc.). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `crf::CrfUsecase`) into this contract, translating between
/// backend-specific DTOs / errors and the `apis` types defined
/// above.
#[async_trait]
pub trait CrfService: Send + Sync {
    // ---- CrfVersion ----

    /// Create a new `(project_code, name)` version. Returns
    /// `ProjectNotFound` if the project does not exist, or
    /// `DuplicateCrfVersion { project_code, name }` if a version
    /// with that pair already exists.
    async fn create_version(
        &self,
        req: CreateCrfVersionRequest,
    ) -> Result<CrfVersionView, CrfApiError>;

    /// Look up a version by its surrogate id.
    async fn get_version_by_id(
        &self,
        req: GetCrfVersionByIdRequest,
    ) -> Result<CrfVersionView, CrfApiError>;

    /// List every version attached to a project, ordered by
    /// id ASC.
    async fn list_versions_by_project(
        &self,
        req: ListCrfVersionsByProjectRequest,
    ) -> Result<Vec<CrfVersionView>, CrfApiError>;

    /// Apply the optional fields on `req` to the version
    /// identified by `req.id`.
    async fn update_version(
        &self,
        req: UpdateCrfVersionRequest,
    ) -> Result<CrfVersionView, CrfApiError>;

    /// Hard delete the version identified by `id`. Cascades to
    /// child forms (and via them to items, options, units,
    /// annotations) at the backend.
    async fn delete_version(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- CrfForm ----

    /// Create a new `(version_id, code)` form.
    async fn create_form(&self, req: CreateCrfFormRequest) -> Result<CrfFormView, CrfApiError>;

    async fn get_form_by_id(&self, req: GetCrfFormByIdRequest) -> Result<CrfFormView, CrfApiError>;

    /// List every form attached to the given version, ordered
    /// by `order ASC, id ASC`.
    async fn list_forms_by_version(
        &self,
        req: ListCrfFormsByVersionRequest,
    ) -> Result<Vec<CrfFormView>, CrfApiError>;

    async fn update_form(&self, req: UpdateCrfFormRequest) -> Result<CrfFormView, CrfApiError>;

    async fn delete_form(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- CrfItem ----

    /// Create a new `(form_id, code)` item. Enforces kind-shape
    /// validation: `Selection | Checkbox` require at least one
    /// option on the item (the create path does not support
    /// batch-attach); `Text | Datetime | Label` reject the
    /// presence of options on update.
    async fn create_item(&self, req: CreateCrfItemRequest) -> Result<CrfItemView, CrfApiError>;

    async fn get_item_by_id(&self, req: GetCrfItemByIdRequest) -> Result<CrfItemView, CrfApiError>;

    /// List every item attached to the given form, ordered by
    /// `order ASC, id ASC`. Returns the scalar view only;
    /// consumers needing the full tree (options / units /
    /// annotations) call the per-item list methods.
    async fn list_items_by_form(
        &self,
        req: ListCrfItemsByFormRequest,
    ) -> Result<Vec<CrfItemView>, CrfApiError>;

    async fn update_item(&self, req: UpdateCrfItemRequest) -> Result<CrfItemView, CrfApiError>;

    async fn delete_item(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- CrfOption ----

    async fn create_option(
        &self,
        req: CreateCrfOptionRequest,
    ) -> Result<CrfOptionView, CrfApiError>;

    async fn get_option_by_id(
        &self,
        req: GetCrfOptionByIdRequest,
    ) -> Result<CrfOptionView, CrfApiError>;

    async fn list_options_by_item(
        &self,
        req: ListCrfOptionsByItemRequest,
    ) -> Result<Vec<CrfOptionView>, CrfApiError>;

    async fn update_option(
        &self,
        req: UpdateCrfOptionRequest,
    ) -> Result<CrfOptionView, CrfApiError>;

    async fn delete_option(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- CrfUnit ----

    async fn create_unit(&self, req: CreateCrfUnitRequest) -> Result<CrfUnitView, CrfApiError>;

    async fn get_unit_by_id(&self, req: GetCrfUnitByIdRequest) -> Result<CrfUnitView, CrfApiError>;

    async fn list_units_by_item(
        &self,
        req: ListCrfUnitsByItemRequest,
    ) -> Result<Vec<CrfUnitView>, CrfApiError>;

    async fn update_unit(&self, req: UpdateCrfUnitRequest) -> Result<CrfUnitView, CrfApiError>;

    async fn delete_unit(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- DomainAnnotation ----

    async fn create_domain_annotation(
        &self,
        req: CreateDomainAnnotationRequest,
    ) -> Result<DomainAnnotationView, CrfApiError>;

    async fn get_domain_annotation_by_id(
        &self,
        req: GetDomainAnnotationByIdRequest,
    ) -> Result<DomainAnnotationView, CrfApiError>;

    async fn list_domain_annotations_by_form(
        &self,
        req: ListDomainAnnotationsByFormRequest,
    ) -> Result<Vec<DomainAnnotationView>, CrfApiError>;

    async fn update_domain_annotation(
        &self,
        req: UpdateDomainAnnotationRequest,
    ) -> Result<DomainAnnotationView, CrfApiError>;

    async fn delete_domain_annotation(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- Annotation ----

    async fn create_annotation(
        &self,
        req: CreateAnnotationRequest,
    ) -> Result<AnnotationView, CrfApiError>;

    async fn get_annotation_by_id(
        &self,
        req: GetAnnotationByIdRequest,
    ) -> Result<AnnotationView, CrfApiError>;

    async fn list_annotations_by_form(
        &self,
        req: ListAnnotationsByFormRequest,
    ) -> Result<Vec<AnnotationView>, CrfApiError>;

    async fn list_annotations_by_item(
        &self,
        req: ListAnnotationsByItemRequest,
    ) -> Result<Vec<AnnotationView>, CrfApiError>;

    async fn list_annotations_by_option(
        &self,
        req: ListAnnotationsByOptionRequest,
    ) -> Result<Vec<AnnotationView>, CrfApiError>;

    async fn list_annotations_by_unit(
        &self,
        req: ListAnnotationsByUnitRequest,
    ) -> Result<Vec<AnnotationView>, CrfApiError>;

    async fn update_annotation(
        &self,
        req: UpdateAnnotationRequest,
    ) -> Result<AnnotationView, CrfApiError>;

    async fn delete_annotation(&self, id: i32) -> Result<(), CrfApiError>;

    // ---- Search ----

    async fn search_forms_by_version(
        &self,
        req: SearchCrfFormsByVersionRequest,
    ) -> Result<Vec<CrfFormView>, CrfApiError>;

    async fn search_items_by_version(
        &self,
        req: SearchCrfItemsByVersionRequest,
    ) -> Result<Vec<CrfItemView>, CrfApiError>;

    async fn search_options_by_version(
        &self,
        req: SearchCrfOptionsByVersionRequest,
    ) -> Result<Vec<CrfOptionView>, CrfApiError>;

    async fn search_units_by_version(
        &self,
        req: SearchCrfUnitsByVersionRequest,
    ) -> Result<Vec<CrfUnitView>, CrfApiError>;

    async fn search_domain_annotations_by_version(
        &self,
        req: SearchDomainAnnotationsByVersionRequest,
    ) -> Result<Vec<DomainAnnotationView>, CrfApiError>;

    async fn search_annotations_by_version(
        &self,
        req: SearchAnnotationsByVersionRequest,
    ) -> Result<Vec<AnnotationView>, CrfApiError>;
}
