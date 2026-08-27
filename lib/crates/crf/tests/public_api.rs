//! Compile-only safety net for the crate's public API. Names
//! every documented consumer import, pins the constructor
//! chain, and asserts the trait bounds the usecase relies on.
//! If a refactor breaks any of these, this test fails to
//! compile, catching the breakage before runtime.

use std::sync::Arc;

use crf::{
    Annotation, AnnotationOwner, AnnotationView, CrfForm, CrfFormNew, CrfFormRepository,
    CrfFormUpdate, CrfFormView, CrfItem, CrfItemKind, CrfItemNew, CrfItemRepository, CrfItemUpdate,
    CrfItemView, CrfOption, CrfOptionNew, CrfOptionRepository, CrfOptionUpdate, CrfOptionView,
    CrfUnit, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate, CrfUnitView, CrfUsecase,
    CrfUsecaseConfig, CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate,
    CrfVersionView, DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository,
    DomainAnnotationUpdate, DomainAnnotationView, DomainError, ProjectLookup, ProjectLookupImpl,
    UsecaseError,
};

use apis::crf::CrfService;

// constructor chain
fn _version_repo_new(pool: sqlx::PgPool) -> crf::CrfVersionRepoPg {
    crf::CrfVersionRepoPg::new(pool)
}

fn _form_repo_new(pool: sqlx::PgPool) -> crf::CrfFormRepoPg {
    crf::CrfFormRepoPg::new(pool)
}

fn _item_repo_new(pool: sqlx::PgPool) -> crf::CrfItemRepoPg {
    crf::CrfItemRepoPg::new(pool)
}

fn _option_repo_new(pool: sqlx::PgPool) -> crf::CrfOptionRepoPg {
    crf::CrfOptionRepoPg::new(pool)
}

fn _unit_repo_new(pool: sqlx::PgPool) -> crf::CrfUnitRepoPg {
    crf::CrfUnitRepoPg::new(pool)
}

fn _domain_annotation_repo_new(pool: sqlx::PgPool) -> crf::DomainAnnotationRepoPg {
    crf::DomainAnnotationRepoPg::new(pool)
}

fn _annotation_repo_new(pool: sqlx::PgPool) -> crf::AnnotationRepoPg {
    crf::AnnotationRepoPg::new(pool)
}

fn _usecase_new<R, F, I, O, U, Da, A, P>(
    cfg: CrfUsecaseConfig<R, F, I, O, U, Da, A, P>,
) -> CrfUsecase<R, F, I, O, U, Da, A, P>
where
    R: CrfVersionRepository,
    F: CrfFormRepository,
    I: CrfItemRepository,
    O: CrfOptionRepository,
    U: CrfUnitRepository,
    Da: DomainAnnotationRepository,
    A: crf::AnnotationRepository,
    P: ProjectLookup,
{
    CrfUsecase::new(cfg)
}

// Send + Sync safety for the trait-object path
fn _assert_send_sync<T: Send + Sync>() {}

#[test]
fn crf_service_is_send_and_sync() {
    _assert_send_sync::<Box<dyn CrfService>>();
}

#[test]
fn usecase_error_is_send_and_sync() {
    _assert_send_sync::<UsecaseError>();
}

#[test]
fn domain_error_is_send_and_sync() {
    _assert_send_sync::<DomainError>();
}

#[test]
fn view_dtos_are_send_and_sync() {
    _assert_send_sync::<CrfVersionView>();
    _assert_send_sync::<CrfFormView>();
    _assert_send_sync::<CrfItemView>();
    _assert_send_sync::<CrfOptionView>();
    _assert_send_sync::<CrfUnitView>();
    _assert_send_sync::<DomainAnnotationView>();
    _assert_send_sync::<AnnotationView>();
}

// ProjectLookupImpl constructor compiles with the expected
// argument shape.
fn _project_lookup_new(projects: Arc<dyn apis::project::ProjectService>) -> ProjectLookupImpl {
    ProjectLookupImpl::new(projects)
}

#[test]
fn annotation_owner_variants_compile() {
    let _ = AnnotationOwner::Form { id: 1 };
    let _ = AnnotationOwner::Item { id: 1 };
    let _ = AnnotationOwner::Option { id: 1 };
    let _ = AnnotationOwner::Unit { id: 1 };
    let _: Annotation = Annotation::for_form(5, "x".into(), false, 7).unwrap();
}

#[test]
fn crf_item_kind_variants_compile() {
    let _ = CrfItemKind::Text;
    let _ = CrfItemKind::Selection;
    let _ = CrfItemKind::Checkbox;
    let _ = CrfItemKind::Datetime;
    let _ = CrfItemKind::Label;
}

// aggregate validating constructors
#[test]
fn aggregate_constructors_compile() {
    let _ = CrfVersion::new("P1".into(), "v1".into()).unwrap();
    let _: CrfForm = CrfForm::new(1, "F1".into(), "Form 1".into(), 0, false).unwrap();
    let _: CrfItem =
        CrfItem::new(1, "I1".into(), "Item 1".into(), CrfItemKind::Text, 0, false).unwrap();
    let _: CrfOption = CrfOption::new(1, "yes".into(), false).unwrap();
    let _: CrfUnit = CrfUnit::new(1, "mg".into(), false).unwrap();
    let _: DomainAnnotation = DomainAnnotation::new(1, "Required".into(), "desc".into()).unwrap();
}

// new / update DTOs compile
#[test]
fn new_and_update_dtos_compile() {
    let _: CrfVersionNew = CrfVersionNew {
        project_code: "P1".into(),
        name: "v1".into(),
    };
    let _: CrfFormNew = CrfFormNew {
        version_id: 1,
        code: "F1".into(),
        name: "F1".into(),
        order: 0,
        not_submitted: false,
    };
    let _: CrfItemNew = CrfItemNew {
        form_id: 1,
        code: "I1".into(),
        name: "I1".into(),
        kind: CrfItemKind::Text,
        order: 0,
        not_submitted: false,
    };
    let _: CrfOptionNew = CrfOptionNew {
        item_id: 1,
        value: "yes".into(),
        not_submitted: false,
    };
    let _: CrfUnitNew = CrfUnitNew {
        item_id: 1,
        value: "mg".into(),
        not_submitted: false,
    };
    let _: DomainAnnotationNew = DomainAnnotationNew {
        form_id: 1,
        name: "Required".into(),
        description: "desc".into(),
    };

    let _: CrfVersionUpdate = CrfVersionUpdate::default();
    let _: CrfFormUpdate = CrfFormUpdate::default();
    let _: CrfItemUpdate = CrfItemUpdate::default();
    let _: CrfOptionUpdate = CrfOptionUpdate::default();
    let _: CrfUnitUpdate = CrfUnitUpdate::default();
    let _: DomainAnnotationUpdate = DomainAnnotationUpdate::default();
}
