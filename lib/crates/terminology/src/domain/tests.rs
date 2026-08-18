use super::{
    CodeItem, CodeList, CodeListNew, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionUpdate,
};

#[test]
fn terminology_kind_parses_lowercase_strings() {
    let sdtm = TerminologyKind::try_from("sdtm").unwrap();
    let adam = TerminologyKind::try_from("adam").unwrap();
    assert_eq!(sdtm.as_str(), "sdtm");
    assert_eq!(adam.as_str(), "adam");
}

#[test]
fn terminology_kind_rejects_unknown_string() {
    let err = TerminologyKind::try_from("OTHER").unwrap_err();
    assert!(matches!(err, DomainError::InvalidKind(ref s) if s == "OTHER"));
}

#[test]
fn terminology_version_new_rejects_empty_name() {
    let err = TerminologyVersion::new(TerminologyKind::Sdtm, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn terminology_version_new_accepts_valid_input() {
    let v = TerminologyVersion::new(TerminologyKind::Sdtm, "2026-03-27".into()).unwrap();
    assert_eq!(v.kind, TerminologyKind::Sdtm);
    assert_eq!(v.name, "2026-03-27");
}

#[test]
fn code_list_new_rejects_empty_code() {
    let err = CodeList::new(
        1,
        "".into(),
        false,
        "AGE".into(),
        "AGE".into(),
        "".into(),
        "".into(),
        "".into(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn code_list_new_accepts_valid_input() {
    let cl = CodeList::new(
        1,
        "C66741".into(),
        true,
        "AGE".into(),
        "AGE".into(),
        "Age".into(),
        "Age in years".into(),
        "Age".into(),
    )
    .unwrap();
    assert_eq!(cl.code, "C66741");
    assert!(cl.extensible);
}

#[test]
fn code_item_new_rejects_empty_code() {
    let err = CodeItem::new(1, "".into(), "X".into(), "".into(), "".into(), "".into())
        .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn code_item_new_accepts_valid_input() {
    let item = CodeItem::new(1, "C12345".into(), "> 0".into(), "".into(), "".into(), "".into())
        .unwrap();
    assert_eq!(item.code, "C12345");
}