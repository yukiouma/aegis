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
    AnnotationNew, AnnotationOwner, AnnotationRepository, AnnotationUpdate, CrfFormNew,
    CrfFormRepository, CrfFormUpdate, CrfItemNew, CrfItemRepository, CrfItemUpdate, CrfOptionNew,
    CrfOptionRepository, CrfOptionUpdate, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate,
    CrfVersionNew, CrfVersionRepository, CrfVersionUpdate, DomainAnnotationNew,
    DomainAnnotationRepository, DomainAnnotationUpdate, DomainError, ProjectLookup,
};

use super::commands::{
    CreateAnnotation, CreateCrfForm, CreateCrfItem, CreateCrfOption, CreateCrfUnit,
    CreateCrfVersion, CreateDomainAnnotation, SearchAnnotationsByVersion, SearchCrfFormsByVersion,
    SearchCrfItemsByVersion, SearchCrfOptionsByVersion, SearchCrfUnitsByVersion,
    SearchDomainAnnotationsByVersion, UpdateAnnotation, UpdateCrfForm, UpdateCrfItem,
    UpdateCrfOption, UpdateCrfUnit, UpdateCrfVersion, UpdateDomainAnnotation,
};
use super::error::UsecaseError;
use super::views::{
    AnnotationView, CrfFormView, CrfItemView, CrfOptionView, CrfUnitView, CrfVersionView,
    DomainAnnotationView,
};

/// Configuration for `CrfUsecase::new`. Wraps the seven
/// concrete (or fake) repositories plus the cross-crate
/// project lookup so the constructor stays readable.
pub struct CrfUsecaseConfig<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
> {
    pub version_repo: V,
    pub form_repo: F,
    pub item_repo: I,
    pub option_repo: O,
    pub unit_repo: U,
    pub domain_annotation_repo: Da,
    pub annotation_repo: A,
    pub projects: Arc<P>,
}

/// Async orchestration for the seven Case Report Form
/// aggregates plus version-scoped search. Generic over all
/// eight ports so tests inject in-memory fakes.
pub struct CrfUsecase<
    V: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: AnnotationRepository,
    P: ProjectLookup,
> {
    version_repo: V,
    form_repo: F,
    item_repo: I,
    option_repo: O,
    unit_repo: U,
    domain_annotation_repo: Da,
    annotation_repo: A,
    projects: Arc<P>,
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
> CrfUsecase<V, F, I, O, U, Da, A, P>
{
    pub fn new(cfg: CrfUsecaseConfig<V, F, I, O, U, Da, A, P>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            form_repo: cfg.form_repo,
            item_repo: cfg.item_repo,
            option_repo: cfg.option_repo,
            unit_repo: cfg.unit_repo,
            domain_annotation_repo: cfg.domain_annotation_repo,
            annotation_repo: cfg.annotation_repo,
            projects: cfg.projects,
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

    pub async fn get_version_by_id(&self, id: i32) -> Result<CrfVersionView, UsecaseError> {
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

    pub async fn delete_version(&self, id: i32) -> Result<(), UsecaseError> {
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

    pub async fn get_form_by_id(&self, id: i32) -> Result<CrfFormView, UsecaseError> {
        let f = self.form_repo.find_by_id(id).await?;
        Ok(f.into())
    }

    pub async fn list_forms_by_version(
        &self,
        version_id: i32,
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

    pub async fn delete_form(&self, id: i32) -> Result<(), UsecaseError> {
        self.form_repo.delete(id).await?;
        Ok(())
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

    pub async fn get_item_by_id(&self, id: i32) -> Result<CrfItemView, UsecaseError> {
        let i = self.item_repo.find_by_id(id).await?;
        Ok(i.into())
    }

    pub async fn list_items_by_form(&self, form_id: i32) -> Result<Vec<CrfItemView>, UsecaseError> {
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

    pub async fn delete_item(&self, id: i32) -> Result<(), UsecaseError> {
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

    pub async fn get_option_by_id(&self, id: i32) -> Result<CrfOptionView, UsecaseError> {
        let o = self.option_repo.find_by_id(id).await?;
        Ok(o.into())
    }

    pub async fn list_options_by_item(
        &self,
        item_id: i32,
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

    pub async fn delete_option(&self, id: i32) -> Result<(), UsecaseError> {
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

    pub async fn get_unit_by_id(&self, id: i32) -> Result<CrfUnitView, UsecaseError> {
        let u = self.unit_repo.find_by_id(id).await?;
        Ok(u.into())
    }

    pub async fn list_units_by_item(&self, item_id: i32) -> Result<Vec<CrfUnitView>, UsecaseError> {
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

    pub async fn delete_unit(&self, id: i32) -> Result<(), UsecaseError> {
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
        id: i32,
    ) -> Result<DomainAnnotationView, UsecaseError> {
        let d = self.domain_annotation_repo.find_by_id(id).await?;
        Ok(d.into())
    }

    pub async fn list_domain_annotations_by_form(
        &self,
        form_id: i32,
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

    pub async fn delete_domain_annotation(&self, id: i32) -> Result<(), UsecaseError> {
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

    pub async fn get_annotation_by_id(&self, id: i32) -> Result<AnnotationView, UsecaseError> {
        let a = self.annotation_repo.find_by_id(id).await?;
        Ok(a.into())
    }

    pub async fn list_annotations_by_form(
        &self,
        form_id: i32,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_form(form_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_item(
        &self,
        item_id: i32,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_item(item_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_option(
        &self,
        option_id: i32,
    ) -> Result<Vec<AnnotationView>, UsecaseError> {
        let as_ = self.annotation_repo.list_by_option(option_id).await?;
        Ok(as_.into_iter().map(Into::into).collect())
    }

    pub async fn list_annotations_by_unit(
        &self,
        unit_id: i32,
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

    pub async fn delete_annotation(&self, id: i32) -> Result<(), UsecaseError> {
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
