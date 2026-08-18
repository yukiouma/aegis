use async_trait::async_trait;

use super::code_item::{
    CodeItem, CodeItemNew, CodeItemSearchHit, CodeItemSearchQuery, CodeItemUpdate,
};
use super::code_list::{
    CodeList, CodeListNew, CodeListSearchHit, CodeListSearchQuery, CodeListUpdate,
};
use super::error::DomainError;
use super::terminology_kind::TerminologyKind;
use super::terminology_version::{
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionUpdate,
};

/// Outbound port for persistence of `TerminologyVersion`
/// aggregates. Implementations live in the adapter layer.
#[async_trait]
pub trait TerminologyVersionRepository: Send + Sync {
    async fn create(&self, input: TerminologyVersionNew)
    -> Result<TerminologyVersion, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError>;

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError>;

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError>;

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError>;

    /// Hard delete; cascades to child code_lists (and via them to
    /// code_items) via the schema's `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `CodeList` aggregates.
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    /// Hard delete; cascades to code_items via the schema's
    /// `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(
        &self,
        query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError>;
}

/// Outbound port for persistence of `CodeItem` aggregates.
#[async_trait]
pub trait CodeItemRepository: Send + Sync {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError>;
    async fn list_by_codelist(&self, codelist_id: i64) -> Result<Vec<CodeItem>, DomainError>;
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(
        &self,
        query: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError>;
}
