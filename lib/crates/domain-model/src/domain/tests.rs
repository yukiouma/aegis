use super::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmRole, SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableType, SdtmVersion,
};

// ---- enums ---------------------------------------------------------------

#[test]
fn domain_category_parses_known_strings() {
    let cases = [
        ("Special Purpose", DomainCategory::SpecialPurpose),
        ("Interventions", DomainCategory::Interventions),
        ("Events", DomainCategory::Events),
        ("Findings", DomainCategory::Findings),
        ("Trial Design", DomainCategory::TrialDesign),
        ("Relationships", DomainCategory::Relationships),
        ("Study Reference", DomainCategory::StudyReference),
    ];
    for (raw, expected) in cases {
        let parsed = DomainCategory::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn domain_category_rejects_unknown_string() {
    let err = DomainCategory::try_from("Bogus").unwrap_err();
    assert!(matches!(err, DomainError::InvalidDomainCategory(ref s) if s == "Bogus"));
}

#[test]
fn variable_type_parses_known_strings() {
    let n = SdtmVariableType::try_from("Numeric").unwrap();
    assert_eq!(n, SdtmVariableType::Numeric);
    assert_eq!(n.as_str(), "Numeric");
    let c = SdtmVariableType::try_from("Character").unwrap();
    assert_eq!(c, SdtmVariableType::Character);
}

#[test]
fn variable_type_rejects_unknown_string() {
    let err = SdtmVariableType::try_from("Date").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableType(ref s) if s == "Date"));
}

#[test]
fn variable_core_parses_known_strings() {
    let cases = [
        ("Req", SdtmVariableCore::Req),
        ("Exp", SdtmVariableCore::Exp),
        ("Perm", SdtmVariableCore::Perm),
        ("Supp", SdtmVariableCore::Supp),
    ];
    for (raw, expected) in cases {
        let parsed = SdtmVariableCore::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn variable_core_rejects_unknown_string() {
    let err = SdtmVariableCore::try_from("Bad").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableCore(ref s) if s == "Bad"));
}

#[test]
fn role_parses_known_strings() {
    let cases = [
        ("Identifier", SdtmRole::Identifier),
        ("Topic", SdtmRole::Topic),
        ("Timing", SdtmRole::Timing),
        ("Record Qualifier", SdtmRole::RecordQualifier),
        ("Synonym Qualifier", SdtmRole::SynonymQualifier),
        ("Variable Qualifier", SdtmRole::VariableQualifier),
        ("Grouping Qualifier", SdtmRole::GroupingQualifier),
        ("Rule", SdtmRole::Rule),
    ];
    for (raw, expected) in cases {
        let parsed = SdtmRole::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn role_rejects_unknown_string() {
    let err = SdtmRole::try_from("Bad").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableRole(ref s) if s == "Bad"));
}

// ---- aggregates ----------------------------------------------------------

#[test]
fn sdtm_version_new_rejects_empty_name() {
    let err = SdtmVersion::new("   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_version_new_accepts_valid_input() {
    let v = SdtmVersion::new("2024-09-27".into()).unwrap();
    assert_eq!(v.name, "2024-09-27");
}

#[test]
fn sdtm_domain_new_rejects_empty_name() {
    let err = SdtmDomain::new(1, "".into(), DomainCategory::Events, Vec::new()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_domain_new_accepts_valid_input() {
    let desc = SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    };
    let d = SdtmDomain::new(1, "AE".into(), DomainCategory::Events, vec![desc]).unwrap();
    assert_eq!(d.name, "AE");
    assert_eq!(d.descriptions.len(), 1);
    assert_eq!(d.descriptions[0].details.description, "Adverse events");
}

#[test]
fn sdtm_variable_new_rejects_empty_name() {
    let err = SdtmVariable::new(
        1,
        "".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        1,
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_variable_new_accepts_valid_input() {
    let desc = SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    };
    let v = SdtmVariable::new(
        1,
        "AETERM".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        11,
        vec![desc],
    )
    .unwrap();
    assert_eq!(v.name, "AETERM");
    assert_eq!(v.variable_sequence, 11);
}
