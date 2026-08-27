use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// Polymorphic owner of an `Annotation`. Exactly one variant
/// is set per row — the DB CHECK constraint enforces this at
/// the storage layer; the type enforces it at the type layer.
///
/// Mirrors `apis::crf::AnnotationOwner`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnnotationOwner {
    Form { id: i64 },
    Item { id: i64 },
    Option { id: i64 },
    Unit { id: i64 },
}

impl AnnotationOwner {
    pub fn id(&self) -> i64 {
        match self {
            Self::Form { id } | Self::Item { id } | Self::Option { id } | Self::Unit { id } => *id,
        }
    }

    /// Wire column name for the FK this owner populates.
    pub fn fk_column(&self) -> &'static str {
        match self {
            Self::Form { .. } => "form_id",
            Self::Item { .. } => "item_id",
            Self::Option { .. } => "option_id",
            Self::Unit { .. } => "unit_id",
        }
    }
}

/// An annotation content instance. A `DomainAnnotation` is the
/// label template; `Annotation` is the actual content attached
/// to one specific owner (form / item / option / unit).
#[derive(Clone, PartialEq, Eq)]
pub struct Annotation {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for Annotation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Annotation")
            .field("id", &self.id)
            .field("domain_annotation_id", &self.domain_annotation_id)
            .field("content", &self.content)
            .field("assign", &self.assign)
            .field("owner", &self.owner)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl Annotation {
    /// Construct an annotation owned by a form.
    pub fn for_form(
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        form_id: i64,
    ) -> Result<Self, DomainError> {
        Self::new(
            domain_annotation_id,
            content,
            assign,
            AnnotationOwner::Form { id: form_id },
        )
    }

    /// Construct an annotation owned by an item.
    pub fn for_item(
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        item_id: i64,
    ) -> Result<Self, DomainError> {
        Self::new(
            domain_annotation_id,
            content,
            assign,
            AnnotationOwner::Item { id: item_id },
        )
    }

    /// Construct an annotation owned by an option.
    pub fn for_option(
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        option_id: i64,
    ) -> Result<Self, DomainError> {
        Self::new(
            domain_annotation_id,
            content,
            assign,
            AnnotationOwner::Option { id: option_id },
        )
    }

    /// Construct an annotation owned by a unit.
    pub fn for_unit(
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        unit_id: i64,
    ) -> Result<Self, DomainError> {
        Self::new(
            domain_annotation_id,
            content,
            assign,
            AnnotationOwner::Unit { id: unit_id },
        )
    }

    fn new(
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        owner: AnnotationOwner,
    ) -> Result<Self, DomainError> {
        if content.trim().is_empty() {
            return Err(DomainError::EmptyContent);
        }
        if domain_annotation_id <= 0 {
            return Err(DomainError::FkDomainAnnotationNotFound(
                domain_annotation_id,
            ));
        }
        Ok(Self {
            id: 0,
            domain_annotation_id,
            content,
            assign,
            owner,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer
    /// when materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        domain_annotation_id: i64,
        content: String,
        assign: bool,
        owner: AnnotationOwner,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            domain_annotation_id,
            content,
            assign,
            owner,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `AnnotationRepository::create`.
#[derive(Debug, Clone)]
pub struct AnnotationNew {
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,
}

/// Input DTO for `AnnotationRepository::update`. Only
/// `content` and `assign` are mutable on an annotation; the
/// owner is fixed at create time.
#[derive(Debug, Clone, Default)]
pub struct AnnotationUpdate {
    pub id: i64,
    pub content: Option<String>,
    pub assign: Option<bool>,
}

/// Persistence port for the `Annotation` aggregate. The four
/// `find_by_*` / `list_by_*` methods scope the polymorphic
/// owner; `search_by_version` UNIONs all four chains.
#[async_trait]
pub trait AnnotationRepository: Send + Sync {
    async fn create(&self, input: AnnotationNew) -> Result<Annotation, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<Annotation, DomainError>;
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<Annotation>, DomainError>;
    async fn list_by_item(&self, item_id: i64) -> Result<Vec<Annotation>, DomainError>;
    async fn list_by_option(&self, option_id: i64) -> Result<Vec<Annotation>, DomainError>;
    async fn list_by_unit(&self, unit_id: i64) -> Result<Vec<Annotation>, DomainError>;
    /// Batch-fetch every annotation whose `item_id` is in
    /// `item_ids` (the other three FK columns null). Returns
    /// `Ok(Vec::new())` for empty input without hitting the
    /// DB. Used by the form-detail usecase to hydrate item-
    /// level annotations in one round-trip.
    async fn list_by_items(&self, item_ids: &[i64]) -> Result<Vec<Annotation>, DomainError>;
    /// Batch-fetch every annotation owned by an option whose
    /// id is in `option_ids`. Returns `Ok(Vec::new())` for
    /// empty input.
    async fn list_by_options(&self, option_ids: &[i64]) -> Result<Vec<Annotation>, DomainError>;
    /// Batch-fetch every annotation owned by a unit whose id
    /// is in `unit_ids`. Returns `Ok(Vec::new())` for empty
    /// input.
    async fn list_by_units(&self, unit_ids: &[i64]) -> Result<Vec<Annotation>, DomainError>;
    async fn update(&self, input: AnnotationUpdate) -> Result<Annotation, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<Annotation>, DomainError>;
}
