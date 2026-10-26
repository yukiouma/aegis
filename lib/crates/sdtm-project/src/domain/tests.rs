use super::{
    DomainCategory, DomainError, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmProjectDomain, SdtmProjectDomainOverride, SdtmProjectVariable, SdtmProjectVariableOverride,
    SdtmProjectVersion, SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableType,
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
    assert_eq!(c.as_str(), "Character");
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
fn sdtm_project_version_new_rejects_blank_project_code() {
    let err = SdtmProjectVersion::new("   ".into(), "v1".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyProjectCode));
}

#[test]
fn sdtm_project_version_new_rejects_blank_version_name() {
    let err = SdtmProjectVersion::new("P".into(), "  ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyVersionName));
}

#[test]
fn sdtm_project_version_new_accepts_valid_input() {
    let v = SdtmProjectVersion::new("P1".into(), "2024-09-27".into()).unwrap();
    assert_eq!(v.project_code, "P1");
    assert_eq!(v.version_name, "2024-09-27");
}

#[test]
fn sdtm_project_domain_new_rejects_blank_domain_code() {
    let err = SdtmProjectDomain::new(1, "  ".into(), Vec::new()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyDomainCode));
}

#[test]
fn sdtm_project_domain_new_rejects_blank_key_variable() {
    let err = SdtmProjectDomain::new(1, "AE".into(), vec!["  ".into()]).unwrap_err();
    assert!(matches!(err, DomainError::EmptyKeyVariable));
}

#[test]
fn sdtm_project_domain_new_accepts_valid_input() {
    let d = SdtmProjectDomain::new(1, "AE".into(), vec!["AETERM".into()]).unwrap();
    assert_eq!(d.domain_code, "AE");
    assert_eq!(d.key_variables, vec!["AETERM".to_string()]);
}

#[test]
fn normalize_key_variables_preserves_caller_order() {
    let raw = vec!["AESEV".to_string(), "AETERM".to_string()];
    let out = SdtmProjectDomain::normalize_key_variables(&raw).unwrap();
    assert_eq!(out, vec!["AESEV".to_string(), "AETERM".to_string()]);
}

#[test]
fn normalize_key_variables_dedups_preserving_first_occurrence() {
    let raw = vec![
        "AETERM".to_string(),
        "AESEV".to_string(),
        "AETERM".to_string(),
    ];
    let out = SdtmProjectDomain::normalize_key_variables(&raw).unwrap();
    assert_eq!(out, vec!["AETERM".to_string(), "AESEV".to_string()]);
}

#[test]
fn normalize_key_variables_accepts_empty_list() {
    let out = SdtmProjectDomain::normalize_key_variables(&[]).unwrap();
    assert!(out.is_empty());
}

#[test]
fn normalize_key_variables_rejects_blank_entry() {
    let raw = vec!["AETERM".to_string(), " ".to_string()];
    let err = SdtmProjectDomain::normalize_key_variables(&raw).unwrap_err();
    assert!(matches!(err, DomainError::EmptyKeyVariable));
}

#[test]
fn sdtm_project_variable_new_rejects_blank_variable_code() {
    let err = SdtmProjectVariable::new(1, " ".into(), 1).unwrap_err();
    assert!(matches!(err, DomainError::EmptyVariableCode));
}

#[test]
fn sdtm_project_variable_new_accepts_valid_input() {
    let v = SdtmProjectVariable::new(1, "AETERM".into(), 11).unwrap();
    assert_eq!(v.variable_code, "AETERM");
    assert_eq!(v.variable_sequence, 11);
}

#[test]
fn domain_override_new_accepts_valid_input() {
    let desc = SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    };
    let o = SdtmProjectDomainOverride::new(1, DomainCategory::Events, vec![desc]).unwrap();
    assert_eq!(o.category, DomainCategory::Events);
    assert_eq!(o.descriptions.len(), 1);
    assert_eq!(o.descriptions[0].details.description, "Adverse events");
}

#[test]
fn variable_override_new_accepts_valid_input() {
    let desc = SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    };
    let o = SdtmProjectVariableOverride::new(
        1,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        vec![desc],
    )
    .unwrap();
    assert_eq!(o.variable_type, SdtmVariableType::Character);
    assert_eq!(o.variable_core, SdtmVariableCore::Req);
    assert_eq!(o.variable_role, Some(SdtmRole::Topic));
    assert_eq!(o.descriptions.len(), 1);
}
