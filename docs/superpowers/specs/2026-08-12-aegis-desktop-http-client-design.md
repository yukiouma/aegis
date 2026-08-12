# aegis-desktop HTTP Client Design

## Goal

Build an outbound HTTP client in the Tauri Rust backend at
`apps/desktop/aegis-desktop/src-tauri` that calls every endpoint exposed
by the `aegis-server` HTTP router, persist auth tokens via
`tauri_plugin_store`, transparently refresh on `401`, and surface the
results as `#[tauri::command]` shims the React frontend can call with
`@tauri-apps/api/core::invoke`.

This delivers all of the following surfaces:

- One Tauri command per server route, covering the full catalog
  (`auth`, `user`, `product`, `project`, `user-credential`, `healthz`).
- Persistent access + refresh tokens, written to `tauri_plugin_store`
  on successful `login` / `login-domain` / `refresh` and cleared on
  logout.
- A single Bearer-authenticated request path that auto-refreshes once
  on `401` (without leaking the retry to the frontend) and surfaces
  structured `ApiError` values.
- A typed TS API surface under `apps/desktop/aegis-desktop/src/api/`
  that mirrors each command 1:1.

The base URL is fixed at compile time via the env var
`AEGIS_SERVER_URL` (default `http://localhost:8080`). The
`login-domain` command reads the OS identity (domain name, hostname,
SID) from the workspace `windows-utils` crate and never asks the
frontend for it.

## Non-Goals

- **Reusing the `apis` port-trait crate.** The existing
  `lib/crates/apis::auth::AuthService` / `::UserService` /
  `::ProjectService` traits describe a slightly different shape than
  the server's HTTP routes (`CreateUserRequest` has an extra `id`,
  `register_user` lives on the auth trait only, etc.). Wiring the
  desktop against those traits would force serde derives onto
  shared abstractions, and adds a translation layer that buys nothing
  for an outbound HTTP client. The desktop's `http::*` modules are a
  direct outbound mirror of the server routes — future reuse can land
  via a refactor of the apis crate.
- **Refresh-token rotation.** The server's `/api/auth/refresh` returns
  only `access_token` today; the client overwrites the stored
  `access_token` and preserves the existing `refresh_token`. If the
  server is later changed to rotate `refresh_token`, the client will
  pick that up automatically without code change.
- **JWT-parsing or any other token introspection.** The client treats
  tokens as opaque strings; "is the access token still valid?" is
  answered exclusively by whether the server returns `401`.
- **A live-server integration test.** No `tests/` integration harness
  spins up `aegis-server` today; tests stay focused on data shapes
  and `wiremock`-stubbed HTTP.
- **Role-based UI gating.** The client returns `ApiError::Http { code:
  "forbidden", .. }` for `403`; the frontend decides how to render
  it. We do not add an extra `Role`-aware wrapper layer in the
  client.
- **Cross-compile support for non-Windows targets.** The
  `windows-utils` dependency forces the desktop crate to fail
  `cargo check` on Linux/macOS. Callers wrap that dependency in
  `#[cfg(target_os = "windows")]` so non-Windows targets return a
  clear `ApiError::NotImplemented` from `loginDomain` instead of
  failing the whole build.

## Architecture

`src-tauri` gains an outbound HTTP layer that knows nothing about
Tauri, plus a thin command layer that does. The two layers are kept in
separate subtrees so the HTTP layer can be tested without booting a
Tauri runtime.

```
apps/desktop/aegis-desktop/src-tauri/
├── Cargo.toml                          # add reqwest, tokio, windows-utils,
│                                       #   thiserror, async-trait, wiremock (dev)
├── build.rs                            # bake AEGIS_SERVER_URL into the binary
├── src/
│   ├── main.rs                         # unchanged
│   ├── lib.rs                          # Builder + .manage(HttpClient) +
│                                       #   invoke_handler (registers commands)
│   ├── http.rs                         # module root: re-exports
│   ├── http/
│   │   ├── client.rs                   # HttpClient + request() + retry
│   │   │                               # TokenStore trait, TauriStore,
│   │   │                               # MemoryStore
│   │   ├── config.rs                   # BASE_URL const (env!), NO_AUTH_PATHS
│   │   ├── dto.rs                      # ApiError, ErrorBody, Role
│   │   ├── auth.rs                     # login/login_domain/refresh/logout +
│   │   │                               #   wire DTOs
│   │   ├── user.rs                     # user CRUD + wire DTOs
│   │   ├── product.rs                  # product CRUD + wire DTOs
│   │   ├── project.rs                  # project CRUD + ProjectMemberData +
│   │   │                               #   wire DTOs
│   │   ├── user_credential.rs          # register/update_user_credential +
│   │   │                               #   wire DTOs
│   │   └── healthz.rs                  # GET /healthz
│   ├── commands.rs                     # module root
│   ├── commands/
│   │   ├── auth.rs                     # #[tauri::command] shims -> http::auth
│   │   ├── user.rs                     # ditto
│   │   ├── product.rs                  # ditto
│   │   ├── project.rs                  # ditto
│   │   ├── user_credential.rs          # ditto
│   │   └── healthz.rs                  # ditto
│   └── system.rs                       # module root
│   └── system/
│       └── identity.rs                 # current() -> Result<Identity, Err>
│                                       #   platform-gated
│   └── tests/                          # (none — see Testing strategy)
└── ui/                                 # (existing React app, unchanged)
```

The crate uses the sibling `src/<module>.rs` + `src/<module>/`
directory style — no `mod.rs`.

## Configuration

### `AEGIS_SERVER_URL` (compile-time env var)

`src-tauri/build.rs`:

```rust
fn main() {
    let url = std::env::var("AEGIS_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8080".into());
    println!("cargo:rustc-env=AEGIS_SERVER_URL={url}");
}
```

`src-tauri/src/http/config.rs`:

```rust
pub const BASE_URL: &str = env!("AEGIS_SERVER_URL");
pub const NO_AUTH_PATHS: &[(&str, &str)] = &[
    ("POST", "/api/auth/login"),
    ("POST", "/api/auth/login-domain"),
    ("GET",  "/healthz"),
    ("POST", "/api/auth/user-credential"),
];
```

`HttpClient::with_base_url(String)` exists for tests; production code
constructs `HttpClient::new(BASE_URL.into(), store)` once in `lib.rs`.

### `NO_AUTH_PATHS` policy

Exactly the four routes the user listed. The client does not attach
`Authorization: Bearer <token>` to those four. Any other path
(including `/api/auth/refresh`) gets a Bearer header if the access
token is present; the server's middleware does not enforce auth on
`refresh`, so a stale Bearer is harmless if attached.

## Dependencies

Added to `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`:

```toml
[dependencies]
# ...existing entries...
tauri            = "2"                                  # already present
tauri-plugin-opener = "2"                                # already present
serde            = { version = "1", features = ["derive"] }  # already present
serde_json       = "1"                                  # already present
tauri-plugin-store = "2"                                # already present

# NEW outbound HTTP + OS identity + error machinery
reqwest          = { version = "0.13", default-features = false,
                     features = ["json", "rustls-tls"] }
tokio            = { workspace = true }
thiserror        = { workspace = true }
async-trait      = { workspace = true }
windows-utils    = { path = "../../../../lib/crates/windows-utils" }

[dev-dependencies]
wiremock         = "0.6"
tokio            = { workspace = true,
                     features = ["macros", "rt-multi-thread", "time"] }
```

`windows-utils` is a path-dep only (not added to workspace deps).
`reqwest` is not added to `[workspace.dependencies]` because nothing
else in the workspace uses it; if a future lib crate needs HTTP
client support, this is the place to lift from.

## HTTP layer

### `HttpClient` (`http/client.rs`)

```rust
pub struct HttpClient {
    http: reqwest::Client,
    base_url: String,                          // e.g. http://localhost:8080
    tokens: Arc<dyn TokenStore>,
    refresh_lock: tokio::sync::Mutex<()>,
}

#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn access_token(&self) -> Result<Option<String>, StoreError>;
    async fn refresh_token(&self) -> Result<Option<String>, StoreError>;
    async fn set_access_token(&self, value: &str) -> Result<(), StoreError>;
    async fn set_refresh_token(&self, value: &str) -> Result<(), StoreError>;
    async fn clear(&self) -> Result<(), StoreError>;
}
```

Three impls exist:

- `TauriStore { store: Arc<tauri_plugin_store::Store<tauri::Wry>> }`
  uses the file `auth.bin` (the default store). Reads / writes via
  `store.get("access_token")` etc.
- `MemoryStore` (`Mutex<HashMap<String, String>>`) for unit tests.
- `NullStore` (returns `Ok(None)` / `Ok(())` everywhere) is the
  optional fallback if `tauri_plugin_store` ever fails to open on
  boot (see Bootstrap).

### `request<TReq, TResp>(...)`

The single chokepoint. Every `http::*` handler delegates to it. The
implementation lives in `http/client.rs::HttpClient::request`:

```rust
pub async fn request<TReq, TResp>(
    &self,
    method: reqwest::Method,
    path: &str,
    body: Option<&TReq>,
) -> Result<TResp, ApiError>
where
    TReq: Serialize + ?Sized,
    TResp: DeserializeOwned,
```

Flow (single attempt):

1. `needs_auth = !NO_AUTH_PATHS.iter().any(|(m, p)| *m == method.as_str() && *p == path)`.
2. If `needs_auth`: pull `tokens.access_token().await?`. If `None` →
   return `ApiError::RefreshFailed`.
3. Build `reqwest::RequestBuilder`. If a token is available and
   `needs_auth`, attach `Authorization: Bearer <token>`.
   `Content-Type: application/json` is set automatically by
   `reqwest::json`.
4. Send. Collect status, body bytes, and any transport error
   (`ApiError::Network(string)` on reqwest failure).
5. If `status == 401` AND `needs_auth` AND a stored refresh token
   exists → enter the **retry path** below.
6. If `2xx` → `serde_json::from_slice::<TResp>(body)` → `Ok`.
7. If non-2xx: parse `serde_json::from_slice::<ErrorBody>(body)`. If
   that succeeds → `ApiError::Http { status, code: body.code,
   message: body.message }`. If parsing fails (server returned 5xx
   without the `ErrorBody` shape, or the body was empty), fall back
   to `ApiError::Http { status, code: "<status_text>",
   message: String::from_utf8_lossy(body) }`.

**Retry path** (called only on first-401, single attempt):

1. `let _guard = self.refresh_lock.lock().await;`
2. Re-read `access_token`. If it's now `Some(t)` and `t !=
   first_attempt_token`, the concurrent racer has already refreshed —
   skip to step 5 with `t`.
3. Otherwise, build a `POST /api/auth/refresh` request with body
   `RefreshRequest { refresh_token }`. **No** Bearer attached (refresh
   goes through `request` with `needs_auth = false` via the
   hard-coded method/path; the lock prevents recursion).
4. On success (`200` + parsed `AccessTokenResponse`):
   - `tokens.set_access_token(&new).await?;`
   - Drop the guard.
   - Build a clone of the *original* request with the new
     `Authorization` header and resend exactly once. The response
     of the retried call is the function's return value (no further
     refresh).
5. On non-200 or refresh body's `access_token` missing → call
   `tokens.clear()` and return `ApiError::RefreshFailed`.

The lock ensures N concurrent 401s map to at most one
`/api/auth/refresh` request.

### Wire DTOs

Each `http/*.rs` file defines the request / response types for that
resource. Naming mirrors `aegis-server/src/transport/http/dto.rs`
exactly so the two can be diffed. All have `Serialize`, `Deserialize`
plus `Debug` where useful. Optional fields use
`#[serde(skip_serializing_if = "Option::is_none")]` to match the
server's patch semantics (omit → leave unchanged; present-empty →
wipe).

Cross-resource types live in `http/dto.rs`:

```rust
#[derive(Debug, thiserror::Error, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiError {
    #[error("network: {0}")]
    Network(String),

    #[error("http {status} ({code}): {message}")]
    Http { status: u16, code: String, message: String },

    #[error("refresh failed; please log in")]
    RefreshFailed,

    #[error("not implemented on this platform: {0}")]
    NotImplemented(&'static str),

    #[error("store error: {0}")]
    Store(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorBody { pub code: String, pub message: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role { Root, Admin, General }
```

`ApiError` serializes as a tagged JSON object; the frontend can
discriminate by `kind`. Implementations:

- `impl From<reqwest::Error> for ApiError` → `Network(err.to_string())`
- `impl From<tauri_plugin_store::Error> for ApiError` → `Store(_)`

### Resource modules

`http/auth.rs` exposes:

```rust
pub async fn login(c: &HttpClient, body: LoginRequest)
    -> Result<(), ApiError>
pub async fn login_domain(c: &HttpClient, code: &str)
    -> Result<(), ApiError>
pub async fn refresh(c: &HttpClient) -> Result<(), ApiError>
pub async fn logout(c: &HttpClient) -> Result<(), ApiError>
```

`login` / `login_domain` write `tokens.set_access_token` and
`set_refresh_token` on `200`. `refresh` calls `POST /api/auth/refresh`
using the stored refresh token, writes the new access token, and
returns `()`. `logout` reads the stored refresh token, calls
`POST /api/auth/logout`, and calls `tokens.clear()` unconditionally
on its way out (network errors get logged via `tracing::warn!` and
the local clear still happens).

`http/user.rs`, `http/product.rs`, `http/project.rs`,
`http/user_credential.rs` each expose one `pub async fn` per
operation (no `login` here — that's the auth module's job) plus the
matching wire DTOs. They delegate to `c.request(method, path,
Some(&body))` or `c.request::<(), _>(method, path, None)`. All return
the unmapped server response (no token touching).

`http/healthz.rs::healthz(c)` calls `GET /healthz` and returns
`String` (`"ok"`).

### OS identity (`system/identity.rs`)

```rust
#[derive(Debug)]
pub struct Identity {
    pub domain: String,
    pub host_machine: String,
    pub sid: String,
    pub userid: String,    // UPN-left (will be sent as LoginDomainRequest.code)
}

#[cfg(target_os = "windows")]
pub fn current() -> Result<Identity, String> {
    let info = windows_utils::get_user_info()
        .map_err(|e| format!("{e}"))?;
    Ok(Identity {
        domain: info.domain,
        host_machine: info.host_machine,
        sid: info.sid,
        userid: info.userid,
    })
}

#[cfg(not(target_os = "windows"))]
pub fn current() -> Result<Identity, &'static str> {
    Err("OS identity lookup requires Windows")
}
```

`http::auth::login_domain(c, code)` calls `system::identity::current()`
and, on success, builds

```rust
LoginDomainRequest {
    code: code.into(),
    domain_name: info.domain,
    hostname: info.host_machine,
    sid: info.sid,
}
```

On non-Windows, `Err("OS identity lookup requires Windows")` is
wrapped into `ApiError::NotImplemented("loginDomain requires Windows")`.

## Commands (`commands/`)

Each command is a one-line delegate. All take `State<'_, HttpClient>`
(no need for a wrapper). Examples from `commands/auth.rs`:

```rust
#[tauri::command]
pub async fn login(
    state: State<'_, HttpClient>,
    code: String,
    password: String,
) -> Result<(), ApiError> {
    http::auth::login(&state, LoginRequest { code, password }).await
}

#[tauri::command]
pub async fn login_domain(
    state: State<'_, HttpClient>,
    code: String,
) -> Result<(), ApiError> {
    http::auth::login_domain(&state, &code).await
}

#[tauri::command]
pub async fn is_logged_in(
    state: State<'_, HttpClient>,
) -> Result<bool, ApiError> {
    Ok(state.tokens.access_token().await?.is_some())
}

#[tauri::command]
pub async fn refresh(state: State<'_, HttpClient>) -> Result<(), ApiError> {
    http::auth::refresh(&state).await
}

#[tauri::command]
pub async fn logout(state: State<'_, HttpClient>) -> Result<(), ApiError> {
    http::auth::logout(&state).await
}
```

### Full command surface

| Command                        | Args                                                    | Returns              |
|--------------------------------|---------------------------------------------------------|----------------------|
| `login`                        | `{ code, password }`                                    | `()`                 |
| `loginDomain`                  | `{ code }`                                              | `()`                 |
| `isLoggedIn`                   | —                                                       | `bool`               |
| `refresh`                      | —                                                       | `()`                 |
| `logout`                       | —                                                       | `()`                 |
| `registerUser`                 | `{ userCode, userName, domainName, hostname, sid, password }` | `RegisteredUser` |
| `updateUserCredential`         | `{ userCode, password? }`                               | `UserCredentialView` |
| `createUser`                   | `{ code, name, role }`                                  | `UserView`           |
| `listUsers`                    | —                                                       | `UserView[]`         |
| `getUserByCode`                | `{ code }`                                              | `UserView`           |
| `updateUser`                   | `{ code, body: { name?, role?, active? } }`             | `UserView`           |
| `createProduct`                | `{ code, name, description }`                           | `ProductView`        |
| `listProducts`                 | —                                                       | `ProductView[]`      |
| `getProductByCode`             | `{ code }`                                              | `ProductView`        |
| `updateProduct`                | `{ code, body: { name?, description?, active? } }`      | `ProductView`        |
| `createProject`                | `{ code, description, productId, members?, unblindMembers? }` | `ProjectView`  |
| `listProjects`                 | —                                                       | `ProjectView[]`      |
| `getProjectByCode`             | `{ code }`                                              | `ProjectView`        |
| `updateProject`                | `{ code, body: { description?, productId?, active?, members?, unblindMembers? } }` | `ProjectView` |
| `healthz`                      | —                                                       | `String`             |

`lib.rs::run` uses `tauri::generate_handler![login, loginDomain,
isLoggedIn, refresh, logout, registerUser, updateUserCredential,
createUser, listUsers, getUserByCode, updateUser, createProduct,
listProducts, getProductByCode, updateProduct, createProject,
listProjects, getProjectByCode, updateProject, healthz]`.

## Bootstrap (`lib.rs`)

`run()` owns the wiring from compile-time config to a Tauri app:

1. `let store = app.store("auth.bin")?;` — wrapped in a `match` so a
   missing-corrupt store falls back to `NullStore` and logs
   `tracing::warn!`. (Today the store is persistent on disk; if it
   can't be opened we want the app to start, just without login
   persistence, so the user can re-login.)
2. `let client = HttpClient::new(http::config::BASE_URL.into(), Arc::new(TauriStore { store: Arc::clone(&store) }));`
3. `.plugin(tauri_plugin_store::Builder::new().build())` (already
   present).
4. `.plugin(tauri_plugin_opener::init())` (already present).
5. `.manage(client)`.
6. `.invoke_handler(tauri::generate_handler![...all 20 commands...])`.
7. `.run(tauri::generate_context!())`.

`reqwest::Client` is constructed inside `HttpClient::new` with a
`Duration::from_secs(15)` overall timeout and a single
`User-Agent: aegis-desktop/0.1.0` header; future debugging can swap
this for a builder that wires tracing spans.

## TS wrapper (`apps/desktop/aegis-desktop/src/api/`)

Hand-written, no codegen. Two files:

- `src/api/index.ts`: `export const api = { login: (...): Promise<void> => invoke("login", args), ... } as const;`
- `src/api/types.ts`: TS aliases that mirror the wire DTOs.

### Wrappers

```ts
export const api = {
  login: (code: string, password: string) =>
    invoke<void>("login", { code, password }),
  loginDomain: (code: string) =>
    invoke<void>("loginDomain", { code }),
  isLoggedIn: () => invoke<boolean>("isLoggedIn"),
  refresh: () => invoke<void>("refresh"),
  logout: () => invoke<void>("logout"),

  registerUser: (input: RegisterUserInput) =>
    invoke<RegisteredUser>("registerUser", input),
  updateUserCredential: (input: { userCode: string; password?: string }) =>
    invoke<UserCredentialView>("updateUserCredential", input),

  createUser: (input: { code: string; name: string; role: Role }) =>
    invoke<UserView>("createUser", input),
  listUsers: () => invoke<UserView[]>("listUsers"),
  getUserByCode: (code: string) => invoke<UserView>("getUserByCode", { code }),
  updateUser: (code: string, body: { name?: string; role?: Role; active?: boolean }) =>
    invoke<UserView>("updateUser", { code, body }),

  createProduct: (input: { code: string; name: string; description: string }) =>
    invoke<ProductView>("createProduct", input),
  listProducts: () => invoke<ProductView[]>("listProducts"),
  getProductByCode: (code: string) => invoke<ProductView>("getProductByCode", { code }),
  updateProduct: (code: string, body: { name?: string; description?: string; active?: boolean }) =>
    invoke<ProductView>("updateProduct", { code, body }),

  createProject: (input: { code: string; description: string; productId: number; members?: ProjectMembers; unblindMembers?: ProjectMembers }) =>
    invoke<ProjectView>("createProject", input),
  listProjects: () => invoke<ProjectView[]>("listProjects"),
  getProjectByCode: (code: string) => invoke<ProjectView>("getProjectByCode", { code }),
  updateProject: (code: string, body: { description?: string; productId?: number; active?: boolean; members?: ProjectMembers; unblindMembers?: ProjectMembers }) =>
    invoke<ProjectView>("updateProject", { code, body }),

  healthz: () => invoke<string>("healthz"),
} as const;
```

The home page replaces the `greet` button with a tiny login form
firing `api.login` + a "who am I" indicator reading `api.isLoggedIn`,
as the first end-to-end smoke test.

## Testing strategy

Three layers:

### Layer 1 — DTO serde round-trips

Each `http/*.rs::tests` module covers its resource's wire DTOs:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn login_request_roundtrips() {
        let j = r#"{"code":"u","password":"p"}"#;
        let req: LoginRequest = serde_json::from_str(j).unwrap();
        assert_eq!(req.code, "u");
        assert_eq!(serde_json::to_string(&req).unwrap(), j);
    }
}
```

### Layer 2 — `wiremock`-based HTTP tests (`http/client.rs::tests`)

- `wiremock::MockServer` is started per test.
- `HttpClient::with_base_url(server.uri())` is constructed with
  `Arc::new(MemoryStore::default())` as the tokens.
- Stub responses verify:
  - Bearer attached on `/api/user/foo`, **not** attached on
    `/api/auth/login` and `/api/auth/user-credential`.
  - 401 → exactly one `POST /api/auth/refresh` → retried once with
    the new token → second response returned.
  - 401 with refresh-token invalid → store cleared,
    `ApiError::RefreshFailed` returned.
  - Network error (`server.kill()`) → `ApiError::Network(_)`.
  - Concurrent first-attempts that both 401: a `tokio::join!` of
    two requests asserts the test saw exactly one
    `/api/auth/refresh` request.
  - 422 with non-`ErrorBody` body → falls back to
    `ApiError::Http { code: status_text, message: ... }`.

### Layer 3 — TS wrapper tests (`src/test/api.test.ts`)

Mirrors the existing convention in
`src/test/routes/index.test.tsx`:

```ts
vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

it("login calls invoke('login', { code, password })", async () => {
  (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined);
  await api.login("alice", "secret");
  expect(invoke).toHaveBeenCalledWith("login", { code: "alice", password: "secret" });
});
```

One test per command (20 total). Resets via the existing
`beforeEach { vi.restoreAllMocks() }` pattern.

### Not done

- No end-to-end test that drives a live `aegis-server`. There's no
  test harness for booting the server today; adding one is outside
  scope.
- No tests for the `commands/*` shims — they are 1-line delegates
  to `http::*`, which are fully exercised by Layer 2.
- No tests for `system::identity` — it is a thin wrapper around the
  `windows-utils` crate, which has its own coverage.

## Open Risks

- **User-credential Bearer exclusion.** The server returns `401
  token_verification_failed` for unauthenticated calls to
  `POST /api/auth/user-credential` (it is admin/root gated). Per
  user direction, the desktop does **not** attach a Bearer on this
  call, so any registration attempt from the current server build
  will fail with `ApiError::Http { status: 401, code:
  "token_verification_failed", .. }`. This is captured here so it is
  not silent; resolving it requires either (a) a server-side change
  to make the endpoint public, or (b) relaxing the desktop's
  exclusion list. Decision deferred to whoever owns the server
  change.
- **Cross-platform compile.** Because `windows-utils` has
  `compile_error!` on non-Windows, the desktop crate will not
  compile on Linux/macOS CI runners until cross-compile targets are
  added. The `system::identity::current` `cfg` gate keeps the
  failure mode limited to one file; the rest of the crate builds
  cleanly off-Windows, but linking `windows-utils` into
  `src-tauri` blocks the whole build. If CI needs to verify the
  non-OS-touching code on Linux, the dependency on `windows-utils`
  must itself be `#[cfg(target_os = "windows")]`-gated in
  `Cargo.toml` (a follow-up if desired).
