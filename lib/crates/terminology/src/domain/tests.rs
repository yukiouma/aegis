use super::{CodeItem, CodeList, DomainError, Page, TerminologyKind, TerminologyVersion};

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
    let err =
        CodeItem::new(1, 1, "".into(), "X".into(), "".into(), "".into(), "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn code_item_new_accepts_valid_input() {
    let item = CodeItem::new(
        1,
        7,
        "C12345".into(),
        "> 0".into(),
        "".into(),
        "".into(),
        "".into(),
    )
    .unwrap();
    assert_eq!(item.code, "C12345");
    assert_eq!(item.version_id, 7);
}

#[test]
fn page_struct_accepts_items_and_optional_next_offset() {
    let p: Page<i32> = Page {
        items: vec![1, 2, 3],
        next_offset: Some(3),
    };
    assert_eq!(p.items, vec![1, 2, 3]);
    assert_eq!(p.next_offset, Some(3));

    let last: Page<i32> = Page {
        items: vec![],
        next_offset: None,
    };
    assert!(last.next_offset.is_none());
}
