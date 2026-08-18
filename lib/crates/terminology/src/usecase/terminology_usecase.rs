use crate::domain::{
    CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery, CodeItemUpdate,
    CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery, CodeListUpdate,
    DomainError, TerminologyKind, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};

use super::commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeItem, UpdateCodeList,
    UpdateTerminologyVersion,
};
use super::error::UsecaseError;
use super::views::{CodeItemView, CodeListView, TerminologyVersionView};

/// Configuration for `TerminologyUsecase::new`. Wraps the three
/// concrete (or fake) repositories so the constructor stays
/// readable.
pub struct TerminologyUsecaseConfig<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    pub version_repo: V,
    pub code_list_repo: L,
    pub code_item_repo: I,
}

/// Async orchestration for terminology lifecycle operations.
///
/// Generic over the three repository ports so tests can inject
/// in-memory fakes. Domain → view projection runs through the
/// `From` impls in `super::views`.
pub struct TerminologyUsecase<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    version_repo: V,
    code_list_repo: L,
    code_item_repo: I,
}

impl<V, L, I> TerminologyUsecase<V, L, I>
where
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
{
    pub fn new(cfg: TerminologyUsecaseConfig<V, L, I>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            code_list_repo: cfg.code_list_repo,
            code_item_repo: cfg.code_item_repo,
        }
    }

    // ---- TerminologyVersion ----

    pub async fn create_version(
        &self,
        cmd: CreateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        let version = self
            .version_repo
            .create(TerminologyVersionNew {
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(version.into())
    }

    pub async fn get_version_by_id(&self, id: i64) -> Result<TerminologyVersionView, UsecaseError> {
        let v = self.version_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn get_version(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        if name.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyName));
        }
        let v = self.version_repo.find_by_kind_and_name(kind, name).await?;
        Ok(v.into())
    }

    pub async fn list_versions(&self) -> Result<Vec<TerminologyVersionView>, UsecaseError> {
        let versions = self.version_repo.list().await?;
        Ok(versions.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(TerminologyVersionUpdate {
                id: cmd.id,
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- CodeList ----

    pub async fn create_code_list(
        &self,
        cmd: CreateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_create_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .create(CodeListNew {
                version_id: cmd.version_id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn list_code_lists(
        &self,
        version_id: i64,
    ) -> Result<Vec<CodeListView>, UsecaseError> {
        let lists = self.code_list_repo.list_by_version(version_id).await?;
        Ok(lists.into_iter().map(Into::into).collect())
    }

    pub async fn update_code_list(
        &self,
        cmd: UpdateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_update_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .update(CodeListUpdate {
                id: cmd.id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn delete_code_list(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_list_repo.delete(id).await?;
        Ok(())
    }

    pub async fn search_code_lists(
        &self,
        q: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, UsecaseError> {
        let hits = self.code_list_repo.search(clamp_query(q)).await?;
        Ok(hits)
    }

    // ---- CodeItem ----

    pub async fn create_code_item(
        &self,
        cmd: CreateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_create_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .create(CodeItemNew {
                codelist_id: cmd.codelist_id,
                version_id: cmd.version_id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn list_code_items(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItemView>, UsecaseError> {
        let items = self.code_item_repo.list_by_codelist(codelist_id).await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    /// Natural-key lookup: items belonging to the codelist
    /// identified by `(version_id, code)` — typically the NCI
    /// C-code on `code_lists`. Lets consumers pass the version
    /// and codelist code directly without first resolving the
    /// surrogate `codelist_id`.
    pub async fn list_code_items_by_version_and_codelist_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItemView>, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let items = self
            .code_item_repo
            .list_by_version_and_codelist_code(version_id, code)
            .await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    pub async fn update_code_item(
        &self,
        cmd: UpdateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_update_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .update(CodeItemUpdate {
                id: cmd.id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn delete_code_item(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_item_repo.delete(id).await?;
        Ok(())
    }

    pub async fn search_code_items(
        &self,
        q: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, UsecaseError> {
        let hits = self
            .code_item_repo
            .search(CodeItemSearchQuery {
                limit: clamp_limit(q.limit),
                ..q
            })
            .await?;
        Ok(hits)
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateTerminologyVersion) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateTerminologyVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_code_list(cmd: &CreateCodeList) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_list(cmd: &UpdateCodeList) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_create_code_item(cmd: &CreateCodeItem) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_item(cmd: &UpdateCodeItem) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

// ---- search-query sanitation ----

/// Apply the default + hard cap to the `limit` field of a search
/// query, returning a new query with the clamped value. The
/// Postgres implementation reads the clamped value, so the cap is
/// enforced even when tests pass an unbounded `u32::MAX`.
fn clamp_query(mut q: CodeListSearchQuery) -> CodeListSearchQuery {
    q.limit = clamp_limit(q.limit);
    q
}

fn clamp_limit(limit: u32) -> u32 {
    if limit == 0 {
        50
    } else if limit > 500 {
        500
    } else {
        limit
    }
}
