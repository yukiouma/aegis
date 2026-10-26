//! Public-API compile-only test for the `sdtm-project` crate.
//!
//! Pins the documented `use sdtm_project::*;` surface, the five
//! concrete repo constructors (`fn(PgPool) -> Repo`), the
//! `SdtmProjectUsecase::new(config)` constructor shape, and the
//! `Send + Sync` bound the usecase config relies on.

use sdtm_project::{
    CreateSdtmProjectDomain, CreateSdtmProjectDomainOverride, CreateSdtmProjectVariable,
    CreateSdtmProjectVariableOverride, CreateSdtmProjectVersion, DomainCategory, DomainError,
    ProjectLookup, ProjectLookupImpl, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmProjectDomain, SdtmProjectDomainNew, SdtmProjectDomainOverride,
    SdtmProjectDomainOverrideNew, SdtmProjectDomainOverrideRepoPg,
    SdtmProjectDomainOverrideRepository, SdtmProjectDomainOverrideUpdate, SdtmProjectDomainRepoPg,
    SdtmProjectDomainRepository, SdtmProjectDomainUpdate, SdtmProjectServiceImpl,
    SdtmProjectUsecase, SdtmProjectUsecaseConfig, SdtmProjectVariable, SdtmProjectVariableNew,
    SdtmProjectVariableOverride, SdtmProjectVariableOverrideNew, SdtmProjectVariableOverrideRepoPg,
    SdtmProjectVariableOverrideRepository, SdtmProjectVariableOverrideUpdate,
    SdtmProjectVariableRepoPg, SdtmProjectVariableRepository, SdtmProjectVariableUpdate,
    SdtmProjectVersion, SdtmProjectVersionNew, SdtmProjectVersionRepoPg,
    SdtmProjectVersionRepository, SdtmProjectVersionUpdate, SdtmRole, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableType,
    UpdateSdtmProjectDomain, UpdateSdtmProjectDomainOverride, UpdateSdtmProjectVariable,
    UpdateSdtmProjectVariableOverride, UpdateSdtmProjectVersion, UsecaseError,
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
    fn assert_version_aggregate(_: SdtmProjectVersion) {}
    fn assert_domain_aggregate(_: SdtmProjectDomain) {}
    fn assert_domain_override_aggregate(_: SdtmProjectDomainOverride) {}
    fn assert_variable_aggregate(_: SdtmProjectVariable) {}
    fn assert_variable_override_aggregate(_: SdtmProjectVariableOverride) {}
    fn assert_create_version(_: CreateSdtmProjectVersion) {}
    fn assert_create_domain(_: CreateSdtmProjectDomain) {}
    fn assert_create_domain_override(_: CreateSdtmProjectDomainOverride) {}
    fn assert_create_variable(_: CreateSdtmProjectVariable) {}
    fn assert_create_variable_override(_: CreateSdtmProjectVariableOverride) {}
    fn assert_update_version(_: UpdateSdtmProjectVersion) {}
    fn assert_update_domain(_: UpdateSdtmProjectDomain) {}
    fn assert_update_domain_override(_: UpdateSdtmProjectDomainOverride) {}
    fn assert_update_variable(_: UpdateSdtmProjectVariable) {}
    fn assert_update_variable_override(_: UpdateSdtmProjectVariableOverride) {}
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
    assert_domain_aggregate(SdtmProjectDomain::new(1, "AE".into(), Vec::new()).unwrap());
    assert_domain_override_aggregate(
        SdtmProjectDomainOverride::new(1, DomainCategory::Events, Vec::new()).unwrap(),
    );
    assert_variable_aggregate(SdtmProjectVariable::new(1, "AETERM".into(), 1).unwrap());
    assert_variable_override_aggregate(
        SdtmProjectVariableOverride::new(
            1,
            SdtmVariableType::Character,
            SdtmVariableCore::Req,
            Some(SdtmRole::Topic),
            Vec::new(),
        )
        .unwrap(),
    );

    // The command DTOs are constructible field-by-field.
    assert_create_version(CreateSdtmProjectVersion {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    });
    assert_create_domain(CreateSdtmProjectDomain {
        version_id: 1,
        domain_code: "AE".into(),
        key_variables: vec!["AETERM".into()],
    });
    assert_create_domain_override(CreateSdtmProjectDomainOverride {
        domain_id: 1,
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    });
    assert_create_variable(CreateSdtmProjectVariable {
        domain_id: 1,
        variable_code: "AETERM".into(),
        variable_sequence: 11,
    });
    assert_create_variable_override(CreateSdtmProjectVariableOverride {
        variable_id: 1,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        descriptions: Vec::new(),
    });
    assert_update_version(UpdateSdtmProjectVersion::default());
    assert_update_domain(UpdateSdtmProjectDomain::default());
    assert_update_domain_override(UpdateSdtmProjectDomainOverride::default());
    assert_update_variable(UpdateSdtmProjectVariable::default());
    assert_update_variable_override(UpdateSdtmProjectVariableOverride::default());

    // The repository input DTOs are nameable from the crate root
    // too — they are what every adapter's `create` / `update` takes.
    fn assert_new_version(_: SdtmProjectVersionNew) {}
    fn assert_new_domain(_: SdtmProjectDomainNew) {}
    fn assert_new_domain_override(_: SdtmProjectDomainOverrideNew) {}
    fn assert_new_variable(_: SdtmProjectVariableNew) {}
    fn assert_new_variable_override(_: SdtmProjectVariableOverrideNew) {}
    fn assert_update_version_dto(_: SdtmProjectVersionUpdate) {}
    fn assert_update_domain_dto(_: SdtmProjectDomainUpdate) {}
    fn assert_update_domain_override_dto(_: SdtmProjectDomainOverrideUpdate) {}
    fn assert_update_variable_dto(_: SdtmProjectVariableUpdate) {}
    fn assert_update_variable_override_dto(_: SdtmProjectVariableOverrideUpdate) {}

    assert_new_version(SdtmProjectVersionNew {
        project_code: "P1".into(),
        version_name: "2024-09-27".into(),
    });
    assert_new_domain(SdtmProjectDomainNew {
        version_id: 1,
        domain_code: "AE".into(),
        key_variables: vec!["AETERM".into()],
    });
    assert_new_domain_override(SdtmProjectDomainOverrideNew {
        domain_id: 1,
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    });
    assert_new_variable(SdtmProjectVariableNew {
        domain_id: 1,
        variable_code: "AETERM".into(),
        variable_sequence: 11,
    });
    assert_new_variable_override(SdtmProjectVariableOverrideNew {
        variable_id: 1,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        descriptions: Vec::new(),
    });
    assert_update_version_dto(SdtmProjectVersionUpdate::default());
    assert_update_domain_dto(SdtmProjectDomainUpdate::default());
    assert_update_domain_override_dto(SdtmProjectDomainOverrideUpdate::default());
    assert_update_variable_dto(SdtmProjectVariableUpdate::default());
    assert_update_variable_override_dto(SdtmProjectVariableOverrideUpdate::default());

    // Touch the error types to keep them from being dead-code-eliminated.
    assert_err(DomainError::NotFound);
    assert_usecase_err(UsecaseError::Repository(DomainError::NotFound));
}

/// The five repository adapters are constructed from a `PgPool`.
#[test]
fn repo_constructors_take_a_pool() {
    let _: fn(PgPool) -> SdtmProjectVersionRepoPg = SdtmProjectVersionRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectDomainRepoPg = SdtmProjectDomainRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectDomainOverrideRepoPg = SdtmProjectDomainOverrideRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectVariableRepoPg = SdtmProjectVariableRepoPg::new;
    let _: fn(PgPool) -> SdtmProjectVariableOverrideRepoPg = SdtmProjectVariableOverrideRepoPg::new;
}

/// The usecase is constructed from its config, and the facade from the
/// usecase.
#[test]
#[allow(clippy::type_complexity)]
fn usecase_constructor_takes_the_config() {
    fn assert_new<V, D, DO, Va, VO, P>(
        f: fn(
            SdtmProjectUsecaseConfig<V, D, DO, Va, VO, P>,
        ) -> SdtmProjectUsecase<V, D, DO, Va, VO, P>,
    ) where
        V: SdtmProjectVersionRepository,
        D: SdtmProjectDomainRepository,
        DO: SdtmProjectDomainOverrideRepository,
        Va: SdtmProjectVariableRepository,
        VO: SdtmProjectVariableOverrideRepository,
        P: ProjectLookup,
    {
        let _ = f;
    }
    assert_new::<
        SdtmProjectVersionRepoPg,
        SdtmProjectDomainRepoPg,
        SdtmProjectDomainOverrideRepoPg,
        SdtmProjectVariableRepoPg,
        SdtmProjectVariableOverrideRepoPg,
        ProjectLookupImpl,
    >(SdtmProjectUsecase::new);
}

/// The facade is constructed from the usecase.
#[test]
#[allow(clippy::type_complexity)]
fn facade_constructor_takes_the_usecase() {
    fn assert_new<V, D, DO, Va, VO, P>(
        f: fn(
            SdtmProjectUsecase<V, D, DO, Va, VO, P>,
        ) -> SdtmProjectServiceImpl<V, D, DO, Va, VO, P>,
    ) where
        V: SdtmProjectVersionRepository,
        D: SdtmProjectDomainRepository,
        DO: SdtmProjectDomainOverrideRepository,
        Va: SdtmProjectVariableRepository,
        VO: SdtmProjectVariableOverrideRepository,
        P: ProjectLookup,
    {
        let _ = f;
    }
    assert_new::<
        SdtmProjectVersionRepoPg,
        SdtmProjectDomainRepoPg,
        SdtmProjectDomainOverrideRepoPg,
        SdtmProjectVariableRepoPg,
        SdtmProjectVariableOverrideRepoPg,
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
