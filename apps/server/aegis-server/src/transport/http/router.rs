//! Top-level HTTP router.
//!
//! Layout:
//! - `/api/auth/login`                  auth flows
//! - `/api/auth/login-domain`
//! - `/api/auth/refresh`
//! - `/api/auth/logout`
//! - `/api/auth/user-credential`        user-credential (self-service: PATCH only — creation is out of band)
//! - `/api/user`                         user CRUD
//! - `/api/user/{code}`
//! - `/healthz`                         liveness probe
//! - `/swagger-ui/`                      swagger-ui HTML
//! - `/swagger-ui/{*rest}`               swagger-ui assets
//! - `/api-docs/openapi.json`            OpenAPI v3 JSON document
//!
//! All `/api/*` routes are also wrapped in a `TraceLayer` so every
//! request emits a tracing span.

use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;
use crate::transport::http::auth;
use crate::transport::http::healthz;
use crate::transport::http::openapi::ApiDoc;
use crate::transport::http::project::router as project_router;
use crate::transport::http::user;

/// Build the full HTTP router with `state` attached.
///
/// The return type is `axum::Router` (the consumer-side type after
/// `split_for_parts()`). The `OpenApiRouter` is composed internally
/// so the per-handler `#[utoipa::path]` annotations are auto-
/// collected into the OpenAPI document; the swagger-ui is then
/// merged on top, and the resulting `Router` is wrapped in a
/// `TraceLayer` for tracing.
pub fn router(state: AppState) -> axum::Router {
    let project_routes = project_router::router();
    let api_routers = OpenApiRouter::new()
        .nest("/auth", auth::router())
        .nest("/user", user::router())
        .nest("/project", project_routes);

    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api", api_routers)
        .nest("/healthz", healthz::router())
        .with_state(state)
        .split_for_parts();

    // `SwaggerUi::new("/swagger-ui").url(...)` returns `SwaggerUi`,
    // which has `From<SwaggerUi> for Router<S>` so we can convert
    // it into a `Router<()>`. `OpenApiRouter::merge` then takes
    // any `Router<S>` (the swagger-ui uses `S = ()` because its
    // handlers do not extract state).
    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    use apis::auth::{
        AuthApiError, AuthClaims as ApiAuthClaims, AuthService, CreateUserCredentialRequest,
        LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
        RefreshRequest, RefreshResponse, RegisterUserRequest, RegisterUserResponse,
        RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
        VerifyRequest,
    };

    #[derive(Clone)]
    struct MockAuth;

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            Ok(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            })
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            Ok(TokenPair {
                access_token: "ACCESS".into(),
                refresh_token: "REFRESH".into(),
            })
        }
        async fn verify(&self, _req: VerifyRequest) -> Result<ApiAuthClaims, AuthApiError> {
            Ok(ApiAuthClaims {
                code: "u1".into(),
                role: apis::user::Role::Admin,
                token_version: 0,
            })
        }
        async fn refresh(&self, _req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
            Ok(RefreshResponse {
                access_token: "NEW".into(),
            })
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<UserCredentialView, AuthApiError> {
            Ok(UserCredentialView {
                user_code: "u1".into(),
                password_hash: "argon2id$v=19$m=...$...".into(),
                token_version: 0,
            })
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
            Ok(LogoutResponse::default())
        }
        async fn register_user(
            &self,
            req: RegisterUserRequest,
        ) -> Result<RegisterUserResponse, AuthApiError> {
            // Router-level integration tests don't exercise the
            // canonical registration flow — they only assert that the
            // route is mounted, the body is parsed, and the handler
            // reaches the `AuthService` port. Echo a synthetic 201
            // view built from the request body.
            Ok(RegisterUserResponse {
                user_code: req.user_code,
                user_name: req.user_name,
                role: apis::user::Role::General,
                active: false,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
            })
        }
    }

    #[derive(Clone)]
    struct NullProjectService;

    #[async_trait]
    impl apis::project::ProjectService for NullProjectService {
        async fn create_project(
            &self,
            _req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn update_project(
            &self,
            _req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
    }

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

    /// User mock for integration tests. Returns a fixed one-user
    /// list from `list()` and a fixed view from `get_by_code` /
    /// `create` / `update`; any other method panics. This is
    /// deliberately minimal — the per-handler tests in
    /// `user::handlers` cover the translation surface in detail.
    #[derive(Clone)]
    struct StubUserService;

    #[async_trait]
    impl apis::user::UserService for StubUserService {
        async fn create(
            &self,
            _req: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            Ok(sample_user_view(1, "u1"))
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
            Ok(sample_user_view(1, "u1"))
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            Ok(vec![sample_user_view(1, "u1")])
        }
        async fn update(
            &self,
            _req: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            Ok(sample_user_view(1, "u1"))
        }
    }

    fn sample_user_view(id: i32, code: &str) -> apis::user::UserView {
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

    fn sample_project_view(id: i32, code: &str) -> apis::project::ProjectView {
        apis::project::ProjectView {
            id,
            code: code.to_string(),
            description: "sample".to_string(),
            members: apis::project::ProjectMemberView::default(),
            unblind_members: apis::project::ProjectMemberView::default(),
            tags: vec![],
            active: true,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        }
    }

    fn test_state() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
        }
    }

    /// State builder for user-integration tests: same `MockAuth`
    /// (so verify returns OK), but a `StubUserService` that returns
    /// positive responses for the user routes.
    fn test_state_with_user() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(StubUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
        }
    }

    /// State builder for project-integration tests: a `StubProjectService`
    /// that returns positive responses for the project routes.
    fn test_state_with_project() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(StubProjectService) as Arc<dyn apis::project::ProjectService>,
        }
    }

    #[derive(Clone)]
    struct StubProjectService;

    #[async_trait]
    impl apis::project::ProjectService for StubProjectService {
        async fn create_project(
            &self,
            _req: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Ok(sample_project_view(1, "pr1"))
        }
        async fn get_project_by_id(
            &self,
            _id: i32,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Ok(sample_project_view(1, "pr1"))
        }
        async fn list_projects(
            &self,
        ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
            Ok(vec![sample_project_view(1, "pr1")])
        }
        async fn update_project(
            &self,
            _req: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            Ok(sample_project_view(1, "pr1"))
        }
    }

    #[tokio::test]
    async fn healthz_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/healthz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn login_route_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"u1","password":"p"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
    }

    #[tokio::test]
    async fn unknown_route_returns_404() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/auth/does-not-exist")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::NOT_FOUND);
    }

    #[tokio::test]
    async fn swagger_ui_root_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/swagger-ui/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // Sibling redirects / assets may return 200 or 304; the
        // important assertion is that the route is registered.
        assert!(
            response.status().is_success() || response.status().is_redirection(),
            "expected 2xx or 3xx, got {}",
            response.status(),
        );
    }

    #[tokio::test]
    async fn openapi_json_returns_200_with_valid_doc() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 64 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(doc["info"]["title"], "aegis-server API");
        assert!(doc["paths"]["/api/auth/login"].is_object());
        assert!(doc["paths"]["/api/auth/login-domain"].is_object());
        assert!(doc["paths"]["/api/auth/refresh"].is_object());
        assert!(doc["paths"]["/api/auth/logout"].is_object());
        assert!(doc["paths"]["/api/auth/user-credential"].is_object());
        assert!(doc["paths"]["/healthz"].is_object());

        // The Bearer security scheme must be registered so
        // refresh / logout advertise the BearerAuth requirement.
        let schemes = &doc["components"]["securitySchemes"];
        assert!(
            schemes["BearerAuth"].is_object(),
            "BearerAuth scheme missing"
        );
        assert_eq!(schemes["BearerAuth"]["type"], "http");
        assert_eq!(schemes["BearerAuth"]["scheme"], "bearer");
        assert_eq!(schemes["BearerAuth"]["bearerFormat"], "JWT");

        // refresh + logout must reference the security scheme;
        // login + login-domain + healthz must not.
        let refresh = &doc["paths"]["/api/auth/refresh"]["post"];
        assert_eq!(refresh["security"][0]["BearerAuth"], serde_json::json!([]));
        let logout = &doc["paths"]["/api/auth/logout"]["post"];
        assert_eq!(logout["security"][0]["BearerAuth"], serde_json::json!([]));
        assert!(doc["paths"]["/api/auth/login"]["post"]["security"].is_null());
        assert!(doc["paths"]["/api/auth/login-domain"]["post"]["security"].is_null());
        assert!(doc["paths"]["/healthz"]["get"]["security"].is_null());

        // /api/user namespace must advertise every CRUD verb with
        // the BearerAuth requirement — the gate is the whole point
        // of the router.
        for (method, path) in [
            ("post", "/api/user"),
            ("get", "/api/user"),
            ("get", "/api/user/{code}"),
            ("patch", "/api/user/{code}"),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }

        // /api/project namespace must advertise every verb with the
        // BearerAuth requirement. Write operations (POST/PATCH) must
        // additionally advertise a 403 response, because non-admin
        // callers are rejected at the role guard.
        for (method, path) in [
            ("post", "/api/project"),
            ("get", "/api/project"),
            ("get", "/api/project/{code}"),
            ("patch", "/api/project/{code}"),
        ] {
            let op = &doc["paths"][path][method];
            assert!(op.is_object(), "missing {method} {path} in openapi");
            assert_eq!(
                op["security"][0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must require BearerAuth",
            );
        }
        for (method, path) in [("post", "/api/project"), ("patch", "/api/project/{code}")] {
            let op = &doc["paths"][path][method];
            let response_keys: Vec<&str> = op["responses"]
                .as_object()
                .expect("responses object")
                .keys()
                .map(|s| s.as_str())
                .collect();
            assert!(
                response_keys.contains(&"403"),
                "{method} {path} must advertise a 403 response (got {response_keys:?})",
            );
        }

        // /api/auth/user-credential namespace advertises two verbs:
        // `PATCH` for self-service rotation and `POST` for the
        // administrator-only registration route. Both sit under
        // BearerAuth. `user_code` for PATCH is derived from the
        // token, so there is no `/{code}` path; the POST body
        // carries `user_code` explicitly.
        let patch_op = &doc["paths"]["/api/auth/user-credential"]["patch"];
        assert!(
            patch_op.is_object(),
            "missing patch /api/auth/user-credential in openapi"
        );
        assert_eq!(
            patch_op["security"][0]["BearerAuth"],
            serde_json::json!([]),
            "patch /api/auth/user-credential must require BearerAuth",
        );
        let post_op = &doc["paths"]["/api/auth/user-credential"]["post"];
        assert!(
            post_op.is_object(),
            "POST /api/auth/user-credential must be advertised for the \
             administrator registration route"
        );
        assert_eq!(
            post_op["security"][0]["BearerAuth"],
            serde_json::json!([]),
            "POST /api/auth/user-credential must require BearerAuth",
        );
    }

    // ---- /api/user integration --------------------------------------

    /// `GET /api/user` round-trips through the top-level router:
    /// `AuthClaims` verifies the bearer, the user router hands off
    /// to `StubUserService.list()`, and the projected body comes
    /// back as 200 OK with the expected shape.
    #[tokio::test]
    async fn user_list_route_is_wired() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let users = value["users"].as_array().expect("users array");
        assert_eq!(users.len(), 1);
        assert_eq!(users[0]["code"], "u1");
        assert_eq!(users[0]["role"], "admin");
    }

    /// `GET /api/user/{code}` round-trips through the top-level
    /// router: 200 OK with the projected body.
    #[tokio::test]
    async fn user_get_by_code_route_is_wired() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user/u1")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "u1");
    }

    /// No bearer at all: the top-level router must reject every
    /// `/api/user/*` route with 401, regardless of HTTP method.
    /// Sample one representative method (GET) — the AuthClaims
    /// extractor gates all four user routes uniformly.
    #[tokio::test]
    async fn user_route_without_authorization_returns_401() {
        let app = router(test_state_with_user());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/user")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "token_verification_failed");
    }

    // ---- /api/project integration ------------------------------------

    #[tokio::test]
    async fn project_list_route_is_wired() {
        let app = router(test_state_with_project());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let projects = value["projects"].as_array().expect("projects array");
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0]["code"], "pr1");
    }

    #[tokio::test]
    async fn project_route_without_authorization_returns_401() {
        let app = router(test_state_with_project());
        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }

    // ---- /api/auth/user-credential integration ----------------------

    /// `POST /api/auth/user-credential` IS a supported method — the
    /// administrator-only registration route. The router must register
    /// it alongside the existing `PATCH` handler. Because the canonical
    /// backend would dispatch this to the auth usecase, the router-only
    /// fixture (which has no `register_user` behaviour) surfaces that
    /// request as a `500` once it reaches the handler. The negative
    /// shape (`405`) is the assertion that the path is registered —
    /// `unimplemented!()` blows up inside the handler rather than the
    /// router.
    #[tokio::test]
    async fn user_credential_create_route_is_registered() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .header("authorization", "Bearer good")
                    .body(Body::from(
                        r#"{"user_code":"u2","user_name":"Bob","domain_name":"aegis.local","hostname":"h","sid":"s","password":"p"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        // The path is registered for POST (alongside PATCH); the
        // router-only MockAuth's `register_user` returns
        // `unimplemented!()`, so we expect a 5xx once the request
        // reaches the handler. The important negative is `405` (the
        // path is NOT unregistered) and `404` (the path IS mounted).
        assert_ne!(
            response.status(),
            AxStatus::METHOD_NOT_ALLOWED,
            "POST should be registered alongside PATCH"
        );
        assert_ne!(
            response.status(),
            AxStatus::NOT_FOUND,
            "POST path must be mounted on the router"
        );
    }

    /// No bearer at all: the top-level router must reject the
    /// `/api/auth/user-credential` PATCH route with 401. (POST is now
    /// also mounted for the administrator registration endpoint; its
    /// 401 path is covered by `user_credential_create_route_without_authorization_returns_401`.)
    #[tokio::test]
    async fn user_credential_route_without_authorization_returns_401() {
        let app = router(test_state());

        let response = app
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["code"], "token_verification_failed");
    }

    /// POST shares the same bearer-auth gate as PATCH. Without a
    /// token the router must reject the request with 401 — the
    /// admin/root gate lives in the handler, but the AuthClaims
    /// extractor fires first.
    #[tokio::test]
    async fn user_credential_create_route_without_authorization_returns_401() {
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/auth/user-credential")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"user_code":"u2","user_name":"Bob","domain_name":"aegis.local","hostname":"h","sid":"s","password":"p"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }
}
