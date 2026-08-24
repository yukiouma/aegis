//! Public-API compile-only test for the `domain-model` crate.
//!
//! Pins the documented `use domain_model::*;` surface, the three
//! concrete repo constructors (`fn(PgPool) -> Repo`), the
//! `DomainModelUsecase::new(config)` constructor shape, and the
//! `Send + Sync` bound the usecase config relies on.

use domain_model::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, DomainCategory,
    DomainModelServiceImpl, DomainModelUsecase, DomainModelUsecaseConfig, SdtmDomain,
    SdtmDomainRepoPg, SdtmRole, SdtmVariable, SdtmVariableCore, SdtmVariableRepoPg,
    SdtmVariableType, SdtmVersion, SdtmVersionRepoPg, UpdateSdtmDomain, UpdateSdtmVariable,
    UpdateSdtmVersion,
};
use sqlx::PgPool;

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_category(_: DomainCategory) {}
    fn assert_var_type(_: SdtmVariableType) {}
    fn assert_var_core(_: SdtmVariableCore) {}
    fn assert_role(_: SdtmRole) {}
    fn assert_version_aggregate(_: SdtmVersion) {}
    fn assert_domain_aggregate(_: SdtmDomain) {}
    fn assert_variable_aggregate(_: SdtmVariable) {}
    fn assert_create_v(_: CreateSdtmVersion) {}
    fn assert_create_d(_: CreateSdtmDomain) {}
    fn assert_create_va(_: CreateSdtmVariable) {}
    fn assert_update_v(_: UpdateSdtmVersion) {}
    fn assert_update_d(_: UpdateSdtmDomain) {}
    fn assert_update_va(_: UpdateSdtmVariable) {}

    assert_category(DomainCategory::Events);
    assert_var_type(SdtmVariableType::Character);
    assert_var_core(SdtmVariableCore::Req);
    assert_role(SdtmRole::Topic);
    // Aggregate constructors don't return concrete instances from the
    // crate root — `assert_*_aggregate` is just a nameability hook.
    assert_create_v(CreateSdtmVersion {
        name: "2026-08-24".into(),
    });
    assert_create_d(CreateSdtmDomain {
        version_id: 1,
        name: "AE".into(),
        category: DomainCategory::Events,
        descriptions: vec![],
    });
    assert_create_va(CreateSdtmVariable {
        domain_id: 1,
        name: "AETERM".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 1,
        descriptions: vec![],
    });
    assert_update_v(UpdateSdtmVersion::default());
    assert_update_d(UpdateSdtmDomain::default());
    assert_update_va(UpdateSdtmVariable::default());
    let _ = (
        assert_version_aggregate,
        assert_domain_aggregate,
        assert_variable_aggregate,
    );
}

#[test]
fn repos_construct_from_pg_pool_via_function_pointer() {
    let v: fn(PgPool) -> SdtmVersionRepoPg = SdtmVersionRepoPg::new;
    let d: fn(PgPool) -> SdtmDomainRepoPg = SdtmDomainRepoPg::new;
    let va: fn(PgPool) -> SdtmVariableRepoPg = SdtmVariableRepoPg::new;
    let _ = (v, d, va);
}

#[test]
fn usecase_constructor_accepts_three_repo_args() {
    #[allow(clippy::type_complexity)]
    fn assert_new_constructor<V, D, Va>(
        _: fn(DomainModelUsecaseConfig<V, D, Va>) -> DomainModelUsecase<V, D, Va>,
    ) where
        V: domain_model::SdtmVersionRepository,
        D: domain_model::SdtmDomainRepository,
        Va: domain_model::SdtmVariableRepository,
    {
    }
    assert_new_constructor::<SdtmVersionRepoPg, SdtmDomainRepoPg, SdtmVariableRepoPg>(
        DomainModelUsecase::new,
    );
}

#[test]
fn service_impl_compiles_for_pg_repos() {
    // `DomainModelServiceImpl<V, D, Va>` is generic over the three
    // repository ports; we just lock that the concrete Postgres
    // repos satisfy those bounds, so `from_repos` will accept them
    // at runtime. We don't run any I/O — only the type system is
    // exercised.
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<
        DomainModelServiceImpl<SdtmVersionRepoPg, SdtmDomainRepoPg, SdtmVariableRepoPg>,
    >();
    assert_send_sync::<Box<dyn apis::domain_model::DomainModelService>>();
}
