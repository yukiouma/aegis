//! Public-API compile test for the `apis` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and
//! the in-crate type names so a regression in `user.rs` is caught
//! at `cargo test -p apis` time.

use apis::user::{
    CreateUserRequest, Role, UpdateUserRequest, UserApiError, UserService, UserView,
};

/// Every public type in `apis::user` is nameable from the test.
#[test]
fn public_types_are_nameable() {
    fn assert_role(_: Role) {}
    fn assert_view(_: UserView) {}
    fn assert_create(_: CreateUserRequest) {}
    fn assert_update(_: UpdateUserRequest) {}
    fn assert_err(_: UserApiError) {}

    // `Role` is constructible from its variants.
    assert_role(Role::General);
    // `UserView` is constructible field-by-field.
    assert_view(UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    // `CreateUserRequest` has no `password` field — this is the
    // shape adapters receive from outside the backend.
    assert_create(CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
    });
    assert_update(UpdateUserRequest {
        id: 1,
        ..Default::default()
    });

    // Touch the error type to keep it from being dead-code-eliminated
    // by the test build's analysis.
    let _: UserApiError = UserApiError::NotFound;
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's
/// signature, object-safety, and `Send + Sync` bounds. Each method
/// returns `todo!()` because the test only exercises the type
/// system — never the runtime behavior.
struct FakeUserService;

#[async_trait::async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_code(&self, _code: &str) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        todo!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
}

/// `UserService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn user_service_is_object_safe() {
    let _boxed: Box<dyn UserService> = Box::new(FakeUserService);
}

/// `UserService` requires `Send + Sync`, so a `Box<dyn UserService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn user_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn UserService>>();
    assert_send_sync::<&FakeUserService>();
}