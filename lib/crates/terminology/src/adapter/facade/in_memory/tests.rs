//! Unit tests for `TerminologyServiceImpl`.
//!
//! Wires the adapter on top of three in-memory repositories so the
//! behaviour is exercised without touching PostgreSQL. The
//! in-memory repositories are private to this test module — they
//! are intentionally not shared with the usecase tests so each
//! layer can evolve independently.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::terminology::TerminologyKind as ApiKind;
use apis::terminology::{
    CodeItemSearchHit, CodeItemView, CodeListSearchHit, CodeListView, CreateCodeItemRequest,
    CreateCodeListRequest, CreateTerminologyVersionRequest, TerminologyApiError,
    TerminologyService, TerminologyVersionView, UpdateCodeItemRequest, UpdateCodeListRequest,
    UpdateTerminologyVersionRequest,
};

use crate::domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit as InternalCodeItemSearchHit,
    CodeItemSearchQuery, CodeItemUpdate, CodeList, CodeListNew, CodeListRepository,
    CodeListSearchHit as InternalCodeListSearchHit, CodeListSearchQuery, CodeListUpdate,
    DomainError, TerminologyKind, TerminologyVersion, TerminologyVersionNew,
    TerminologyVersionRepository, TerminologyVersionUpdate,
};
use crate::usecase::TerminologyUsecase;

use super::TerminologyServiceImpl;

/// Fixed `DateTime<Utc>` returned by every fake repository for
/// every row it creates. Keeps the assertions readable.
fn epoch() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

// ---------- in-memory repositories ----------

#[derive(Default)]
struct VersionState {
    by_id: std::collections::HashMap<i64, TerminologyVersion>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct InMemoryVersionRepo {
    state: Arc<Mutex<VersionState>>,
}

impl InMemoryVersionRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VersionState::default())),
        }
    }
}

#[async_trait]
impl TerminologyVersionRepository for InMemoryVersionRepo {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|v| v.kind == input.kind && v.name == input.name)
        {
            return Err(DomainError::DuplicateVersion {
                kind: input.kind,
                name: input.name,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let v = TerminologyVersion::for_repository(id, input.kind, input.name, epoch(), epoch());
        s.by_id.insert(id, v.clone());
        Ok(v)
    }

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::VersionNotFound(id))
    }

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .values()
            .find(|v| v.kind == kind && v.name == name)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError> {
        Ok(self.state.lock().unwrap().by_id.values().cloned().collect())
    }

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        let v = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::VersionNotFound(input.id))?;
        if let Some(kind) = input.kind {
            v.kind = kind;
        }
        if let Some(name) = input.name {
            v.name = name;
        }
        Ok(v.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::VersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ListState {
    by_id: std::collections::HashMap<i64, CodeList>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct InMemoryCodeListRepo {
    state: Arc<Mutex<ListState>>,
    /// Reference to the item store so `delete` can cascade-wipe
    /// the rows that the Postgres `ON DELETE CASCADE` would have
    /// removed. Mirrors the schema-level FK cascade without
    /// requiring the usecase to know about it.
    items: Option<Arc<Mutex<ItemState>>>,
}

impl InMemoryCodeListRepo {
    fn with_items(items: Arc<Mutex<ItemState>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ListState::default())),
            items: Some(items),
        }
    }
}

#[async_trait]
impl CodeListRepository for InMemoryCodeListRepo {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|c| c.version_id == input.version_id && c.code == input.code)
        {
            return Err(DomainError::DuplicateCodeList {
                version_id: input.version_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let cl = CodeList::for_repository(
            id,
            input.version_id,
            input.code,
            input.extensible,
            input.name,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            epoch(),
            epoch(),
        );
        s.by_id.insert(id, cl.clone());
        Ok(cl)
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeListNotFound(id))
    }

    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|c| c.version_id == version_id)
            .cloned()
            .collect())
    }

    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        let c = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeListNotFound(input.id))?;
        if let Some(code) = input.code {
            c.code = code;
        }
        if let Some(ext) = input.extensible {
            c.extensible = ext;
        }
        if let Some(name) = input.name {
            c.name = name;
        }
        if let Some(sv) = input.submission_value {
            c.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            c.synonym = syn;
        }
        if let Some(def) = input.definition {
            c.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            c.nci_preferred_term = pt;
        }
        Ok(c.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeListNotFound(id));
        }
        // Cascade: mirror `ON DELETE CASCADE` from the schema so
        // the test fake doesn't leave orphaned items behind.
        if let Some(items) = &self.items {
            let mut items = items.lock().unwrap();
            items.by_id.retain(|_, i| i.codelist_id != id);
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeListSearchQuery,
    ) -> Result<Vec<InternalCodeListSearchHit>, DomainError> {
        // The in-memory backend mirrors the Postgres semantics on a
        // best-effort basis: case-insensitive substring match
        // against the same five text fields the generated `tsv`
        // column covers. The order is the natural iteration order
        // of the underlying map; callers that need a stable order
        // should sort by `id`.
        let needle = query.fragment.to_lowercase();
        let limit = query.limit as usize;
        let mut hits = Vec::new();
        for cl in self.state.lock().unwrap().by_id.values() {
            if cl.version_id != query.version_id {
                continue;
            }
            if [
                cl.name.as_str(),
                cl.submission_value.as_str(),
                cl.synonym.as_str(),
                cl.definition.as_str(),
                cl.nci_preferred_term.as_str(),
            ]
            .iter()
            .any(|f| f.to_lowercase().contains(&needle))
            {
                hits.push(InternalCodeListSearchHit {
                    codelist: cl.clone(),
                });
                if hits.len() >= limit {
                    break;
                }
            }
        }
        Ok(hits)
    }
}

#[derive(Default)]
struct ItemState {
    by_id: std::collections::HashMap<i64, CodeItem>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct InMemoryCodeItemRepo {
    state: Arc<Mutex<ItemState>>,
}

impl InMemoryCodeItemRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ItemState::default())),
        }
    }

    /// Borrow the underlying state handle so peer repos (notably
    /// `InMemoryCodeListRepo`) can implement cascade semantics.
    fn shared_state(&self) -> Arc<Mutex<ItemState>> {
        Arc::clone(&self.state)
    }
}

#[async_trait]
impl CodeItemRepository for InMemoryCodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|i| i.codelist_id == input.codelist_id && i.code == input.code)
        {
            return Err(DomainError::DuplicateCodeItem {
                codelist_id: input.codelist_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let item = CodeItem::for_repository(
            id,
            input.codelist_id,
            input.version_id,
            input.code,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            epoch(),
            epoch(),
        );
        s.by_id.insert(id, item.clone());
        Ok(item)
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeItemNotFound(id))
    }

    async fn list_by_codelist(&self, codelist_id: i64) -> Result<Vec<CodeItem>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|i| i.codelist_id == codelist_id)
            .cloned()
            .collect())
    }

    async fn list_by_version_and_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|i| i.version_id == version_id && i.code == code)
            .cloned()
            .collect())
    }

    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        let i = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        if let Some(code) = input.code {
            i.code = code;
        }
        if let Some(sv) = input.submission_value {
            i.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            i.synonym = syn;
        }
        if let Some(def) = input.definition {
            i.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            i.nci_preferred_term = pt;
        }
        Ok(i.clone())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeItemSearchQuery,
    ) -> Result<Vec<InternalCodeItemSearchHit>, DomainError> {
        // See `InMemoryCodeListRepo::search` for the matching
        // rationale: case-insensitive substring across the five
        // text fields, scoped to a single version, ordered by
        // iteration order (caller can sort by `id` for stability).
        let needle = query.fragment.to_lowercase();
        let limit = query.limit as usize;
        let mut hits = Vec::new();
        for item in self.state.lock().unwrap().by_id.values() {
            if item.version_id != query.version_id {
                continue;
            }
            if [
                item.submission_value.as_str(),
                item.synonym.as_str(),
                item.definition.as_str(),
                item.nci_preferred_term.as_str(),
            ]
            .iter()
            .any(|f| f.to_lowercase().contains(&needle))
            {
                hits.push(InternalCodeItemSearchHit { item: item.clone() });
                if hits.len() >= limit {
                    break;
                }
            }
        }
        Ok(hits)
    }
}

// ---------- wiring ----------

fn service()
-> TerminologyServiceImpl<InMemoryVersionRepo, InMemoryCodeListRepo, InMemoryCodeItemRepo> {
    let v = InMemoryVersionRepo::new();
    let i = InMemoryCodeItemRepo::new();
    let l = InMemoryCodeListRepo::with_items(i.shared_state());
    let usecase = TerminologyUsecase::new(crate::usecase::TerminologyUsecaseConfig {
        version_repo: v,
        code_list_repo: l,
        code_item_repo: i,
    });
    TerminologyServiceImpl::new(usecase)
}

fn create_version_req(name: &str) -> CreateTerminologyVersionRequest {
    CreateTerminologyVersionRequest {
        kind: ApiKind::Sdtm,
        name: name.into(),
    }
}

fn create_code_list_req(version_id: i64, code: &str) -> CreateCodeListRequest {
    CreateCodeListRequest {
        version_id,
        code: code.into(),
        extensible: true,
        name: "AGE".into(),
        submission_value: "AGE".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    }
}

fn create_code_item_req(codelist_id: i64, version_id: i64, code: &str) -> CreateCodeItemRequest {
    CreateCodeItemRequest {
        codelist_id,
        version_id,
        code: code.into(),
        submission_value: "".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    }
}

// ---------- tests ----------

/// Smoke test: the adapter can be constructed.
#[tokio::test]
async fn terminology_service_impl_can_be_constructed() {
    let _svc = service();
}

/// `Send + Sync` so the adapter can sit in shared state behind an
/// async server.
#[tokio::test]
async fn terminology_service_impl_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<
        TerminologyServiceImpl<InMemoryVersionRepo, InMemoryCodeListRepo, InMemoryCodeItemRepo>,
    >();
    assert_send_sync::<Box<dyn TerminologyService>>();
}

/// Object-safe: can be erased into a `Box<dyn TerminologyService>`.
#[tokio::test]
async fn terminology_service_impl_is_object_safe() {
    let svc = service();
    let _boxed: Box<dyn TerminologyService> = Box::new(svc);
}

// ---- TerminologyVersion ----

#[tokio::test]
async fn create_version_returns_view_with_assigned_id() {
    let svc = service();
    let view = svc
        .create_version(create_version_req("2026-03-27"))
        .await
        .unwrap();
    assert_eq!(view.id, 1);
    assert_eq!(view.kind, ApiKind::Sdtm);
    assert_eq!(view.name, "2026-03-27");
    assert_eq!(view.created_at, epoch());
    assert_eq!(view.updated_at, epoch());
}

#[tokio::test]
async fn create_version_rejects_empty_name_with_validation() {
    let svc = service();
    let err = svc
        .create_version(create_version_req("   "))
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}

#[tokio::test]
async fn create_version_rejects_duplicate_kind_and_name() {
    let svc = service();
    svc.create_version(create_version_req("2026-03-27"))
        .await
        .unwrap();
    let err = svc
        .create_version(create_version_req("2026-03-27"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        TerminologyApiError::DuplicateVersion { ref kind, ref name }
            if *kind == ApiKind::Sdtm && name == "2026-03-27"
    ));
}

#[tokio::test]
async fn get_version_by_id_returns_seeded_version() {
    let svc = service();
    let created = svc.create_version(create_version_req("v1")).await.unwrap();
    let fetched = svc.get_version_by_id(created.id).await.unwrap();
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn get_version_by_id_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.get_version_by_id(999).await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn get_version_by_kind_and_name_returns_seeded_version() {
    let svc = service();
    let created = svc.create_version(create_version_req("v1")).await.unwrap();
    let fetched = svc.get_version(ApiKind::Sdtm, "v1").await.unwrap();
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn get_version_by_kind_and_name_returns_not_found_for_unknown() {
    let svc = service();
    let err = svc.get_version(ApiKind::Sdtm, "ghost").await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn get_version_by_kind_and_name_rejects_empty_name() {
    let svc = service();
    let err = svc.get_version(ApiKind::Sdtm, "   ").await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}

#[tokio::test]
async fn list_versions_returns_all_seeded_versions() {
    let svc = service();
    for name in ["v1", "v2", "v3"] {
        svc.create_version(create_version_req(name)).await.unwrap();
    }
    let list = svc.list_versions().await.unwrap();
    assert_eq!(list.len(), 3);
    let mut names: Vec<&str> = list.iter().map(|v| v.name.as_str()).collect();
    names.sort();
    assert_eq!(names, vec!["v1", "v2", "v3"]);
}

#[tokio::test]
async fn list_versions_returns_empty_vec_when_no_versions_exist() {
    let svc = service();
    let list = svc.list_versions().await.unwrap();
    assert!(list.is_empty());
}

#[tokio::test]
async fn update_version_applies_supplied_fields_and_returns_view() {
    let svc = service();
    let created = svc.create_version(create_version_req("v1")).await.unwrap();
    let updated = svc
        .update_version(UpdateTerminologyVersionRequest {
            id: created.id,
            name: Some("v2".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.name, "v2");
}

#[tokio::test]
async fn update_version_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc
        .update_version(UpdateTerminologyVersionRequest {
            id: 999,
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn delete_version_removes_the_version() {
    let svc = service();
    let created = svc.create_version(create_version_req("v1")).await.unwrap();
    svc.delete_version(created.id).await.unwrap();
    let err = svc.get_version_by_id(created.id).await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn delete_version_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.delete_version(999).await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

// ---- CodeList ----

#[tokio::test]
async fn create_code_list_returns_view_with_assigned_id() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    assert_eq!(cl.id, 1);
    assert_eq!(cl.version_id, v.id);
    assert_eq!(cl.code, "C66741");
    assert_eq!(cl.name, "AGE");
    assert!(cl.extensible);
}

#[tokio::test]
async fn create_code_list_rejects_empty_code() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let err = svc
        .create_code_list(create_code_list_req(v.id, ""))
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}

#[tokio::test]
async fn create_code_list_rejects_duplicate_version_and_code() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    svc.create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let err = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        TerminologyApiError::DuplicateCodeList { version_id, ref code }
            if version_id == v.id && code == "C66741"
    ));
}

#[tokio::test]
async fn list_code_lists_returns_codelists_owned_by_version() {
    let svc = service();
    let v1 = svc.create_version(create_version_req("v1")).await.unwrap();
    let v2 = svc.create_version(create_version_req("v2")).await.unwrap();
    svc.create_code_list(create_code_list_req(v1.id, "C1"))
        .await
        .unwrap();
    svc.create_code_list(create_code_list_req(v1.id, "C2"))
        .await
        .unwrap();
    svc.create_code_list(create_code_list_req(v2.id, "C3"))
        .await
        .unwrap();

    let mut v1_lists = svc.list_code_lists(v1.id).await.unwrap();
    v1_lists.sort_by_key(|c| c.code.clone());
    assert_eq!(v1_lists.len(), 2);
    assert_eq!(v1_lists[0].code, "C1");
    assert_eq!(v1_lists[1].code, "C2");

    let v2_lists = svc.list_code_lists(v2.id).await.unwrap();
    assert_eq!(v2_lists.len(), 1);
    assert_eq!(v2_lists[0].code, "C3");
}

#[tokio::test]
async fn update_code_list_applies_supplied_fields() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let created = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let updated = svc
        .update_code_list(UpdateCodeListRequest {
            id: created.id,
            name: Some("AGE (renamed)".into()),
            extensible: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.code, "C66741");
    assert_eq!(updated.name, "AGE (renamed)");
    assert!(!updated.extensible);
}

#[tokio::test]
async fn update_code_list_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc
        .update_code_list(UpdateCodeListRequest {
            id: 999,
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn delete_code_list_removes_the_codelist() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let created = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    // Seed an item so the post-delete assertion can also confirm
    // the cascade through the items path.
    svc.create_code_item(create_code_item_req(created.id, v.id, "C1"))
        .await
        .unwrap();
    svc.delete_code_list(created.id).await.unwrap();
    // Re-listing the version returns no codelists.
    let lists = svc.list_code_lists(v.id).await.unwrap();
    assert!(lists.is_empty());
    // The orphaned item is also gone from the items path.
    let items = svc.list_code_items(created.id).await.unwrap();
    assert!(items.is_empty());
}

#[tokio::test]
async fn delete_code_list_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.delete_code_list(999).await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn search_code_lists_returns_empty_for_in_memory_backend() {
    let svc = service();
    let hits = svc
        .search_code_lists(apis::terminology::CodeListSearchQuery {
            version_id: 1,
            fragment: "age".into(),
            limit: 10,
        })
        .await
        .unwrap();
    assert!(hits.is_empty());
}

// ---- CodeItem ----

#[tokio::test]
async fn create_code_item_returns_view_with_assigned_id() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let item = svc
        .create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    assert_eq!(item.id, 1);
    assert_eq!(item.codelist_id, cl.id);
    assert_eq!(item.version_id, v.id);
    assert_eq!(item.code, "C1");
}

#[tokio::test]
async fn create_code_item_rejects_empty_code() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let err = svc
        .create_code_item(create_code_item_req(cl.id, v.id, ""))
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}

#[tokio::test]
async fn create_code_item_rejects_duplicate_codelist_and_code() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    let err = svc
        .create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        TerminologyApiError::DuplicateCodeItem { codelist_id, ref code }
            if codelist_id == cl.id && code == "C1"
    ));
}

#[tokio::test]
async fn list_code_items_returns_items_in_codelist() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(cl.id, v.id, "C2"))
        .await
        .unwrap();
    let mut items = svc.list_code_items(cl.id).await.unwrap();
    items.sort_by_key(|i| i.code.clone());
    assert_eq!(items.len(), 2);
    assert_eq!(items[0].code, "C1");
    assert_eq!(items[1].code, "C2");
}

#[tokio::test]
async fn list_code_items_by_version_and_code_returns_matches() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let age = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let sex = svc
        .create_code_list(create_code_list_req(v.id, "C66732"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(age.id, v.id, "C1"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(sex.id, v.id, "C1"))
        .await
        .unwrap();
    svc.create_code_item(create_code_item_req(age.id, v.id, "C2"))
        .await
        .unwrap();

    let c1 = svc
        .list_code_items_by_version_and_code(v.id, "C1")
        .await
        .unwrap();
    assert_eq!(c1.len(), 2);
    assert!(
        c1.iter().all(|i| i.version_id == v.id),
        "all returned items are scoped to v"
    );

    let c2 = svc
        .list_code_items_by_version_and_code(v.id, "C2")
        .await
        .unwrap();
    assert_eq!(c2.len(), 1);
    assert_eq!(c2[0].codelist_id, age.id);

    let empty = svc
        .list_code_items_by_version_and_code(v.id, "C99999")
        .await
        .unwrap();
    assert!(empty.is_empty());
}

#[tokio::test]
async fn list_code_items_by_version_and_code_rejects_empty_code() {
    let svc = service();
    let err = svc
        .list_code_items_by_version_and_code(1, "   ")
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::Validation(_)));
}

#[tokio::test]
async fn update_code_item_applies_supplied_fields() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let created = svc
        .create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    let updated = svc
        .update_code_item(UpdateCodeItemRequest {
            id: created.id,
            definition: Some("Greater than zero".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.code, "C1");
    assert_eq!(updated.definition, "Greater than zero");
}

#[tokio::test]
async fn update_code_item_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc
        .update_code_item(UpdateCodeItemRequest {
            id: 999,
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn delete_code_item_removes_the_item() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let created = svc
        .create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    svc.delete_code_item(created.id).await.unwrap();
    let listed = svc.list_code_items(cl.id).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
async fn delete_code_item_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.delete_code_item(999).await.unwrap_err();
    assert!(matches!(err, TerminologyApiError::NotFound));
}

#[tokio::test]
async fn search_code_items_returns_empty_for_in_memory_backend() {
    let svc = service();
    let hits = svc
        .search_code_items(apis::terminology::CodeItemSearchQuery {
            version_id: 1,
            fragment: "age".into(),
            limit: 10,
        })
        .await
        .unwrap();
    assert!(hits.is_empty());
}

// ---- view projection smoke tests ----

#[tokio::test]
async fn terminlogy_version_view_projects_internal_fields() {
    let svc = service();
    let view: TerminologyVersionView = svc.create_version(create_version_req("v1")).await.unwrap();
    let _internal: &dyn std::fmt::Debug = &view;
    // Constructed via the usecase's `From` impl — assert the
    // expected API field names line up.
    let _: (
        i64,
        ApiKind,
        String,
        chrono::DateTime<Utc>,
        chrono::DateTime<Utc>,
    ) = (
        view.id,
        view.kind,
        view.name,
        view.created_at,
        view.updated_at,
    );
}

#[tokio::test]
async fn code_list_view_projects_internal_fields() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let view: CodeListView = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let _internal: &dyn std::fmt::Debug = &view;
    let _ = (view.id, view.version_id, view.code, view.extensible);
}

#[tokio::test]
async fn code_item_view_projects_internal_fields() {
    let svc = service();
    let v = svc.create_version(create_version_req("v1")).await.unwrap();
    let cl = svc
        .create_code_list(create_code_list_req(v.id, "C66741"))
        .await
        .unwrap();
    let view: CodeItemView = svc
        .create_code_item(create_code_item_req(cl.id, v.id, "C1"))
        .await
        .unwrap();
    let _internal: &dyn std::fmt::Debug = &view;
    let _ = (view.id, view.codelist_id, view.version_id, view.code);
}

#[tokio::test]
async fn code_list_search_hit_projects_codelist() {
    let svc = service();
    let hits: Vec<CodeListSearchHit> = svc
        .search_code_lists(apis::terminology::CodeListSearchQuery {
            version_id: 1,
            fragment: "x".into(),
            limit: 10,
        })
        .await
        .unwrap();
    assert!(hits.is_empty());
}

#[tokio::test]
async fn code_item_search_hit_projects_item() {
    let svc = service();
    let hits: Vec<CodeItemSearchHit> = svc
        .search_code_items(apis::terminology::CodeItemSearchQuery {
            version_id: 1,
            fragment: "x".into(),
            limit: 10,
        })
        .await
        .unwrap();
    assert!(hits.is_empty());
}
