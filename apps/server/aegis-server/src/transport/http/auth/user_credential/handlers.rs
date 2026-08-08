//! HTTP handlers for the user-credential management namespace.
//!
//! Each handler is a thin adapter that:
//! 1. Translates the wire DTO (from [`crate::transport::http::dto`])
//!    into an apis DTO.
//! 2. Calls the corresponding [`apis::auth::AuthService`] method on
//!    [`crate::state::AppState`].
//! 3. Translates the apis response back into a wire DTO.
//!
//! `AuthApiError` is funnelled through [`ApiError::from`] so each
//! route returns `Result<Json<T>, ApiError>` and the error mapping
//! in [`crate::transport::http::error`] does the rest.
//!
//! The `AuthClaims` extractor in the argument list gates the route on
//! a valid `Authorization: Bearer <token>` header. Handler bodies
//! ignore the claims value today — role-gated admin operations
//! (e.g. only `Root` may create / delete) are out of scope for this
//! first slice.

// `RemoveUserCredentialResponse` is used only by the test mock
// (`#[cfg(test)]`); the real `remove` handler returns the wire DTO
// directly rather than translating from the apis response.
#[allow(unused_imports)]
use apis::auth::{
    CreateUserCredentialRequest, RemoveUserCredentialResponse, UpdateUserCredentialRequest,
};
use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto;
use crate::transport::http::error::ApiError;

// Stubs: GREEN step replaces these bodies with real implementations.
// They exist now so the test `app(...)` references compile and each
// per-handler test fails with a clear panic (RED), pointing straight
// at the missing implementation. The `#[utoipa::path]` annotations
// match the GREEN implementation's path layout.

#[utoipa::path(
    post, path = "/", tag = "user-credential",
    request_body = dto::CreateUserCredentialRequest,
    responses(
        (status = 201, description = "User credential created", body = dto::UserCredentialViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "User code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateUserCredentialRequest>,
) -> Result<(StatusCode, Json<dto::UserCredentialViewResponse>), ApiError> {
    let user_code = claims.0.code.clone();
    let view = state
        .auth
        .create_user_credential(CreateUserCredentialRequest {
            user_code,
            password_hash: req.password_hash,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

#[utoipa::path(
    patch, path = "/", tag = "user-credential",
    request_body = dto::UpdateUserCredentialRequest,
    responses(
        (status = 200, description = "User credential updated", body = dto::UserCredentialViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User credential not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::UpdateUserCredentialRequest>,
) -> Result<Json<dto::UserCredentialViewResponse>, ApiError> {
    let user_code = claims.0.code.clone();
    let view = state
        .auth
        .update_user_credential(UpdateUserCredentialRequest {
            user_code,
            password_hash: req.password_hash,
        })
        .await?;
    Ok(Json(view.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use axum::routing::{patch, post};
    use std::sync::{Arc, Mutex};
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
        LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
        TokenPair, VerifyRequest,
    };

    /// Configurable mock for the four credential methods. Each
    /// method stores the success variant and the failure variant
    /// separately; the `*_args` fields capture the last request each
    /// method received so handler-translation tests can assert on
    /// them. `verify` is exercised by the `AuthClaims` extractor.
    #[derive(Clone, Default)]
    struct MockAuth {
        find_by_code: Option<apis::auth::UserCredentialView>,
        find_by_code_err: Option<AuthApiError>,
        create: Option<apis::auth::UserCredentialView>,
        create_err: Option<AuthApiError>,
        update: Option<apis::auth::UserCredentialView>,
        update_err: Option<AuthApiError>,
        remove: bool,
        remove_err: Option<AuthApiError>,
        verify_ok: bool,
        verify_err: Option<AuthApiError>,

        last_find_code: Arc<Mutex<Option<String>>>,
        last_create_args: Arc<Mutex<Option<CreateUserCredentialRequest>>>,
        last_update_args: Arc<Mutex<Option<UpdateUserCredentialRequest>>>,
        last_remove_code: Arc<Mutex<Option<String>>>,
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
            assert!(
                self.verify_ok,
                "verify_ok must be set when no error is configured"
            );
            Ok(AuthClaims {
                code: "admin".into(),
                role: apis::user::Role::Admin,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            unimplemented!()
        }
        async fn find_user_credential_by_code(
            &self,
            code: &str,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            *self.last_find_code.lock().unwrap() = Some(code.to_string());
            if let Some(err) = self.find_by_code_err.clone() {
                return Err(err);
            }
            Ok(self
                .find_by_code
                .clone()
                .expect("find_by_code result configured"))
        }
        async fn create_user_credential(
            &self,
            req: CreateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            *self.last_create_args.lock().unwrap() = Some(req);
            if let Some(err) = self.create_err.clone() {
                return Err(err);
            }
            Ok(self.create.clone().expect("create result configured"))
        }
        async fn update_user_credential(
            &self,
            req: UpdateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            *self.last_update_args.lock().unwrap() = Some(req);
            if let Some(err) = self.update_err.clone() {
                return Err(err);
            }
            Ok(self.update.clone().expect("update result configured"))
        }
        async fn remove_user_credential(
            &self,
            code: &str,
        ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
            *self.last_remove_code.lock().unwrap() = Some(code.to_string());
            if let Some(err) = self.remove_err.clone() {
                return Err(err);
            }
            assert!(
                self.remove,
                "remove_ok must be set when no error is configured"
            );
            Ok(RemoveUserCredentialResponse::default())
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            unimplemented!()
        }
    }

    /// `UserService` lives on `AppState` but the credential handlers
    /// never use it. The trait is non-trivial (real impls need a
    /// `PgPool`) so this stub keeps the handler tests free of any
    /// user-service surface.
    #[derive(Clone)]
    struct NullUserService;

    #[async_trait]
    impl apis::user::UserService for NullUserService {
        async fn create(
            &self,
            _req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn get_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            unimplemented!()
        }
        async fn update(
            &self,
            _req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unimplemented!()
        }
    }

    fn test_state(mock: MockAuth) -> AppState {
        AppState {
            auth: Arc::new(mock) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
        }
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/api/auth/user-credential", post(create))
            .route("/api/auth/user-credential/{code}", patch(update))
            .with_state(state)
    }

    async fn read_json(response: axum::response::Response) -> (AxStatus, serde_json::Value) {
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
        (status, value)
    }

    fn build_request(
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
        b.body(body.map(Body::from).unwrap_or(Body::empty()))
            .unwrap()
    }

    fn sample_credential(code: &str, token_version: u32) -> apis::auth::UserCredentialView {
        apis::auth::UserCredentialView {
            user_code: code.into(),
            password_hash: "argon2id$v=19$m=...$...".into(),
            token_version,
        }
    }

    fn empty_token() -> &'static str {
        "Bearer good"
    }

    // ---- create ----------------------------------------------------

    #[tokio::test]
    async fn create_returns_201_with_view_on_success() {
        let mock = MockAuth {
            verify_ok: true,
            create: Some(sample_credential("u1", 0)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(r#"{"user_code":"u1","password_hash":"argon2id$..."}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["user_code"], "u1");
        assert_eq!(body["password_hash"], "argon2id$v=19$m=...$...");
        assert_eq!(body["token_version"], 0);

        // Verify the wire->apis translation captured both fields.
        let captured = mock.last_create_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert_eq!(captured.password_hash, "argon2id$...");
    }

    #[tokio::test]
    async fn create_maps_duplicate_code_to_409() {
        let mock = MockAuth {
            verify_ok: true,
            create_err: Some(AuthApiError::DuplicateCode("u1".into())),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(r#"{"user_code":"u1","password_hash":"argon2id$..."}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CONFLICT);
        assert_eq!(body["code"], "duplicate_code");
    }

    #[tokio::test]
    async fn create_maps_repository_to_500() {
        let mock = MockAuth {
            verify_ok: true,
            create_err: Some(AuthApiError::Repository("db down".into())),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(r#"{"user_code":"u1","password_hash":"argon2id$..."}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn create_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/auth/user-credential",
                Some(r#"{"user_code":"u1","password_hash":"argon2id$..."}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- find_by_code ----------------------------------------------

    #[tokio::test]
    async fn find_by_code_returns_200_with_view_on_success() {
        let mock = MockAuth {
            verify_ok: true,
            find_by_code: Some(sample_credential("u1", 7)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/auth/user-credential/u1",
                None,
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["user_code"], "u1");
        assert_eq!(body["token_version"], 7);

        // Verify the URL `{code}` made it through.
        assert_eq!(
            mock.last_find_code.lock().unwrap().clone().as_deref(),
            Some("u1"),
        );
    }

    #[tokio::test]
    async fn find_by_code_maps_not_found_to_404() {
        let mock = MockAuth {
            verify_ok: true,
            find_by_code_err: Some(AuthApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/auth/user-credential/missing",
                None,
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn find_by_code_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "GET",
                "/api/auth/user-credential/u1",
                None,
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- update ----------------------------------------------------

    #[tokio::test]
    async fn update_returns_200_with_view_on_success() {
        let mock = MockAuth {
            verify_ok: true,
            update: Some(sample_credential("u1", 7)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential/u1",
                Some(r#"{"password_hash":"argon2id$new"}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["user_code"], "u1");
        assert_eq!(body["password_hash"], "argon2id$v=19$m=...$...");
        assert_eq!(body["token_version"], 7);

        // The handler must thread the URL `{code}` into the apis
        // DTO's `user_code` even though the body omits it.
        let captured = mock.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert_eq!(captured.password_hash.as_deref(), Some("argon2id$new"));
    }

    #[tokio::test]
    async fn update_with_empty_body_returns_unchanged_view() {
        // The apis trait permits a no-op update — body `{}` with
        // URL `{code}` must succeed and return the existing view.
        let mock = MockAuth {
            verify_ok: true,
            update: Some(sample_credential("u1", 3)),
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential/u1",
                Some("{}".to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["token_version"], 3);

        let captured = mock.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.user_code, "u1");
        assert!(captured.password_hash.is_none());
    }

    #[tokio::test]
    async fn update_maps_not_found_to_404() {
        let mock = MockAuth {
            verify_ok: true,
            update_err: Some(AuthApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential/missing",
                Some(r#"{"password_hash":"x"}"#.to_string()),
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn update_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/auth/user-credential/u1",
                Some(r#"{"password_hash":"x"}"#.to_string()),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }

    // ---- remove ----------------------------------------------------

    #[tokio::test]
    async fn remove_returns_200_with_empty_object_on_success() {
        let mock = MockAuth {
            verify_ok: true,
            remove: true,
            ..Default::default()
        };
        let app = app(test_state(mock.clone()));
        let response = app
            .oneshot(build_request(
                "DELETE",
                "/api/auth/user-credential/u1",
                None,
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body, serde_json::json!({}));

        // The handler must thread the URL `{code}` into
        // `remove_user_credential`.
        assert_eq!(
            mock.last_remove_code.lock().unwrap().clone().as_deref(),
            Some("u1"),
        );
    }

    #[tokio::test]
    async fn remove_maps_not_found_to_404() {
        let mock = MockAuth {
            verify_ok: true,
            remove_err: Some(AuthApiError::NotFound),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "DELETE",
                "/api/auth/user-credential/missing",
                None,
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn remove_maps_repository_to_500() {
        let mock = MockAuth {
            verify_ok: true,
            remove: true,
            remove_err: Some(AuthApiError::Repository("db down".into())),
            ..Default::default()
        };
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "DELETE",
                "/api/auth/user-credential/u1",
                None,
                Some(empty_token()),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn remove_without_authorization_returns_401() {
        let mock = MockAuth::default();
        let app = app(test_state(mock));
        let response = app
            .oneshot(build_request(
                "DELETE",
                "/api/auth/user-credential/u1",
                None,
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
}
