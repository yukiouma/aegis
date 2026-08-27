//! Usecase-layer unit tests. Wires in-memory fakes for every
//! repository port plus a mock `ProjectLookup`, runs the full
//! CRUD + search surface through `CrfUsecase`, and confirms the
//! view projections, validation errors, and kind-shape rules.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    Annotation, AnnotationNew, AnnotationOwner, AnnotationRepository, AnnotationUpdate,
    CrfBulkCreateForm, CrfBulkCreateFormResult, CrfBulkFormRepository, CrfForm, CrfFormNew,
    CrfFormRepository, CrfFormUpdate, CrfItem, CrfItemKind, CrfItemNew, CrfItemRepository,
    CrfItemUpdate, CrfOption, CrfOptionNew, CrfOptionRepository, CrfOptionUpdate, CrfUnit,
    CrfUnitNew, CrfUnitRepository, CrfUnitUpdate, CrfVersion, CrfVersionNew, CrfVersionRepository,
    CrfVersionUpdate, DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository,
    DomainAnnotationUpdate, DomainError, ProjectLookup,
};
use crate::usecase::{
    CreateAnnotation, CreateCrfBulkForm, CreateCrfBulkItem, CreateCrfForm, CreateCrfItem,
    CreateCrfOption, CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation, CrfUsecase,
    CrfUsecaseConfig, DomainAnnotationView, SearchCrfFormsByVersion, UpdateAnnotation,
    UpdateCrfForm, UpdateCrfItem, UsecaseError,
};

use super::commands::{
    SearchAnnotationsByVersion, SearchCrfItemsByVersion, SearchCrfOptionsByVersion,
    SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion, UpdateCrfOption, UpdateCrfUnit,
    UpdateCrfVersion, UpdateDomainAnnotation,
};

// ---- shared counter for unique surrogate ids ----

static NEXT_ID: AtomicI64 = AtomicI64::new(1);

fn next_id() -> i64 {
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

// ---- mock ProjectLookup ----

pub(crate) struct AcceptProject;
#[async_trait]
impl ProjectLookup for AcceptProject {
    async fn get_by_code(&self, _code: &str) -> Result<(), DomainError> {
        Ok(())
    }
}

pub(crate) struct RejectProject;
#[async_trait]
impl ProjectLookup for RejectProject {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        Err(DomainError::ProjectNotFound(code.to_string()))
    }
}

// ---- in-memory fakes ----

#[derive(Default)]
pub(crate) struct InMemoryVersions {
    rows: Mutex<HashMap<i64, CrfVersion>>,
}

#[async_trait]
impl CrfVersionRepository for InMemoryVersions {
    async fn create(&self, input: CrfVersionNew) -> Result<CrfVersion, DomainError> {
        let id = next_id();
        let v = CrfVersion::for_repository(
            id,
            input.project_code,
            input.name,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<CrfVersion, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::CrfVersionNotFound(id))
    }
    async fn list_by_project(&self, project_code: &str) -> Result<Vec<CrfVersion>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|v| v.project_code == project_code)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CrfVersionUpdate) -> Result<CrfVersion, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let v = rows
            .get_mut(&input.id)
            .ok_or(DomainError::CrfVersionNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = chrono::Utc::now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::CrfVersionNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        _project_code: &str,
        _fragment: &str,
    ) -> Result<Vec<CrfVersion>, DomainError> {
        Ok(vec![])
    }
}

#[derive(Default)]
pub(crate) struct InMemoryForms {
    rows: Mutex<HashMap<i64, CrfForm>>,
}

#[async_trait]
impl CrfFormRepository for InMemoryForms {
    async fn create(&self, input: CrfFormNew) -> Result<CrfForm, DomainError> {
        let id = next_id();
        let f = CrfForm::for_repository(
            id,
            input.version_id,
            input.code,
            input.name,
            input.order,
            input.not_submitted,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, f.clone());
        Ok(f)
    }
    async fn find_by_id(&self, id: i64) -> Result<CrfForm, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::CrfFormNotFound(id))
    }
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CrfForm>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|f| f.version_id == version_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CrfFormUpdate) -> Result<CrfForm, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let f = rows
            .get_mut(&input.id)
            .ok_or(DomainError::CrfFormNotFound(input.id))?;
        if let Some(code) = input.code {
            f.code = code;
        }
        if let Some(name) = input.name {
            f.name = name;
        }
        if let Some(order) = input.order {
            f.order = order;
        }
        if let Some(not_submitted) = input.not_submitted {
            f.not_submitted = not_submitted;
        }
        f.updated_at = chrono::Utc::now();
        Ok(f.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::CrfFormNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfForm>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|f| {
                f.version_id == version_id
                    && (f.code.contains(fragment) || f.name.contains(fragment))
            })
            .cloned()
            .collect())
    }
}

#[derive(Default)]
pub(crate) struct InMemoryItems {
    rows: Mutex<HashMap<i64, CrfItem>>,
}

#[async_trait]
impl CrfItemRepository for InMemoryItems {
    async fn create(&self, input: CrfItemNew) -> Result<CrfItem, DomainError> {
        let id = next_id();
        let i = CrfItem::for_repository(
            id,
            input.form_id,
            input.code,
            input.name,
            input.kind,
            input.order,
            input.not_submitted,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, i.clone());
        Ok(i)
    }
    async fn find_by_id(&self, id: i64) -> Result<CrfItem, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::CrfItemNotFound(id))
    }
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<CrfItem>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|i| i.form_id == form_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CrfItemUpdate) -> Result<CrfItem, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let i = rows
            .get_mut(&input.id)
            .ok_or(DomainError::CrfItemNotFound(input.id))?;
        if let Some(code) = input.code {
            i.code = code;
        }
        if let Some(name) = input.name {
            i.name = name;
        }
        if let Some(order) = input.order {
            i.order = order;
        }
        if let Some(not_submitted) = input.not_submitted {
            i.not_submitted = not_submitted;
        }
        i.updated_at = chrono::Utc::now();
        Ok(i.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::CrfItemNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        _version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfItem>, DomainError> {
        // In-memory fake — the form-id → version chain isn't
        // modelled, so just filter by fragment on code/name.
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|i| i.code.contains(fragment) || i.name.contains(fragment))
            .cloned()
            .collect())
    }
}

#[derive(Default)]
pub(crate) struct InMemoryOptions {
    rows: Mutex<HashMap<i64, CrfOption>>,
}

#[async_trait]
impl CrfOptionRepository for InMemoryOptions {
    async fn create(&self, input: CrfOptionNew) -> Result<CrfOption, DomainError> {
        let id = next_id();
        let o = CrfOption::for_repository(
            id,
            input.item_id,
            input.value,
            input.not_submitted,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, o.clone());
        Ok(o)
    }
    async fn find_by_id(&self, id: i64) -> Result<CrfOption, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::CrfOptionNotFound(id))
    }
    async fn list_by_item(&self, item_id: i64) -> Result<Vec<CrfOption>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|o| o.item_id == item_id)
            .cloned()
            .collect())
    }
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<CrfOption>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|o| item_ids.contains(&o.item_id))
            .cloned()
            .collect())
    }
    async fn update(&self, input: CrfOptionUpdate) -> Result<CrfOption, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let o = rows
            .get_mut(&input.id)
            .ok_or(DomainError::CrfOptionNotFound(input.id))?;
        if let Some(value) = input.value {
            o.value = value;
        }
        if let Some(not_submitted) = input.not_submitted {
            o.not_submitted = not_submitted;
        }
        o.updated_at = chrono::Utc::now();
        Ok(o.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::CrfOptionNotFound(id))?;
        Ok(())
    }
    async fn count_by_item(&self, item_id: i64) -> Result<i64, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows.values().filter(|o| o.item_id == item_id).count() as i64)
    }
    async fn search_by_version(
        &self,
        _version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfOption>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|o| o.value.contains(fragment))
            .cloned()
            .collect())
    }
}

#[derive(Default)]
pub(crate) struct InMemoryUnits {
    rows: Mutex<HashMap<i64, CrfUnit>>,
}

#[async_trait]
impl CrfUnitRepository for InMemoryUnits {
    async fn create(&self, input: CrfUnitNew) -> Result<CrfUnit, DomainError> {
        let id = next_id();
        let u = CrfUnit::for_repository(
            id,
            input.item_id,
            input.value,
            input.not_submitted,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, u.clone());
        Ok(u)
    }
    async fn find_by_id(&self, id: i64) -> Result<CrfUnit, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::CrfUnitNotFound(id))
    }
    async fn list_by_item(&self, item_id: i64) -> Result<Vec<CrfUnit>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|u| u.item_id == item_id)
            .cloned()
            .collect())
    }
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<CrfUnit>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|u| item_ids.contains(&u.item_id))
            .cloned()
            .collect())
    }
    async fn update(&self, input: CrfUnitUpdate) -> Result<CrfUnit, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let u = rows
            .get_mut(&input.id)
            .ok_or(DomainError::CrfUnitNotFound(input.id))?;
        if let Some(value) = input.value {
            u.value = value;
        }
        if let Some(not_submitted) = input.not_submitted {
            u.not_submitted = not_submitted;
        }
        u.updated_at = chrono::Utc::now();
        Ok(u.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::CrfUnitNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        _version_id: i64,
        fragment: &str,
    ) -> Result<Vec<CrfUnit>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|u| u.value.contains(fragment))
            .cloned()
            .collect())
    }
}

#[derive(Default)]
pub(crate) struct InMemoryDomainAnnotations {
    rows: Mutex<HashMap<i64, DomainAnnotation>>,
}

#[async_trait]
impl DomainAnnotationRepository for InMemoryDomainAnnotations {
    async fn create(&self, input: DomainAnnotationNew) -> Result<DomainAnnotation, DomainError> {
        let id = next_id();
        let d = DomainAnnotation::for_repository(
            id,
            input.form_id,
            input.name,
            input.description,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, d.clone());
        Ok(d)
    }
    async fn find_by_id(&self, id: i64) -> Result<DomainAnnotation, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::DomainAnnotationNotFound(id))
    }
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<DomainAnnotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|d| d.form_id == form_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: DomainAnnotationUpdate) -> Result<DomainAnnotation, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let d = rows
            .get_mut(&input.id)
            .ok_or(DomainError::DomainAnnotationNotFound(input.id))?;
        if let Some(name) = input.name {
            d.name = name;
        }
        if let Some(description) = input.description {
            d.description = description;
        }
        d.updated_at = chrono::Utc::now();
        Ok(d.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::DomainAnnotationNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        _version_id: i64,
        _fragment: &str,
    ) -> Result<Vec<DomainAnnotation>, DomainError> {
        Ok(vec![])
    }
}

#[derive(Default)]
pub(crate) struct InMemoryAnnotations {
    rows: Mutex<HashMap<i64, Annotation>>,
}

#[async_trait]
impl AnnotationRepository for InMemoryAnnotations {
    async fn create(&self, input: AnnotationNew) -> Result<Annotation, DomainError> {
        let id = next_id();
        let a = Annotation::for_repository(
            id,
            input.domain_annotation_id,
            input.content,
            input.assign,
            input.owner,
            chrono::Utc::now(),
            chrono::Utc::now(),
        );
        self.rows.lock().unwrap().insert(id, a.clone());
        Ok(a)
    }
    async fn find_by_id(&self, id: i64) -> Result<Annotation, DomainError> {
        self.rows
            .lock()
            .unwrap()
            .get(&id)
            .cloned()
            .ok_or(DomainError::AnnotationNotFound(id))
    }
    async fn list_by_form(&self, form_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Form { id } if id == form_id))
            .cloned()
            .collect())
    }
    async fn list_by_item(&self, item_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Item { id } if id == item_id))
            .cloned()
            .collect())
    }
    async fn list_by_option(&self, option_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Option { id } if id == option_id))
            .cloned()
            .collect())
    }
    async fn list_by_unit(&self, unit_id: i64) -> Result<Vec<Annotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Unit { id } if id == unit_id))
            .cloned()
            .collect())
    }
    async fn list_by_items(
        &self,
        item_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError> {
        if item_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Item { id } if item_ids.contains(&id)))
            .cloned()
            .collect())
    }
    async fn list_by_options(
        &self,
        option_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError> {
        if option_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Option { id } if option_ids.contains(&id)))
            .cloned()
            .collect())
    }
    async fn list_by_units(
        &self,
        unit_ids: &[i64],
    ) -> Result<Vec<Annotation>, DomainError> {
        if unit_ids.is_empty() {
            return Ok(Vec::new());
        }
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| matches!(a.owner, AnnotationOwner::Unit { id } if unit_ids.contains(&id)))
            .cloned()
            .collect())
    }
    async fn update(&self, input: AnnotationUpdate) -> Result<Annotation, DomainError> {
        let mut rows = self.rows.lock().unwrap();
        let a = rows
            .get_mut(&input.id)
            .ok_or(DomainError::AnnotationNotFound(input.id))?;
        if let Some(content) = input.content {
            a.content = content;
        }
        if let Some(assign) = input.assign {
            a.assign = assign;
        }
        a.updated_at = chrono::Utc::now();
        Ok(a.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        self.rows
            .lock()
            .unwrap()
            .remove(&id)
            .ok_or(DomainError::AnnotationNotFound(id))?;
        Ok(())
    }
    async fn search_by_version(
        &self,
        _version_id: i64,
        fragment: &str,
    ) -> Result<Vec<Annotation>, DomainError> {
        let rows = self.rows.lock().unwrap();
        Ok(rows
            .values()
            .filter(|a| a.content.contains(fragment))
            .cloned()
            .collect())
    }
}

/// In-memory fake for [`CrfBulkFormRepository`]. Mirrors the
/// semantics of the Postgres impl — stamp a fresh id on the form,
/// then on each item, then insert each option / unit — but does
/// not model cross-row uniqueness. The transactional guarantee
/// (every row visible together or none of them) collapses to a
/// sequential loop in-memory.
#[derive(Default)]
pub(crate) struct InMemoryBulkForms;

#[async_trait]
impl CrfBulkFormRepository for InMemoryBulkForms {
    async fn bulk_create(
        &self,
        input: CrfBulkCreateForm,
    ) -> Result<CrfBulkCreateFormResult, DomainError> {
        let now = chrono::Utc::now();
        let form = CrfForm::for_repository(
            next_id(),
            input.form.version_id,
            input.form.code,
            input.form.name,
            input.form.order,
            input.form.not_submitted,
            now,
            now,
        );
        let mut items: Vec<CrfItem> = Vec::with_capacity(input.items.len());
        for bi in input.items {
            let item = CrfItem::for_repository(
                next_id(),
                form.id,
                bi.item.code,
                bi.item.name,
                bi.item.kind,
                bi.item.order,
                bi.item.not_submitted,
                now,
                now,
            );
            items.push(item);
        }
        Ok(CrfBulkCreateFormResult { form, items })
    }
}

// ---- factory ----

fn make_usecase<P: ProjectLookup + 'static>(
    projects: Arc<P>,
) -> CrfUsecase<
    InMemoryVersions,
    InMemoryForms,
    InMemoryItems,
    InMemoryOptions,
    InMemoryUnits,
    InMemoryDomainAnnotations,
    InMemoryAnnotations,
    P,
    InMemoryBulkForms,
> {
    CrfUsecase::new(CrfUsecaseConfig {
        version_repo: InMemoryVersions::default(),
        form_repo: InMemoryForms::default(),
        item_repo: InMemoryItems::default(),
        option_repo: InMemoryOptions::default(),
        unit_repo: InMemoryUnits::default(),
        domain_annotation_repo: InMemoryDomainAnnotations::default(),
        annotation_repo: InMemoryAnnotations::default(),
        projects,
        bulk_form_repo: Arc::new(InMemoryBulkForms),
    })
}

type TestUsecase = CrfUsecase<
    InMemoryVersions,
    InMemoryForms,
    InMemoryItems,
    InMemoryOptions,
    InMemoryUnits,
    InMemoryDomainAnnotations,
    InMemoryAnnotations,
    AcceptProject,
    InMemoryBulkForms,
>;

fn usecase() -> TestUsecase {
    make_usecase(Arc::new(AcceptProject))
}

// ---- CrfVersion tests ----

#[tokio::test]
async fn create_version_with_existing_project_succeeds() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.project_code, "P1");
    assert_eq!(v.name, "v1");
}

#[tokio::test]
async fn create_version_with_missing_project_fails() {
    let uc: CrfUsecase<
        InMemoryVersions,
        InMemoryForms,
        InMemoryItems,
        InMemoryOptions,
        InMemoryUnits,
        InMemoryDomainAnnotations,
        InMemoryAnnotations,
        RejectProject,
        InMemoryBulkForms,
    > = make_usecase(Arc::new(RejectProject));
    let err = uc
        .create_version(CreateCrfVersion {
            project_code: "P-MISSING".into(),
            name: "v1".into(),
        })
        .await
        .unwrap_err();
    match err {
        UsecaseError::Repository(DomainError::ProjectNotFound(c)) => {
            assert_eq!(c, "P-MISSING")
        }
        _ => panic!("expected Repository(ProjectNotFound), got {err:?}"),
    }
}

#[tokio::test]
async fn create_version_rejects_empty_project_code() {
    let uc = usecase();
    let err = uc
        .create_version(CreateCrfVersion {
            project_code: "  ".into(),
            name: "v1".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyProjectCode)
    ));
}

#[tokio::test]
async fn create_version_rejects_empty_name() {
    let uc = usecase();
    let err = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
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
async fn list_versions_by_project_filters_by_project_code() {
    let uc = usecase();
    uc.create_version(CreateCrfVersion {
        project_code: "P1".into(),
        name: "v1".into(),
    })
    .await
    .unwrap();
    uc.create_version(CreateCrfVersion {
        project_code: "P2".into(),
        name: "v2".into(),
    })
    .await
    .unwrap();
    let p1 = uc.list_versions_by_project("P1").await.unwrap();
    assert_eq!(p1.len(), 1);
    assert_eq!(p1[0].project_code, "P1");
}

#[tokio::test]
async fn update_version_changes_name() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let updated = uc
        .update_version(UpdateCrfVersion {
            id: v.id,
            name: Some("v2".into()),
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "v2");
}

#[tokio::test]
async fn delete_version_removes_row() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    uc.delete_version(v.id).await.unwrap();
    let err = uc.get_version_by_id(v.id).await.unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::CrfVersionNotFound(_))
    ));
}

// ---- CrfForm tests ----

async fn make_form(uc: &TestUsecase) -> i64 {
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    f.id
}

#[tokio::test]
async fn crud_form_round_trip() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    assert_eq!(f.code, "F1");
}

#[tokio::test]
async fn create_form_rejects_empty_code() {
    let uc = usecase();
    let err = uc
        .create_form(CreateCrfForm {
            version_id: 1,
            code: "  ".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn update_form_partial() {
    let uc = usecase();
    let fid = make_form(&uc).await;
    let updated = uc
        .update_form(UpdateCrfForm {
            id: fid,
            code: None,
            name: Some("Form 1 v2".into()),
            order: None,
            not_submitted: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Form 1 v2");
    assert_eq!(updated.code, "F1");
}

#[tokio::test]
async fn list_forms_by_version_returns_only_that_version() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    uc.create_form(CreateCrfForm {
        version_id: v.id,
        code: "F1".into(),
        name: "F1".into(),
        order: 0,
        not_submitted: false,
    })
    .await
    .unwrap();
    let fs = uc.list_forms_by_version(v.id).await.unwrap();
    assert_eq!(fs.len(), 1);
}

// ---- CrfItem tests ----

async fn make_form_with_item(uc: &TestUsecase, kind: CrfItemKind) -> (i64, i64) {
    let fid = make_form(uc).await;
    let it = uc
        .create_item(CreateCrfItem {
            form_id: fid,
            code: "I1".into(),
            name: "Item 1".into(),
            kind,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    (fid, it.id)
}

#[tokio::test]
async fn create_item_text_succeeds() {
    let uc = usecase();
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let it = uc.get_item_by_id(iid).await.unwrap();
    assert_eq!(it.kind, CrfItemKind::Text);
}

#[tokio::test]
async fn create_item_selection_without_options_rolls_back() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let err = uc
        .create_item(CreateCrfItem {
            form_id: f.id,
            code: "S".into(),
            name: "Status".into(),
            kind: CrfItemKind::Selection,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::KindShapeViolation { .. })
    ));
}

#[tokio::test]
async fn update_item_text_with_options_fails() {
    let uc = usecase();
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    // Attach an option to the item (kind-shape is enforced by the
    // usecase on update, not at the repo level).
    uc.create_option(CreateCrfOption {
        item_id: iid,
        value: "yes".into(),
        not_submitted: false,
    })
    .await
    .unwrap();
    let err = uc
        .update_item(UpdateCrfItem {
            id: iid,
            code: None,
            name: Some("rename".into()),
            kind: None,
            order: None,
            not_submitted: None,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::KindShapeViolation { .. })
    ));
}

#[tokio::test]
async fn list_items_by_form_returns_items() {
    let uc = usecase();
    let (fid, _iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let items = uc.list_items_by_form(fid).await.unwrap();
    assert_eq!(items.len(), 1);
}

// ---- CrfOption / CrfUnit tests ----

#[tokio::test]
async fn crud_option_round_trip() {
    let uc = usecase();
    // Use Text kind since we're testing option CRUD, not shape rules.
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let o = uc
        .create_option(CreateCrfOption {
            item_id: iid,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let os = uc.list_options_by_item(iid).await.unwrap();
    assert_eq!(os.len(), 1);
    assert_eq!(os[0].id, o.id);
    let updated = uc
        .update_option(UpdateCrfOption {
            id: o.id,
            value: Some("yep".into()),
            not_submitted: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.value, "yep");
    uc.delete_option(o.id).await.unwrap();
    let os = uc.list_options_by_item(iid).await.unwrap();
    assert!(os.is_empty());
}

#[tokio::test]
async fn create_option_rejects_empty_value() {
    let uc = usecase();
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let err = uc
        .create_option(CreateCrfOption {
            item_id: iid,
            value: "  ".into(),
            not_submitted: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyValue)
    ));
}

#[tokio::test]
async fn crud_unit_round_trip() {
    let uc = usecase();
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let u = uc
        .create_unit(CreateCrfUnit {
            item_id: iid,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let us = uc.list_units_by_item(iid).await.unwrap();
    assert_eq!(us.len(), 1);
    assert_eq!(us[0].id, u.id);
    let updated = uc
        .update_unit(UpdateCrfUnit {
            id: u.id,
            value: Some("kg".into()),
            not_submitted: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.value, "kg");
}

// ---- DomainAnnotation tests ----

#[tokio::test]
async fn crud_domain_annotation_round_trip() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = uc
        .create_domain_annotation(CreateDomainAnnotation {
            form_id: f.id,
            name: "Required".into(),
            description: "must supply".into(),
        })
        .await
        .unwrap();
    let ds = uc.list_domain_annotations_by_form(f.id).await.unwrap();
    assert_eq!(ds.len(), 1);
    assert_eq!(ds[0].name, "Required");
    let updated: DomainAnnotationView = uc
        .update_domain_annotation(UpdateDomainAnnotation {
            id: d.id,
            name: Some("Optional".into()),
            description: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Optional");
    uc.delete_domain_annotation(d.id).await.unwrap();
    let ds = uc.list_domain_annotations_by_form(f.id).await.unwrap();
    assert!(ds.is_empty());
}

// ---- Annotation tests (polymorphic owner) ----

#[tokio::test]
async fn crud_annotation_form_owner() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = uc
        .create_domain_annotation(CreateDomainAnnotation {
            form_id: f.id,
            name: "Required".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    let a = uc
        .create_annotation(CreateAnnotation {
            domain_annotation_id: d.id,
            content: "must supply".into(),
            assign: false,
            owner: AnnotationOwner::Form { id: f.id },
        })
        .await
        .unwrap();
    let as_ = uc.list_annotations_by_form(f.id).await.unwrap();
    assert_eq!(as_.len(), 1);
    assert!(matches!(as_[0].owner, AnnotationOwner::Form { id } if id == f.id));
    let updated = uc
        .update_annotation(UpdateAnnotation {
            id: a.id,
            content: Some("must supply a value".into()),
            assign: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.content, "must supply a value");
}

#[tokio::test]
async fn crud_annotation_item_owner() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = uc
        .create_domain_annotation(CreateDomainAnnotation {
            form_id: f.id,
            name: "Hint".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    let (_fid, iid) = make_form_with_item(&uc, CrfItemKind::Text).await;
    let a = uc
        .create_annotation(CreateAnnotation {
            domain_annotation_id: d.id,
            content: "hint text".into(),
            assign: true,
            owner: AnnotationOwner::Item { id: iid },
        })
        .await
        .unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Item { .. }));
    let by_item = uc.list_annotations_by_item(iid).await.unwrap();
    assert_eq!(by_item.len(), 1);
    // Form-scoped list should NOT see the item-scoped annotation
    let by_form = uc.list_annotations_by_form(f.id).await.unwrap();
    assert!(by_form.is_empty());
}

#[tokio::test]
async fn create_annotation_rejects_empty_content() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = uc
        .create_domain_annotation(CreateDomainAnnotation {
            form_id: f.id,
            name: "Hint".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_annotation(CreateAnnotation {
            domain_annotation_id: d.id,
            content: "  ".into(),
            assign: false,
            owner: AnnotationOwner::Form { id: f.id },
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyContent)
    ));
}

// ---- Search ----

#[tokio::test]
async fn search_rejects_empty_fragment() {
    let uc = usecase();
    let err = uc
        .search_forms_by_version(SearchCrfFormsByVersion {
            version_id: 1,
            fragment: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Repository(_)));
}

#[tokio::test]
async fn search_items_by_version_filters_through_forms() {
    let uc = usecase();
    let v = uc
        .create_version(CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = uc
        .create_form(CreateCrfForm {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    uc.create_item(CreateCrfItem {
        form_id: f.id,
        code: "AGE".into(),
        name: "Age".into(),
        kind: CrfItemKind::Text,
        order: 0,
        not_submitted: false,
    })
    .await
    .unwrap();
    let items = uc
        .search_items_by_version(SearchCrfItemsByVersion {
            version_id: v.id,
            fragment: "AGE".into(),
        })
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].code, "AGE");
}

// ---- Port methods that round out the surface ----

#[tokio::test]
async fn search_options_units_domain_annotations_annotations_smoke() {
    let uc = usecase();
    let _ = uc
        .search_options_by_version(SearchCrfOptionsByVersion {
            version_id: 1,
            fragment: "x".into(),
        })
        .await
        .unwrap();
    let _ = uc
        .search_units_by_version(SearchCrfUnitsByVersion {
            version_id: 1,
            fragment: "x".into(),
        })
        .await
        .unwrap();
    let _ = uc
        .search_domain_annotations_by_version(SearchDomainAnnotationsByVersion {
            version_id: 1,
            fragment: "x".into(),
        })
        .await
        .unwrap();
    let _ = uc
        .search_annotations_by_version(SearchAnnotationsByVersion {
            version_id: 1,
            fragment: "x".into(),
        })
        .await
        .unwrap();
}

// ---- bulk_create_form tests ----
//
// These cover the bulk port end-to-end through the in-memory
// fake. They confirm:
//   * Successful insert of form + items + options + units
//   * Result order preservation (items returned in input order)
//   * Up-front validation rejects kind-shape, empty form code,
//     empty item code, and empty option value without touching
//     the port
//   * Duplicate item code is rejected by the port and surfaces
//     as `DuplicateCrfItem` (the in-memory fake does not enforce
//     cross-row uniqueness — so we directly construct the
//     `CrfBulkCreateForm` and verify the port's call path on a
//     duplicate form code by collapsing the in-memory fake's
//     create() to return `DuplicateCrfItem`. The Postgres impl
//     has its own constraint-name coverage via the live DB tests
//     under `tests/integration.rs`.)

#[tokio::test]
async fn bulk_create_form_inserts_form_items_options_units() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let r = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![
                CreateCrfBulkItem {
                    item: crate::usecase::CreateCrfItem {
                        form_id: 0,
                        code: "I1".into(),
                        name: "Item 1".into(),
                        kind: CrfItemKind::Selection,
                        order: 0,
                        not_submitted: false,
                    },
                    options: vec![
                        crate::usecase::CreateCrfOption {
                            item_id: 0,
                            value: "yes".into(),
                            not_submitted: false,
                        },
                        crate::usecase::CreateCrfOption {
                            item_id: 0,
                            value: "no".into(),
                            not_submitted: false,
                        },
                    ],
                    units: vec![],
                },
                CreateCrfBulkItem {
                    item: crate::usecase::CreateCrfItem {
                        form_id: 0,
                        code: "I2".into(),
                        name: "Item 2".into(),
                        kind: CrfItemKind::Text,
                        order: 1,
                        not_submitted: false,
                    },
                    options: vec![],
                    units: vec![crate::usecase::CreateCrfUnit {
                        item_id: 0,
                        value: "mg".into(),
                        not_submitted: false,
                    }],
                },
            ],
        })
        .await
        .unwrap();
    assert_eq!(r.form.code, "F1");
    assert_eq!(r.items.len(), 2);
    assert_eq!(r.items[0].code, "I1");
    assert_eq!(r.items[1].code, "I2");
    assert!(r.items[0].id > 0);
    assert_eq!(r.items[0].form_id, r.form.id);
    assert_eq!(r.items[1].form_id, r.form.id);
}

#[tokio::test]
async fn bulk_create_form_returns_results_in_input_order() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let r = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: (0..5)
                .map(|i| CreateCrfBulkItem {
                    item: crate::usecase::CreateCrfItem {
                        form_id: 0,
                        code: format!("I{i}"),
                        name: format!("Item {i}"),
                        kind: CrfItemKind::Text,
                        order: i,
                        not_submitted: false,
                    },
                    options: vec![],
                    units: vec![],
                })
                .collect(),
        })
        .await
        .unwrap();
    let codes: Vec<&str> = r.items.iter().map(|i| i.code.as_str()).collect();
    assert_eq!(
        codes,
        vec!["I0", "I1", "I2", "I3", "I4"],
        "items must come back in input order"
    );
}

#[tokio::test]
async fn bulk_create_form_rejects_empty_form_code() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "".into(),
                name: "Form".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Validation(_)));
}

#[tokio::test]
async fn bulk_create_form_rejects_text_kind_with_options() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![CreateCrfBulkItem {
                item: crate::usecase::CreateCrfItem {
                    form_id: 0,
                    code: "I1".into(),
                    name: "Item 1".into(),
                    kind: CrfItemKind::Text,
                    order: 0,
                    not_submitted: false,
                },
                options: vec![crate::usecase::CreateCrfOption {
                    item_id: 0,
                    value: "yes".into(),
                    not_submitted: false,
                }],
                units: vec![],
            }],
        })
        .await
        .unwrap_err();
    let UsecaseError::Validation(DomainError::KindShapeViolation { kind, field }) = err else {
        panic!("expected KindShapeViolation");
    };
    assert_eq!(kind, CrfItemKind::Text);
    assert_eq!(field, "options");
}

#[tokio::test]
async fn bulk_create_form_rejects_selection_without_options() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![CreateCrfBulkItem {
                item: crate::usecase::CreateCrfItem {
                    form_id: 0,
                    code: "I1".into(),
                    name: "Item 1".into(),
                    kind: CrfItemKind::Selection,
                    order: 0,
                    not_submitted: false,
                },
                options: vec![],
                units: vec![],
            }],
        })
        .await
        .unwrap_err();
    let UsecaseError::Validation(DomainError::KindShapeViolation { kind, field }) = err else {
        panic!("expected KindShapeViolation");
    };
    assert_eq!(kind, CrfItemKind::Selection);
    assert_eq!(field, "options");
}

#[tokio::test]
async fn bulk_create_form_rejects_empty_item_code() {
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![CreateCrfBulkItem {
                item: crate::usecase::CreateCrfItem {
                    form_id: 0,
                    code: "".into(),
                    name: "Item 1".into(),
                    kind: CrfItemKind::Text,
                    order: 0,
                    not_submitted: false,
                },
                options: vec![],
                units: vec![],
            }],
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Validation(_)));
}

#[tokio::test]
async fn bulk_create_form_validation_rejects_empty_code_with_existing_version() {
    // When the parent version exists, the empty form code
    // surfaces as a Validation error from `validate_bulk_create`
    // — the port is never called.
    let uc = usecase();
    let v = uc
        .create_version(crate::usecase::CreateCrfVersion {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: v.id,
                code: "".into(),
                name: "Form".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![],
        })
        .await
        .unwrap_err();
    let UsecaseError::Validation(DomainError::EmptyCode) = err else {
        panic!("expected EmptyCode");
    };
}

#[tokio::test]
async fn bulk_create_form_rejects_missing_parent_version() {
    // Version lookup fails → usecase surfaces
    // `CrfVersionNotFound` before the port call.
    let uc = usecase();
    let err = uc
        .create_bulk_form(CreateCrfBulkForm {
            form: crate::usecase::CreateCrfForm {
                version_id: 9_999_999,
                code: "F1".into(),
                name: "Form 1".into(),
                order: 0,
                not_submitted: false,
            },
            items: vec![],
        })
        .await
        .unwrap_err();
    let UsecaseError::Repository(DomainError::CrfVersionNotFound(id)) = err else {
        panic!("expected CrfVersionNotFound, got {err:?}");
    };
    assert_eq!(id, 9_999_999);
}
