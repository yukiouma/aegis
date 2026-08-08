# aegis-server User Router Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the `apis::user::UserService` port over HTTP under `/api/user/*` inside `apps/server/aegis-server`. Four endpoints (create / list / get_by_code / update) require a valid `Authorization: Bearer <token>` header verified through the existing `AuthClaims` extractor. The `ApiError` type is refactored from a single-type newtype into an enum so auth and user handlers can share the same `?` path without losing exhaustiveness.

**Architecture:** New `transport/http/user/` sub-module mirrors the existing `auth/` shape (one handler file + one router file per namespace). The four handlers translate wire DTOs ↔ apis DTOs at the boundary, call `AppState.user`, and route failures through the new `ApiError` enum. `OpenApiRouter::nest("/api/user", ...)` is added to the top-level composition; utoipa auto-collects the per-handler `#[utoipa::path]` annotations into the existing OpenAPI document. Every user route advertises `security: [{ BearerAuth: [] }]`.

**Tech Stack:** Rust 2024, `axum 0.8`, `utoipa`, `utoipa-axum`, `utoipa-swagger-ui`, `serde`, `serde_json`, `chrono`, `thiserror`, `async-trait`, `apis` (path), `user` (path), `auth` (path).

**Spec:** [docs/superpowers/specs/2026-08-08-user-router-design.md](../specs/2026-08-08-user-router-design.md)

---

## Global Constraints

- `apps/server/aegis-server` uses `<module>.rs` + `<module>/` directory style — never `mod.rs`. The convention is locked at the workspace level by `docs/guidelines/lib-crate-development.md` § 2; the same rule applies to server crates.
- Every dependency in `apps/server/aegis-server/Cargo.toml` is either a workspace dep or a path-dep. No direct version pinning.
- Every `Result<_, ApiError>` return in handlers uses `?` on either `AuthApiError` or `UserApiError`; the new `From` impls on `ApiError` do the wrapping.
- The `AuthClaims` extractor (in `transport/http/auth/middleware.rs`) is reused as-is by every user handler. Its presence in the handler argument list is what gates the route; the handler body may ignore the value.
- `tower_http::trace::TraceLayer::new_for_http()` is the only middleware layer. There is no `from_fn_with_state` auth layer.
- State attaches exactly once via `Router::with_state(state)` on the top-level Router. Never on a sub-router.
- Commit messages follow the project's existing convention (`feat(aegis-server):`, `test(aegis-server):`, `docs(aegis-server):`, `chore(aegis-server):`).
- All non-DB tests run on a plain `cargo test -p aegis-server`. No new live-DB integration tests are added in this plan.
- Handler unit tests follow the existing per-file inline `#[cfg(test)] mod tests` pattern. `MockAuth` and `MockUserService` are duplicated per module rather than shared via `tests/common`.

---

## Task 1: Refactor `ApiError` to enum variants

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`

- [ ] **Step 1: Write the failing tests for the new `UserApiError` variants**

Open `apps/server/aegis-server/src/transport/http/error.rs`. Inside the existing `#[cfg(test)] mod tests { … }`, append these tests at the end of the module (before the closing `}`):

```rust
    #[tokio::test]
    async fn user_validation_maps_to_400() {
        let (status, body) = render_user(apis::user::UserApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn user_not_found_maps_to_404() {
        let (status, body) = render_user(apis::user::UserApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn user_duplicate_code_maps_to_409() {
        let (status, body) = render_user(apis::user::UserApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn user_hashing_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Hashing("oops".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "hashing_failed");
    }

    #[tokio::test]
    async fn user_repository_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    /// Drive `IntoResponse::into_response` for a `UserApiError` (after
    /// the new `From` impl exists) and recover the status + JSON body
    /// so each variant can be asserted directly.
    async fn render_user(err: apis::user::UserApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }
```

- [ ] **Step 2: Run the new tests to verify they fail to compile**

Run: `cargo test -p aegis-server --lib transport::http::error::tests::user_validation_maps_to_400`
Expected: COMPILE ERROR — `ApiError::from(apis::user::UserApiError)` does not exist yet. (`From<AuthApiError>` exists, but no `From<UserApiError>` yet.)

- [ ] **Step 3: Replace `ApiError` with an enum + add private helpers**

Replace the entire current body of `apps/server/aegis-server/src/transport/http/error.rs` with the following. Keep the existing `ErrorBody` struct unchanged.

```rust
//! HTTP error mapping.
//!
//! [`ApiError`] is an enum that wraps every apis error type the HTTP
//! layer surfaces today — [`apis::auth::AuthApiError`] and
//! [`apis::user::UserApiError`]. Every handler returns
//! `Result<Json<T>, ApiError>` and uses `?` on either inner error; the
//! [`From`] impls (derived via `#[from]`) do the wrapping.
//!
//! New apis services land as additional enum variants; the `status()`
//! and `code()` dispatch tables pick them up.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use utoipa::ToSchema;

/// Stable JSON error envelope returned to clients.
///
/// `code` is a machine-readable string (e.g. `invalid_credentials`)
/// that clients should switch on. `message` is human-readable and
/// may be surfaced in a UI.
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Error type returned by every HTTP handler.
///
/// Each variant wraps an apis-level error and implements
/// [`IntoResponse`] so handlers can return `Result<_, ApiError>` and
/// let the `?` operator do the conversion. The dispatch tables live
/// in private `*_status` / `*_code` helpers so the public `status()`
/// and `code()` methods stay table-shaped.
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(#[from] apis::auth::AuthApiError),

    #[error("{0}")]
    User(#[from] apis::user::UserApiError),
}

impl ApiError {
    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Auth(e) => auth_status(e),
            Self::User(e) => user_status(e),
        }
    }

    /// Stable machine-readable code used as `ErrorBody.code`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth(e) => auth_code(e),
            Self::User(e) => user_code(e),
        }
    }
}

fn auth_status(e: &apis::auth::AuthApiError) -> StatusCode {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => StatusCode::BAD_REQUEST,
        AuthApiError::NotFound => StatusCode::NOT_FOUND,
        AuthApiError::Inactive => StatusCode::FORBIDDEN,
        AuthApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
        AuthApiError::Verification(_) => StatusCode::UNAUTHORIZED,
        AuthApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        AuthApiError::Signing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        AuthApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn auth_code(e: &apis::auth::AuthApiError) -> &'static str {
    use apis::auth::AuthApiError;
    match e {
        AuthApiError::Validation(_) => "validation_failed",
        AuthApiError::NotFound => "not_found",
        AuthApiError::Inactive => "user_inactive",
        AuthApiError::InvalidCredentials => "invalid_credentials",
        AuthApiError::Verification(_) => "token_verification_failed",
        AuthApiError::DuplicateCode(_) => "duplicate_code",
        AuthApiError::Signing(_) => "signing_failed",
        AuthApiError::Repository(_) => "repository_error",
    }
}

fn user_status(e: &apis::user::UserApiError) -> StatusCode {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => StatusCode::BAD_REQUEST,
        UserApiError::NotFound => StatusCode::NOT_FOUND,
        UserApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        UserApiError::Hashing(_) => StatusCode::INTERNAL_SERVER_ERROR,
        UserApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn user_code(e: &apis::user::UserApiError) -> &'static str {
    use apis::user::UserApiError;
    match e {
        UserApiError::Validation(_) => "validation_failed",
        UserApiError::NotFound => "not_found",
        UserApiError::DuplicateCode(_) => "duplicate_code",
        UserApiError::Hashing(_) => "hashing_failed",
        UserApiError::Repository(_) => "repository_error",
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(
                code = self.code(),
                error = %self,
                "api error",
            );
        }
        let body = ErrorBody {
            code: self.code().to_string(),
            message: self.to_string(),
        };
        (status, Json(body)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `IntoResponse::into_response` and recover the status +
    /// JSON body so each `AuthApiError` variant can be asserted
    /// directly. The body bytes are re-parsed into `ErrorBody` for a
    /// structured comparison.
    async fn render(err: apis::auth::AuthApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    /// Drive `IntoResponse::into_response` for a `UserApiError` and
    /// recover the status + JSON body so each variant can be
    /// asserted directly.
    async fn render_user(err: apis::user::UserApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    // ---- AuthApiError mapping (unchanged from prior behaviour) -----

    #[tokio::test]
    async fn validation_maps_to_400() {
        let (status, body) = render(apis::auth::AuthApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let (status, body) = render(apis::auth::AuthApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn inactive_maps_to_403() {
        let (status, body) = render(apis::auth::AuthApiError::Inactive).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, "user_inactive");
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::InvalidCredentials).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "invalid_credentials");
    }

    #[tokio::test]
    async fn verification_maps_to_401() {
        let (status, body) = render(apis::auth::AuthApiError::Verification("bad sig".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn duplicate_code_maps_to_409() {
        let (status, body) = render(apis::auth::AuthApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn signing_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Signing("boom".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "signing_failed");
    }

    #[tokio::test]
    async fn repository_maps_to_500() {
        let (status, body) = render(apis::auth::AuthApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    #[test]
    fn from_auth_api_error_wraps() {
        let inner = apis::auth::AuthApiError::NotFound;
        let outer = ApiError::from(inner);
        // `AuthApiError` does not implement `PartialEq`, so assert
        // through the rendering path which is stable for these
        // unit-style variants.
        assert_eq!(outer.status(), StatusCode::NOT_FOUND);
        assert_eq!(outer.code(), "not_found");
    }

    // ---- UserApiError mapping (new) -----

    #[tokio::test]
    async fn user_validation_maps_to_400() {
        let (status, body) = render_user(apis::user::UserApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn user_not_found_maps_to_404() {
        let (status, body) = render_user(apis::user::UserApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn user_duplicate_code_maps_to_409() {
        let (status, body) = render_user(apis::user::UserApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn user_hashing_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Hashing("oops".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "hashing_failed");
    }

    #[tokio::test]
    async fn user_repository_maps_to_500() {
        let (status, body) = render_user(apis::user::UserApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }
}
```

- [ ] **Step 4: Verify the full test suite passes**

Run: `cargo test -p aegis-server --lib transport::http::error`
Expected: all `error::tests::*` tests pass — both the existing 9 `AuthApiError` tests and the 5 new `UserApiError` tests.

- [ ] **Step 5: Verify the broader crate still builds**

Run: `cargo test -p aegis-server --lib`
Expected: every test passes. The existing auth handlers continue to compile because `?` on `AuthApiError` now wraps into `ApiError::Auth(...)` via the `#[from]` impl.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/error.rs
git commit -m "refactor(aegis-server): make ApiError an enum over apis error types"
```

---

## Task 2: Add user wire DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

- [ ] **Step 1: Write the failing round-trip tests**

Open `apps/server/aegis-server/src/transport/http/dto.rs`. Inside the existing `#[cfg(test)] mod tests { … }`, append these tests at the end of the module (before the closing `}`):

```rust
    #[test]
    fn create_user_request_roundtrip() {
        let json = r#"{"code":"u1","name":"Alice","role":"admin"}"#;
        let req: CreateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.name, "Alice");
        assert!(matches!(req.role, Role::Admin));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn update_user_request_partial_roundtrip() {
        let json = r#"{"name":"Alice"}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert!(req.code.is_none());
        assert_eq!(req.name.as_deref(), Some("Alice"));
        assert!(req.role.is_none());
        assert!(req.active.is_none());
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn update_user_request_full_roundtrip() {
        let json = r#"{"code":"u2","name":"Bob","role":"root","active":true}"#;
        let req: UpdateUserRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code.as_deref(), Some("u2"));
        assert_eq!(req.name.as_deref(), Some("Bob"));
        assert!(matches!(req.role, Some(Role::Root)));
        assert_eq!(req.active, Some(true));
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn path_code_roundtrip() {
        let json = r#"{"code":"u1"}"#;
        let p: PathCode = serde_json::from_str(json).unwrap();
        assert_eq!(p.code, "u1");
        assert_eq!(serde_json::to_string(&p).unwrap(), json);
    }

    #[test]
    fn user_view_response_roundtrip() {
        let json = r#"{"id":42,"code":"u1","name":"Alice","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}"#;
        let v: UserViewResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.id, 42);
        assert_eq!(v.code, "u1");
        assert_eq!(v.name, "Alice");
        assert!(matches!(v.role, Role::Admin));
        assert!(v.active);
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_list_response_roundtrip() {
        let json = r#"{"users":[{"id":1,"code":"u1","name":"A","role":"admin","active":true,"created_at":"2026-01-02T03:04:05Z","updated_at":"2026-01-02T03:04:05Z"}]}"#;
        let v: UserListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(v.users.len(), 1);
        assert_eq!(v.users[0].code, "u1");
        assert_eq!(serde_json::to_string(&v).unwrap(), json);
    }

    #[test]
    fn user_view_response_from_apis_user_view() {
        let apis_view = apis::user::UserView {
            id: 7,
            code: "u7".into(),
            name: "Seven".into(),
            role: apis::user::Role::General,
            active: false,
            created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
            updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
                .unwrap()
                .with_timezone(&chrono::Utc),
        };
        let resp: UserViewResponse = apis_view.into();
        assert_eq!(resp.id, 7);
        assert_eq!(resp.code, "u7");
        assert_eq!(resp.name, "Seven");
        assert!(matches!(resp.role, Role::General));
        assert!(!resp.active);
    }
```

- [ ] **Step 2: Run the new tests to verify they fail to compile**

Run: `cargo test -p aegis-server --lib transport::http::dto::tests::create_user_request_roundtrip`
Expected: COMPILE ERROR — `CreateUserRequest`, `UpdateUserRequest`, `PathCode`, `UserViewResponse`, `UserListResponse` are not defined.

- [ ] **Step 3: Append the five DTOs + the `From` impl**

In `apps/server/aegis-server/src/transport/http/dto.rs`, append the following block at the end of the file (after the existing `tests` module's closing `}` — actually no, **before** the `#[cfg(test)] mod tests { … }` block, since DTOs must be defined before tests can name them):

Find the existing `Role` impls in `dto.rs` (the `From<apis::user::Role>` and `From<Role>` blocks). Add the new DTOs and `From` impl immediately after those `Role` impls, and **before** the `#[cfg(test)] mod tests {` line. The final file structure is:

1. Existing request DTOs (`LoginRequest`, `LoginDomainRequest`, `RefreshRequest`, `LogoutRequest`).
2. Existing response DTOs (`TokenPairResponse`, `AccessTokenResponse`, `LogoutResponse`, `AuthClaimsResponse`).
3. Existing `Role` enum + `From` impls.
4. **NEW** — the block below.
5. Existing `#[cfg(test)] mod tests { … }` (with the **NEW** tests appended inside).

Add the new block:

```rust
// -- user requests / responses ---------------------------------------------

/// Wire-level request body for `POST /api/user`. Mirrors
/// `apis::user::CreateUserRequest`; the handler translates at the
/// boundary so the apis crate stays free of serde / utoipa.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

/// Wire-level request body for `PATCH /api/user/{code}`. Every field
/// is optional — only the fields that actually changed need to be
/// supplied. Deliberately omits `id`: the handler resolves the URL
/// `{code}` to a `UserView` via `get_by_code` and threads the
/// resulting `id` into `apis::user::UpdateUserRequest` internally.
#[derive(Serialize, Deserialize, ToSchema, Default)]
pub struct UpdateUserRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

/// Wire-level extractor for the `{code}` URL parameter.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct PathCode {
    pub code: String,
}

/// Wire-level projection of a user — mirrors `apis::user::UserView`
/// field-for-field. Carries `Serialize` / `Deserialize` / `ToSchema`
/// so utoipa can document the response shape and the handler can
/// return it directly via `Json<UserViewResponse>`.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

/// Wire-level wrapper for `GET /api/user` responses. Wrapping the
/// vector in a struct leaves room for future pagination metadata
/// (`total`, `next_cursor`, …) without breaking the response shape.
#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}

impl From<apis::user::UserView> for UserViewResponse {
    fn from(view: apis::user::UserView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            name: view.name,
            role: view.role.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p aegis-server --lib transport::http::dto::tests`
Expected: all DTO tests pass, including the 7 new round-trip tests and the `From` conversion test. The existing DTO tests also still pass.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(aegis-server): add user wire DTOs + apis->wire conversion"
```

---

## Task 3: Register user schemas + tag in `openapi.rs`

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

- [ ] **Step 1: Write the failing test extension**

Open `apps/server/aegis-server/src/transport/http/openapi.rs`. In the existing `openapi_registers_wire_dto_schemas` test, the assertion iterates over a fixed list of schema names. Replace that test with one that asserts both the existing schemas and the five new ones are present:

```rust
    #[test]
    fn openapi_registers_wire_dto_schemas() {
        let doc = openapi();
        let schemas = &doc
            .components
            .as_ref()
            .unwrap()
            .schemas;
        for name in [
            "LoginRequest",
            "LoginDomainRequest",
            "RefreshRequest",
            "LogoutRequest",
            "TokenPairResponse",
            "AccessTokenResponse",
            "LogoutResponse",
            "AuthClaimsResponse",
            "Role",
            "CreateUserRequest",
            "UpdateUserRequest",
            "PathCode",
            "UserViewResponse",
            "UserListResponse",
        ] {
            let entry: &RefOr<_> = schemas
                .get(name)
                .unwrap_or_else(|| panic!("missing schema for {name}"));
            let _ = entry; // schema presence is the assertion
        }
    }
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p aegis-server --lib transport::http::openapi::tests::openapi_registers_wire_dto_schemas`
Expected: FAIL with `missing schema for CreateUserRequest` (and the other four new names).

- [ ] **Step 3: Register the new schemas + tag in `ApiDoc`**

In `apps/server/aegis-server/src/transport/http/openapi.rs`, modify the `ApiDoc` derive macro:

1. Inside `components(schemas(...))`, append five lines after `dto::Role,`:
   ```rust
       dto::CreateUserRequest,
       dto::UpdateUserRequest,
       dto::PathCode,
       dto::UserViewResponse,
       dto::UserListResponse,
   ```
2. Inside `tags(...)`, append one entry after the existing two:
   ```rust
       (name = "user", description = "User CRUD endpoints"),
   ```

The full edited block should read (showing only the `#[openapi(...)]` attribute body, not the surrounding `#[derive(OpenApi)]` / `pub struct ApiDoc;`):

```rust
#[openapi(
    info(
        title = "aegis-server API",
        version = "0.1.0",
        description = "HTTP transport for the aegis auth + user services."
    ),
    modifiers(&SecurityAddon),
    components(schemas(
        dto::LoginRequest,
        dto::LoginDomainRequest,
        dto::RefreshRequest,
        dto::LogoutRequest,
        dto::TokenPairResponse,
        dto::AccessTokenResponse,
        dto::LogoutResponse,
        dto::AuthClaimsResponse,
        dto::Role,
        dto::CreateUserRequest,
        dto::UpdateUserRequest,
        dto::PathCode,
        dto::UserViewResponse,
        dto::UserListResponse,
        ErrorBody,
    )),
    tags(
        (name = "auth", description = "Authentication endpoints"),
        (name = "system", description = "Operational endpoints"),
        (name = "user", description = "User CRUD endpoints"),
    ),
)]
```

- [ ] **Step 4: Verify the test passes**

Run: `cargo test -p aegis-server --lib transport::http::openapi`
Expected: every `openapi::tests::*` test passes, including the extended `openapi_registers_wire_dto_schemas` test.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(aegis-server): register user schemas + tag in OpenAPI doc"
```

---

## Task 4: Scaffold the user module (mod file + router + empty handlers)

**Files:**
- Create: `apps/server/aegis-server/src/transport/http/user.rs`
- Create: `apps/server/aegis-server/src/transport/http/user/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/user/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http.rs`

- [ ] **Step 1: Create `apps/server/aegis-server/src/transport/http/user.rs`**

Write:

```rust
//! HTTP transport for the user CRUD namespace.
//!
//! Hosts the four `apis::user::UserService` handlers under
//! `/api/user/*`. Every handler requires a valid
//! `Authorization: Bearer <token>` header — verification is done by
//! the [`AuthClaims`](crate::transport::http::auth::middleware::AuthClaims)
//! extractor from the sibling `auth/` module; no role-based
//! authorization is enforced at this stage.
//!
//! The router here is an `OpenApiRouter<AppState>` composed from the
//! per-handler `routes!()` registrations; the top-level
//! [`crate::transport::http::router`] nests it under `/api/user`.

pub mod handlers;
pub mod router;
```

- [ ] **Step 2: Create `apps/server/aegis-server/src/transport/http/user/router.rs`**

Write:

```rust
//! `OpenApiRouter` sub-router for the user CRUD namespace.
//!
//! Mirrors [`crate::transport::http::auth::router`] composition but
//! scoped to the `/api/user/*` prefix. The top-level router composes
//! this via [`OpenApiRouter::nest`] so the user namespace can grow
//! without inflating `router.rs`.
//!
//! Each `routes!` call registers a single handler. The `routes!`
//! macro panics when two handlers of the same HTTP method appear in
//! the same invocation, so we issue one call per handler. Today the
//! four handlers span three methods (POST + GET on `/`, GET + PATCH
//! on `/{code}`) and no two share an HTTP method, so each
//! registration is single-handler by construction.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

/// Build the `/api/user` sub-router.
///
/// The returned `OpenApiRouter<AppState>` is ready to be passed to
/// [`OpenApiRouter::nest`]. The handlers are reachable under:
///
/// - `POST   /api/user`
/// - `GET    /api/user`
/// - `GET    /api/user/{code}`
/// - `PATCH  /api/user/{code}`
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create))
        .routes(routes!(handlers::list))
        .routes(routes!(handlers::get_by_code))
        .routes(routes!(handlers::update))
}
```

- [ ] **Step 3: Create `apps/server/aegis-server/src/transport/http/user/handlers.rs` (stub)**

The handlers themselves land in Tasks 5-8. For this task, write a stub `handlers.rs` so the rest of the crate still compiles. The stub declares the four handler symbols (each as an `async fn` returning `Result<axum::Json<()>, crate::transport::http::error::ApiError>`) and provides the test scaffolding (`MockUserService` + `MockAuth` + `test_state` + `app`) used by the upcoming Tasks 5-8.

Write:

```rust
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

use apis::user::UserService;
use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{self, PathCode};
use crate::transport::http::error::ApiError;

// Stubs: Tasks 5-8 replace these bodies with real implementations.
// They exist now so the router (Task 4) compiles end-to-end.

/// `POST /api/user` — create a user.
pub async fn create(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Json(_req): Json<dto::CreateUserRequest>,
) -> Result<(StatusCode, Json<dto::UserViewResponse>), ApiError> {
    unimplemented!("populated in Task 5")
}

/// `GET /api/user` — list users.
pub async fn list(
    State(_state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::UserListResponse>, ApiError> {
    unimplemented!("populated in Task 6")
}

/// `GET /api/user/{code}` — fetch a user by code.
pub async fn get_by_code(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { .. }): Path<PathCode>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    unimplemented!("populated in Task 7")
}

/// `PATCH /api/user/{code}` — update a user.
pub async fn update(
    State(_state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { .. }): Path<PathCode>,
    Json(_req): Json<dto::UpdateUserRequest>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    unimplemented!("populated in Task 8")
}

// Path extractor alias: the wire `dto::PathCode` shadows the handler
// arg's name so axum can resolve the `{code}` URL segment. Aliasing
// here keeps the handler signatures terse.

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
    /// (or `None` to omit it). Used by every per-handler test below.
    pub fn build_request(method: &str, uri: &str, body: Option<&str>, auth: Option<&str>) -> Request<Body> {
        let mut b = Request::builder().method(method).uri(uri);
        if let Some(token) = auth {
            b = b.header("authorization", token);
        }
        if body.is_some() {
            b = b.header("content-type", "application/json");
        }
        b.body(body.map(|s| Body::from(s)).unwrap_or(Body::empty())).unwrap()
    }
}
```

- [ ] **Step 4: Register the new module**

In `apps/server/aegis-server/src/transport/http.rs`, add `pub mod user;` so the new module becomes visible to `http::user::router`. The full edited module declaration is:

```rust
pub mod auth;
pub mod dto;
pub mod error;
pub mod healthz;
pub mod openapi;
pub mod router;
pub mod user;

pub use router::router;
```

- [ ] **Step 5: Verify the crate still builds (stubs compile)**

Run: `cargo check -p aegis-server`
Expected: success. The new module compiles end-to-end; the four `unimplemented!()` stubs satisfy the router's `routes!()` registrations.

- [ ] **Step 6: Verify the test scaffolding compiles**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests --no-run`
Expected: the test binary builds (no tests to run yet, but the `#[cfg(test)] mod tests { … }` block compiles cleanly).

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server/src/transport/http.rs \
        apps/server/aegis-server/src/transport/http/user.rs \
        apps/server/aegis-server/src/transport/http/user/router.rs \
        apps/server/aegis-server/src/transport/http/user/handlers.rs
git commit -m "feat(aegis-server): scaffold user router module with handler stubs"
```

---

## Task 5: Implement `create` handler + tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/user/handlers.rs`

- [ ] **Step 1: Write the failing tests**

Inside the existing `#[cfg(test)] mod tests { … }` block (appended at the end of the module, before the closing `}`), add the create-handler tests:

```rust
    // ---- create ----------------------------------------------------

    #[tokio::test]
    async fn create_returns_201_with_user_view_on_success() {
        let user = MockUserService {
            create: Some(sample_user(42, "u1")),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user.clone(), auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CREATED);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");
        assert_eq!(body["name"], "Alice");
        assert_eq!(body["role"], "admin");
        assert_eq!(body["active"], true);

        // Verify the wire->apis translation captured the right DTO.
        let captured = user.last_create_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.code, "u1");
        assert_eq!(captured.name, "Alice");
        assert!(matches!(captured.role, apis::user::Role::Admin));
    }

    #[tokio::test]
    async fn create_maps_duplicate_code_to_409() {
        let user = MockUserService {
            create_err: Some(apis::user::UserApiError::DuplicateCode("u1".into())),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::CONFLICT);
        assert_eq!(body["code"], "duplicate_code");
    }

    #[tokio::test]
    async fn create_maps_validation_to_400() {
        let user = MockUserService {
            create_err: Some(apis::user::UserApiError::Validation("empty code".into())),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"","name":"Alice","role":"admin"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::BAD_REQUEST);
        assert_eq!(body["code"], "validation_failed");
    }

    #[tokio::test]
    async fn create_without_authorization_returns_401() {
        // No Authorization header — `AuthClaims` rejects the request
        // before the handler body runs.
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "POST",
                "/api/user",
                Some(r#"{"code":"u1","name":"Alice","role":"admin"}"#),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::create_returns_201_with_user_view_on_success`
Expected: FAIL with `not implemented: populated in Task 5` (the handler is still a stub).

- [ ] **Step 3: Replace the `create` stub with the real implementation**

Replace the existing `create` function in `apps/server/aegis-server/src/transport/http/user/handlers.rs` (currently `unimplemented!("populated in Task 5")`) with:

```rust
/// `POST /api/user` — create a user. Returns `201 Created` with the
/// resulting `UserViewResponse`. Requires a valid `Authorization:
/// Bearer <token>` header (enforced by `AuthClaims`).
#[utoipa::path(
    post,
    path = "/",
    tag = "user",
    request_body = dto::CreateUserRequest,
    responses(
        (status = 201, description = "User created", body = dto::UserViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "User code already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository / hashing failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<dto::CreateUserRequest>,
) -> Result<(StatusCode, Json<dto::UserViewResponse>), ApiError> {
    let view = state
        .user
        .create(apis::user::CreateUserRequest {
            code: req.code,
            name: req.name,
            role: req.role.into(),
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::create`
Expected: all four `create*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/user/handlers.rs
git commit -m "feat(aegis-server): implement POST /api/user create handler"
```

---

## Task 6: Implement `list` handler + tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/user/handlers.rs`

- [ ] **Step 1: Write the failing tests**

Inside the existing `#[cfg(test)] mod tests { … }` block, add the list-handler tests right after the create-handler tests:

```rust
    // ---- list ------------------------------------------------------

    #[tokio::test]
    async fn list_returns_200_with_user_list_on_success() {
        let user = MockUserService {
            list: Some(vec![sample_user(1, "u1"), sample_user(2, "u2")]),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["users"].as_array().unwrap().len(), 2);
        assert_eq!(body["users"][0]["code"], "u1");
        assert_eq!(body["users"][1]["code"], "u2");
    }

    #[tokio::test]
    async fn list_returns_empty_users_array_on_empty_repository() {
        let user = MockUserService { list: Some(vec![]), ..Default::default() };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["users"].as_array().unwrap().len(), 0);
    }

    #[tokio::test]
    async fn list_maps_repository_to_500() {
        let user = MockUserService {
            list_err: Some(apis::user::UserApiError::Repository("oops".into())),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn list_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user", None, None))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::list_returns_200_with_user_list_on_success`
Expected: FAIL with `not implemented: populated in Task 6`.

- [ ] **Step 3: Replace the `list` stub with the real implementation**

Replace the existing `list` function (`unimplemented!("populated in Task 6")`) with:

```rust
/// `GET /api/user` — list every user as `UserListResponse`. Requires
/// a valid `Authorization: Bearer <token>` header.
#[utoipa::path(
    get,
    path = "/",
    tag = "user",
    responses(
        (status = 200, description = "All users", body = dto::UserListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<dto::UserListResponse>, ApiError> {
    let views = state.user.list().await?;
    Ok(Json(dto::UserListResponse {
        users: views.into_iter().map(Into::into).collect(),
    }))
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::list`
Expected: all four `list*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/user/handlers.rs
git commit -m "feat(aegis-server): implement GET /api/user list handler"
```

---

## Task 7: Implement `get_by_code` handler + tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/user/handlers.rs`

- [ ] **Step 1: Write the failing tests**

Inside the existing `#[cfg(test)] mod tests { … }` block, add the `get_by_code` tests after the list tests:

```rust
    // ---- get_by_code ----------------------------------------------

    #[tokio::test]
    async fn get_by_code_returns_200_with_user_view_on_success() {
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user/u1", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");
    }

    #[tokio::test]
    async fn get_by_code_maps_not_found_to_404() {
        let user = MockUserService {
            get_by_code_err: Some(apis::user::UserApiError::NotFound),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user/missing", None, Some("Bearer good")))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn get_by_code_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request("GET", "/api/user/u1", None, None))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::get_by_code_returns_200_with_user_view_on_success`
Expected: FAIL with `not implemented: populated in Task 7`.

- [ ] **Step 3: Replace the `get_by_code` stub with the real implementation**

Replace the existing `get_by_code` function (`unimplemented!("populated in Task 7")`) with:

```rust
/// `GET /api/user/{code}` — fetch a user by their unique `code`.
/// Returns `404` when no user with that code exists. Requires a
/// valid `Authorization: Bearer <token>` header.
#[utoipa::path(
    get,
    path = "/{code}",
    tag = "user",
    params(
        ("code" = String, Path, description = "User code"),
    ),
    responses(
        (status = 200, description = "User found", body = dto::UserViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_by_code(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathCode { code }): Path<dto::PathCode>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    let view = state.user.get_by_code(&code).await?;
    Ok(Json(view.into()))
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::get_by_code`
Expected: all three `get_by_code*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/user/handlers.rs
git commit -m "feat(aegis-server): implement GET /api/user/{code} get_by_code handler"
```

---

## Task 8: Implement `update` handler + tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/user/handlers.rs`

- [ ] **Step 1: Write the failing tests**

Inside the existing `#[cfg(test)] mod tests { … }` block, add the `update` tests after the `get_by_code` tests:

```rust
    // ---- update ----------------------------------------------------

    #[tokio::test]
    async fn update_returns_200_with_user_view_on_success() {
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            update: Some(sample_user(42, "u1")),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user.clone(), auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"Alice"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::OK);
        assert_eq!(body["id"], 42);
        assert_eq!(body["code"], "u1");

        // The handler must resolve {code} -> id via get_by_code and
        // thread that id into the apis update request. The mock
        // captures the last call so we can assert on the translation.
        let captured = user.last_update_args.lock().unwrap().clone().unwrap();
        assert_eq!(captured.id, 42);
        assert!(captured.code.is_none());
        assert_eq!(captured.name.as_deref(), Some("Alice"));
    }

    #[tokio::test]
    async fn update_maps_get_by_code_not_found_to_404() {
        // get_by_code fails (no such user) — handler returns 404
        // without ever calling update.
        let user = MockUserService {
            get_by_code_err: Some(apis::user::UserApiError::NotFound),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/missing",
                Some(r#"{"name":"Alice"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::NOT_FOUND);
        assert_eq!(body["code"], "not_found");
    }

    #[tokio::test]
    async fn update_maps_update_failure_to_500() {
        // get_by_code succeeds but update fails — same handler, but
        // the error originates from the second service call.
        let user = MockUserService {
            get_by_code: Some(sample_user(42, "u1")),
            update_err: Some(apis::user::UserApiError::Repository("oops".into())),
            ..Default::default()
        };
        let auth = MockAuth { verify_ok: true, ..Default::default() };
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"Alice"}"#),
                Some("Bearer good"),
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::INTERNAL_SERVER_ERROR);
        assert_eq!(body["code"], "repository_error");
    }

    #[tokio::test]
    async fn update_without_authorization_returns_401() {
        let user = MockUserService::default();
        let auth = MockAuth::default();
        let app = app(test_state(user, auth));
        let response = app
            .oneshot(build_request(
                "PATCH",
                "/api/user/u1",
                Some(r#"{"name":"Alice"}"#),
                None,
            ))
            .await
            .unwrap();
        let (status, body) = read_json(response).await;
        assert_eq!(status, AxStatus::UNAUTHORIZED);
        assert_eq!(body["code"], "token_verification_failed");
    }
```

- [ ] **Step 2: Run the new tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::update_returns_200_with_user_view_on_success`
Expected: FAIL with `not implemented: populated in Task 8`.

- [ ] **Step 3: Replace the `update` stub with the real implementation**

Replace the existing `update` function (`unimplemented!("populated in Task 8")`) with:

```rust
/// `PATCH /api/user/{code}` — update a user. The handler resolves
/// the URL `{code}` to the internal `id` via `get_by_code` and
/// threads that `id` into the apis update request (the wire DTO has
/// no `id` field by design). Returns the updated `UserViewResponse`.
/// Requires a valid `Authorization: Bearer <token>` header.
#[utoipa::path(
    patch,
    path = "/{code}",
    tag = "user",
    params(
        ("code" = String, Path, description = "User code"),
    ),
    request_body = dto::UpdateUserRequest,
    responses(
        (status = 200, description = "User updated", body = dto::UserViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "User not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn update(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(dto::PathCode { code }): Path<dto::PathCode>,
    Json(req): Json<dto::UpdateUserRequest>,
) -> Result<Json<dto::UserViewResponse>, ApiError> {
    // Resolve {code} -> id via the apis lookup so the update DTO
    // carries the correct internal id. If the code is unknown, this
    // surfaces as 404 via the AuthApiError::NotFound -> ApiError::User
    // path.
    let current = state.user.get_by_code(&code).await?;
    let result = state
        .user
        .update(apis::user::UpdateUserRequest {
            id: current.id,
            code: req.code,
            name: req.name,
            role: req.role.map(Into::into),
            active: req.active,
        })
        .await?;
    Ok(Json(result.into()))
}
```

- [ ] **Step 4: Verify the tests pass**

Run: `cargo test -p aegis-server --lib transport::http::user::handlers::tests::update`
Expected: all four `update*` tests pass.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/user/handlers.rs
git commit -m "feat(aegis-server): implement PATCH /api/user/{code} update handler"
```

---

## Task 9: Wire user module into top-level router + integration tests

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`

- [ ] **Step 1: Nest `/api/user` in the top-level `OpenApiRouter`**

Open `apps/server/aegis-server/src/transport/http/router.rs`. The `router()` function currently looks like:

```rust
pub fn router(state: AppState) -> axum::Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/auth", auth::router())
        .nest("/healthz", healthz::router())
        .with_state(state)
        .split_for_parts();

    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
```

Add `.nest("/api/user", user::router())` to the chain, immediately after `.nest("/api/auth", auth::router())`:

```rust
pub fn router(state: AppState) -> axum::Router {
    let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
        .nest("/api/auth", auth::router())
        .nest("/api/user", user::router())
        .nest("/healthz", healthz::router())
        .with_state(state)
        .split_for_parts();

    router
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
```

Also add `use crate::transport::http::user;` next to the existing `use crate::transport::http::auth;` line near the top of the file.

- [ ] **Step 2: Write the failing integration test**

Inside the existing `#[cfg(test)] mod tests { … }` block in `router.rs`, add the new integration test after `openapi_json_returns_200_with_valid_doc`:

```rust
    #[tokio::test]
    async fn user_route_returns_401_without_jwt() {
        // No Authorization header — `AuthClaims` rejects the request
        // before any user handler runs.
        let app = router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/user")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"u1","name":"Alice","role":"admin"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::UNAUTHORIZED);
    }
```

Also extend the existing `openapi_json_returns_200_with_valid_doc` test by appending the user-route assertions immediately before its final closing `}` (i.e. after the existing `assert!(doc["paths"]["/healthz"]["get"]["security"].is_null());` line):

```rust
        // User CRUD routes must be registered, and all four must
        // advertise the BearerAuth security scheme.
        assert!(doc["paths"]["/api/user"]["post"].is_object());
        assert!(doc["paths"]["/api/user"]["get"].is_object());
        assert!(doc["paths"]["/api/user/{code}"]["get"].is_object());
        assert!(doc["paths"]["/api/user/{code}"]["patch"].is_object());
        for (path, method) in [
            ("/api/user", "post"),
            ("/api/user", "get"),
            ("/api/user/{code}", "get"),
            ("/api/user/{code}", "patch"),
        ] {
            let entry = &doc["paths"][path][method]["security"];
            assert_eq!(
                entry[0]["BearerAuth"],
                serde_json::json!([]),
                "{method} {path} must reference BearerAuth",
            );
        }
```

(The existing `assert!(doc["paths"]["/healthz"]["get"]["security"].is_null());` line stays in place — no need to duplicate it.)

- [ ] **Step 3: Run the new tests to verify the user-route ones fail**

Run: `cargo test -p aegis-server --lib transport::http::router::tests::user_route_returns_401_without_jwt`
Expected: FAIL with 404 (no route registered) until Step 1 lands. After Step 1, the route is registered but the integration test's `test_state()` returns 401 because `NullUserService` is unimplemented — but the handler never runs because `AuthClaims` rejects first. So the test passes immediately after Step 1.

- [ ] **Step 4: Run the extended OpenAPI document test**

Run: `cargo test -p aegis-server --lib transport::http::router::tests::openapi_json_returns_200_with_valid_doc`
Expected: PASS. The four user paths exist in the document, all four reference `BearerAuth`, and `/healthz` still has no security requirement.

- [ ] **Step 5: Run the full test suite to make sure nothing regressed**

Run: `cargo test -p aegis-server --lib`
Expected: every test passes. The existing `healthz_returns_200`, `login_route_returns_200`, `unknown_route_returns_404`, `swagger_ui_root_returns_200`, and the original `openapi_json_returns_200_with_valid_doc` assertions all continue to pass.

- [ ] **Step 6: Run clippy**

Run: `cargo clippy -p aegis-server --all-targets -- -D warnings`
Expected: no warnings.

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/router.rs
git commit -m "feat(aegis-server): nest /api/user in top-level router + integration tests"
```

---

## Task 10: Verification gate

**Files:** none modified.

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`
Expected: no formatting issues.

- [ ] **Step 2: Lint check**

Run: `cargo clippy -p aegis-server --all-targets -- -D warnings`
Expected: clean.

- [ ] **Step 3: Full test run**

Run: `cargo test -p aegis-server`
Expected: every test passes (unit + integration; the live-DB integration test remains `#[ignore]`-gated and is not exercised here).

- [ ] **Step 4: Doctest check**

Run: `cargo test -p aegis-server --doc`
Expected: the existing doctest in `src/lib.rs` (which pins the public surface) still resolves. No new public-API re-exports were added; the test is unchanged.

- [ ] **Step 5: Doc build**

Run: `cargo doc -p aegis-server --no-deps`
Expected: documentation builds without errors.

- [ ] **Step 6: Final summary commit (only if any cleanup was needed)**

If Steps 1-5 surfaced any cleanup (e.g., a `#[allow(dead_code)]` left over from the stub phase, a stale comment, an unused import), fold it into a single closing commit:

```bash
git add -A
git commit -m "chore(aegis-server): final cleanup for user router PR"
```

Skip this step if Steps 1-5 already pass cleanly.

---

## Done

The user router is now wired:

- `POST /api/user` creates a user (201 Created).
- `GET /api/user` lists users.
- `GET /api/user/{code}` fetches a user by code.
- `PATCH /api/user/{code}` updates a user.

Every route requires a valid `Authorization: Bearer <token>` header (verified by the existing `AuthClaims` extractor). All four routes advertise `security: [{ BearerAuth: [] }]` in the generated OpenAPI document, visible in swagger-ui. The `ApiError` enum refactor lets the auth and user sub-routers share the same `?` path without losing exhaustiveness; adding a third apis service in the future is one helper + one variant.