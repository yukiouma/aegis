//! Outbound port for the SDTM domain-model service.
//!
//! Mirrors the surface of
//! `domain_model::usecase::DomainModelUsecase` so adapters in any
//! backend (in-memory, PostgreSQL, …) can adapt their own types to
//! the shared contract defined here. All supporting DTOs (request
//! shapes, view projections, enum re-declarations, and
//! [`DomainModelApiError`]) live alongside the trait so a single
//! `use apis::domain_model::*;` brings the whole contract into
//! scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// SDTM domain category.
///
/// Mirrors `domain_model::DomainCategory`. The two enums are
/// kept in sync layer by layer — adapter implementations convert
/// losslessly via the matching variant constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

/// SDTM variable type.
///
/// Mirrors `domain_model::SdtmVariableType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

/// SDTM variable core.
///
/// Mirrors `domain_model::SdtmVariableCore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

/// SDTM variable role.
///
/// Mirrors `domain_model::SdtmRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

/// Error surface returned by every [`DomainModelService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `domain_model::UsecaseError` / `domain_model::DomainError`) into
/// this type at the implementation boundary. The shape
/// intentionally combines validation, lookup, duplicate, and
/// infrastructure concerns into a single type so handlers can
/// match exhaustively.
#[derive(Debug, Clone, Error)]
pub enum DomainModelApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("sdtm version not found: {0}")]
    SdtmVersionNotFound(i64),
    #[error("sdtm domain not found: {0}")]
    SdtmDomainNotFound(i64),
    #[error("sdtm variable not found: {0}")]
    SdtmVariableNotFound(i64),

    #[error("sdtm version already exists: {name}")]
    DuplicateSdtmVersion { name: String },
    #[error("sdtm domain already exists for version {version_id} / {name}")]
    DuplicateSdtmDomain { version_id: i64, name: String },
    #[error("sdtm variable already exists for domain {domain_id} / {name}")]
    DuplicateSdtmVariable { domain_id: i64, name: String },

    #[error("referenced sdtm version not found: {0}")]
    FkSdtmVersionNotFound(i64),
    #[error("referenced sdtm domain not found: {0}")]
    FkSdtmDomainNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}

// ---- description DTOs ----

/// Localised description of an SDTM domain. Carried on
/// [`SdtmDomainView`] and round-trips through the backend as a
/// single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// Localised description of an SDTM variable. Carried on
/// [`SdtmVariableView`] and round-trips through the backend as a
/// single JSONB blob.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

// ---- view projections ----

/// Safe projection of an `SdtmVersion` aggregate — exactly what
/// adapters hand back to whatever consumes the API.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVersionView {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmDomain` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainView {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of an `SdtmVariable` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

/// Input DTO for [`DomainModelService::create_version`].
#[derive(Debug, Clone)]
pub struct CreateSdtmVersionRequest {
    pub name: String,
}

/// Input DTO for [`DomainModelService::update_version`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied.
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmVersionRequest {
    pub id: i64,
    pub name: Option<String>,
}

/// Input DTO for [`DomainModelService::create_domain`].
#[derive(Debug, Clone)]
pub struct CreateSdtmDomainRequest {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Input DTO for [`DomainModelService::update_domain`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied. `descriptions: None` means
/// "don't touch"; `Some(vec)` means "replace with this list"
/// (use an empty `vec![]` to clear the column).
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmDomainRequest {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

/// Input DTO for [`DomainModelService::create_variable`].
#[derive(Debug, Clone)]
pub struct CreateSdtmVariableRequest {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Input DTO for [`DomainModelService::update_variable`]. Every
/// field except `id` is optional; only the fields that actually
/// changed need to be supplied. `variable_controlled` and
/// `variable_role` use `Option<Option<T>>` so the caller can
/// distinguish "don't change" (outer `None`) from "clear the
/// field" (outer `Some(None)`); the other fields use flat
/// `Option<T>` where `None` means "don't change" and
/// `Some(value)` means "replace".
#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmVariableRequest {
    pub id: i64,
    pub name: Option<String>,
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

/// Outbound port for SDTM domain-model lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn DomainModelService>` can be shared
/// state in an async server (axum, tarpc, etc.). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `domain_model::DomainModelUsecase`) into this contract,
/// translating between backend-specific DTOs / errors and the
/// `apis` types defined above.
#[async_trait]
pub trait DomainModelService: Send + Sync {
    // ---- SdtmVersion ----

    /// Create a new version. Returns
    /// `DuplicateSdtmVersion { name }` if a version with that
    /// name already exists.
    async fn create_version(
        &self,
        req: CreateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError>;

    /// List every version known to the backend. Order is
    /// backend-defined.
    async fn list_versions(&self) -> Result<Vec<SdtmVersionView>, DomainModelApiError>;

    /// Apply the optional fields on `req` to the version
    /// identified by `req.id`. Returns `SdtmVersionNotFound` if no
    /// such version exists.
    async fn update_version(
        &self,
        req: UpdateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError>;

    /// Hard delete the version identified by `id`. Cascades to
    /// child domains (and via them to variables) at the backend.
    async fn delete_version(&self, id: i64) -> Result<(), DomainModelApiError>;

    // ---- SdtmDomain ----

    /// Create a new `(version_id, name)` domain. Returns
    /// `DuplicateSdtmDomain { version_id, name }` if a domain with
    /// that pair already exists, or `FkSdtmVersionNotFound` if the
    /// parent version is missing.
    async fn create_domain(
        &self,
        req: CreateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError>;

    /// Look up a domain by its surrogate id. Returns
    /// `SdtmDomainNotFound` if no such domain exists.
    async fn get_domain_by_id(&self, id: i64) -> Result<SdtmDomainView, DomainModelApiError>;

    /// List every domain attached to the given version, ordered by
    /// id ASC.
    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmDomainView>, DomainModelApiError>;

    /// Apply the optional fields on `req` to the domain identified
    /// by `req.id`. Returns `SdtmDomainNotFound` if no such domain
    /// exists.
    async fn update_domain(
        &self,
        req: UpdateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError>;

    /// Hard delete the domain identified by `id`. Cascades to
    /// child variables at the backend.
    async fn delete_domain(&self, id: i64) -> Result<(), DomainModelApiError>;

    // ---- SdtmVariable ----

    /// Create a new `(domain_id, name)` variable. Returns
    /// `DuplicateSdtmVariable { domain_id, name }` if a variable
    /// with that pair already exists, or `FkSdtmDomainNotFound` if
    /// the parent domain is missing.
    async fn create_variable(
        &self,
        req: CreateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError>;

    /// Look up a variable by its surrogate id. Returns
    /// `SdtmVariableNotFound` if no such variable exists.
    async fn get_variable_by_id(&self, id: i64) -> Result<SdtmVariableView, DomainModelApiError>;

    /// List every variable attached to the given domain, ordered
    /// by `variable_sequence ASC, id ASC`.
    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmVariableView>, DomainModelApiError>;

    /// Apply the optional fields on `req` to the variable
    /// identified by `req.id`. Returns `SdtmVariableNotFound` if
    /// no such variable exists.
    async fn update_variable(
        &self,
        req: UpdateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError>;

    /// Hard delete the variable identified by `id`.
    async fn delete_variable(&self, id: i64) -> Result<(), DomainModelApiError>;
}
