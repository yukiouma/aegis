//! Public-API compile-only test for the `sdtm-project` crate.
//!
//! Pins the documented `use sdtm_project::*;` surface, the five
//! concrete repo constructors (`fn(PgPool) -> Repo`), the
//! `SdtmProjectUsecase::new(config)` constructor shape, and the
//! `Send + Sync` bound the usecase config relies on.

use sdtm_project::{
    ConversionRule, CreateSdtmProjectDomain, CreateSdtmProjectVariable, CreateSdtmProjectVersion,
    DomainCategory, DomainError, FromCrf, ProjectLookup, ProjectLookupImpl, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainRepoPg,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectServiceImpl,
    SdtmProjectUsecase, SdtmProjectUsecaseConfig, SdtmProjectVariable, SdtmProjectVariableNew,
    SdtmProjectVariableRepoPg, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepoPg,
    SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole, SdtmVariableControlled,
    SdtmVariableCore, SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableType,
    UpdateSdtmProjectDomain, UpdateSdtmProjectVariable, UpdateSdtmProjectVersion, UsecaseError,
    VariableSource,
};
use sqlx::PgPool;

/// Every documented consumer import resolves from the crate root.
#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_category(_: DomainCategory) {}
    fn assert_var_type(_: SdtmVariableType) {}
    fn assert_var_core(_: SdtmVariableCore) {}
    fn assert_role(_: SdtmRole) {}
    fn assert_domain_desc(_: SdtmDomainDescription) {}
    fn assert_domain_desc_detail(_: SdtmDomainDescriptionDetail) {}
    fn assert_var_desc(_: SdtmVariableDescription) {}
    fn assert_var_desc_detail(_: SdtmVariableDescriptionDetail) {}
    fn assert_conversion_rule(_: ConversionRule) {}
    fn assert_from_crf(_: FromCrf) {}
    fn assert_variable_source(_: VariableSource) {}
    fn assert_var_controlled(_: SdtmVariableControlled) {}
    fn assert_version_aggregate(_: SdtmProjectVersion) {}
    fn assert_domain_aggregate(_: SdtmProjectDomain) {}
    fn assert_variable_aggregate(_: SdtmProjectVariable) {}
    fn assert_create_version(_: CreateSdtmProjectVersion) {}
    fn assert_create_domain(_: CreateSdtmProjectDomain) {}
    fn assert_create_variable(_: CreateSdtmProjectVariable) {}
    fn assert_update_version(_: UpdateSdtmProjectVersion) {}
    fn assert_update_domain(_: UpdateSdtmProjectDomain) {}
    fn assert_update_variable(_: UpdateSdtmProjectVariable) {}
    fn assert_err(_: DomainError) {}
    fn assert_usecase_err(_: UsecaseError) {}

    assert_category(DomainCategory::Events);
    assert_var_type(SdtmVariableType::Character);
    assert_var_core(SdtmVariableCore::Req);
    assert_role(SdtmRole::Topic);
    assert_domain_desc(SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure: "One record per AE".into(),
        },
    });
    assert_domain_desc_detail(SdtmDomainDescriptionDetail {
        description: "Adverse events".into(),
        structure: "One record per AE".into(),
    });
    assert_var_desc(SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    });
    assert_var_desc_detail(SdtmVariableDescriptionDetail {
        label: "Adverse Event Term".into(),
    });

    // Aggregate constructors are nameable from the crate root.
    assert_version_aggregate(SdtmProjectVersion::new("P1".into(), "v".into()).unwrap());
    assert_domain_aggregate(
        SdtmProjectDomain::new(
            1,
            "AE".into(),
            DomainCategory::Events,
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    );
    assert_variable_aggregate(
        SdtmProjectVariable::new(
            1,
            "AETERM".into(),
            None,
            SdtmVariableType::Character,
            SdtmVariableCore::Req,
            Some(SdtmRole::Topic),
            11,
            Vec::new(),
            Vec::new(),
        )
        .unwrap(),
    );
    assert_conversion_rule(ConversionRule {
        source: VariableSource::Protocol,
        rule: "AESEV".into(),
    });
    assert_from_crf(FromCrf {
        form_code: "AE".into(),
    });
    assert_variable_source(VariableSource::Crf(Some(FromCrf {
        form_code: "AE".into(),
    })));
    assert_var_controlled(SdtmVariableControlled {
        code: Some(vec!["C66731".into()]),
        format: None,
    });

    // The command DTOs are constructible field-by-field.
    assert_create_version(CreateSdtmProjectVersion {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    });
    assert_create_domain(CreateSdtmProjectDomain {
        sdtm_project_version_id: 1,
        domain_code: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        key_variables: Vec::new(),
    });
    assert_create_variable(CreateSdtmProjectVariable {
        sdtm_project_domain_id: 1,
        variable_code: "AETERM".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: Vec::new(),
        conversion_rules: Vec::new(),
    });
    assert_update_version(UpdateSdtmProjectVersion::default());
    assert_update_domain(UpdateSdtmProjectDomain::default());
    assert_update_variable(UpdateSdtmProjectVariable::default());

    // The repository input DTOs are nameable from the crate root
    // too — they are what every adapter's `create` / `update` takes.
    fn assert_new_version(_: SdtmProjectVersionNew) {}
    fn assert_new_domain(_: SdtmProjectDomainNew) {}
    fn assert_new_variable(_: SdtmProjectVariableNew) {}
    fn assert_update_version_dto(_: SdtmProjectVersionUpdate) {}
    fn assert_update_domain_dto(_: SdtmProjectDomainUpdate) {}
    fn assert_update_variable_dto(_: SdtmProjectVariableUpdate) {}

    assert_new_version(SdtmProjectVersionNew {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    });
    assert_new_domain(SdtmProjectDomainNew {
        sdtm_project_version_id: 1,
        domain_code: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
        key_variables: Vec::new(),
    });
    assert_new_variable(SdtmProjectVariableNew {
        sdtm_project_domain_id: 1,
        variable_code: "AETERM".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: Vec::new(),
        conversion_rules: Vec::new(),
    });
    assert_update_version_dto(SdtmProjectVersionUpdate::default());
    assert_update_domain_dto(SdtmProjectDomainUpdate::default());
    assert_update_variable_dto(SdtmProjectVariableUpdate::default());

    // Touch the error types to keep them from being dead-code-eliminated.
    assert_err(DomainError::NotFound);
    assert_usecase_err(UsecaseError::Repository(DomainError::NotFound));
}

/// The three repository adapters are constructed from a `PgPool`.
#[test]
fn repo_constructors_take_a_pool() {
    let _: fn(PgPool) -> SdtmProjectVersionRepoPg = SdtmProjectVersionRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectDomainRepoPg = SdtmProjectDomainRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectVariableRepoPg = SdtmProjectVariableRepoPg::new;
}

/// The usecase is constructed from its config, and the facade from the
/// usecase.
#[test]
#[allow(clippy::type_complexity)]
fn usecase_constructor_takes_the_config() {
    fn assert_new<V, D, Va, P>(
        f: fn(SdtmProjectUsecaseConfig<V, D, Va, P>) -> SdtmProjectUsecase<V, D, Va, P>,
    ) where
        V: SdtmProjectVersionRepository,
        D: SdtmProjectDomainRepository,
        Va: SdtmProjectVariableRepository,
        P: ProjectLookup,
    {
        let _ = f;
    }
    assert_new::<
        SdtmProjectVersionRepoPg,
        SdtmProjectDomainRepoPg,
        SdtmProjectVariableRepoPg,
        ProjectLookupImpl,
    >(SdtmProjectUsecase::new);
}

/// The facade is constructed from the usecase.
#[test]
#[allow(clippy::type_complexity)]
fn facade_constructor_takes_the_usecase() {
    fn assert_new<V, D, Va, P>(
        f: fn(SdtmProjectUsecase<V, D, Va, P>) -> SdtmProjectServiceImpl<V, D, Va, P>,
    ) where
        V: SdtmProjectVersionRepository,
        D: SdtmProjectDomainRepository,
        Va: SdtmProjectVariableRepository,
        P: ProjectLookup,
    {
        let _ = f;
    }
    assert_new::<
        SdtmProjectVersionRepoPg,
        SdtmProjectDomainRepoPg,
        SdtmProjectVariableRepoPg,
        ProjectLookupImpl,
    >(SdtmProjectServiceImpl::new);
}

/// The `ProjectLookup` port and the `SdtmProjectService` port are
/// object-safe and `Send + Sync`, so the server can hold them behind
/// `Arc<dyn …>`.
#[test]
fn ports_are_object_safe_and_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn apis::sdtm_project::SdtmProjectService>>();
    assert_send_sync::<Box<dyn ProjectLookup>>();
}
