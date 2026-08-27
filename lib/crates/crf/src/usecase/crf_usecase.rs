//! `CrfUsecase` — async orchestrator over the seven
//! repository ports and the cross-crate project lookup.
//! Generic over all eight so tests inject in-memory fakes.
//!
//! Layered concerns:
//!
//! 1. Validate the command against the domain invariants
//!    (`name.trim().is_empty()` etc.) — failures land in
//!    `UsecaseError::Validation`.
//! 2. Cross-crate validation (`create_version` only):
//!    `ProjectLookup::get_by_code` — failures land in
//!    `UsecaseError::Repository`.
//! 3. Kind-shape validation for `Selection` / `Checkbox` /
//!    `Text` / `Datetime` / `Label` items — post-insert
//!    option count check on create, pre-update option count
//!    check on update.
//! 4. Repository call — failures surface as
//!    `UsecaseError::Repository`.
//! 5. Domain → view projection through the `From` impls in
//!    `super::views`.

use std::sync::Arc;

use crate::domain::{
    Annotation, AnnotationNew, AnnotationOwner, AnnotationRepository, AnnotationUpdate,
    CrfBulkFormRepository, CrfFormNew, CrfFormRepository, CrfFormUpdate, CrfItem, CrfItemNew,
    CrfItemRepository, CrfItemUpdate, CrfOption, CrfOptionNew, CrfOptionRepository,
    CrfOptionUpdate, CrfUnit, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate, CrfVersionNew,
    CrfVersionRepository, CrfVersionUpdate, DomainAnnotation, DomainAnnotationNew,
    DomainAnnotationRepository, DomainAnnotationUpdate, DomainError, ProjectLookup,
    validate_bulk_create,
};

use super::commands::{
    CreateAnnotation, CreateCrfBulkForm, CreateCrfForm, CreateCrfItem, CreateCrfOption,
    CreateCrfUnit, CreateCrfVersion, CreateDomainAnnotation, SearchAnnotationsByVersion,
    SearchCrfFormsByVersion, SearchCrfItemsByVersion, SearchCrfOptionsByVersion,
    SearchCrfUnitsByVersion, SearchDomainAnnotationsByVersion, UpdateAnnotation, UpdateCrfForm,
    UpdateCrfItem, UpdateCrfOption, UpdateCrfUnit, UpdateCrfVersion, UpdateDomainAnnotation,
};
use super::error::UsecaseError;
use super::views::{
    AnnotationView, CrfBulkFormResult, CrfFormDetailView, CrfFormView, CrfItemDetailView,
    CrfItemView, CrfOptionDetailView, CrfOptionView, CrfUnitDetailView, CrfUnitView,
    CrfVersionView, DomainAnnotationView,
};

/// Configuration for `CrfUsecase::new`. Wraps the seven
/// concrete (or fake) repositories plus the cross-crate
/// project lookup plus the bulk-form port so the constructor
/// stays readable.
pub struct CrfUsecaseConfig<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
    B: CrfBulkFormRepository,
> {
    pub version_repo: V,
    pub form_repo: F,
    pub item_repo: I,
    pub option_repo: O,
    pub unit_repo: U,
    pub domain_annotation_repo: Da,
    pub annotation_repo: A,
    pub projects: Arc<P>,
    pub bulk_form_repo: Arc<B>,
}

/// Async orchestration for the seven Case Report Form
/// aggregates plus version-scoped search plus the bulk-form
/// transaction. Generic over all nine ports so tests inject
/// in-memory fakes.
pub struct CrfUsecase<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
    B: CrfBulkFormRepository,
> {
    version_repo: V,
    form_repo: F,
    item_repo: I,
    option_repo: O,
    unit_repo: U,
    domain_annotation_repo: Da,
    annotation_repo: A,
    projects: Arc<P>,
    bulk_form_repo: Arc<B>,
}

impl<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
    B: CrfBulkFormRepository,
> CrfUsecase<V, F, I, O, U, Da, A, P, B>
{
    pub fn new(cfg: CrfUsecaseConfig<V, F, I, O, U, Da, A, P, B>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            form_repo: cfg.form_repo,
            item_repo: cfg.item_repo,
            option_repo: cfg.option_repo,
            unit_repo: cfg.unit_repo,
            domain_annotation_repo: cfg.domain_annotation_repo,
            annotation_repo: cfg.annotation_repo,
            projects: cfg.projects,
            bulk_form_repo: cfg.bulk_form_repo,
        }
    }

    // ---- CrfVersion ----

    /// Create a new `(project_code, name)` version.
    ///
    /// Validates the project exists via the cross-crate
    /// `ProjectLookup` port. `update_version` does NOT validate
    /// — it does not touch `project_code`.
    pub async fn create_version(
        &self,
        cmd: CreateCrfVersion,
    ) -> Result<CrfVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        self.projects
            .get_by_code(&cmd.project_code)
            .await
            .map_err(UsecaseError::Repository)?;
        let v = self
            .version_repo
            .create(CrfVersionNew {
                project_code: cmd.project_code,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn get_version_by_id(&self, id: i64) -> Result<CrfVersionView, UsecaseError> {
        let v = self.version_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_versions_by_project(
        &self,
        project_code: &str,
    ) -> Result<Vec<CrfVersionView>, UsecaseError> {
        let vs = self.version_repo.list_by_project(project_code).await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateCrfVersion,
    ) -> Result<CrfVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(CrfVersionUpdate {
                id: cmd.id,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- CrfForm ----

    pub async fn create_form(&self, cmd: CreateCrfForm) -> Result<CrfFormView, UsecaseError> {
        validate_create_form(&cmd)?;
        // Confirm parent version exists so we surface a
        // meaningful FK error rather than letting the DB CHECK
        // bubble.
        let _ = self.version_repo.find_by_id(cmd.version_id).await?;
        let f = self
            .form_repo
            .create(CrfFormNew {
                version_id: cmd.version_id,
                code: cmd.code,
                name: cmd.name,
                order: cmd.order,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(f.into())
    }

    pub async fn get_form_by_id(&self, id: i64) -> Result<CrfFormView, UsecaseError> {
        let f = self.form_repo.find_by_id(id).await?;
        Ok(f.into())
    }

    pub async fn list_forms_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<CrfFormView>, UsecaseError> {
        let fs = self.form_repo.list_by_version(version_id).await?;
        Ok(fs.into_iter().map(Into::into).collect())
    }

    pub async fn update_form(&self, cmd: UpdateCrfForm) -> Result<CrfFormView, UsecaseError> {
        validate_update_form(&cmd)?;
        let f = self
            .form_repo
            .update(CrfFormUpdate {
                id: cmd.id,
                code: cmd.code,
                name: cmd.name,
                order: cmd.order,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(f.into())
    }

    pub async fn delete_form(&self, id: i64) -> Result<(), UsecaseError> {
        self.form_repo.delete(id).await?;
        Ok(())
    }

    /// Return every piece of state owned by this form (items
    /// composed with their options / units / annotations,
    /// domain annotations, and form-level annotations) in a
    /// single response. Returns
    /// `UsecaseError::Repository(CrfFormNotFound)` if the form
    /// does not exist.
    ///
    /// Wave structure: 4 concurrent reads in wave 1, 3 in
    /// wave 2, 1 each in waves 3 and 4 (9 queries, max 4 in
    /// flight). Waves 2-4 are skipped entirely when their
    /// inputs are empty.
    pub async fn get_form_detail(
        &self,
        form_id: i64,
    ) -> Result<CrfFormDetailView, UsecaseError> {
        use std::collections::HashMap;

        // Wave 1: form + items + domain_annotations + form-level annotations.
        let (form, items, domain_annotations, form_annotations) = tokio::try_join!(
            self.form_repo.find_by_id(form_id),
            self.item_repo.list_by_form(form_id),
            self.domain_annotation_repo.list_by_form(form_id),
            self.annotation_repo.list_by_form(form_id),
        )?;

        if items.is_empty() {
            let mut sorted_da = domain_annotations;
            sorted_da.sort_by_key(|d| d.id);
            return Ok(CrfFormDetailView {
                form: form.into(),
                form_annotations: form_annotations.into_iter().map(Into::into).collect(),
                items: Vec::new(),
                domain_annotations: sorted_da.into_iter().map(Into::into).collect(),
            });
        }

        let item_ids: Vec<i64> = items.iter().map(|i| i.id).collect();

        // Wave 2: options + units + item-level annotations.
        let (options, units, item_annotations) = tokio::try_join!(
            self.option_repo.list_by_items(&item_ids),
            self.unit_repo.list_by_items(&item_ids),
            self.annotation_repo.list_by_items(&item_ids),
        )?;

        // Build maps for O(1) parent lookups during assembly.
        let mut options_by_item: HashMap<i64, Vec<CrfOption>> = HashMap::new();
        for o in options {
            options_by_item.entry(o.item_id).or_default().push(o);
        }
        let mut units_by_item: HashMap<i64, Vec<CrfUnit>> = HashMap::new();
        for u in units {
            units_by_item.entry(u.item_id).or_default().push(u);
        }
        let mut item_anns_by_item: HashMap<i64, Vec<Annotation>> = HashMap::new();
        for a in item_annotations {
            if let AnnotationOwner::Item { id } = a.owner {
                item_anns_by_item.entry(id).or_default().push(a);
            }
        }

        // Collect option / unit ids across all items for waves 3 & 4.
        let option_ids: Vec<i64> = options_by_item
            .values()
            .flat_map(|v| v.iter().map(|o| o.id))
            .collect();
        let unit_ids: Vec<i64> = units_by_item
            .values()
            .flat_map(|v| v.iter().map(|u| u.id))
            .collect();

        // Wave 3: option-level annotations.
        let option_anns = if option_ids.is_empty() {
            Vec::new()
        } else {
            self.annotation_repo.list_by_options(&option_ids).await?
        };
        let mut option_anns_by_option: HashMap<i64, Vec<Annotation>> = HashMap::new();
        for a in option_anns {
            if let AnnotationOwner::Option { id } = a.owner {
                option_anns_by_option.entry(id).or_default().push(a);
            }
        }

        // Wave 4: unit-level annotations.
        let unit_anns = if unit_ids.is_empty() {
            Vec::new()
        } else {
            self.annotation_repo.list_by_units(&unit_ids).await?
        };
        let mut unit_anns_by_unit: HashMap<i64, Vec<Annotation>> = HashMap::new();
        for a in unit_anns {
            if let AnnotationOwner::Unit { id } = a.owner {
                unit_anns_by_unit.entry(id).or_default().push(a);
            }
        }

        // Items come back ordered; sort defensively.
        let mut sorted_items: Vec<CrfItem> = items;
        sorted_items.sort_by(|a, b| a.order.cmp(&b.order).then(a.id.cmp(&b.id)));

        let item_views: Vec<CrfItemDetailView> = sorted_items
            .into_iter()
            .map(|item| {
                let mut opts = options_by_item.remove(&item.id).unwrap_or_default();
                opts.sort_by_key(|o| o.id);
                let mut uns = units_by_item.remove(&item.id).unwrap_or_default();
                uns.sort_by_key(|u| u.id);
                let mut item_anns = item_anns_by_item.remove(&item.id).unwrap_or_default();
                item_anns.sort_by_key(|a| a.id);

                let option_views = opts
                    .into_iter()
                    .map(|o| {
                        let mut anns = option_anns_by_option.remove(&o.id).unwrap_or_default();
                        anns.sort_by_key(|a| a.id);
                        CrfOptionDetailView {
                            option: o.into(),
                            annotations: anns.into_iter().map(Into::into).collect(),
                        }
                    })
                    .collect();
                let unit_views = uns
                    .into_iter()
                    .map(|u| {
                        let mut anns = unit_anns_by_unit.remove(&u.id).unwrap_or_default();
                        anns.sort_by_key(|a| a.id);
                        CrfUnitDetailView {
                            unit: u.into(),
                            annotations: anns.into_iter().map(Into::into).collect(),
                        }
                    })
                    .collect();

                CrfItemDetailView {
                    item: item.into(),
                    options: option_views,
                    units: unit_views,
                    annotations: item_anns.into_iter().map(Into::into).collect(),
                }
            })
            .collect();

        let mut sorted_domain_annotations: Vec<DomainAnnotation> = domain_annotations;
        sorted_domain_annotations.sort_by_key(|d| d.id);

        Ok(CrfFormDetailView {
            form: CrfFormView::from(form),
            form_annotations: form_annotations.into_iter().map(Into::into).collect(),
            items: item_views,
            domain_annotations: sorted_domain_annotations
                .into_iter()
                .map(Into::into)
                .collect(),
        })
    }

    /// Atomically create a form, every item, and each item's
    /// options + units. Validates up-front so a kind-shape or
    /// empty-field violation never leaves partial state. The
    /// adapter owns the transaction.
    pub async fn create_bulk_form(
        &self,
        cmd: CreateCrfBulkForm,
    ) -> Result<CrfBulkFormResult, UsecaseError> {
        // 1. Confirm parent version exists so a stale `version_id`
        //    surfaces as a meaningful error rather than an FK
        //    violation.
        let _ = self.version_repo.find_by_id(cmd.form.version_id).await?;

        // 2. Up-front validation — empty fields + kind-shape.
        let domain_input: crate::domain::CrfBulkCreateForm = cmd.into();
        validate_bulk_create(&domain_input).map_err(UsecaseError::Validation)?;

        // 3. Single atomic insert through the bulk port.
        let result = self.bulk_form_repo.bulk_create(domain_input).await?;

        // 4. Project to view DTOs.
        Ok(CrfBulkFormResult {
            form: result.form.into(),
            items: result.items.into_iter().map(Into::into).collect(),
        })
    }

    // ---- CrfItem ----

    /// Create a new item. Enforces kind-shape: if the kind is
    /// `Selection` or `Checkbox`, the caller must attach
    /// options via the `create_option` path *after* create;
    /// we count them here and roll back on failure.
    pub async fn create_item(&self, cmd: CreateCrfItem) -> Result<CrfItemView, UsecaseError> {
        validate_create_item(&cmd)?;
        // Confirm parent form exists.
        let _ = self.form_repo.find_by_id(cmd.form_id).await?;
        let i = self
            .item_repo
            .create(CrfItemNew {
                form_id: cmd.form_id,
                code: cmd.code,
                name: cmd.name,
                kind: cmd.kind,
                order: cmd.order,
                not_submitted: cmd.not_submitted,
            })
            .await?;

        // Kind-shape post-insert check: Selection/Checkbox
        // require at least one option. The create path does
        // not batch-attach options, so a Selection/Checkbox
        // created without immediate option insertion is
        // rolled back here.
        if cmd.kind.requires_options() {
            let n = self.option_repo.count_by_item(i.id).await?;
            if n == 0 {
                // Roll back: delete the just-inserted item
                // before reporting the shape violation.
                let _ = self.item_repo.delete(i.id).await;
                return Err(UsecaseError::Validation(DomainError::KindShapeViolation {
                    kind: cmd.kind,
                    field: "options".to_string(),
                }));
            }
        }
        Ok(i.into())
    }

    pub async fn get_item_by_id(&self, id: i64) -> Result<CrfItemView, UsecaseError> {
        let i = self.item_repo.find_by_id(id).await?;
        Ok(i.into())
    }

    pub async fn list_items_by_form(&self, form_id: i64) -> Result<Vec<CrfItemView>, UsecaseError> {
        let its = self.item_repo.list_by_form(form_id).await?;
        Ok(its.into_iter().map(Into::into).collect())
    }

    pub async fn update_item(&self, cmd: UpdateCrfItem) -> Result<CrfItemView, UsecaseError> {
        validate_update_item(&cmd)?;
        // Re-read so we can decide kind-shape based on the
        // resulting kind (current or new).
        let current = self.item_repo.find_by_id(cmd.id).await?;
        let resulting_kind = cmd.kind.unwrap_or(current.kind);
        // Kind-shape pre-update check: Text/Datetime/Label
        // reject the presence of options on the item.
        if !resulting_kind.requires_options() {
            let n = self.option_repo.count_by_item(cmd.id).await?;
            if n > 0 {
                return Err(UsecaseError::Validation(DomainError::KindShapeViolation {
                    kind: resulting_kind,
                    field: "options".to_string(),
                }));
            }
        }
        let updated = self
            .item_repo
            .update(CrfItemUpdate {
                id: cmd.id,
                code: cmd.code,
                name: cmd.name,
                order: cmd.order,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(updated.into())
    }

    pub async fn delete_item(&self, id: i64) -> Result<(), UsecaseError> {
        self.item_repo.delete(id).await?;
        Ok(())
    }

    // ---- CrfOption ----

    pub async fn create_option(&self, cmd: CreateCrfOption) -> Result<CrfOptionView, UsecaseError> {
        validate_create_option(&cmd)?;
        // Confirm parent item exists.
        let _ = self.item_repo.find_by_id(cmd.item_id).await?;
        let o = self
            .option_repo
            .create(CrfOptionNew {
                item_id: cmd.item_id,
                value: cmd.value,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(o.into())
    }

    pub async fn get_option_by_id(&self, id: i64) -> Result<CrfOptionView, UsecaseError> {
        let o = self.option_repo.find_by_id(id).await?;
        Ok(o.into())
    }

    pub async fn list_options_by_item(
        &self,
        item_id: i64,
    ) -> Result<Vec<CrfOptionView>, UsecaseError> {
        let os = self.option_repo.list_by_item(item_id).await?;
        Ok(os.into_iter().map(Into::into).collect())
    }

    pub async fn update_option(&self, cmd: UpdateCrfOption) -> Result<CrfOptionView, UsecaseError> {
        validate_update_option(&cmd)?;
        let o = self
            .option_repo
            .update(CrfOptionUpdate {
                id: cmd.id,
                value: cmd.value,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(o.into())
    }

    pub async fn delete_option(&self, id: i64) -> Result<(), UsecaseError> {
        self.option_repo.delete(id).await?;
        Ok(())
    }

    // ---- CrfUnit ----

    pub async fn create_unit(&self, cmd: CreateCrfUnit) -> Result<CrfUnitView, UsecaseError> {
        validate_create_unit(&cmd)?;
        // Confirm parent item exists.
        let _ = self.item_repo.find_by_id(cmd.item_id).await?;
        let u = self
            .unit_repo
            .create(CrfUnitNew {
                item_id: cmd.item_id,
                value: cmd.value,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(u.into())
    }

    pub async fn get_unit_by_id(&self, id: i64) -> Result<CrfUnitView, UsecaseError> {
        let u = self.unit_repo.find_by_id(id).await?;
        Ok(u.into())
    }

    pub async fn list_units_by_item(&self, item_id: i64) -> Result<Vec<CrfUnitView>, UsecaseError> {
        let us = self.unit_repo.list_by_item(item_id).await?;
        Ok(us.into_iter().map(Into::into).collect())
    }

    pub async fn update_unit(&self, cmd: UpdateCrfUnit) -> Result<CrfUnitView, UsecaseError> {
        validate_update_unit(&cmd)?;
        let u = self
            .unit_repo
            .update(CrfUnitUpdate {
                id: cmd.id,
                value: cmd.value,
                not_submitted: cmd.not_submitted,
            })
            .await?;
        Ok(u.into())
    }

    pub async fn delete_unit(&self, id: i64) -> Result<(), UsecaseError> {
        self.unit_repo.delete(id).await?;
        Ok(())
    }

    // ---- DomainAnnotation ----

    pub async fn create_domain_annotation(
        &self,
        cmd: CreateDomainAnnotation,
    ) -> Result<DomainAnnotationView, UsecaseError> {
        validate_create_domain_annotation(&cmd)?;
        // Confirm parent form exists.
        let _ = self.form_repo.find_by_id(cmd.form_id).await?;
        let d = self
            .domain_annotation_repo
            .create(DomainAnnotationNew {
                form_id: cmd.form_id,
                name: cmd.name,
                description: cmd.description,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn get_domain_annotation_by_id(
        &self,
        id: i64,
    ) -> Result<DomainAnnotationView, UsecaseError> {
        let d = self.domain_annotation_repo.find_by_id(id).await?;
        Ok(d.into())
    }

    pub async fn list_domain_annotations_by_form(
        &self,
        form_id: i64,
    ) -> Result<Vec<DomainAnnotationView>, UsecaseError> {
        let ds = self.domain_annotation_repo.list_by_form(form_id).await?;
        Ok(ds.into_iter().map(Into::into).collect())
    }

    pub async fn update_domain_annotation(
        &self,
        cmd: UpdateDomainAnnotation,
    ) -> Result<DomainAnnotationView, UsecaseError> {
        validate_update_domain_annotation(&cmd)?;
        let d = self
            .domain_annotation_repo
            .update(DomainAnnotationUpdate {
                id: cmd.id,
                name: cmd.name,
                description: cmd.description,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn delete_domain_annotation(&self, id: i64) -> Result<(), UsecaseError> {
        self.domain_annotation_repo.delete(id).await?;
        Ok(())
    }

    // ---- Annotation ----

    pub async fn create_annotation(
        &self,
        cmd: CreateAnnotation,
    ) -> Result<AnnotationView, UsecaseError> {
        validate_create_annotation(&cmd)?;
        // Confirm the parent owner row exists.
        match cmd.owner {
            AnnotationOwner::Form { id } => {
                let _ = self.form_repo.find_by_id(id).await?;
            }
            AnnotationOwner::Item { id } => {
                let _ = self.item_repo.find_by_id(id).await?;
            }
            AnnotationOwner::Option { id } => {
                let _ = self.option_repo.find_by_id(id).await?;
            }
            AnnotationOwner::Unit { id } => {
                let _ = self.unit_repo.find_by_id(id).await?;
            }
        }
        let a = self
            .annotation_repo
            .create(AnnotationNew {
                domain_annotation_id: cmd.domain_annotation_id,
                content: cmd.content,
                assign: cmd.assign,
                owner: cmd.owner,
            })
            .await?;
        Ok(a.into())
    }

    pub async fn get_annotation_by_id(&self, id: i64) -> Result<AnnotationView, UsecaseError> {
        let a = self.annotation_repo.find_by_id(id).await?;
        Ok(a.into())
    }

    pub async fn list_annotations_by_form(
        &self,
        form_id: i64,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_form(form_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_item(
        &self,
        item_id: i64,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_item(item_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_option(
        &self,
        option_id: i64,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_option(option_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_unit(
        &self,
        unit_id: i64,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_unit(unit_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn update_annotation(
        &self,
        cmd: UpdateAnnotation,
    ) -> Result<AnnotationView, UsecaseError> {
        validate_update_annotation(&cmd)?;
        let a = self
            .annotation_repo
            .update(AnnotationUpdate {
                id: cmd.id,
                content: cmd.content,
                assign: cmd.assign,
            })
            .await?;
        Ok(a.into())
    }

    pub async fn delete_annotation(&self, id: i64) -> Result<(), UsecaseError> {
        self.annotation_repo.delete(id).await?;
        Ok(())
    }

    // ---- Search ----

    fn require_non_empty_fragment(fragment: &str) -> Result<(), UsecaseError> {
        if fragment.trim().is_empty() {
            return Err(UsecaseError::Repository(DomainError::Repository(
                "search fragment cannot be empty".into(),
            )));
        }
        Ok(())
    }

    pub async fn search_forms_by_version(
        &self,
        query: SearchCrfFormsByVersion,
    ) -> Result<Vec<CrfFormView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let fs = self
            .form_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(fs.into_iter().map(Into::into).collect())
    }

    pub async fn search_items_by_version(
        &self,
        query: SearchCrfItemsByVersion,
    ) -> Result<Vec<CrfItemView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let its = self
            .item_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(its.into_iter().map(Into::into).collect())
    }

    pub async fn search_options_by_version(
        &self,
        query: SearchCrfOptionsByVersion,
    ) -> Result<Vec<CrfOptionView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let os = self
            .option_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(os.into_iter().map(Into::into).collect())
    }

    pub async fn search_units_by_version(
        &self,
        query: SearchCrfUnitsByVersion,
    ) -> Result<Vec<CrfUnitView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let us = self
            .unit_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(us.into_iter().map(Into::into).collect())
    }

    pub async fn search_domain_annotations_by_version(
        &self,
        query: SearchDomainAnnotationsByVersion,
    ) -> Result<Vec<DomainAnnotationView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let ds = self
            .domain_annotation_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(ds.into_iter().map(Into::into).collect())
    }

    pub async fn search_annotations_by_version(
        &self,
        query: SearchAnnotationsByVersion,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        Self::require_non_empty_fragment(&query.fragment)?;
        let as_ = self
            .annotation_repo
            .search_by_version(query.version_id, &query.fragment)
            .await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateCrfVersion) -> Result<(), UsecaseError> {
    if cmd.project_code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyProjectCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateCrfVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_form(cmd: &CreateCrfForm) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_form(cmd: &UpdateCrfForm) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_item(cmd: &CreateCrfItem) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_item(cmd: &UpdateCrfItem) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_option(cmd: &CreateCrfOption) -> Result<(), UsecaseError> {
    if cmd.value.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyValue));
    }
    Ok(())
}

fn validate_update_option(cmd: &UpdateCrfOption) -> Result<(), UsecaseError> {
    if let Some(ref value) = cmd.value
        && value.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyValue));
    }
    Ok(())
}

fn validate_create_unit(cmd: &CreateCrfUnit) -> Result<(), UsecaseError> {
    if cmd.value.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyValue));
    }
    Ok(())
}

fn validate_update_unit(cmd: &UpdateCrfUnit) -> Result<(), UsecaseError> {
    if let Some(ref value) = cmd.value
        && value.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyValue));
    }
    Ok(())
}

fn validate_create_domain_annotation(cmd: &CreateDomainAnnotation) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_domain_annotation(cmd: &UpdateDomainAnnotation) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_annotation(cmd: &CreateAnnotation) -> Result<(), UsecaseError> {
    if cmd.content.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyContent));
    }
    Ok(())
}

fn validate_update_annotation(cmd: &UpdateAnnotation) -> Result<(), UsecaseError> {
    if let Some(ref content) = cmd.content
        && content.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyContent));
    }
    Ok(())
}
