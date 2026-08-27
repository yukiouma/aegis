//! Facade tests. Wires the in-memory fakes from the usecase
//! layer into `CrfServiceImpl` via `from_usecase`, then drives
//! the public `apis::crf::CrfService` trait — confirming the
//! request → command, view → view, error → error projections.

use std::sync::Arc;

use apis::crf::{
    AnnotationOwner, CreateAnnotationRequest, CreateCrfFormRequest, CreateCrfItemRequest,
    CreateCrfOptionRequest, CreateCrfUnitRequest, CreateCrfVersionRequest,
    CreateDomainAnnotationRequest, CrfApiError, CrfService, GetCrfVersionByIdRequest,
    ListCrfItemsByFormRequest, ListCrfVersionsByProjectRequest, SearchCrfFormsByVersionRequest,
    UpdateCrfFormRequest,
};

use crate::usecase::tests::{
    AcceptProject, InMemoryAnnotations, InMemoryDomainAnnotations, InMemoryForms, InMemoryItems,
    InMemoryOptions, InMemoryUnits, InMemoryVersions,
};
use crate::usecase::{CrfUsecase, CrfUsecaseConfig};

use super::CrfServiceImpl;

type TestService = CrfServiceImpl<
    InMemoryVersions,
    InMemoryForms,
    InMemoryItems,
    InMemoryOptions,
    InMemoryUnits,
    InMemoryDomainAnnotations,
    InMemoryAnnotations,
    AcceptProject,
>;

fn service() -> TestService {
    let usecase: CrfUsecase<
        InMemoryVersions,
        InMemoryForms,
        InMemoryItems,
        InMemoryOptions,
        InMemoryUnits,
        InMemoryDomainAnnotations,
        InMemoryAnnotations,
        AcceptProject,
    > = CrfUsecase::new(CrfUsecaseConfig {
        version_repo: InMemoryVersions::default(),
        form_repo: InMemoryForms::default(),
        item_repo: InMemoryItems::default(),
        option_repo: InMemoryOptions::default(),
        unit_repo: InMemoryUnits::default(),
        domain_annotation_repo: InMemoryDomainAnnotations::default(),
        annotation_repo: InMemoryAnnotations::default(),
        projects: Arc::new(AcceptProject),
    });
    CrfServiceImpl::from_usecase(usecase)
}

#[tokio::test]
async fn facade_round_trips_version() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    assert_eq!(v.project_code, "P1");

    let got = svc
        .get_version_by_id(GetCrfVersionByIdRequest { id: v.id })
        .await
        .unwrap();
    assert_eq!(got.id, v.id);

    let listed = svc
        .list_versions_by_project(ListCrfVersionsByProjectRequest {
            project_code: "P1".into(),
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    svc.delete_version(v.id).await.unwrap();
}

#[tokio::test]
async fn facade_form_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    assert_eq!(f.code, "F1");
    let updated = svc
        .update_form(UpdateCrfFormRequest {
            id: f.id,
            code: None,
            name: Some("Form 1 v2".into()),
            order: None,
            not_submitted: None,
        })
        .await
        .unwrap();
    assert_eq!(updated.name, "Form 1 v2");
    svc.delete_form(f.id).await.unwrap();
}

#[tokio::test]
async fn facade_item_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let it = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let items = svc
        .list_items_by_form(ListCrfItemsByFormRequest { form_id: f.id })
        .await
        .unwrap();
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].id, it.id);
}

#[tokio::test]
async fn facade_option_unit_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "Form 1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    // Use Text kind so we can attach options without the
    // shape rule kicking in.
    let it = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "I1".into(),
            name: "Item 1".into(),
            kind: apis::crf::CrfItemKind::Text,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let _o = svc
        .create_option(CreateCrfOptionRequest {
            item_id: it.id,
            value: "yes".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
    let _u = svc
        .create_unit(CreateCrfUnitRequest {
            item_id: it.id,
            value: "mg".into(),
            not_submitted: false,
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn facade_selection_without_options_returns_kind_shape_violation() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let err = svc
        .create_item(CreateCrfItemRequest {
            form_id: f.id,
            code: "S".into(),
            name: "Status".into(),
            kind: apis::crf::CrfItemKind::Selection,
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::KindShapeViolation { .. }));
}

#[tokio::test]
async fn facade_search_rejects_empty_fragment() {
    let svc = service();
    let err = svc
        .search_forms_by_version(SearchCrfFormsByVersionRequest {
            version_id: 1,
            fragment: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, CrfApiError::EmptySearchFragment));
}

#[tokio::test]
async fn facade_domain_annotation_crud() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id: f.id,
            name: "Required".into(),
            description: "must supply".into(),
        })
        .await
        .unwrap();
    assert_eq!(d.name, "Required");
    svc.delete_domain_annotation(d.id).await.unwrap();
}

#[tokio::test]
async fn facade_annotation_polymorphic() {
    let svc = service();
    let v = svc
        .create_version(CreateCrfVersionRequest {
            project_code: "P1".into(),
            name: "v1".into(),
        })
        .await
        .unwrap();
    let f = svc
        .create_form(CreateCrfFormRequest {
            version_id: v.id,
            code: "F1".into(),
            name: "F1".into(),
            order: 0,
            not_submitted: false,
        })
        .await
        .unwrap();
    let d = svc
        .create_domain_annotation(CreateDomainAnnotationRequest {
            form_id: f.id,
            name: "Required".into(),
            description: "".into(),
        })
        .await
        .unwrap();
    let a = svc
        .create_annotation(CreateAnnotationRequest {
            domain_annotation_id: d.id,
            content: "must supply".into(),
            assign: false,
            owner: AnnotationOwner::Form(f.id),
        })
        .await
        .unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Form(_)));
    svc.delete_annotation(a.id).await.unwrap();
}
