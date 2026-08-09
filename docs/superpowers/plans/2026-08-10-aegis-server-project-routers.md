# Aegis Server Project Routers Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose authenticated product and project HTTP endpoints backed by `apis::project::ProjectService`, with writes restricted to `Root` and `Admin` users.

**Architecture:** Add one flat `transport::http::project` feature module containing all eight handlers and a router that returns two resource routers for mounting at `/api/product` and `/api/project`. Extend `AppState` with the shared project service, construct its project-crate adapters at startup, keep wire DTOs in the server, and map project and authorization failures through the existing `ApiError` response path.

**Tech Stack:** Rust, Axum 0.8, utoipa/utoipa-axum, async-trait, Serde, SQLx PostgreSQL adapters, Tokio, Tower test utilities.

---

## File Structure

### Create

- `apps/server/aegis-server/src/transport/http/project.rs` — declares and re-exports the feature router.
- `apps/server/aegis-server/src/transport/http/project/router.rs` — builds separate product and project `OpenApiRouter<AppState>` values.
- `apps/server/aegis-server/src/transport/http/project/handlers.rs` — owns all eight handlers, the write-role guard, and focused handler tests.

### Modify

- `apps/server/aegis-server/Cargo.toml` — adds the project crate dependency.
- `apps/server/aegis-server/src/state.rs` — stores `Arc<dyn ProjectService>`.
- `apps/server/aegis-server/src/run.rs` — constructs and injects `ProjectServiceImpl`.
- `apps/server/aegis-server/src/transport/http.rs` — declares the project feature module.
- `apps/server/aegis-server/src/transport/http/dto.rs` — adds product/project wire DTOs and conversions.
- `apps/server/aegis-server/src/transport/http/error.rs` — maps `ProjectApiError` and `Forbidden`.
- `apps/server/aegis-server/src/transport/http/openapi.rs` — registers schemas and tags.
- `apps/server/aegis-server/src/transport/http/router.rs` — mounts both routers and extends full-router/OpenAPI tests.
- Existing server test modules containing `AppState { ... }` literals — supply a stub project service.
- `apps/server/aegis-server/README.md` — documents routes and authorization.

## Route Contract

| Method | Path | Handler | Access |
|---|---|---|---|
| POST | `/api/product` | `create_product` | Root/Admin |
| GET | `/api/product` | `list_products` | authenticated |
| GET | `/api/product/{code}` | `get_product_by_code` | authenticated |
| PATCH | `/api/product/{code}` | `update_product` | Root/Admin |
| POST | `/api/project` | `create_project` | Root/Admin |
| GET | `/api/project` | `list_projects` | authenticated |
| GET | `/api/project/{code}` | `get_project_by_code` | authenticated |
| PATCH | `/api/project/{code}` | `update_project` | Root/Admin |

---

### Task 1: Add the project dependency and state port

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`
- Modify: `apps/server/aegis-server/src/state.rs`

- [ ] **Step 1: Add the project crate dependency**

Add beside the existing `auth` and `user` path dependencies:

```toml
project = { path = "../../../lib/crates/project" }
```

- [ ] **Step 2: Add the service port to application state**

Change `AppState` to:

```rust
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
    pub project: Arc<dyn apis::project::ProjectService>,
}
```

- [ ] **Step 3: Run a compile check and record the expected failure**

Run:

```bash
cargo check -p aegis-server
```

Expected: compilation fails at existing `AppState` struct literals with “missing field `project`”. This confirms the new dependency is visible and identifies builders repaired in Task 2.

- [ ] **Step 4: Commit the dependency and state boundary**

```bash
git add apps/server/aegis-server/Cargo.toml apps/server/aegis-server/src/state.rs Cargo.lock
git commit -m "chore(server): add project service dependency

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 2: Construct the service and repair state builders

**Files:**
- Modify every file reported by `rg -n 'AppState\s*\{' apps/server/aegis-server`
- Prefer placing a shared `StubProjectService` in the existing test-support location if one exists; otherwise define a small local stub in each affected `#[cfg(test)]` module, matching current server conventions.

- [ ] **Step 1: Enumerate every state literal**

Run:

```bash
rg -n 'AppState\s*\{' apps/server/aegis-server
```

Expected: production wiring in `run.rs` and test builders in HTTP router/auth/user modules.

- [ ] **Step 2: Add a test-only no-op implementation**

Use this complete trait shape in each test module that needs an inert project dependency (or move it to existing shared test support and import it):

```rust
#[derive(Clone, Default)]
struct StubProjectService;

#[async_trait::async_trait]
impl apis::project::ProjectService for StubProjectService {
    async fn create_product(
        &self,
        _req: apis::project::CreateProductRequest,
    ) -> Result<apis::project::ProductView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn get_product_by_id(
        &self,
        _id: i32,
    ) -> Result<apis::project::ProductView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn get_product_by_code(
        &self,
        _code: &str,
    ) -> Result<apis::project::ProductView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn list_products(
        &self,
    ) -> Result<Vec<apis::project::ProductView>, apis::project::ProjectApiError> {
        Ok(Vec::new())
    }

    async fn update_product(
        &self,
        _req: apis::project::UpdateProductRequest,
    ) -> Result<apis::project::ProductView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn create_project(
        &self,
        _req: apis::project::CreateProjectRequest,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn get_project_by_id(
        &self,
        _id: i32,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn get_project_by_code(
        &self,
        _code: &str,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }

    async fn list_projects(
        &self,
    ) -> Result<Vec<apis::project::ProjectView>, apis::project::ProjectApiError> {
        Ok(Vec::new())
    }

    async fn update_project(
        &self,
        _req: apis::project::UpdateProjectRequest,
    ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
        Err(apis::project::ProjectApiError::NotFound)
    }
}
```

- [ ] **Step 3: Supply the stub in every test state builder**

Add:

```rust
project: Arc::new(StubProjectService) as Arc<dyn apis::project::ProjectService>,
```

Do not alter mock auth or user behavior.

- [ ] **Step 4: Construct and inject the production project service**

In `run.rs`, add:

```rust
fn build_project_service(
    pool: PgPool,
    user: Arc<dyn apis::user::UserService>,
) -> Arc<dyn apis::project::ProjectService> {
    let product_repo = project::ProductRepo::new(pool.clone());
    let project_repo = project::ProjectRepo::new(pool);
    let users = project::UserServiceImpl::new(user);
    let usecase = project::ProjectUsecase::new(project::ProjectUsecaseConfig {
        product_repo,
        project_repo,
        users,
    });
    Arc::new(project::ProjectServiceImpl::new(usecase))
}
```

Build the user trait object first, clone it into the project adapter, and populate all three state fields:

```rust
let user: Arc<dyn apis::user::UserService> = build_user_service(pool.clone());
let project = build_project_service(pool.clone(), user.clone());
let state = AppState { auth, user, project };
```

Adapt the local auth variable to the existing helper return type without changing behavior.

- [ ] **Step 5: Run existing library tests**

```bash
cargo check -p aegis-server
cargo test -p aegis-server --lib
```

Expected: production and test state literals compile, and all pre-existing assertions pass.

- [ ] **Step 6: Commit construction and fixture compatibility**

```bash
git add apps/server/aegis-server/src/run.rs apps/server/aegis-server/src
git commit -m "feat(server): inject project service into app state

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 3: Map project and authorization errors

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`

- [ ] **Step 1: Write failing error-response tests**

Add tests that call `IntoResponse`, decode `ErrorBody`, and cover this table:

```rust
#[tokio::test]
async fn project_errors_have_stable_status_and_code() {
    use apis::project::ProjectApiError as E;

    let cases = [
        (E::Validation("bad".into()), StatusCode::BAD_REQUEST, "validation_failed"),
        (E::NotFound, StatusCode::NOT_FOUND, "not_found"),
        (E::ProductNotFound("product-x".into()), StatusCode::NOT_FOUND, "product_not_found"),
        (E::UserNotFound("user-x".into()), StatusCode::NOT_FOUND, "user_not_found"),
        (E::DuplicateCode("duplicate".into()), StatusCode::CONFLICT, "duplicate_code"),
        (E::Repository("offline".into()), StatusCode::INTERNAL_SERVER_ERROR, "repository_error"),
    ];

    for (source, expected_status, expected_code) in cases {
        let response = ApiError::from(source).into_response();
        assert_eq!(response.status(), expected_status);
        let bytes = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
        let body: ErrorBody = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(body.code, expected_code);
    }
}

#[tokio::test]
async fn forbidden_has_stable_response() {
    let response = ApiError::Forbidden.into_response();
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    let bytes = axum::body::to_bytes(response.into_body(), 1024).await.unwrap();
    let body: ErrorBody = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body.code, "forbidden");
    assert_eq!(body.message, "admin or root role required");
}
```

- [ ] **Step 2: Run the tests and verify failure**

```bash
cargo test -p aegis-server --lib transport::http::error::tests::project_errors_have_stable_status_and_code
cargo test -p aegis-server --lib transport::http::error::tests::forbidden_has_stable_response
```

Expected: compile failure because `ApiError` lacks `From<ProjectApiError>` and `Forbidden`.

- [ ] **Step 3: Add the error variants**

```rust
#[derive(Debug, Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(#[from] apis::auth::AuthApiError),
    #[error("{0}")]
    User(#[from] apis::user::UserApiError),
    #[error("{0}")]
    Project(#[from] apis::project::ProjectApiError),
    #[error("admin or root role required")]
    Forbidden,
}
```

Extend the existing status and code dispatch:

```rust
Self::Project(error) => project_status(error),
Self::Forbidden => StatusCode::FORBIDDEN,
```

```rust
Self::Project(error) => project_code(error),
Self::Forbidden => "forbidden",
```

Add exact mapping helpers:

```rust
fn project_status(error: &apis::project::ProjectApiError) -> StatusCode {
    use apis::project::ProjectApiError;
    match error {
        ProjectApiError::Validation(_) => StatusCode::BAD_REQUEST,
        ProjectApiError::NotFound
        | ProjectApiError::ProductNotFound(_)
        | ProjectApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
        ProjectApiError::DuplicateCode(_) => StatusCode::CONFLICT,
        ProjectApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn project_code(error: &apis::project::ProjectApiError) -> &'static str {
    use apis::project::ProjectApiError;
    match error {
        ProjectApiError::Validation(_) => "validation_failed",
        ProjectApiError::NotFound => "not_found",
        ProjectApiError::ProductNotFound(_) => "product_not_found",
        ProjectApiError::UserNotFound(_) => "user_not_found",
        ProjectApiError::DuplicateCode(_) => "duplicate_code",
        ProjectApiError::Repository(_) => "repository_error",
    }
}
```

- [ ] **Step 4: Run the focused and existing error tests**

```bash
cargo test -p aegis-server --lib transport::http::error::tests
```

Expected: all error tests pass; 5xx cases may emit expected tracing output.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/error.rs
git commit -m "feat(server): map project authorization errors

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 4: Add product and project wire DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

- [ ] **Step 1: Write failing serialization and conversion tests**

Cover the wire contracts with these representative tests and equivalent assertions for nested project responses:

```rust
#[test]
fn update_project_distinguishes_omitted_and_empty_members() {
    let omitted: UpdateProjectRequest = serde_json::from_str(r#"{"description":"new"}"#).unwrap();
    assert!(omitted.members.is_none());

    let empty: UpdateProjectRequest = serde_json::from_str(r#"{"members":{}}"#).unwrap();
    let members = empty.members.expect("members must be present");
    assert!(members.leaders.is_empty());
    assert!(members.workers.is_empty());
}

#[test]
fn update_product_omits_none_fields() {
    let request = UpdateProductRequest {
        code: None,
        name: Some("Renamed".into()),
        description: None,
        active: None,
    };
    assert_eq!(serde_json::to_value(request).unwrap(), serde_json::json!({"name": "Renamed"}));
}

#[test]
fn project_view_conversion_preserves_nested_data() {
    let response: ProjectViewResponse = sample_project_view().into();
    assert_eq!(response.product.code, "product-1");
    assert_eq!(response.members.leaders[0].code, "leader-1");
    assert_eq!(response.unblind_members.workers[0].code, "worker-2");
}
```

Also test:

- `ProductView -> ProductViewResponse` preserves every field.
- `ProjectMemberView -> ProjectMemberViewResponse` maps both vectors.
- `ProductListResponse` serializes as `{"products": [...]}`.
- `ProjectListResponse` serializes as `{"projects": [...]}`.
- Create requests preserve optional memberships.

- [ ] **Step 2: Run DTO tests and verify failure**

```bash
cargo test -p aegis-server --lib transport::http::dto::tests
```

Expected: compile failure because the new DTO types do not exist.

- [ ] **Step 3: Add request DTOs**

```rust
#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateProductRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct ProjectMemberDataRequest {
    #[serde(default)]
    pub leaders: Vec<String>,
    #[serde(default)]
    pub workers: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}
```

Standard `Option<T>` semantics preserve missing as `None`; `{}` becomes `Some(ProjectMemberDataRequest::default())` because both vector fields use `serde(default)`.

- [ ] **Step 4: Add response DTOs and complete conversions**

Define `ProductViewResponse`, `ProductListResponse`, `UserSummaryViewResponse`, `ProjectMemberViewResponse`, `ProjectViewResponse`, and `ProjectListResponse` with the exact fields from `apis::project` and derive `Debug`, `Serialize`, `Deserialize`, and `ToSchema`.

Use complete field-by-field conversions:

```rust
impl From<apis::project::ProductView> for ProductViewResponse {
    fn from(value: apis::project::ProductView) -> Self {
        Self {
            id: value.id,
            code: value.code,
            name: value.name,
            description: value.description,
            active: value.active,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}

impl From<apis::project::UserSummaryView> for UserSummaryViewResponse {
    fn from(value: apis::project::UserSummaryView) -> Self {
        Self { code: value.code, name: value.name }
    }
}

impl From<apis::project::ProjectMemberView> for ProjectMemberViewResponse {
    fn from(value: apis::project::ProjectMemberView) -> Self {
        Self {
            leaders: value.leaders.into_iter().map(Into::into).collect(),
            workers: value.workers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<apis::project::ProjectView> for ProjectViewResponse {
    fn from(value: apis::project::ProjectView) -> Self {
        Self {
            id: value.id,
            code: value.code,
            description: value.description,
            product: value.product.into(),
            members: value.members.into(),
            unblind_members: value.unblind_members.into(),
            active: value.active,
            created_at: value.created_at,
            updated_at: value.updated_at,
        }
    }
}
```

- [ ] **Step 5: Run DTO tests**

```bash
cargo test -p aegis-server --lib transport::http::dto::tests
```

Expected: all existing and new DTO tests pass.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(server): add product project wire types

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 5: Create the feature module and role guard

**Files:**
- Create: `apps/server/aegis-server/src/transport/http/project.rs`
- Create: `apps/server/aegis-server/src/transport/http/project/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http.rs`

- [ ] **Step 1: Write role-guard tests**

In `handlers.rs`, add a `#[cfg(test)]` module with:

```rust
fn claims(role: apis::user::Role) -> AuthClaims {
    AuthClaims(apis::auth::AuthClaims {
        code: "caller".into(),
        role,
        token_version: 0,
    })
}

#[test]
fn write_guard_accepts_root_and_admin() {
    require_admin_or_root(&claims(apis::user::Role::Root)).unwrap();
    require_admin_or_root(&claims(apis::user::Role::Admin)).unwrap();
}

#[test]
fn write_guard_rejects_general() {
    let error = require_admin_or_root(&claims(apis::user::Role::General)).unwrap_err();
    assert_eq!(error.status(), StatusCode::FORBIDDEN);
    assert_eq!(error.code(), "forbidden");
}
```

- [ ] **Step 2: Run and verify failure**

```bash
cargo test -p aegis-server --lib transport::http::project::handlers::tests::write_guard
```

Expected: module/function-not-found compile failure.

- [ ] **Step 3: Declare the module hierarchy**

`project.rs`:

```rust
mod handlers;
pub mod router;
```

Add `pub mod project;` to `transport/http.rs`.

- [ ] **Step 4: Implement the local guard**

```rust
fn require_admin_or_root(claims: &AuthClaims) -> Result<(), ApiError> {
    match claims.0.role {
        apis::user::Role::Root | apis::user::Role::Admin => Ok(()),
        apis::user::Role::General => Err(ApiError::Forbidden),
    }
}
```

- [ ] **Step 5: Define the two-router return type**

The feature owns one handler module but must mount two independent URL prefixes. In `router.rs`:

```rust
pub struct ProjectRouters {
    pub product: OpenApiRouter<AppState>,
    pub project: OpenApiRouter<AppState>,
}

pub fn routers() -> ProjectRouters {
    ProjectRouters {
        product: OpenApiRouter::new(),
        project: OpenApiRouter::new(),
    }
}
```

Do not nest one router under `/project`; that would incorrectly produce `/api/project/product` or collapse both resource roots.

- [ ] **Step 6: Run guard tests**

```bash
cargo test -p aegis-server --lib transport::http::project::handlers::tests::write_guard
```

Expected: both pass.

- [ ] **Step 7: Commit the feature skeleton**

```bash
git add apps/server/aegis-server/src/transport/http.rs apps/server/aegis-server/src/transport/http/project.rs apps/server/aegis-server/src/transport/http/project
git commit -m "feat(server): scaffold project http feature

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 6: Implement product handlers test-first

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/router.rs`

- [ ] **Step 1: Add a configurable `MockProjectService` to handler tests**

Implement all ten trait methods. Store configured `Result` values for the four product calls and captured update/create requests in `Arc<Mutex<Option<_>>>`; return `NotFound` for unused ID methods. Add `sample_product(id, code)` using fixed UTC timestamps.

The test state must contain a mock auth service whose `verify` returns the configured role, an inert user service, and the mock project service.

- [ ] **Step 2: Write failing create-product tests**

Cover:

- Root receives `201` and the service receives all request fields.
- Admin receives `201`.
- General receives `403` and the service is not called.
- Missing bearer token receives `401`.
- `Validation`, `DuplicateCode`, and `Repository` use the mappings from Task 3.

Use this request shape:

```rust
Request::builder()
    .method(Method::POST)
    .uri("/")
    .header(header::AUTHORIZATION, "Bearer valid")
    .header(header::CONTENT_TYPE, "application/json")
    .body(Body::from(r#"{"code":"product-1","name":"Product","description":"Description"}"#))
    .unwrap()
```

- [ ] **Step 3: Implement `create_product`**

```rust
pub async fn create_product(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(request): Json<dto::CreateProductRequest>,
) -> Result<(StatusCode, Json<dto::ProductViewResponse>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state.project.create_product(apis::project::CreateProductRequest {
        code: request.code,
        name: request.name,
        description: request.description,
    }).await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}
```

Annotate `create_product` with the complete route contract:

```rust
#[utoipa::path(
    post,
    path = "",
    tag = "product",
    request_body = dto::CreateProductRequest,
    responses(
        (status = 201, description = "Product created", body = dto::ProductViewResponse),
        (status = 400, description = "Validation failed", body = ErrorBody),
        (status = 401, description = "Authentication failed", body = ErrorBody),
        (status = 403, description = "Admin or root required", body = ErrorBody),
        (status = 409, description = "Product code already exists", body = ErrorBody),
        (status = 500, description = "Repository failure", body = ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
```

Use the same explicit response style on the remaining handlers, omitting statuses that the operation cannot produce and including `404` on item lookup/update routes.

- [ ] **Step 4: Write and implement list/get tests and handlers**

`list_products` returns `Json(ProductListResponse { products: views.into_iter().map(Into::into).collect() })`; `get_product_by_code` extracts `PathCode` and forwards `&code`. Both require `AuthClaims` but no role guard. Test success, `401`, `404`, and `500`.

- [ ] **Step 5: Write failing update-product tests**

Test Root and Admin success, General `403`, missing bearer `401`, lookup `404`, update errors, and that the ID from `get_product_by_code` is inserted into the captured `apis::project::UpdateProductRequest` while optional body fields remain unchanged.

- [ ] **Step 6: Implement `update_product`**

```rust
pub async fn update_product(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(request): Json<dto::UpdateProductRequest>,
) -> Result<Json<dto::ProductViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let id = state.project.get_product_by_code(&code).await?.id;
    let view = state.project.update_product(apis::project::UpdateProductRequest {
        id,
        code: request.code,
        name: request.name,
        description: request.description,
        active: request.active,
    }).await?;
    Ok(Json(view.into()))
}
```

Document 200, 400, 401, 403, 404, 409, and 500.

- [ ] **Step 7: Register product routes**

```rust
let product = OpenApiRouter::new()
    .routes(routes!(handlers::create_product))
    .routes(routes!(handlers::list_products))
    .routes(routes!(handlers::get_product_by_code))
    .routes(routes!(handlers::update_product));
```

Each handler must have the correct annotation path (`""` for collection; `"/{code}"` for item) so utoipa-axum merges methods correctly.

- [ ] **Step 8: Run product handler tests**

```bash
cargo test -p aegis-server --lib transport::http::project::handlers::tests::product
```

Expected: all product tests pass.

- [ ] **Step 9: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/project
git commit -m "feat(server): add product http handlers

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 7: Implement project handlers test-first

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/router.rs`

- [ ] **Step 1: Extend the mock for project operations**

Capture `CreateProjectRequest` and `UpdateProjectRequest`, configure project list/get/create/update results, and add `sample_project()` with nested product and both membership teams.

- [ ] **Step 2: Write and implement `create_project` tests**

Cover Root/Admin success, General `403`, `401`, `Validation`, `ProductNotFound`, `UserNotFound`, `DuplicateCode`, and `Repository`. Assert `members` and `unblind_members` convert to `apis::project::ProjectMemberData` without losing leader/worker codes.

Implementation conversion:

```rust
fn member_data(value: dto::ProjectMemberDataRequest) -> apis::project::ProjectMemberData {
    apis::project::ProjectMemberData {
        leaders: value.leaders,
        workers: value.workers,
    }
}
```

```rust
members: request.members.map(member_data),
unblind_members: request.unblind_members.map(member_data),
```

Return `(StatusCode::CREATED, Json(view.into()))`.

- [ ] **Step 3: Write and implement read handler tests**

`list_projects` wraps results under `projects`; `get_project_by_code` forwards the path code. Reads require authentication but accept `General`. Test `200`, `401`, `404`, and `500`.

- [ ] **Step 4: Write failing update-project tests**

Cover Root/Admin success, General `403`, `401`, lookup errors, update errors, and the following request-preservation cases:

- Omitted `members` stays `None`.
- `"members": {}` becomes `Some(ProjectMemberData { leaders: [], workers: [] })`.
- Non-empty `unblind_members` preserves both vectors.
- Resolved project ID overrides any client input because the wire DTO has no `id`.

- [ ] **Step 5: Implement `update_project`**

```rust
pub async fn update_project(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(request): Json<dto::UpdateProjectRequest>,
) -> Result<Json<dto::ProjectViewResponse>, ApiError> {
    require_admin_or_root(&claims)?;
    let id = state.project.get_project_by_code(&code).await?.id;
    let view = state.project.update_project(apis::project::UpdateProjectRequest {
        id,
        code: request.code,
        description: request.description,
        product_id: request.product_id,
        active: request.active,
        members: request.members.map(member_data),
        unblind_members: request.unblind_members.map(member_data),
    }).await?;
    Ok(Json(view.into()))
}
```

- [ ] **Step 6: Register project routes**

```rust
let project = OpenApiRouter::new()
    .routes(routes!(handlers::create_project))
    .routes(routes!(handlers::list_projects))
    .routes(routes!(handlers::get_project_by_code))
    .routes(routes!(handlers::update_project));
```

Return `ProjectRouters { product, project }`.

- [ ] **Step 7: Run all feature tests**

```bash
cargo test -p aegis-server --lib transport::http::project
```

Expected: all role, product, project, DTO forwarding, and error tests pass.

- [ ] **Step 8: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/project
git commit -m "feat(server): add project http handlers

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 8: Mount routers and register OpenAPI metadata

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

- [ ] **Step 1: Write failing full-router route tests**

Using `tower::ServiceExt::oneshot`, add tests that assert:

- Authenticated `GET /api/product` and `GET /api/project` return `200` with wrapped arrays.
- `GET /api/product/{code}` and `/api/project/{code}` reach the correct mock methods.
- General-role `POST` and `PATCH` return `403`.
- Missing bearer on both resource families returns `401`.
- `DELETE` on collection/item paths returns `405`.

- [ ] **Step 2: Extend the OpenAPI JSON test before mounting**

Assert operations exist and require bearer security:

```rust
for (method, path) in [
    ("post", "/api/product"),
    ("get", "/api/product"),
    ("get", "/api/product/{code}"),
    ("patch", "/api/product/{code}"),
    ("post", "/api/project"),
    ("get", "/api/project"),
    ("get", "/api/project/{code}"),
    ("patch", "/api/project/{code}"),
] {
    let operation = &document["paths"][path][method];
    assert!(operation.is_object(), "missing {method} {path}");
    assert_eq!(operation["security"][0]["BearerAuth"], serde_json::json!([]));
}
```

Also assert POST/PATCH operations include response key `403`.

- [ ] **Step 3: Run and verify failure**

```bash
cargo test -p aegis-server --lib transport::http::router::tests
```

Expected: route `404` or missing OpenAPI operation because the feature routers are not mounted.

- [ ] **Step 4: Mount both resource routers**

In the top-level `router` function:

```rust
let project_routers = project::router::routers();
let api_routers = OpenApiRouter::new()
    .nest("/auth", auth::router())
    .nest("/user", user::router())
    .nest("/product", project_routers.product)
    .nest("/project", project_routers.project);
```

- [ ] **Step 5: Register schemas and tags**

Append all eleven new DTO types to `components(schemas(...))`:

```rust
dto::CreateProductRequest,
dto::UpdateProductRequest,
dto::ProductViewResponse,
dto::ProductListResponse,
dto::CreateProjectRequest,
dto::UpdateProjectRequest,
dto::ProjectMemberDataRequest,
dto::ProjectViewResponse,
dto::ProjectMemberViewResponse,
dto::UserSummaryViewResponse,
dto::ProjectListResponse,
```

Append tags:

```rust
(name = "product", description = "Product lifecycle endpoints"),
(name = "project", description = "Project lifecycle endpoints"),
```

Do not add a manual `paths(...)` attribute.

- [ ] **Step 6: Run router and OpenAPI tests**

```bash
cargo test -p aegis-server --lib transport::http::router::tests
cargo test -p aegis-server --lib transport::http::openapi::tests
```

Expected: all pass; `/api/product` and `/api/project` appear once with merged method operations.

- [ ] **Step 7: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/router.rs apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(server): mount project routers in openapi api

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 9: Add startup wiring regression coverage

**Files:**
- Modify: `apps/server/aegis-server/src/run.rs`

- [ ] **Step 1: Add a compile-time signature test**

Inside the existing `run.rs` test module (or a new `#[cfg(test)]` module), lock the helper boundary established in Task 2:

```rust
#[test]
fn project_service_builder_has_expected_signature() {
    let _: fn(
        PgPool,
        Arc<dyn apis::user::UserService>,
    ) -> Arc<dyn apis::project::ProjectService> = build_project_service;
}
```

- [ ] **Step 2: Run the focused test**

```bash
cargo test -p aegis-server --lib run::tests::project_service_builder_has_expected_signature
```

Expected: pass, proving startup retains the project-service trait-object boundary and shared user dependency.

- [ ] **Step 3: Run the binary compile gate**

```bash
cargo check -p aegis-server --all-targets
```

Expected: pass; both production and every test-only `AppState` literal provide `project`.

- [ ] **Step 4: Commit the wiring regression test**

```bash
git add apps/server/aegis-server/src/run.rs
git commit -m "test(server): lock project service startup wiring

Co-Authored-By: Claude <noreply@anthropic.com>"
```

---

### Task 10: Document routes and run the verification gate

**Files:**
- Modify: `apps/server/aegis-server/README.md`

- [ ] **Step 1: Update the route table**

Add all eight routes with these access labels:

```markdown
| POST | `/api/product` | Create product | Bearer (`Root`/`Admin`) |
| GET | `/api/product` | List products | Bearer |
| GET | `/api/product/{code}` | Get product by code | Bearer |
| PATCH | `/api/product/{code}` | Update product | Bearer (`Root`/`Admin`) |
| POST | `/api/project` | Create project | Bearer (`Root`/`Admin`) |
| GET | `/api/project` | List projects | Bearer |
| GET | `/api/project/{code}` | Get project by code | Bearer |
| PATCH | `/api/project/{code}` | Update project | Bearer (`Root`/`Admin`) |
```

Add: “Bearer routes require `Authorization: Bearer <access-token>`. Product and project writes accept only `root` or `admin`; other authenticated roles receive `403 forbidden`. Reads accept every authenticated role.”

- [ ] **Step 2: Format the workspace**

```bash
cargo fmt --all
cargo fmt --all -- --check
```

Expected: check exits 0.

- [ ] **Step 3: Run targeted tests**

```bash
cargo test -p aegis-server --lib transport::http::dto
cargo test -p aegis-server --lib transport::http::error
cargo test -p aegis-server --lib transport::http::project
cargo test -p aegis-server --lib transport::http::router
cargo test -p aegis-server --lib transport::http::openapi
```

Expected: all pass.

- [ ] **Step 4: Run crate verification**

```bash
cargo test -p aegis-server
cargo test -p apis
cargo test -p project
cargo clippy -p aegis-server --all-targets -- -D warnings
```

Expected: all commands exit 0. Ignored live-database tests remain ignored; no database is required.

- [ ] **Step 5: Verify route documentation and diff quality**

```bash
rg -n '/api/(product|project)' apps/server/aegis-server/README.md
git diff --check
git status --short
```

Expected: all eight routes are listed, `git diff --check` emits nothing, and status contains only intended implementation/doc changes.

- [ ] **Step 6: Commit documentation and final formatting**

```bash
git add apps/server/aegis-server/README.md apps/server/aegis-server/src apps/server/aegis-server/Cargo.toml Cargo.lock
git commit -m "docs(server): document project routes

Co-Authored-By: Claude <noreply@anthropic.com>"
```

## Acceptance Checklist

- [ ] Exactly eight code-based routes exist; no numeric-ID HTTP route exists.
- [ ] Every route rejects missing/invalid bearer authentication with `401`.
- [ ] Every read accepts `General`, `Admin`, and `Root`.
- [ ] Every create/update accepts `Admin` and `Root` and rejects `General` with `403 forbidden`.
- [ ] Update handlers resolve path code to the internal ID before calling update.
- [ ] Missing update membership remains `None`; explicit `{}` becomes `Some(empty)`.
- [ ] Project service errors use the approved status/code table.
- [ ] OpenAPI lists all routes, schemas, tags, bearer security, and write `403` responses.
- [ ] Startup constructs the real project service using the shared pool and user service.
- [ ] README and all verification commands pass.
