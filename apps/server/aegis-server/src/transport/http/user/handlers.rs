//! HTTP handlers for the user CRUD namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from [`dto`](crate::transport::http::dto))
//!    into an apis DTO.
//! 2. Calls the corresponding [`apis::user::UserService`] method on
//!    [`AppState`](crate::state::AppState).
//! 3. Translates the apis response back into a wire DTO.
//!
//! `UserApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in [`error`](crate::transport::http::error) does the rest.
//!
//! The `AuthClaims` extractor in the argument list is what gates
//! the route on a valid `Authorization: Bearer <token>` header. The
//! handler bodies ignore the claims value (no role checks at this
//! stage).

// `UserService` is in scope so the future handler bodies (Tasks 5-8)
// can call `state.user.X(...)`; the stub bodies don't use it yet.
#[allow(unused_imports)]
use apis::user::UserService;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{self, PathCode};
use crate::transport::http::error::ApiError;

// Stubs: Tasks 5-8 replace these bodies with real implementations.
// They exist now so the router (Task 4) compiles end-to-end. Each
// stub carries a minimal `#[utoipa::path]` annotation so the
// `routes!` macro can register it; Tasks 5-8 expand the annotation
// with the right method, path, and request/response schemas.

/// `POST /api/user` — create a user.
#[utoipa::path(post, path = "/", tag = "user")]
pub async fn create(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Json(_req): Json<dto::CreateUserRequest>,
) -> Result<(StatusCode, Json<dto::UserViewResponse>), ApiError> {
    unimplemented!("populated in Task 5")
}

/// `GET /api/user` — list users.
#[utoipa::path(get, path = "/", tag = "user")]
pub async fn list(
    State(_state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::UserListResponse>, ApiError> {
    unimplemented!("populated in Task 6")
}

/// `GET /api/user/{code}` — fetch a user by code.
#[utoipa::path(get, path = "/{code}", tag = "user")]
pub async fn get_by_code(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { .. }): Path<PathCode>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    unimplemented!("populated in Task 7")
}

/// `PATCH /api/user/{code}` — update a user.
#[utoipa::path(patch, path = "/{code}", tag = "user")]
pub async fn update(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { .. }): Path<PathCode>,
    Json(_req): Json<dto::UpdateUserRequest>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    unimplemented!("populated in Task 8")
}

#[cfg(test)]
mod tests {
    //! Test scaffolding shared by the per-handler tests in Tasks 5-8.
    //!
    //! `MockUserService` and `MockAuth` are configurable fakes; each
    //! per-handler test sets the relevant fields and asserts the
    //! response status / body / error. `NullAuth` and `NullUserService`
    //! exist for the auth module's existing tests and are not used
    //! here.

    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::{get, patch, post};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest,
        LogoutResponse, RefreshRequest, RefreshResponse, RemoveUserCredentialResponse,
        TokenPair, UpdateUserCredentialRequest, UserCredentialView, VerifyRequest,
    };

    /// Configurable mock for [`apis::user::UserService`]. Each method
    /// returns either a preconfigured success value or a preconfigured
    /// error; the `*_args` fields record the last request the method
    /// received so handler-translation tests can assert on it.
    #[derive(Clone, Default)]
    pub struct MockUserService {
        pub create: Option<apis::user::UserView>,
        pub create_err: Option<apis::user::UserApiError>,
        pub get_by_code: Option<apis::user::UserView>,
        pub get_by_code_err: Option<apis::user::UserApiError>,
        pub list: Option<Vec<apis::user::UserView>>,
        pub list_err: Option<apis::user::UserApiError>,
        pub update: Option<apis::user::UserView>,
        pub update_err: Option<apis::user::UserApiError>,

        pub last_create_args: Arc<Mutex<Option<apis::user::CreateUserRequest>>>,
        pub last_update_args: Arc<Mutex<Option<apis::user::UpdateUserRequest>>>,
    }

    #[async_trait]
    impl apis::user::UserService for MockUserService {
        async fn create(
            &self,
            req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            *self.last_create_args.lock().unwrap() = Some(req);
            if let Some(err) = self.create_err.clone() {
                return Err(err);
            }
            Ok(self.create.clone().expect("create result configured"))
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!("not exercised by the user router")
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            if let Some(err) = self.get_by_code_err.clone() {
                return Err(err);
            }
            Ok(self.get_by_code.clone().expect("get_by_code result configured"))
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            if let Some(err) = self.list_err.clone() {
                return Err(err);
            }
            Ok(self.list.clone().expect("list result configured"))
        }
        async fn update(
            &self,
            req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            *self.last_update_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_err.clone() {
                return Err(err);
            }
            Ok(self.update.clone().expect("update result configured"))
        }
    }

    /// Configurable mock for [`apis::auth::AuthService`]. Only the
    /// `verify` method is exercised (by `AuthClaims::from_request_parts`);
    /// the rest are stubbed with `unimplemented!()`.
    #[derive(Clone, Default)]
    pub struct MockAuth {
        pub verify_ok: bool,
        pub verify_err: Option<AuthApiError>,
    }

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            unimplemented!()
        }
        async fn verify(&self, _req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
            if let Some(err) = self.verify_err.clone() {
                return Err(err);
            }
            assert!(self.verify_ok, "verify_ok must be set when no error is configured");
            Ok(AuthClaims {
                code: "u1".into(),
                role: apis::user::Role::Admin,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            unimplemented!()
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn create_user_credential(
            &self,
            _req: CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn update_user_credential(
            &self,
            _req: UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> {
            unimplemented!()
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
            unimplemented!()
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            unimplemented!()
        }
    }

    /// Build an `AppState` from the supplied mocks.
    pub fn test_state(user: MockUserService, auth: MockAuth) -> AppState {
        AppState {
            auth: Arc::new(auth) as Arc<dyn AuthService>,
            user: Arc::new(user) as Arc<dyn apis::user::UserService>,
        }
    }

    /// Build a router that mounts all four handlers at the
    /// `/api/user` prefix (matching the production nest path). Each
    /// per-handler test in Tasks 5-8 drives this router via
    /// `tower::ServiceExt::oneshot`.
    pub fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/user", post(create).get(list))
            .route("/api/user/{code}", get(get_by_code).patch(update))
            .with_state(state)
    }

    /// Decode a response into (status, JSON body) for assertions.
    pub async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096).await.unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    /// Construct a sample `UserView` for test fixtures.
    pub fn sample_user(id: i32, code: &str) -> apis::user::UserView {
        apis::user::UserView {
            id,
            code: code.to_string(),
            name: format!("User {code}"),
            role: apis::user::Role::Admin,
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    /// Drive a request with the supplied Authorization header value
    /// (or `None` to omit it). Body, when present, is owned by the
    /// caller so the resulting `Request<Body>` is `'static`.
    pub fn build_request(
        method: &str,
        uri: &str,
        body: Option<String>,
        auth: Option<&str>,
    ) -> Request<Body> {
        let mut b = Request::builder().method(method).uri(uri);
        if let Some(token) = auth {
            b = b.header("authorization", token);
        }
        if body.is_some() {
            b = b.header("content-type", "application/json");
        }
        b.body(body.map(Body::from).unwrap_or(Body::empty())).unwrap()
    }
}