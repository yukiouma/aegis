//! Public-API compile test for the `mission` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p mission` time.

use apis::mission::{
    Actor, AppendCommentRequest, AssigneeData, AssigneeView as ApiAssigneeView, CloseIssueRequest,
    CreateIssueRequest, CreateMissionRequest, IssueCommentView as ApiIssueCommentView,
    IssueState as ApiIssueState, IssueView as ApiIssueView, ListIssuesByMissionRequest,
    MissionApiError, MissionKind as ApiMissionKind, MissionRole as ApiMissionRole, MissionService,
    MissionView as ApiMissionView, ReopenIssueRequest, UpdateIssueDescriptionRequest,
};
use chrono::{TimeZone, Utc};
use mission::{
    Assignee, AssigneeRepo, AssigneeRepository, CreateIssue, DomainError, IssueComment, IssueRepo,
    IssueState, Mission, MissionIssue, MissionIssueRepository, MissionIssueUsecase,
    MissionIssueUsecaseConfig, MissionKind, MissionRepo, MissionRepository, MissionRole,
    MissionServiceImpl, MissionUsecase, MissionUsecaseConfig, ProjectLookup, ProjectLookupImpl,
    UsecaseError, UserLookup, UserLookupImpl,
};
use sqlx::PgPool;

#[test]
fn domain_types_are_nameable_from_crate_root() {
    fn assert_mission(_: Mission) {}
    fn assert_assignee(_: Assignee) {}
    fn assert_mission_issue(_: MissionIssue) {}
    fn assert_issue_comment(_: IssueComment) {}
    fn assert_mission_kind(_: MissionKind) {}
    fn assert_mission_role(_: MissionRole) {}
    fn assert_issue_state(_: IssueState) {}

    assert_mission_kind(MissionKind::Crf);
    assert_mission_kind(MissionKind::Sdtm);
    assert_mission_kind(MissionKind::Adam);
    assert_mission_kind(MissionKind::Tfl);
    assert_mission_role(MissionRole::Dev);
    assert_mission_role(MissionRole::Qc);
    assert_issue_state(IssueState::Opened);
    assert_issue_state(IssueState::Closed);
    let _ = assert_mission;
    let _ = assert_assignee;
    let _ = assert_mission_issue;
    let _ = assert_issue_comment;
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
    assert_dom(DomainError::MissionIssueNotFound);
    assert_dom(DomainError::EmptyIssueDescription);
    assert_dom(DomainError::EmptyIssueIssuer);
    assert_dom(DomainError::EmptyCommentContent);
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
fn issue_usecase_config_has_expected_field_shape() {
    let _assert_config_shape: fn(
        MissionIssueUsecaseConfig<MissionRepo, AssigneeRepo, ProjectLookupImpl, IssueRepo>,
    ) = |cfg| {
        let _: &MissionRepo = &cfg.mission_repo;
        let _: &AssigneeRepo = &cfg.assignee_repo;
        let _: &ProjectLookupImpl = &cfg.project_lookup;
        let _: &IssueRepo = &cfg.issue_repo;
    };
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor_m: fn(PgPool) -> MissionRepo = MissionRepo::new;
    let ctor_a: fn(PgPool) -> AssigneeRepo = AssigneeRepo::new;
    let ctor_i: fn(PgPool) -> IssueRepo = IssueRepo::new;
    let _ = (ctor_m, ctor_a, ctor_i);
}

#[test]
fn ports_can_be_dispatched_dynamically() {
    fn assert_box_dyn_mr<R: MissionRepository + 'static>() {}
    fn assert_box_dyn_ar<R: AssigneeRepository + 'static>() {}
    fn assert_box_dyn_ir<R: MissionIssueRepository + 'static>() {}
    fn assert_box_dyn_pl<P: ProjectLookup + 'static>() {}
    fn assert_box_dyn_ul<U: UserLookup + 'static>() {}
    assert_box_dyn_mr::<MissionRepo>();
    assert_box_dyn_ar::<AssigneeRepo>();
    assert_box_dyn_ir::<IssueRepo>();
    assert_box_dyn_pl::<ProjectLookupImpl>();
    assert_box_dyn_ul::<UserLookupImpl>();
}

#[test]
fn api_view_dtos_are_nameable() {
    fn assert_mission(_: ApiMissionView) {}
    fn assert_assignee(_: ApiAssigneeView) {}
    fn assert_issue(_: ApiIssueView) {}
    fn assert_issue_comment(_: ApiIssueCommentView) {}
    fn assert_kind(_: ApiMissionKind) {}
    fn assert_role(_: ApiMissionRole) {}
    fn assert_state(_: ApiIssueState) {}
    let now = Utc.timestamp_opt(0, 0).unwrap();
    assert_kind(ApiMissionKind::Crf);
    assert_role(ApiMissionRole::Qc);
    assert_state(ApiIssueState::Opened);
    assert_state(ApiIssueState::Closed);
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
    assert_issue(ApiIssueView {
        id: 1,
        mission_id: 1,
        target_item: None,
        issuer: "u1".into(),
        description: "d".into(),
        state: ApiIssueState::Opened,
        comments: vec![ApiIssueCommentView {
            user: "u2".into(),
            content: "c".into(),
            created_at: now,
        }],
        created_at: now,
        updated_at: now,
    });
    let _ = assert_issue_comment;
}

#[test]
fn api_error_variants_are_nameable() {
    fn assert_err(_: MissionApiError) {}
    assert_err(MissionApiError::Validation("bad".into()));
    assert_err(MissionApiError::NotFound);
    assert_err(MissionApiError::AssigneeNotFound);
    assert_err(MissionApiError::IssueNotFound);
    assert_err(MissionApiError::MissionNotFoundForIssue(1));
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
    let _create_issue = CreateIssueRequest {
        mission_id: 1,
        target_item: Some("dm.x".into()),
        description: "d".into(),
    };
    let _close = CloseIssueRequest::default();
    let _reopen = ReopenIssueRequest::default();
    let _upd_desc = UpdateIssueDescriptionRequest {
        description: "d".into(),
    };
    let _append = AppendCommentRequest {
        content: "c".into(),
    };
    let _list = ListIssuesByMissionRequest {
        mission_id: 1,
        state: Some(ApiIssueState::Closed),
    };
    // Lock the field shape of CreateIssue inside the usecase layer.
    let _uc: fn(CreateIssue) -> (i64, Option<String>, String) =
        |c| (c.mission_id, c.target_item, c.description);
}

#[test]
#[allow(clippy::type_complexity)]
fn mission_service_impl_is_object_safe() {
    // Pin the trait surface through a function pointer so
    // object-safety is checked at compile time without ever
    // constructing an instance.
    let _: fn(
        MissionServiceImpl<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl, IssueRepo>,
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

#[test]
#[allow(clippy::type_complexity)]
fn issue_usecase_can_be_built_from_config() {
    let _: fn(
        MissionIssueUsecaseConfig<MissionRepo, AssigneeRepo, ProjectLookupImpl, IssueRepo>,
    ) -> MissionIssueUsecase<MissionRepo, AssigneeRepo, ProjectLookupImpl, IssueRepo> =
        |cfg| MissionIssueUsecase::new(cfg);
}
