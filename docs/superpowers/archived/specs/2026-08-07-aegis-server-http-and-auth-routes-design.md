# aegis-server HTTP Server & Auth Routes Design

## Goal

Build the `aegis-server` HTTP server in `apps/server/aegis-server` on top
of `axum 0.8` + `utoipa` + `utoipa-axum` + `utoipa-swagger-ui`. Expose the
auth-flow endpoints (`login`, `login-domain`, `refresh`, `logout`) wired to
the `apis::auth::AuthService` port, with the `auth` crate's
`AuthServiceImpl` as the production backend (against Postgres + an
in-memory token-version cache).

The four endpoints are mounted under `/api/auth/*`. A `Bearer`-token
verifier is implemented as an axum extractor (`AuthClaims`) so future
protected handlers can pick up the verified claims by argument — the
`verify` operation is **not** exposed as a public route. Swagger-ui
mounts at `/swagger-ui` and the OpenAPI doc root at
`/api-docs/openapi.json`; neither carries the `/api` prefix.

Wire-level DTOs live in `aegis-server` (with `Serialize`, `Deserialize`,
`ToSchema`). The `apis` crate stays free of `serde` / `utoipa` derives;
handler code translates JSON ↔ apis DTOs at the boundary.

## Non-Goals

- gRPC, tarpc, or any non-HTTP transport. (The `transport/` module is
  laid out so a future `transport/grpc/` slot can land as a sibling, but
  that work is out of scope here.)
- Auto-migrating the schema at startup. Migrations are an ops step
  (matches the auth crate README's current stance).
- The five `AuthService` credential-management methods
  (`find_user_credential_by_code`, `create_user_credential`,
  `update_user_credential`, `remove_user_credential`) — those land as
  admin routes in a follow-up.
- Refresh-token rotation. The usecase refreshes the access token but
  keeps the same refresh token (per the apis trait contract).
- Authn middleware that scopes a whole sub-tree. Today's four routes
  don't need a logged-in user; the `AuthClaims` extractor exists for
  future handlers.

## Architecture

`aegis-server` is a thin binary crate. Cross-cutting concerns (`config`,
`state`) sit above the transport boundary; HTTP-specific code lives
under `transport/http/`. The crate uses `src/<module>.rs` +
`src/<module>/` directory style — no `mod.rs`.

```
apps/server/aegis-server/
├── Cargo.toml
├── README.md                          # env vars, run command, swagger URL
└── src/
    ├── main.rs                         # thin: parse env, init tracing, call run
    ├── lib.rs                          # pub async fn run(Config)
    ├── config.rs                       # env-loaded Config
    ├── state.rs                        # AppState
    └── transport/
        ├── transport.rs                # pub mod http; pub use http::router
        └── http/
            ├── http.rs                 # pub mod auth, dto, error, healthz,
            │                          #     openapi, router;
            │                          # pub use router::router
            ├── router.rs               # Router composition (nest("/api", …)
            │                          #   + SwaggerUi at root)
            ├── auth.rs                 # 4 POST handlers
            ├── auth/
            │   └── middleware.rs       # AuthClaims FromRequestParts extractor
            ├── dto.rs                  # wire-level DTOs (Serialize + Deserialize
            │                          #   + ToSchema)
            ├── error.rs                # ApiError + ErrorBody + IntoResponse
            ├── healthz.rs              # GET /healthz
            └── openapi.rs              # utoipa OpenApi builder
```

`transport.rs` and `http.rs` contain no logic — both are thin re-export
seams so `use aegis_server::transport::http::router` (or
`aegis_server::transport::router` via the outer re-export) resolves
cleanly. Future transports (`grpc/`, `cli/`) sit as siblings of
`http/` inside `transport/`.

## Dependencies

All added to `[workspace.dependencies]` (or already there after commit
`876ee2d`):

```toml
# workspace additions
serde      = { version = "1", features = ["derive"] }
serde_json = "1"

# aegis-server additions
axum                = { workspace = true }
tower               = { workspace = true }
tower-http          = { workspace = true, features = ["trace"] }
tokio               = { workspace = true, features = ["macros", "rt-multi-thread", "signal"] }
utoipa              = { workspace = true }
utoipa-axum         = { workspace = true }
utoipa-swagger-ui   = { workspace = true }
tracing             = { workspace = true }
tracing-subscriber  = { workspace = true }
sqlx                = { workspace = true }
serde               = { workspace = true }
serde_json          = { workspace = true }
chrono              = { workspace = true }
thiserror           = { workspace = true }
async-trait         = { workspace = true }
dotenvy             = { workspace = true }
auth                = { path = "../../lib/crates/auth" }
apis                = { path = "../../lib/crates/apis" }
user                = { path = "../../lib/crates/user" }
```

`aegis-server` itself does not add anything crate-specific; every
dependency is inherited from the workspace or a path-dep. `chrono`
appears because the `apps::server::aegis_server` is not the only
consumer — handlers will produce `tracing` events with timestamps and
`serde` deserializes `chrono` types if any wire DTO ever carries one
(currently none do, but it stays pinned at the workspace root for
consistency with the lib crates).

## Config (`src/config.rs`)

```rust
pub struct Config {
    pub database_url: String,
    pub signing_key: Vec<u8>,    // raw bytes; never logged
    pub bind_addr: std::net::SocketAddr,
    pub access_ttl: std::time::Duration,
    pub refresh_ttl: std::time::Duration,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> { /* panic message on missing var */ }
}
```

Env vars:

| var                      | required | default        | notes                                              |
|--------------------------|----------|----------------|----------------------------------------------------|
| `AEGIS_DATABASE_URL`     | yes      | —              | Postgres URL; `Config::from_env` errors on missing |
| `AEGIS_AUTH_SIGNING_KEY` | yes      | —              | hex-encoded; ≥32 bytes decoded; never logged       |
| `AEGIS_HTTP_BIND`        | no       | `0.0.0.0:8080` | `SocketAddr`                                       |
| `AEGIS_ACCESS_TTL_SECS`  | no       | `900` (15 m)   | `u64` → `Duration::from_secs`                      |
| `AEGIS_REFRESH_TTL_SECS` | no       | `604800` (7 d) | `u64` → `Duration::from_secs`                      |

`ConfigError` is a `#[derive(thiserror::Error)]` enum covering missing
required var, invalid `SocketAddr`, invalid hex / too-short signing key,
non-numeric TTL.

## State (`src/state.rs`)

```rust
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
}
```

`AppState: Clone` is required because axum extracts it by `Clone` (one
per worker task). Both inner fields are `Arc`, so the clone is cheap.

## Bootstrap (`src/lib.rs::run`)

`pub async fn run(config: Config) -> anyhow::Result<()>` owns the
wiring from `Config` to a running `tokio::net::TcpListener`:

1. `PgPool::connect(&config.database_url).await?` with
   `sqlx::postgres::PgPoolOptions::new().max_connections(10)`.
2. Build the three Postgres-backed repos: `user::UserRepo::new(pool.clone())`,
   `auth::UserCredentialsRepo::new(pool.clone())`,
   `auth::DomainIdentityRepo::new(pool)`.
3. Build the user `UserServiceImpl` (re-exported from the `user` crate
   as `user::UserServiceImpl`) from the `UserRepo`.
4. Build the auth `UserServiceImpl` (the apis→domain adapter at
   `auth::UserServiceImpl`, re-exported from the `auth` crate) from the
   user `Arc<dyn UserService>`.
5. Build `Arc<auth::InMemoryTokenVersionCache>`.
6. Build `AuthUsecase` from `AuthUsecaseConfig { credentials, identities,
   user_service, cache, signing_key, access_ttl, refresh_ttl }`.
7. Wrap as `Arc<dyn apis::auth::AuthService>` via `AuthServiceImpl::new(usecase)`.
8. Construct `AppState { auth, user }`.
9. `let app = transport::router(state);`
10. `axum::serve(TcpListener::bind(config.bind_addr).await?, app).await?`

`main.rs` is ~15 lines: load `.env` via `dotenvy::dotenv().ok()`, init
`tracing-subscriber::fmt().json()`, parse `Config::from_env()`,
`tokio::main { aegis_server::run(config).await }`, exit code from
`Result`.

## Routes & Middleware

### Route table

| Method | Path                          | Handler              | AuthService call            | Body in                                               | Body out                                       |
|--------|-------------------------------|----------------------|-----------------------------|-------------------------------------------------------|------------------------------------------------|
| POST   | `/api/auth/login`             | `auth::login`        | `login_with_password`       | `LoginRequest { code, password }`                     | `TokenPairResponse { access_token, refresh_token }` |
| POST   | `/api/auth/login-domain`      | `auth::login_domain` | `login_with_domain_user_info` | `LoginDomainRequest { code, domain_name, hostname, sid }` | `TokenPairResponse`                            |
| POST   | `/api/auth/refresh`           | `auth::refresh`      | `refresh`                   | `RefreshRequest { refresh_token }`                    | `AccessTokenResponse { access_token }`         |
| POST   | `/api/auth/logout`            | `auth::logout`       | `logout`                    | `LogoutRequest { refresh_token }`                     | `LogoutResponse {}`                            |
| GET    | `/api/healthz`                | `healthz::healthz`   | —                           | —                                                     | `"ok"` (text/plain)                            |
| GET    | `/api-docs/openapi.json`      | utoipa-axum          | —                           | —                                                     | OpenAPI v3 JSON                                |
| GET    | `/swagger-ui`                 | utoipa-swagger-ui    | —                           | —                                                     | HTML                                           |

API paths get `/api`; swagger-ui and the OpenAPI doc stay at root.

### `AuthClaims` extractor (`transport/http/auth/middleware.rs`)

`verify` is not a route — it is an axum extractor that future
protected handlers can take as an argument. Reading the
`Authorization: Bearer <token>` header, calling
`AuthService::verify`, and exposing the resulting `AuthClaims`:

```rust
pub struct AuthClaims(pub apis::auth::AuthClaims);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = ApiError;
    async fn from_request_parts(parts: &mut Parts, state: &AppState)
        -> Result<Self, Self::Rejection>
    { /* see section "Error mapping" for the four rejection paths */ }
}
```

Four rejection paths all surface as `ApiError` with
`code = "token_verification_failed"` and HTTP `401`:

1. No `Authorization` header.
2. Header value not parseable as `&str`.
3. Header value does not begin with `Bearer `.
4. `AuthService::verify` returns any `AuthApiError::Verification` /
   `InvalidCredentials` / `Inactive` / `NotFound`.

### Router composition (`transport/http/router.rs`)

```rust
pub fn router(state: AppState) -> axum::Router {
    use axum::routing::get;
    use utoipa_swagger_ui::SwaggerUi;

    let (api_router, api) = OpenApiRouter::new()
        .nest("/auth", auth::router())
        .route("/healthz", get(healthz::healthz))
        .split_for_openapi();

    let api_scope = axum::Router::new().merge(api_router);

    axum::Router::new()
        .nest("/api", api_scope)
        .merge(SwaggerUi::new("/swagger-ui")
            .url("/api-docs/openapi.json", api))
        .with_state(state)
        .layer(tower_http::trace::TraceLayer::new_for_http())
}
```

State is attached exactly once at the top-level `Router`. `SwaggerUi`
does not need state — it serves static HTML/JS that fetches the
`openapi.json` document directly. `auth::router()` is a sub-`Router`
that mounts the four POST handlers under `/auth/*`; each handler uses
`#[axum::debug_handler(state = AppState)]` so extractor mismatches
show up at compile / first request.

## Wire DTOs (`transport/http/dto.rs`)

One wire type per apis request / response, with `Serialize`,
`Deserialize`, `ToSchema`. Field names are `snake_case` to match the
apis surface; utoipa documents the JSON shape via the `ToSchema`
derive. `Role` is duplicated here (with the same three-variant enum)
because the apis `Role` deliberately has no serde derives.

```rust
#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest { pub code: String, pub password: String }

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginDomainRequest {
    pub code: String, pub domain_name: String,
    pub hostname: String, pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest { pub refresh_token: String }

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest { pub refresh_token: String }

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TokenPairResponse {
    pub access_token: String, pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessTokenResponse { pub access_token: String }

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthClaimsResponse {
    pub code: String, pub role: Role, pub token_version: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub enum Role { Root, Admin, General }
```

A single 3-arm `match` maps `apis::user::Role` ↔ `dto::Role`; that
function lives next to the handlers in `auth.rs`.

## Error Mapping (`transport/http/error.rs`)

Single `ApiError` newtype wraps `AuthApiError` (the only error source
today) and implements `IntoResponse`, so every handler returns
`Result<Json<T>, ApiError>`.

```rust
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable detail. Safe to log; safe to show to end users.
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
pub struct ApiError(pub AuthApiError);

impl From<AuthApiError> for ApiError {
    fn from(err: AuthApiError) -> Self { Self(err) }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(code = self.code(), error = %self.0, "api error");
        }
        (status, Json(ErrorBody {
            code: self.code().to_string(),
            message: self.0.to_string(),
        })).into_response()
    }
}
```

Mapping table:

| `AuthApiError` variant       | HTTP status                  | `ErrorBody.code`              |
|-----------------------------|------------------------------|-------------------------------|
| `Validation(String)`        | `400 Bad Request`            | `validation_failed`           |
| `NotFound`                  | `404 Not Found`              | `not_found`                   |
| `Inactive`                  | `403 Forbidden`              | `user_inactive`               |
| `InvalidCredentials`        | `401 Unauthorized`           | `invalid_credentials`         |
| `Verification(String)`      | `401 Unauthorized`           | `token_verification_failed`   |
| `DuplicateCode(String)`     | `409 Conflict`               | `duplicate_code`              |
| `Signing(String)`           | `500 Internal Server Error`  | `signing_failed`              |
| `Repository(String)`        | `500 Internal Server Error`  | `repository_error`            |

`ErrorBody` is registered in `openapi()`'s `components(schemas(…))` so
swagger-ui documents the error shape.

## OpenAPI Document (`transport/http/openapi.rs`)

```rust
#[derive(utoipa::OpenApi)]
#[openapi(
    info(title = "aegis-server", version = "0.1.0"),
    paths(
        auth::login, auth::login_domain, auth::refresh,
        auth::logout, healthz::healthz,
    ),
    components(schemas(
        dto::LoginRequest, dto::LoginDomainRequest,
        dto::RefreshRequest, dto::LogoutRequest,
        dto::TokenPairResponse, dto::AccessTokenResponse,
        dto::LogoutResponse, dto::AuthClaimsResponse,
        dto::Role, error::ErrorBody,
    )),
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi { ApiDoc::openapi() }
```

`AuthClaims` is intentionally not listed in `paths(…)` because it is
an extractor, not a handler.

## Testing

Six test layers, modeled on the lib-crate-development guideline
(adjusted for a binary server crate). All non-DB tests run on a plain
`cargo test -p aegis-server`; DB integration tests are `#[ignore]`-gated.

| # | Layer                       | Where                                              | What it covers                                              |
|---|-----------------------------|----------------------------------------------------|-------------------------------------------------------------|
| 1 | Error-mapping unit          | `src/transport/http/error.rs` (inline)             | Each `AuthApiError` variant → right `StatusCode` + `ErrorBody.code` |
| 2 | Handler unit                | `src/transport/http/auth.rs` (inline)              | Each handler: parse JSON, call service, return right status + body |
| 3 | `AuthClaims` extractor unit | `src/transport/http/auth/middleware.rs` (inline)   | Four rejection paths + happy path                           |
| 4 | OpenAPI document            | `src/transport/http/openapi.rs` (inline)           | `openapi()` paths contain the 5 handlers; schemas list every wire DTO + `ErrorBody` |
| 5 | Public-API compile          | `tests/public_api.rs`                              | Locks every `aegis_server::*` re-export                     |
| 6 | Live-DB integration         | `tests/integration_auth.rs` (`#[ignore]`)          | Real `Config::from_env` + migrations + real `POST` round-trips via `tower::ServiceExt::oneshot` |

`FakeAuthService` is local to each test module that needs one
(matches the auth crate's pattern of duplicating the fake per file
rather than sharing via `tests/common`). Handler tests build the full
`Router` and drive it with `tower::ServiceExt::oneshot`:

```rust
let state = AppState {
    auth: Arc::new(FakeAuthService::returning_login(Ok(TokenPair {
        access_token: "a".into(), refresh_token: "r".into(),
    }))),
    user: Arc::new(FakeUserService),
};
let app = transport::router(state);
let res = app.oneshot(
    Request::builder()
        .method("POST")
        .uri("/api/auth/login")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"code":"u1","password":"p"}"#))
        .unwrap()
).await.unwrap();
assert_eq!(res.status(), StatusCode::OK);
```

**Coverage goals** (so the next reviewer can verify the suite is
adequate):

- Every `AuthApiError → StatusCode` arm tested.
- Every handler has a happy-path + at least one error-path test.
- `AuthClaims` extractor tested for all four rejection paths plus the
  happy path.
- OpenAPI document contains exactly the expected 5 paths and the full
  schema set.
- Public-API compile test covers every re-export.

**Verification gate** (mirrors the lib-crate guideline):

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-server --all-targets -- -D warnings
cargo test -p aegis-server
cargo doc -p aegis-server --no-deps
# DB tests:
AEGIS_DATABASE_URL=… cargo test -p aegis-server -- --ignored --test-threads=1
```

## Public API Surface

`aegis-server` is a binary crate but exposes a small library surface
for the binary entry point and the public-API compile test:

- `aegis_server::run(Config) -> anyhow::Result<()>`
- `aegis_server::Config` + `Config::from_env()`
- `aegis_server::ConfigError`
- `aegis_server::AppState`
- `aegis_server::transport::router(AppState) -> axum::Router`
- `aegis_server::transport::http::router(AppState) -> axum::Router`
  (re-export of the above)
- `aegis_server::transport::http::auth::AuthClaims`
- `aegis_server::transport::http::dto::{LoginRequest, LoginDomainRequest,
  RefreshRequest, LogoutRequest, TokenPairResponse, AccessTokenResponse,
  LogoutResponse, AuthClaimsResponse, Role}`
- `aegis_server::transport::http::error::ErrorBody`

`tests/public_api.rs` enumerates every name above and locks the
constructor signatures (`Config::from_env`, `AppState { auth, user }`,
`AuthClaims` extractor) so a refactor that breaks the documented
surface fails at `cargo test -p aegis-server`.

## What Stays Untouched

- `apis::auth::*` — port trait and DTOs are not modified. The
  reverted PR #13 (`feat/apis_serde-and-utopia`) confirmed the team's
  current stance that wire-format derives do not belong on the apis
  crate.
- `auth::AuthServiceImpl`, `AuthUsecase`, `AuthUsecaseConfig`,
  `InMemoryTokenVersionCache`, `UserCredentialsRepo`, `DomainIdentityRepo`
  — all production adapters stay exactly as they are. The server crate
  is purely a consumer.
- `user::UserRepo`, `UserServiceImpl`, `UserUsecase`.
- The `auth` and `user` migration files.
- `apps/desktop/aegis-desktop` and the frontend packages.

## Out of Scope

- The five credential-management routes
  (`find_user_credential_by_code`, `create_user_credential`,
  `update_user_credential`, `remove_user_credential`).
- A protected route that consumes `AuthClaims` (the extractor is
  built so future handlers can, but no current route uses it).
- Auto-migration on startup (migrations are an ops step).
- Refresh-token rotation.
- gRPC / tarpc / WebSocket transports.
- Authentication middleware that gates a whole sub-tree
  (`Router::route_layer` / `from_fn_with_state`).
- Production hardening: graceful shutdown, request body size limits,
  rate limiting, CORS.