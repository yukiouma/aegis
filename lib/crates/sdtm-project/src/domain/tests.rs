use super::{
    ConversionRule, DomainCategory, DomainError, FromCrf, SdtmProjectDomain, SdtmProjectVariable,
    SdtmProjectVersion, SdtmRole, SdtmVariableControlled, SdtmVariableCore, SdtmVariableType,
    VariableSource,
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
    let err = SdtmProjectDomain::new(
        1,
        "  ".into(),
        DomainCategory::Events,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyDomainCode));
}

#[test]
fn sdtm_project_domain_new_rejects_blank_key_variable() {
    let err = SdtmProjectDomain::new(
        1,
        "AE".into(),
        DomainCategory::Events,
        Vec::new(),
        vec!["  ".into()],
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyKeyVariable));
}

#[test]
fn sdtm_project_domain_new_accepts_valid_input() {
    let d = SdtmProjectDomain::new(
        1,
        "AE".into(),
        DomainCategory::Events,
        Vec::new(),
        vec!["AETERM".into()],
    )
    .unwrap();
    assert_eq!(d.domain_code, "AE");
    assert_eq!(d.category, DomainCategory::Events);
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
    let err = SdtmProjectVariable::new(
        1,
        " ".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        None,
        1,
        Vec::new(),
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyVariableCode));
}

#[test]
fn sdtm_project_variable_new_rejects_a_blank_conversion_rule() {
    let err = SdtmProjectVariable::new(
        1,
        "AETERM".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        None,
        1,
        Vec::new(),
        vec![ConversionRule {
            source: VariableSource::Protocol,
            rule: "   ".into(),
        }],
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyConversionRule));
}

#[test]
fn sdtm_project_variable_new_accepts_valid_input() {
    let v = SdtmProjectVariable::new(
        1,
        "AETERM".into(),
        Some(SdtmVariableControlled {
            code: Some(vec!["C66731".into()]),
            format: None,
        }),
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        11,
        Vec::new(),
        vec![ConversionRule {
            source: VariableSource::Protocol,
            rule: "AESEV".into(),
        }],
    )
    .unwrap();
    assert_eq!(v.variable_code, "AETERM");
    assert_eq!(v.variable_sequence, 11);
    assert_eq!(v.variable_type, SdtmVariableType::Character);
    assert_eq!(v.variable_core, SdtmVariableCore::Req);
    assert_eq!(v.variable_role, Some(SdtmRole::Topic));
    assert_eq!(v.conversion_rules.len(), 1);
    assert_eq!(v.conversion_rules[0].rule, "AESEV");
}

/// The wire spelling of `VariableSource` is pinned: unit variants omit
/// the `from` key entirely, payload variants carry it (as `null` when
/// the form is absent). A flat `Option<FromCrf>` field cannot express
/// both, which is why the enum is adjacently tagged.
#[test]
fn variable_source_serializes_to_the_agreed_json() {
    let cases: Vec<(VariableSource, &str)> = vec![
        (VariableSource::Protocol, r#"{"source":"Protocol"}"#),
        (VariableSource::Derived, r#"{"source":"Derived"}"#),
        (VariableSource::Edt, r#"{"source":"eDT"}"#),
        (
            VariableSource::Crf(Some(FromCrf {
                form_code: "AE".into(),
            })),
            r#"{"source":"CRF","from":{"form_code":"AE"}}"#,
        ),
        (
            VariableSource::Assigned(None),
            r#"{"source":"Assigned","from":null}"#,
        ),
    ];
    for (source, expected) in cases {
        assert_eq!(serde_json::to_string(&source).unwrap(), expected);
    }
}

/// A conversion rule flattens its source, so the source's own fields
/// sit beside the rule text rather than nesting under a second
/// `source` key.
#[test]
fn conversion_rule_serializes_the_source_flat() {
    let rule = ConversionRule {
        source: VariableSource::Crf(Some(FromCrf {
            form_code: "AE".into(),
        })),
        rule: "AESEV".into(),
    };
    let json = serde_json::to_string(&rule).unwrap();
    assert_eq!(
        json,
        r#"{"source":"CRF","from":{"form_code":"AE"},"rule":"AESEV"}"#
    );
    let back: ConversionRule = serde_json::from_str(&json).unwrap();
    assert_eq!(back, rule);
}

#[test]
fn sdtm_variable_controlled_omits_unset_members() {
    let json = serde_json::to_string(&SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    })
    .unwrap();
    assert_eq!(json, r#"{"code":["C66731"]}"#);
}
