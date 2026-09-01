//! Public-API compile test for the `mission` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p mission` time.

use apis::mission::{
    Actor, AssigneeData, AssigneeView as ApiAssigneeView, CreateMissionRequest, MissionApiError,
    MissionKind as ApiMissionKind, MissionRole as ApiMissionRole, MissionService,
    MissionView as ApiMissionView,
};
use chrono::{TimeZone, Utc};
use mission::{
    Assignee, AssigneeRepo, AssigneeRepository, DomainError, Mission, MissionKind, MissionRepo,
    MissionRepository, MissionRole, MissionServiceImpl, MissionUsecase, MissionUsecaseConfig,
    ProjectLookup, ProjectLookupImpl, UsecaseError, UserLookup, UserLookupImpl,
};
use sqlx::PgPool;

#[test]
fn domain_types_are_nameable_from_crate_root() {
    fn assert_mission(_: Mission) {}
    fn assert_assignee(_: Assignee) {}
    fn assert_mission_kind(_: MissionKind) {}
    fn assert_mission_role(_: MissionRole) {}

    assert_mission_kind(MissionKind::Crf);
    assert_mission_kind(MissionKind::Sdtm);
    assert_mission_kind(MissionKind::Adam);
    assert_mission_kind(MissionKind::Tfl);
    assert_mission_role(MissionRole::Dev);
    assert_mission_role(MissionRole::Qc);
    let _ = assert_mission;
    let _ = assert_assignee;
}

#[test]
fn domain_error_variants_are_nameable() {
    fn assert_dom(_: DomainError) {}
    assert_dom(DomainError::EmptyMissionCode);
    assert_dom(DomainError::EmptyUserCode);
    assert_dom(DomainError::UnknownMissionKind("crf".into()));
    assert_dom(DomainError::UnknownMissionRole("dev".into()));
    assert_dom(DomainError::NotFound);
    assert_dom(DomainError::AssigneeNotFound);
    assert_dom(DomainError::ProjectNotFound("p1".into()));
    assert_dom(DomainError::UserNotFound("u1".into()));
    assert_dom(DomainError::Repository("boom".into()));
}

#[test]
fn usecase_error_wraps_domain_error() {
    fn assert_us(_: UsecaseError) {}
    assert_us(UsecaseError::Domain(DomainError::NotFound));
    assert_us(UsecaseError::Forbidden {
        user_code: "u1".into(),
        project_code: "p1".into(),
    });
}

#[test]
fn usecase_config_has_expected_field_shape() {
    // Lock the field names of MissionUsecaseConfig so any
    // misnamed-field rename downstream breaks the build here, not
    // at the call site in run.rs.
    let _assert_config_shape: fn(
        MissionUsecaseConfig<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl>,
    ) = |cfg| {
        let _: &MissionRepo = &cfg.mission_repo;
        let _: &AssigneeRepo = &cfg.assignee_repo;
        let _: &ProjectLookupImpl = &cfg.project_lookup;
        let _: &UserLookupImpl = &cfg.user_lookup;
    };
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor_m: fn(PgPool) -> MissionRepo = MissionRepo::new;
    let ctor_a: fn(PgPool) -> AssigneeRepo = AssigneeRepo::new;
    let _ = (ctor_m, ctor_a);
}

#[test]
fn ports_can_be_dispatched_dynamically() {
    fn assert_box_dyn_mr<R: MissionRepository + 'static>() {}
    fn assert_box_dyn_ar<R: AssigneeRepository + 'static>() {}
    fn assert_box_dyn_pl<P: ProjectLookup + 'static>() {}
    fn assert_box_dyn_ul<U: UserLookup + 'static>() {}
    assert_box_dyn_mr::<MissionRepo>();
    assert_box_dyn_ar::<AssigneeRepo>();
    assert_box_dyn_pl::<ProjectLookupImpl>();
    assert_box_dyn_ul::<UserLookupImpl>();
}

#[test]
fn api_view_dtos_are_nameable() {
    fn assert_mission(_: ApiMissionView) {}
    fn assert_assignee(_: ApiAssigneeView) {}
    fn assert_kind(_: ApiMissionKind) {}
    fn assert_role(_: ApiMissionRole) {}
    let now = Utc.timestamp_opt(0, 0).unwrap();
    assert_kind(ApiMissionKind::Crf);
    assert_role(ApiMissionRole::Dev);
    assert_mission(ApiMissionView {
        id: 1,
        project_code: "p1".into(),
        mission_kind: ApiMissionKind::Sdtm,
        mission_code: "m1".into(),
        assignees: vec![],
        created_at: now,
        updated_at: now,
    });
    assert_assignee(ApiAssigneeView {
        id: 1,
        user_code: "u1".into(),
        role: ApiMissionRole::Qc,
        created_at: now,
        updated_at: now,
    });
}

#[test]
fn api_error_variants_are_nameable() {
    fn assert_err(_: MissionApiError) {}
    assert_err(MissionApiError::Validation("bad".into()));
    assert_err(MissionApiError::NotFound);
    assert_err(MissionApiError::AssigneeNotFound);
    assert_err(MissionApiError::ProjectNotFound("p1".into()));
    assert_err(MissionApiError::UserNotFound("u1".into()));
    assert_err(MissionApiError::Forbidden {
        user_code: "u1".into(),
        project_code: "p1".into(),
    });
    assert_err(MissionApiError::DuplicateMission {
        project_code: "p1".into(),
        mission_kind: ApiMissionKind::Adam,
        mission_code: "m1".into(),
    });
    assert_err(MissionApiError::DuplicateAssignee {
        mission_id: 1,
        user_code: "u1".into(),
        role: ApiMissionRole::Dev,
    });
    assert_err(MissionApiError::Repository("boom".into()));
}

#[test]
fn api_requests_have_expected_field_shape() {
    let _create = CreateMissionRequest {
        project_code: "p1".into(),
        mission_kind: ApiMissionKind::Tfl,
        mission_code: "m1".into(),
        assignees: vec![AssigneeData {
            user_code: "u1".into(),
            role: ApiMissionRole::Dev,
        }],
    };
    let _actor = Actor {
        user_code: "u1".into(),
    };
}

#[test]
#[allow(clippy::type_complexity)]
fn mission_service_impl_is_object_safe() {
    // Pin the trait surface through a function pointer so
    // object-safety is checked at compile time without ever
    // constructing an instance.
    let _: fn(
        MissionServiceImpl<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl>,
    ) -> Box<dyn MissionService> = |s| Box::new(s);
}

#[test]
#[allow(clippy::type_complexity)]
fn mission_usecase_can_be_built_from_config() {
    // Lock the MissionUsecase generic shape so a future type-param
    // rename breaks the build here, not at the call site.
    let _: fn(
        MissionUsecaseConfig<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl>,
    ) -> MissionUsecase<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl> =
        |cfg| MissionUsecase::new(cfg);
}
