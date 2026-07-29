//! Public-API compile test for the `user` crate.
//!
//! This test does NOT connect to PostgreSQL. Its only job is to
//! type-check the documented public surface and the constructor
//! dependency chain `UserRepo::new(pool) -> UserUsecase::new(repo)`.
//!
//! We want the test to type-check the dependency chain without
//! performing I/O, so we hold `UserRepo::new` and `UserUsecase::new`
//! as function pointers. The `fn(PgPool) -> _` / `fn(R) -> _` shape
//! is the most explicit way to assert that the documented constructor
//! signatures are stable, and avoids ever opening a real connection.

use chrono::{TimeZone, Utc};
use user::{CreateUser, Role, UpdateUser, UserRepo, UserUsecase, UserView};

/// The crate-root imports compile. Touching the type tokens below is
/// enough to force a resolution error if any of the re-exports
/// regress.
#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_role(_: Role) {}
    fn assert_view(_: UserView) {}
    fn assert_create(_: CreateUser) {}
    fn assert_update(_: UpdateUser) {}

    // Inline `Role` ctor so we can build a value without importing
    // anything from `user::domain` directly.
    assert_role(Role::General);
    assert_view(UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap(),
        updated_at: Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap(),
    });
    assert_create(CreateUser {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        password: "hunter2".into(),
    });
    assert_update(UpdateUser {
        id: 1,
        ..Default::default()
    });
}

/// `UserRepo::new` accepts a `sqlx::PgPool` and returns a
/// `UserRepo`. We hold the constructor as a function pointer so the
/// test never actually opens a connection.
#[test]
fn user_repo_new_accepts_a_pg_pool() {
    let ctor: fn(sqlx::PgPool) -> UserRepo = UserRepo::new;
    // Materialise the function pointer in a no-op `let` binding so
    // it is not flagged as an unused-variable assignment. The
    // function pointer itself is not `#[must_use]`; the binding is
    // here purely to silence the unused-variable warning.
    let _ = ctor;
}

/// The dependency chain `UserRepo::new(pool) -> UserUsecase::new(repo)`
/// type-checks. `UserRepo` is `Send + Sync` (it carries only a
/// `PgPool`), so it satisfies the `UserRepository` port the usecase
/// is generic over.
#[test]
fn usecase_can_be_constructed_from_user_repo() {
    fn assert_repo_is_repository<R: user::UserRepository>() {}
    assert_repo_is_repository::<UserRepo>();

    fn assert_new_constructor<R: user::UserRepository>(_: fn(R) -> UserUsecase<R>) {}
    assert_new_constructor::<UserRepo>(UserUsecase::new);
}
