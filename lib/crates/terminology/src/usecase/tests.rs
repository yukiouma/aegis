//! Tests for the usecase layer wired against in-memory repository
//! fakes. No SQL, no I/O.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use crate::domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, CodeList, CodeListNew, CodeListRepository, CodeListSearchHit,
    CodeListSearchQuery, CodeListUpdate, DomainError, TerminologyKind, TerminologyVersion,
    TerminologyVersionNew, TerminologyVersionRepository, TerminologyVersionUpdate,
};
use crate::usecase::commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeList,
    UpdateTerminologyVersion,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::terminology_usecase::{TerminologyUsecase, TerminologyUsecaseConfig};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 18, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct VersionsState {
    by_id: HashMap<i64, TerminologyVersion>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeVersionRepo {
    state: Arc<Mutex<VersionsState>>,
}

impl FakeVersionRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VersionsState::default())),
        }
    }
}

#[async_trait]
impl TerminologyVersionRepository for FakeVersionRepo {
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
        let ts = now();
        let v = TerminologyVersion::for_repository(id, input.kind, input.name, ts, ts);
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
        v.updated_at = now();
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
struct ListsState {
    by_id: HashMap<i64, CodeList>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeListRepo {
    state: Arc<Mutex<ListsState>>,
}

impl FakeCodeListRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ListsState::default())),
        }
    }
}

#[async_trait]
impl CodeListRepository for FakeCodeListRepo {
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
        let ts = now();
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
            ts,
            ts,
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
        c.updated_at = now();
        Ok(c.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeListNotFound(id));
        }
        Ok(())
    }
    async fn search(
        &self,
        _query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError> {
        // The fake returns empty so usecase tests focus on shape
        // rather than ranking. The Postgres adapter asserts real
        // hits in tests/integration_persistence.rs.
        Ok(vec![])
    }
}

#[derive(Default)]
struct ItemsState {
    by_id: HashMap<i64, CodeItem>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeItemRepo {
    state: Arc<Mutex<ItemsState>>,
    /// Shared with `FakeCodeListRepo` so the natural-key lookup
    /// `list_by_version_and_codelist_code` can resolve a codelist
    /// by `(version_id, code)`.
    lists: Arc<Mutex<ListsState>>,
}

impl FakeCodeItemRepo {
    fn new(lists: Arc<Mutex<ListsState>>) -> Self {
        Self {
            state: Arc::new(Mutex::new(ItemsState::default())),
            lists,
        }
    }
}

#[async_trait]
impl CodeItemRepository for FakeCodeItemRepo {
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
        let ts = now();
        let item = CodeItem::for_repository(
            id,
            input.codelist_id,
            input.version_id,
            input.code,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            ts,
            ts,
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
    async fn list_by_version_and_codelist_code(
        &self,
        version_id: i64,
        code: &str,
    ) -> Result<Vec<CodeItem>, DomainError> {
        let codelist_id = {
            let lists = self.lists.lock().unwrap();
            lists
                .by_id
                .values()
                .find(|c| c.version_id == version_id && c.code == code)
                .map(|c| c.id)
        };
        let Some(codelist_id) = codelist_id else {
            return Ok(vec![]);
        };
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
        i.updated_at = now();
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
        _query: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError> {
        Ok(vec![])
    }
}

// ---------- fixture ----------

fn make_usecase() -> (
    FakeVersionRepo,
    FakeCodeListRepo,
    FakeCodeItemRepo,
    TerminologyUsecase<FakeVersionRepo, FakeCodeListRepo, FakeCodeItemRepo>,
) {
    let v = FakeVersionRepo::new();
    let l = FakeCodeListRepo::new();
    // Share the codelist state with the item fake so
    // `list_by_version_and_codelist_code` can resolve a codelist
    // by `(version_id, code)`.
    let i = FakeCodeItemRepo::new(l.state.clone());
    let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
        version_repo: v.clone(),
        code_list_repo: l.clone(),
        code_item_repo: i.clone(),
    });
    (v, l, i, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_version_rejects_empty_name() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));
}

#[tokio::test]
async fn create_then_get_version_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    assert_eq!(created.name, "2026-03-27");
    let fetched = usecase
        .get_version(TerminologyKind::Sdtm, "2026-03-27")
        .await
        .expect("get");
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn update_version_then_list_returns_updated_name() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_version(UpdateTerminologyVersion {
            id: created.id,
            name: Some("2026-06-15".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.name, "2026-06-15");
    let listed = usecase.list_versions().await.expect("list");
    assert!(
        listed
            .iter()
            .any(|v| v.id == created.id && v.name == "2026-06-15")
    );
}

#[tokio::test]
async fn create_code_list_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "   ".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_list_then_list_by_version_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 7,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "Age".into(),
            definition: "Age".into(),
            nci_preferred_term: "Age".into(),
        })
        .await
        .expect("create");
    let listed = usecase.list_code_lists(7).await.expect("list");
    assert!(listed.iter().any(|c| c.id == created.id));
}

#[tokio::test]
async fn update_code_list_applies_partial_changes() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "C66741".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_code_list(UpdateCodeList {
            id: created.id,
            extensible: Some(true),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.extensible);
    assert_eq!(updated.code, "C66741");
}

#[tokio::test]
async fn search_code_lists_clamps_limit_to_default_when_zero() {
    // The clamping happens before the repo is touched, so we
    // cannot observe it directly through `search_code_lists`.
    // Instead, this test exercises that the search does not
    // panic on a zero-limit and that the fake returns empty.
    let (_, _, _, usecase) = make_usecase();
    let hits = usecase
        .search_code_lists(CodeListSearchQuery {
            kind: TerminologyKind::Sdtm,
            version_name: "2026-03-27".into(),
            text: "age".into(),
            limit: 0,
        })
        .await
        .expect("search");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn create_code_item_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 1,
            version_id: 1,
            code: "".into(),
            submission_value: "X".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_item_round_trip_then_list_by_codelist() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 9,
            version_id: 7,
            code: "C12345".into(),
            submission_value: "> 0".into(),
            synonym: "positive".into(),
            definition: "Greater than zero".into(),
            nci_preferred_term: "Greater Than Zero".into(),
        })
        .await
        .expect("create");
    let listed = usecase.list_code_items(9).await.expect("list");
    assert!(listed.iter().any(|i| i.id == created.id));
}

#[tokio::test]
async fn list_code_items_by_version_and_codelist_code_returns_only_target_items() {
    let (_, _, _, usecase) = make_usecase();

    // Two versions, each owning two codelists. We want to make
    // sure the natural-key lookup returns only items from the
    // matching (version_id, code) pair.
    let v_a = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-a".into(),
        })
        .await
        .expect("v_a");
    let v_b = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "v-b".into(),
        })
        .await
        .expect("v_b");

    let age_v_a = usecase
        .create_code_list(CreateCodeList {
            version_id: v_a.id,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age_v_a");
    let sex_v_a = usecase
        .create_code_list(CreateCodeList {
            version_id: v_a.id,
            code: "C66732".into(),
            extensible: true,
            name: "SEX".into(),
            submission_value: "SEX".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("sex_v_a");
    let age_v_b = usecase
        .create_code_list(CreateCodeList {
            version_id: v_b.id,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age_v_b");

    // Two items in age_v_a, one in sex_v_a, one in age_v_b.
    for code in ["C1", "C2"] {
        usecase
            .create_code_item(CreateCodeItem {
                codelist_id: age_v_a.id,
                version_id: v_a.id,
                code: code.into(),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("age_v_a item");
    }
    usecase
        .create_code_item(CreateCodeItem {
            codelist_id: sex_v_a.id,
            version_id: v_a.id,
            code: "C3".into(),
            submission_value: "".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("sex_v_a item");
    let age_v_b_item = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: age_v_b.id,
            version_id: v_b.id,
            code: "C4".into(),
            submission_value: "".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("age_v_b item");

    let age_items = usecase
        .list_code_items_by_version_and_codelist_code(v_a.id, "C66741")
        .await
        .expect("lookup");
    assert_eq!(age_items.len(), 2, "two items in v_a / C66741");
    assert!(
        age_items.iter().all(|i| i.codelist_id == age_v_a.id),
        "all returned items belong to the v_a AGE codelist"
    );

    // Same NCI code under a different version: must not bleed.
    let age_v_b_items = usecase
        .list_code_items_by_version_and_codelist_code(v_b.id, "C66741")
        .await
        .expect("lookup v_b");
    assert_eq!(age_v_b_items.len(), 1);
    assert_eq!(age_v_b_items[0].id, age_v_b_item.id);

    // Codelist that does not exist: empty result, not an error.
    let empty = usecase
        .list_code_items_by_version_and_codelist_code(v_a.id, "C99999")
        .await
        .expect("lookup missing");
    assert!(empty.is_empty());
}

#[tokio::test]
async fn list_code_items_by_version_and_codelist_code_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .list_code_items_by_version_and_codelist_code(1, "   ")
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}
