# aegis-server User Router Design

## Goal

Expose the `apis::user::UserService` port over HTTP under
`/api/user/*` inside `apps/server/aegis-server`. Every route requires a
valid `Authorization: Bearer <token>` header verified through the
existing `AuthClaims` extractor; the verified claims are used only to
gate the route, not to authorize it (no role checks at this stage).

The four endpoints — `create`, `list`, `get_by_code`, `update` — back
onto the four corresponding `UserService` methods. `get_by_id` is not
exposed: the router exposes a code-based resource, matching the
unique identifier `User.code`. The user's `id` is an internal numeric
handle that stays out of the wire contract; the `update` handler
resolves the URL's `{code}` to an `id` via a single `get_by_code`
lookup before invoking the usecase.

This change keeps `user::UserServiceImpl` (Postgres-backed) as the
production backend and is purely a transport-layer addition on top of
the existing `apps/server/aegis-server` skeleton. It also refactors
`ApiError` from a single-type newtype into an enum so handlers in two
sub-routers (`auth` + `user`) can share the same `?` path without
losing exhaustiveness.

## Non-Goals

- The five `UserService` credential-management methods on
  `AuthService` (`find_user_credential_by_code`, …) — out of scope;
  they live behind a separate admin surface.
- Role-based authorization. `AuthClaims.role` is decoded but not
  enforced. A future `AdminClaims` extractor (gating `create` /
  `update`) lands separately if and when needed.
- Exposing `get_by_id` over HTTP. The numeric `id` is treated as an
  internal handle; callers go through `code`.
- User-creation that also mints a password. `create` is metadata-only
  (`code`, `name`, `role`); credential setup is a separate flow that
  will land in the credential-management follow-up.
- Live-DB integration tests against `UserRepo`. User CRUD against
  Postgres is out of scope for this PR; the trait stays mockable for
  the new handler tests.
- Pagination on `list` — `Vec<UserViewResponse>` for now; pagination
  lands when the surface outgrows it.
- Any change to `state.rs` or `run.rs` — `AppState.user` and
  `build_user_service` are already in place.

## Architecture

The user sub-router mirrors the existing auth sub-router so the two
read as siblings. New `pub mod user;` is added to
`transport/http.rs` next to `pub mod auth;`.

```
apps/server/aegis-server/
└── src/
    └── transport/
        └── http/
            ├── http.rs                 # +pub mod user;
            ├── router.rs               # +.nest("/api/user", user::router());
            │                           # +integration-test updates
            ├── dto.rs                  # +CreateUserRequest, +UpdateUserRequest,
            │                           # +PathCode, +UserViewResponse,
            │                           # +UserListResponse
            ├── error.rs                # ApiError refactor: enum variants
            │                           # (Auth / User) with helper tables
            ├── openapi.rs              # +user DTO schemas, +user tag
            ├── auth/                   # unchanged — From<AuthApiError> still works
            └── user/                   # new
                ├── user.rs             # pub mod handlers; pub mod router;
                ├── router.rs           # OpenApiRouter composition
                └── handlers.rs         # create / list / get_by_code / update + tests
```

No crate-dependency changes: every symbol used (`axum`, `utoipa`,
`utoipa-axum`, `serde`, `chrono`, `async-trait`, `thiserror`,
`apis`, `user`, `auth`) is already in `aegis-server`'s
`Cargo.toml`.

## Routes

| Method | Path                  | Handler (in `transport/http/user/handlers.rs`) | `UserService` call | Auth |
|--------|-----------------------|------------------------------------------------|--------------------|------|
| POST   | `/api/user`           | `create`                                       | `create`           | JWT  |
| GET    | `/api/user`           | `list`                                         | `list`             | JWT  |
| GET    | `/api/user/{code}`    | `get_by_code`                                  | `get_by_code`      | JWT  |
| PATCH  | `/api/user/{code}`    | `update`                                       | `get_by_code` → `update` | JWT |

All four advertise `security: [{ BearerAuth: [] }]` in the generated
OpenAPI document, matching the auth router's `refresh` / `logout`
treatment.

The `update` handler resolves `{code}` to a `UserView` via
`get_by_code` and threads the resulting `id` into
`apis::user::UpdateUserRequest`. This costs one extra read on every
PATCH and acts as a 404 check. The wire `UpdateUserRequest` has no
`id` field; the field is internal-only.

## Wire DTOs (`transport/http/dto.rs`)

All five new DTOs carry `Serialize`, `Deserialize`, `ToSchema`. Field
names are `snake_case` to match the apis surface.

```rust
#[derive(Serialize, Deserialize, ToSchema)]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,                       // reuses existing dto::Role
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
    // no id — derived from the path
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct PathCode {
    pub code: String,
}

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

#[derive(Serialize, Deserialize, ToSchema)]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}
```

`From<apis::user::UserView> for UserViewResponse` lives next to the
DTO and parallels the existing `Role` conversions. `chrono` is
already in `Cargo.toml`.

## Handler Signatures

```rust
pub async fn create(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<UserViewResponse>), ApiError>;
// returns 201 Created

pub async fn list(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<UserListResponse>, ApiError>;

pub async fn get_by_code(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
) -> Result<Json<UserViewResponse>, ApiError>;

pub async fn update(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathCode { code }): Path<PathCode>,
    Json(req): Json<UpdateUserRequest>,
) -> Result<Json<UserViewResponse>, ApiError>;
```

Each handler carries a `#[utoipa::path]` annotation with
`tag = "user"`, `security(("BearerAuth" = []))`, and the relevant
request/response schemas. The `responses(...)` block lists 200 / 201 /
400 / 401 / 404 / 409 / 500 with `ErrorBody` for the error codes.

`_claims` is bound to `AuthClaims` purely so the extractor runs and
rejects the request before the handler body executes. The handler body
ignores the value (analogous to `auth::refresh` / `auth::logout`).

## Router Composition (`transport/http/user/router.rs`)

Same shape as `auth/router.rs` — one `routes!` call per handler to
avoid the "Overlapping method route" panic:

```rust
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create))
        .routes(routes!(handlers::list))
        .routes(routes!(handlers::get_by_code))
        .routes(routes!(handlers::update))
}
```

The top-level `transport/http/router.rs` adds
`.nest("/api/user", user::router())` to the existing
`OpenApiRouter::with_openpi(ApiDoc::openapi()).nest("/api/auth", …)`
chain.

## Error Model Refactor (`transport/http/error.rs`)

`ApiError` becomes an enum with `#[from]`-derived conversions from
both apis error types. The mapping tables move into private helpers
so `status()` / `code()` stay table-shaped:

```rust
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("{0}")]
    Auth(#[from] apis::auth::AuthApiError),
    #[error("{0}")]
    User(#[from] apis::user::UserApiError),
}

fn auth_status(e: &apis::auth::AuthApiError) -> StatusCode { /* 8 arms */ }
fn auth_code(e: &apis::auth::AuthApiError) -> &'static str { /* 8 arms */ }
fn user_status(e: &apis::user::UserApiError) -> StatusCode { /* 5 arms */ }
fn user_code(e: &apis::user::UserApiError) -> &'static str { /* 5 arms */ }

impl ApiError {
    pub fn status(&self) -> StatusCode {
        match self {
            Self::Auth(e) => auth_status(e),
            Self::User(e) => user_status(e),
        }
    }
    pub fn code(&self) -> &'static str {
        match self {
            Self::Auth(e) => auth_code(e),
            Self::User(e) => user_code(e),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(code = self.code(), error = %self, "api error");
        }
        (status, Json(ErrorBody {
            code: self.code().to_string(),
            message: self.to_string(),
        })).into_response()
    }
}
```

`UserApiError` mapping (auth mapping unchanged):

| Variant              | Status | `ErrorBody.code` |
|----------------------|--------|------------------|
| `Validation(_)`      | 400    | `validation_failed` |
| `NotFound`           | 404    | `not_found` |
| `DuplicateCode(_)`   | 409    | `duplicate_code` |
| `Hashing(_)`         | 500    | `hashing_failed` |
| `Repository(_)`      | 500    | `repository_error` |

Every existing `?` on `AuthApiError` in `auth/handlers.rs` and
`auth/middleware.rs` continues to compile via the new `#[from]` —
no edits there.

## OpenAPI (`transport/http/openapi.rs`)

`components(schemas(...))` adds:

```
dto::CreateUserRequest,
dto::UpdateUserRequest,
dto::UserViewResponse,
dto::UserListResponse,
dto::PathCode,
```

`tags(...)` adds:

```
(name = "user", description = "User CRUD endpoints"),
```

`SecurityAddon` (the `BearerAuth` scheme registration) stays as-is.
Handler `#[utoipa::path]` annotations reference
`security(("BearerAuth" = []))` so all four user routes advertise
the requirement in swagger-ui.

The `paths(...)` list is intentionally left empty: `OpenApiRouter`
+ `routes!()` auto-collect paths from `#[utoipa::path]` annotations
and applies the `nest("/api/user", …)` prefix.

## Testing

| # | Layer | Where | What it covers |
|---|-------|-------|----------------|
| 1 | DTO round-trip | `transport/http/dto.rs` (inline) | `CreateUserRequest`, `UpdateUserRequest`, `PathCode`, `UserViewResponse`, `UserListResponse` serialize / deserialize as expected. |
| 2 | Error-mapping unit | `transport/http/error.rs` (inline) | Existing auth tests stay; add `UserApiError` tests for each of the 5 variants. |
| 3 | Handler unit | `transport/http/user/handlers.rs` (inline) | Mock `UserService` + Mock `AuthService` (for `AuthClaims::verify`); for each handler: happy path, one error path, and a 401-on-missing-JWT case. The `update` handler test asserts the in-test `get_by_code` → `update` translation. |
| 4 | OpenAPI document | `transport/http/openapi.rs` (inline) | Existing schemas test extended for the 5 new schemas; existing tag-presence test extended for the `user` tag. |
| 5 | Router integration | `transport/http/router.rs` (inline) | Existing tests stay; add `user_route_returns_401_without_jwt`. Extend the `openapi_json_returns_200_with_valid_doc` test to assert `/api/user` (POST + GET), `/api/user/{code}` (GET + PATCH) are present, and that all four reference `security[0].BearerAuth`. |

Mock `UserService` follows the same per-module duplication pattern
already used for `MockAuth` / `NullUserService` across the auth
router tests — one inline per test module, no shared `tests/common`.

`NullUserService` already lives in `router.rs::tests` and
`auth/handlers.rs::tests` with `unimplemented!()` stubs; the new
`user/handlers.rs::tests` swaps one of those for a configurable
`MockUserService` whose methods return either a configured
`UserView` / `Vec<UserView>` or a configured `UserApiError`.

No live-DB integration tests. The existing `integration_auth.rs`
remains auth-only.

## File-by-File Change List

**New (3):**
- `apps/server/aegis-server/src/transport/http/user.rs`
- `apps/server/aegis-server/src/transport/http/user/router.rs`
- `apps/server/aegis-server/src/transport/http/user/handlers.rs`

**Modified (5):**
- `apps/server/aegis-server/src/transport/http.rs` — `+pub mod user;`
- `apps/server/aegis-server/src/transport/http/router.rs` — `+.nest("/api/user", user::router())`; update integration test
- `apps/server/aegis-server/src/transport/http/dto.rs` — append 5 user DTOs + `From<apis::user::UserView>`
- `apps/server/aegis-server/src/transport/http/error.rs` — `ApiError` → enum variants; add 4 helpers; `IntoResponse` body unchanged
- `apps/server/aegis-server/src/transport/http/openapi.rs` — register 5 new schemas + `user` tag

**Unchanged (verified):**
- `apps/server/aegis-server/src/state.rs` — `AppState.user` already present
- `apps/server/aegis-server/src/run.rs` — `build_user_service` already wires `Arc<dyn UserService>`
- `apps/server/aegis-server/src/transport/http/auth/{handlers,middleware}.rs` — existing `?` on `AuthApiError` keeps compiling via `#[from]`

**Implementation order** (smallest blast-radius first):
1. `error.rs` refactor (additive — auth `?` paths keep working).
2. `dto.rs` appends + round-trip tests.
3. `openapi.rs` schema + tag registration.
4. `transport/http/user/{router,handlers}.rs` (new module).
5. `transport/http.rs` + `transport/http/router.rs` wiring + integration-test updates.

## Verification Gate

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-server --all-targets -- -D warnings
cargo test -p aegis-server
cargo doc -p aegis-server --no-deps
```

The lib-crate doctest in `src/lib.rs` that pins the public surface
must continue to resolve. No new public-API re-exports are added by
this change; the existing surface is unchanged.

## Out of Scope

- Credential-management routes (`create_user_credential`, …).
- Role-based authorization on user endpoints.
- Pagination on `list`.
- Live-DB integration tests for the user router.
- Auto-migration on startup.
- Production hardening (rate limits, request body limits, CORS).