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

// -- apis::auth ---------------------------------------------------------

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
    TokenPair, VerifyRequest,
};

/// Every public type in `apis::auth` is nameable from the test.
#[test]
fn auth_public_types_are_nameable() {
    fn assert_pair(_: TokenPair) {}
    fn assert_claims(_: AuthClaims) {}
    fn assert_login_pw(_: LoginWithPasswordRequest) {}
    fn assert_login_domain(_: LoginWithDomainUserInfoRequest) {}
    fn assert_logout_req(_: LogoutRequest) {}
    fn assert_logout_res(_: LogoutResponse) {}
    fn assert_verify_req(_: VerifyRequest) {}
    fn assert_refresh_req(_: RefreshRequest) {}
    fn assert_refresh_res(_: RefreshResponse) {}
    fn assert_err(_: AuthApiError) {}

    // `TokenPair` is constructible field-by-field.
    assert_pair(TokenPair {
        access_token: "a".into(),
        refresh_token: "r".into(),
    });
    // `AuthClaims` is constructible field-by-field; `role` reuses
    // `apis::user::Role`.
    assert_claims(AuthClaims {
        code: "u1".into(),
        role: apis::user::Role::General,
        token_version: 0,
    });
    // Every request DTO owns its string field — that is the shape
    // adapters receive from outside the backend.
    assert_login_pw(LoginWithPasswordRequest {
        code: "u1".into(),
        password: "p".into(),
    });
    assert_login_domain(LoginWithDomainUserInfoRequest {
        code: "u1".into(),
        domain_name: "d".into(),
        hostname: "h".into(),
        sid: "s".into(),
    });
    assert_logout_req(LogoutRequest { code: "u1".into() });
    assert_verify_req(VerifyRequest {
        access_token: "a".into(),
    });
    assert_refresh_req(RefreshRequest {
        refresh_token: "r".into(),
    });
    // Every response DTO is constructible field-by-field.
    assert_logout_res(LogoutResponse { code: "u1".into() });
    assert_refresh_res(RefreshResponse {
        access_token: "a".into(),
    });

    // Touch every variant of the error type to keep it from being
    // dead-code-eliminated by the test build's analysis.
    let _: AuthApiError = AuthApiError::Validation("".into());
    let _: AuthApiError = AuthApiError::NotFound;
    let _: AuthApiError = AuthApiError::Inactive;
    let _: AuthApiError = AuthApiError::InvalidCredentials;
    let _: AuthApiError = AuthApiError::Signing("".into());
    let _: AuthApiError = AuthApiError::Verification("".into());
    let _: AuthApiError = AuthApiError::Repository("".into());
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's signature,
/// object-safety, and `Send + Sync` bounds. Each method returns
/// `todo!()` because the test only exercises the type system — never
/// the runtime behavior.
struct FakeAuthService;

#[async_trait::async_trait]
impl AuthService for FakeAuthService {
    async fn login_with_password(
        &self,
        _req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn login_with_domain_user_info(
        &self,
        _req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn logout(
        &self,
        _req: LogoutRequest,
    ) -> Result<LogoutResponse, AuthApiError> {
        todo!()
    }
    async fn verify(
        &self,
        _req: VerifyRequest,
    ) -> Result<AuthClaims, AuthApiError> {
        todo!()
    }
    async fn refresh(
        &self,
        _req: RefreshRequest,
    ) -> Result<RefreshResponse, AuthApiError> {
        todo!()
    }
}

/// `AuthService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn auth_service_is_object_safe() {
    let _boxed: Box<dyn AuthService> = Box::new(FakeAuthService);
}

/// `AuthService` requires `Send + Sync`, so a `Box<dyn AuthService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn auth_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn AuthService>>();
    assert_send_sync::<&FakeAuthService>();
}