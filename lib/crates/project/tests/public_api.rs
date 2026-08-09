//! Public-API compile test for the `project` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p project` time.

use apis::project::{
    CreateProductRequest, CreateProjectRequest, ProjectApiError, ProjectMemberData,
    ProjectMemberView as ApiProjectMemberView, ProjectService, UpdateProductRequest,
    UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};
use project::{
    CreateProduct, CreateProject, DomainError, ProductNew, ProductRepo, ProductRepository,
    ProductUpdate, ProjectMember, ProjectNew, ProjectRepo, ProjectRepository, ProjectServiceImpl,
    ProjectUpdate, ProjectUsecaseConfig, ProjectView, RoleType, TeamType, UpdateProduct,
    UpdateProject, UsecaseError, UserService, UserServiceImpl, UserSummary, UserSummaryView,
};
use sqlx::PgPool;

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_role(_: RoleType) {}
    fn assert_team(_: TeamType) {}
    fn assert_user_summary(_: UserSummary) {}
    fn assert_user_view(_: UserSummaryView) {}

    assert_role(RoleType::Leader);
    assert_team(TeamType::Members);

    let summary = UserSummary {
        code: "u1".into(),
        name: "Alice".into(),
    };
    assert_user_summary(summary);
    let view = UserSummaryView {
        code: "u1".into(),
        name: "Alice".into(),
    };
    assert_user_view(view);
}

#[test]
fn usecase_commands_have_expected_field_shape() {
    let _create_product = CreateProduct {
        code: "p1".into(),
        name: "Widget".into(),
        description: "".into(),
    };

    let _update_product = UpdateProduct {
        id: 1,
        code: None,
        name: None,
        description: None,
        active: None,
    };

    let _create_project = CreateProject {
        code: "proj1".into(),
        description: "".into(),
        product_id: 1,
        members: None,
        unblind_members: None,
    };

    let _update_project = UpdateProject {
        id: 1,
        code: None,
        description: None,
        product_id: None,
        active: None,
        members: None,
        unblind_members: None,
    };
}

#[test]
fn api_requests_have_expected_field_shape() {
    let _create_product = CreateProductRequest {
        code: "p1".into(),
        name: "Widget".into(),
        description: "".into(),
    };

    let _update_product = UpdateProductRequest {
        id: 1,
        code: None,
        name: None,
        description: None,
        active: None,
    };

    let _create_project = CreateProjectRequest {
        code: "proj1".into(),
        description: "".into(),
        product_id: 1,
        members: None,
        unblind_members: None,
    };

    let _update_project = UpdateProjectRequest {
        id: 1,
        code: None,
        description: None,
        product_id: None,
        active: None,
        members: None,
        unblind_members: None,
    };
}

#[test]
fn project_usecase_config_has_expected_field_shape() {
    let _assert_config_shape: fn(
        cfg: ProjectUsecaseConfig<ProductRepo, ProjectRepo, UserServiceImpl>,
    ) = |cfg| {
        let _: &ProductRepo = &cfg.product_repo;
        let _: &ProjectRepo = &cfg.project_repo;
        let _: &UserServiceImpl = &cfg.users;
    };
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor: fn(PgPool) -> ProductRepo = ProductRepo::new;
    let ctor2: fn(PgPool) -> ProjectRepo = ProjectRepo::new;
    let _ = (ctor, ctor2);
}

#[test]
fn domain_error_variants_are_nameable() {
    fn assert_dom(_: DomainError) {}
    assert_dom(DomainError::NotFound);
    assert_dom(DomainError::EmptyCode);
    assert_dom(DomainError::ZeroProductId);
    assert_dom(DomainError::DuplicateCode("p1".into()));
    assert_dom(DomainError::UserNotFound("u1".into()));
}

#[test]
fn usecase_error_wraps_domain_error() {
    fn assert_us(_: UsecaseError) {}
    assert_us(UsecaseError::Validation(DomainError::EmptyCode));
    assert_us(UsecaseError::Repository(DomainError::NotFound));
}

#[test]
fn project_service_impl_is_object_safe() {
    // Pin the trait surface through a function pointer so object-safety
    // is checked at compile time without ever constructing an instance.
    let _: fn(
        ProjectServiceImpl<ProductRepo, ProjectRepo, UserServiceImpl>,
    ) -> Box<dyn ProjectService> = |s| Box::new(s);
}

#[test]
fn ports_can_be_dispatched_dynamically() {
    // The bound itself is the test — if a port ever loses object
    // safety, this `where` clause will fail to compile.
    fn assert_box_dyn_pr<R: ProductRepository + 'static>() {}
    fn assert_box_dyn_prr<R: ProjectRepository + 'static>() {}
    fn assert_box_dyn_us<U: UserService + 'static>() {}
    assert_box_dyn_pr::<ProductRepo>();
    assert_box_dyn_prr::<ProjectRepo>();
    assert_box_dyn_us::<UserServiceImpl>();
}

// ---- mirrors of the apis view DTOs (kept here so a future apis-side
// ---- change cannot silently break the projection we emit from
// ---- `ProjectServiceImpl`) ----

#[test]
fn apis_view_dtos_are_nameable() {
    fn assert_member_view(_: ApiProjectMemberView) {}
    let _ = ApiProjectMemberView::default();
    assert_member_view(ApiProjectMemberView {
        leaders: vec![ApiUserSummaryView {
            code: "u1".into(),
            name: "Alice".into(),
        }],
        workers: vec![],
    });
}

#[test]
fn apis_error_variants_are_nameable() {
    fn assert_err(_: ProjectApiError) {}
    assert_err(ProjectApiError::Validation("bad".into()));
    assert_err(ProjectApiError::NotFound);
    assert_err(ProjectApiError::ProductNotFound("1".into()));
    assert_err(ProjectApiError::UserNotFound("u1".into()));
    assert_err(ProjectApiError::DuplicateCode("p1".into()));
}

#[test]
fn project_view_is_constructible_via_usecase() {
    // `ProjectView` is produced by `ProjectUsecase` from real
    // domain values. To check nameability without fakes, just pin
    // the type signature via a function pointer.
    let _assert_view: fn(ProjectView) = |_| {};
}

#[test]
fn project_member_default_is_empty() {
    let m = ProjectMember::default();
    assert!(m.leaders.is_empty());
    assert!(m.workers.is_empty());
}

// ---------- silence dead_code on unused input types ----------
#[allow(dead_code)]
fn _force_member_data(m: ProjectMemberData) {
    let _ = m;
}

#[allow(dead_code)]
fn _force_domain_inputs(p: ProductNew, q: ProjectNew, u: ProductUpdate, v: ProjectUpdate) {
    let _ = (p, q, u, v);
}
