//! Domain-layer unit tests. Pure — no I/O, no `sqlx`, no
//! `tokio`. Confirms the validating constructors reject empty
//! / whitespace inputs, `CrfItemKind::as_str` / `try_from_str`
//! round-trip, and `Annotation::for_*` rejects empty `content`
//! / non-positive `domain_annotation_id`.

use super::{
    Annotation, AnnotationOwner, CrfForm, CrfItem, CrfItemKind, CrfOption, CrfUnit, CrfVersion,
    DomainAnnotation, DomainError,
};

// ---- CrfItemKind ----

#[test]
fn crf_item_kind_round_trips_through_str() {
    for k in [
        CrfItemKind::Text,
        CrfItemKind::Selection,
        CrfItemKind::Checkbox,
        CrfItemKind::Datetime,
        CrfItemKind::Label,
    ] {
        assert_eq!(CrfItemKind::try_from_str(k.as_str()).unwrap(), k);
    }
}

#[test]
fn crf_item_kind_rejects_unknown_string() {
    let err = CrfItemKind::try_from_str("Bogus").unwrap_err();
    assert!(matches!(err, DomainError::InvalidCrfItemKind(s) if s == "Bogus"));
}

#[test]
fn crf_item_kind_requires_options() {
    assert!(CrfItemKind::Selection.requires_options());
    assert!(CrfItemKind::Checkbox.requires_options());
    assert!(!CrfItemKind::Text.requires_options());
    assert!(!CrfItemKind::Datetime.requires_options());
    assert!(!CrfItemKind::Label.requires_options());
}

// ---- CrfVersion ----

#[test]
fn crf_version_new_accepts_valid_inputs() {
    let v = CrfVersion::new("P1".into(), "v1".into()).unwrap();
    assert_eq!(v.project_code, "P1");
    assert_eq!(v.name, "v1");
}

#[test]
fn crf_version_new_rejects_empty_project_code() {
    let err = CrfVersion::new("  ".into(), "v1".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyProjectCode));
}

#[test]
fn crf_version_new_rejects_empty_name() {
    let err = CrfVersion::new("P1".into(), "  ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

// ---- CrfForm ----

#[test]
fn crf_form_new_accepts_valid_inputs() {
    let f = CrfForm::new(1, "F1".into(), "Form 1".into(), 0, false).unwrap();
    assert_eq!(f.code, "F1");
    assert_eq!(f.name, "Form 1");
    assert!(!f.not_submitted);
}

#[test]
fn crf_form_new_rejects_empty_code() {
    let err = CrfForm::new(1, "  ".into(), "Form 1".into(), 0, false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn crf_form_new_rejects_empty_name() {
    let err = CrfForm::new(1, "F1".into(), "  ".into(), 0, false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

// ---- CrfItem ----

#[test]
fn crf_item_new_accepts_valid_inputs() {
    let i = CrfItem::new(1, "I1".into(), "Item 1".into(), CrfItemKind::Text, 0, false).unwrap();
    assert_eq!(i.kind, CrfItemKind::Text);
}

#[test]
fn crf_item_new_rejects_empty_code() {
    let err = CrfItem::new(1, "".into(), "Item 1".into(), CrfItemKind::Text, 0, false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn crf_item_new_rejects_empty_name() {
    let err = CrfItem::new(1, "I1".into(), "".into(), CrfItemKind::Text, 0, false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

// ---- CrfOption ----

#[test]
fn crf_option_new_accepts_valid_inputs() {
    let o = CrfOption::new(1, "yes".into(), false).unwrap();
    assert_eq!(o.value, "yes");
}

#[test]
fn crf_option_new_rejects_empty_value() {
    let err = CrfOption::new(1, "  ".into(), false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyValue));
}

// ---- CrfUnit ----

#[test]
fn crf_unit_new_accepts_valid_inputs() {
    let u = CrfUnit::new(1, "mg".into(), false).unwrap();
    assert_eq!(u.value, "mg");
}

#[test]
fn crf_unit_new_rejects_empty_value() {
    let err = CrfUnit::new(1, "".into(), false).unwrap_err();
    assert!(matches!(err, DomainError::EmptyValue));
}

// ---- DomainAnnotation ----

#[test]
fn domain_annotation_new_accepts_valid_inputs() {
    let d = DomainAnnotation::new(1, "Required".into(), "must supply".into()).unwrap();
    assert_eq!(d.name, "Required");
    assert_eq!(d.description, "must supply");
}

#[test]
fn domain_annotation_new_accepts_empty_description() {
    let d = DomainAnnotation::new(1, "Required".into(), "".into()).unwrap();
    assert_eq!(d.description, "");
}

#[test]
fn domain_annotation_new_rejects_empty_name() {
    let err = DomainAnnotation::new(1, "  ".into(), "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

// ---- Annotation ----

#[test]
fn annotation_for_form_accepts_valid_inputs() {
    let a = Annotation::for_form(1, "must supply".into(), false, 7).unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Form { id: 7 }));
    assert_eq!(a.content, "must supply");
    assert_eq!(a.domain_annotation_id, 1);
}

#[test]
fn annotation_for_item_accepts_valid_inputs() {
    let a = Annotation::for_item(1, "x".into(), true, 9).unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Item { id: 9 }));
    assert!(a.assign);
}

#[test]
fn annotation_for_option_accepts_valid_inputs() {
    let a = Annotation::for_option(1, "x".into(), false, 11).unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Option { id: 11 }));
}

#[test]
fn annotation_for_unit_accepts_valid_inputs() {
    let a = Annotation::for_unit(1, "x".into(), false, 13).unwrap();
    assert!(matches!(a.owner, AnnotationOwner::Unit { id: 13 }));
}

#[test]
fn annotation_rejects_empty_content() {
    let err = Annotation::for_form(1, "  ".into(), false, 7).unwrap_err();
    assert!(matches!(err, DomainError::EmptyContent));
}

#[test]
fn annotation_rejects_non_positive_domain_annotation_id() {
    let err = Annotation::for_form(0, "x".into(), false, 7).unwrap_err();
    assert!(matches!(err, DomainError::FkDomainAnnotationNotFound(0)));
}

// ---- AnnotationOwner ----

#[test]
fn annotation_owner_id_and_fk_column() {
    assert_eq!(AnnotationOwner::Form { id: 1 }.id(), 1);
    assert_eq!(AnnotationOwner::Form { id: 1 }.fk_column(), "form_id");
    assert_eq!(AnnotationOwner::Item { id: 2 }.fk_column(), "item_id");
    assert_eq!(AnnotationOwner::Option { id: 3 }.fk_column(), "option_id");
    assert_eq!(AnnotationOwner::Unit { id: 4 }.fk_column(), "unit_id");
}
