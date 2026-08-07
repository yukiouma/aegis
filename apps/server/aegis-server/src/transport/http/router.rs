//! Top-level HTTP router.
//!
//! Layout:
//! - `/healthz`                              liveness probe
//! - `/api/auth/login`                       auth flows
//! - `/api/auth/login-domain`
//! - `/api/auth/refresh`
//! - `/api/auth/logout`
//! - `/swagger-ui/`                          swagger-ui HTML
//! - `/swagger-ui/{*rest}`                   swagger-ui assets
//! - `/api-docs/openapi.json`                OpenAPI v3 JSON document
//!
//! All `/api/*` routes are also wrapped in a `TraceLayer` so every
//! request emits a tracing span.

use axum::Router;
use axum::routing::{get, post};
use tower_http::trace::TraceLayer;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;
use crate::transport::http::auth::handlers;
use crate::transport::http::healthz::healthz;
use crate::transport::http::openapi;

/// Build the full HTTP router with `state` attached.
///
/// The return type is intentionally `Router<()>` — once `with_state`
/// has consumed the [`AppState`], the router no longer needs
/// anything else added before it can run (`Router<()>` is what
/// implements `Service`, not `Router<AppState>`).
pub fn router(state: AppState) -> Router {
    // The OpenApi document is built up by `openapi::openapi()` (which
    // calls `ApiDoc::openapi()` and records each handler's
    // `#[utoipa::path]` annotation). The schema registry + info block
    // are shared with the static copy; the route paths come from the
    // handler attribute macros.
    let api_router = Router::new()
        .route("/api/auth/login", post(handlers::login))
        .route("/api/auth/login-domain", post(handlers::login_domain))
        .route("/api/auth/refresh", post(handlers::refresh))
        .route("/api/auth/logout", post(handlers::logout))
        .route("/healthz", get(healthz))
        .with_state(state);

    // Swagger-ui mounts the HTML + assets at `/swagger-ui/` and the
    // openapi JSON at `/api-docs/openapi.json`. Its `Into<Router>`
    // impl is generic over the state type, so it infers `Router<()>`
    // from the surrounding `merge` call.
    let swagger: Router = Router::from(
        SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", openapi::openapi()),
    );

    api_router
        .merge(swagger)
        .layer(TraceLayer::new_for_http())
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
        RefreshRequest, RefreshResponse, RemoveUserCredentialResponse, TokenPair,
        UpdateUserCredentialRequest, UserCredentialView, VerifyRequest,
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
            Ok(LogoutResponse::default())
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

    fn test_state() -> AppState {
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
        }
    }

    #[tokio::test]
    async fn healthz_returns_200() {
        let app = router(test_state());
        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
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
    }
}