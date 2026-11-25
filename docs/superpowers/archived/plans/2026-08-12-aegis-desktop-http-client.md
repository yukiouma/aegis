# aegis-desktop HTTP Client Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Wire the aegis-server HTTP catalog into the Tauri desktop app as 20 `#[tauri::command]` shims, with `tauri_plugin_store`-backed token persistence and a single-attempt auto-refresh on `401`.

**Architecture:** A pure-Rust outbound HTTP layer under `src-tauri/src/http/` (testable without Tauri) plus a thin `#[tauri::command]` layer under `src-tauri/src/commands/` that delegates 1:1 into it. Tokens flow through a `TokenStore` trait with a Tauri-backed impl for prod and an in-memory impl for tests. `loginDomain` reads OS identity via `windows-utils` under a `cfg`-gated wrapper.

**Tech Stack:** Rust 2024 (workspace), Tauri v2, `reqwest 0.13` (rustls-tls), `tokio 1.53`, `tauri_plugin_store 2`, `tauri_plugin_opener 2`, `windows-utils` (path-dep), `wiremock 0.6` (dev-dep), `thiserror 2`, `async-trait`. Frontend: TypeScript, React 19, Vite 7, Vitest 2, existing `@aegis/ui` workspace package.

**Spec:** `docs/superpowers/specs/2026-08-12-aegis-desktop-http-client-design.md`

## Global Constraints

- Source module style: `src/<module>.rs` + `src/<module>/` directory. **No `mod.rs`.**
- Workspace is on `resolver = "3"`. `tokio`, `thiserror`, `async-trait`, `serde`, `serde_json`, `chrono` are inherited via `{ workspace = true }`.
- `reqwest` and `wiremock` are local to `src-tauri/Cargo.toml`; do not promote them to the workspace.
- `windows-utils = { path = "../../../../lib/crates/windows-utils" }` is a relative path dep. The crate is Windows-only by design (`compile_error!` on non-Windows). `system::identity::current()` is `#[cfg]`-gated so the call sites compile on every target; the crate's overall build is still gated by the host platform via the standard `cfg(target_os = "windows")` rule. CI on non-Windows hosts is an out-of-scope follow-up — do not include `#[cfg(target_os = "windows")]` gates on the `windows-utils` dependency itself in Cargo.toml.
- Token store file: `auth.bin` (the tauri_plugin_store default), keys: `access_token`, `refresh_token`.
- Bearer header (`Authorization: Bearer <token>`) attached to every request whose `(method, path)` is **not** in `NO_AUTH_PATHS`:

  ```rust
  const NO_AUTH_PATHS: &[(&str, &str)] = &[
      ("POST", "/api/auth/login"),
      ("POST", "/api/auth/login-domain"),
      ("GET",  "/healthz"),
      ("POST", "/api/auth/user-credential"),
      ("POST", "/api/auth/refresh"),
  ];
  ```

  The first four are user-specified; `/api/auth/refresh` is added so the retry-on-401 path uses the same policy (server doesn't enforce auth on refresh; a stale Bearer there is just log noise).
- Error model: the single `ApiError` in `http/dto.rs` is what every command returns. `serde(tag = "kind", rename_all = "snake_case")` — variants `network`, `http`, `refresh_failed`, `not_implemented`, `store`.
- Frontend module style: `src/<module>.ts(x)` + `src/<module>/` directory. **No `index.ts` style barrel re-exports for app code**; the API wrapper module is `src/api/index.ts` by name, not by re-export convention.
- Tests for HTTP layer use `wiremock`. Tests for TS wrappers use `vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }))` per the existing convention in `src/test/routes/index.test.tsx`.

## File Structure

Files created or modified by this plan. Every file is owned by exactly one task; tasks are sequenced so a file's producer task runs before any task that consumes it.

| File                                                                          | Owner task | Notes                                                  |
|-------------------------------------------------------------------------------|------------|-------------------------------------------------------|
| `src-tauri/Cargo.toml`                                                        | T1         | Add 5 new deps + dev-deps                              |
| `src-tauri/build.rs`                                                          | T1         | Bakes `AEGIS_SERVER_URL` into the binary              |
| `src-tauri/src/http.rs`                                                       | T2         | Module root, `pub use` re-exports                     |
| `src-tauri/src/http/config.rs`                                                | T2         | `BASE_URL`, `NO_AUTH_PATHS` constants                 |
| `src-tauri/src/http/dto.rs`                                                   | T2         | `ApiError`, `ErrorBody`, `Role`                        |
| `src-tauri/src/system.rs`                                                     | T3         | Module root                                           |
| `src-tauri/src/system/identity.rs`                                            | T3         | `cfg`-gated wrapper over `windows-utils`              |
| `src-tauri/src/http/client.rs`                                                | T4         | `HttpClient`, `TokenStore` trait + 2 impls, retry     |
| `src-tauri/src/http/auth.rs` + `src-tauri/src/commands/auth.rs`               | T5         | 5 commands + 4 http functions                         |
| `src-tauri/src/http/user_credential.rs` + `src-tauri/src/commands/user_credential.rs` | T6  | 2 commands + 2 http functions                         |
| `src-tauri/src/http/user.rs` + `src-tauri/src/commands/user.rs`               | T7         | 4 commands + 4 http functions                         |
| `src-tauri/src/http/product.rs` + `src-tauri/src/commands/product.rs`         | T8         | 4 commands + 4 http functions                         |
| `src-tauri/src/http/project.rs` + `src-tauri/src/commands/project.rs`         | T9         | 4 commands + 4 http functions                         |
| `src-tauri/src/http/healthz.rs` + `src-tauri/src/commands/healthz.rs`         | T10        | 1 command + 1 http function                           |
| `src-tauri/src/commands.rs`                                                   | T5–T10     | Incremental `pub mod` lines; consolidated in T10     |
| `src-tauri/src/main.rs`                                                       | T11        | Cosmetic: propagate `Result` from `run`              |
| `src-tauri/src/lib.rs`                                                        | T11        | Bootstrap + `invoke_handler!`                          |
| `apps/desktop/aegis-desktop/src/api/types.ts`                                 | T12        | TS wire-DTO type aliases                              |
| `apps/desktop/aegis-desktop/src/api/index.ts`                                 | T13        | Typed `api.*` wrapper functions                        |
| `apps/desktop/aegis-desktop/src/test/api.test.ts`                             | T13        | Verifies each `api.*` calls the right command name    |
| `apps/desktop/aegis-desktop/src/pages/home.tsx`                               | T14        | Replace greet button with login-form smoke test      |

---

### Task 1: Cargo dependencies + build.rs

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`
- Create: `apps/desktop/aegis-desktop/src-tauri/build.rs`

**Interfaces:**
- Produces: A `src-tauri/` crate that compiles after these additions. `AEGIS_SERVER_URL` env var baked into a `BASE_URL` constant. No behavior change in `lib.rs` yet.

- [ ] **Step 1: Open `Cargo.toml` and add new dependencies**

Open `apps/desktop/aegis-desktop/src-tauri/Cargo.toml`. The existing `[dependencies]` block is:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri-plugin-store = "2"
```

Replace that block with:

```toml
[dependencies]
tauri = { version = "2", features = [] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tauri-plugin-store = "2"
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls-tls"] }
tokio = { workspace = true }
thiserror = { workspace = true }
async-trait = { workspace = true }
windows-utils = { path = "../../../../lib/crates/windows-utils" }
```

Then append a new section at the bottom of the same file:

```toml
[dev-dependencies]
wiremock = "0.6"
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "time"] }
```

- [ ] **Step 2: Create `build.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/build.rs` with this exact content:

```rust
fn main() {
    let url = std::env::var("AEGIS_SERVER_URL")
        .unwrap_or_else(|_| "http://localhost:8080".into());
    println!("cargo:rustc-env=AEGIS_SERVER_URL={url}");
}
```

- [ ] **Step 3: Verify `cargo check` succeeds**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo check
```

Expected: `Finished` with no errors. New dependencies resolve. No new behavior in `lib.rs` yet — the crate still compiles with `cargo check`.

- [ ] **Step 4: Verify the env var is baked**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
AEGIS_SERVER_URL=http://example.test:9000 cargo build 2>&1 | tail -5
```

Expected: build succeeds; no error about `env!` macro lookup (we'll see the actual `env!` lookup fail later if the var isn't set). The build step prints `cargo:rustc-env=AEGIS_SERVER_URL=http://example.test:9000` to stderr.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/Cargo.toml \
        apps/desktop/aegis-desktop/src-tauri/build.rs
git commit -m "chore(desktop): add reqwest, tokio, windows-utils + build.rs"
```

---

### Task 2: http module root + config + cross-resource DTOs

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/config.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs`

**Interfaces:**
- Produces:
  - `pub mod http` declared in `lib.rs` (added in Task 11).
  - `http::config::BASE_URL: &str` and `http::config::NO_AUTH_PATHS`.
  - `http::dto::ApiError`, `http::dto::ErrorBody`, `http::dto::Role`.
- Consumes: nothing yet.

- [ ] **Step 1: Create `src/http/config.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/config.rs` with this exact content:

```rust
//! Compile-time configuration for the outbound HTTP client.

/// Base URL of the aegis-server, baked at build time by `build.rs`.
pub const BASE_URL: &str = env!("AEGIS_SERVER_URL");

/// `(method, path)` pairs that must NOT carry an `Authorization: Bearer` header.
/// Login gates cannot be reached with a stale token; `/healthz` is a public
/// probe; `/api/auth/user-credential` is user-specified to be unauthenticated
/// from the desktop client (see design NO_AUTH_PATHS for the discrepancy with
/// the server's own admin/root gate). `/api/auth/refresh` is added so the
/// auto-refresh path uses the same policy — the server does not enforce
/// Bearer on `/refresh` by design.
pub const NO_AUTH_PATHS: &[(&str, &str)] = &[
    ("POST", "/api/auth/login"),
    ("POST", "/api/auth/login-domain"),
    ("GET",  "/healthz"),
    ("POST", "/api/auth/user-credential"),
    ("POST", "/api/auth/refresh"),
];

/// Returns true if the given `(method, path)` is exempt from Bearer auth.
pub fn is_no_auth(method: &str, path: &str) -> bool {
    NO_AUTH_PATHS.iter().any(|(m, p)| *m == method && *p == path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_is_no_auth() {
        assert!(is_no_auth("POST", "/api/auth/login"));
    }

    #[test]
    fn refresh_is_no_auth() {
        assert!(is_no_auth("POST", "/api/auth/refresh"));
    }

    #[test]
    fn user_list_needs_auth() {
        assert!(!is_no_auth("GET", "/api/user"));
    }

    #[test]
    fn method_mismatch_is_not_no_auth() {
        assert!(!is_no_auth("GET", "/api/auth/login"));
    }
}
```

- [ ] **Step 2: Create `src/http/dto.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs` with this exact content:

```rust
//! Cross-resource wire DTOs and the single `ApiError` returned by every command.

use serde::{Deserialize, Serialize};

/// Stable, machine-readable error code returned as part of the server
/// `ErrorBody`. The desktop client does not dispatch on these codes;
/// errors are forwarded to the frontend as opaque `ApiError::Http` records.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Three administrative tiers. Wire form is `snake_case` to match the
/// server's `Role` serialization.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Root,
    Admin,
    General,
}

/// The single error type every `#[tauri::command]` returns to the frontend.
/// Serialized as a tagged object (`{"kind": "http", ...}` etc.) so the
/// frontend can discriminate by `kind`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiError {
    /// Reqwest returned a transport error (DNS, connect, TLS, timeout).
    #[error("network: {0}")]
    Network(String),

    /// Server returned a non-2xx response; `code` is the body's stable
    /// machine-readable token (or `status_text` for non-JSON 5xx).
    #[error("http {status} ({code}): {message}")]
    Http {
        status: u16,
        code: String,
        message: String,
    },

    /// Auth refresh failed (or no refresh token left). Frontend should
    /// route to login.
    #[error("refresh failed; please log in")]
    RefreshFailed,

    /// Functionality not available on this platform (e.g. `loginDomain` on
    /// non-Windows).
    #[error("not implemented on this platform: {0}")]
    NotImplemented(&'static str),

    /// Persistent token-store error.
    #[error("store error: {0}")]
    Store(String),
}

impl From<reqwest::Error> for ApiError {
    fn from(err: reqwest::Error) -> Self {
        ApiError::Network(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_serializes_snake_case() {
        assert_eq!(serde_json::to_string(&Role::Root).unwrap(), "\"root\"");
        assert_eq!(serde_json::to_string(&Role::Admin).unwrap(), "\"admin\"");
        assert_eq!(serde_json::to_string(&Role::General).unwrap(), "\"general\"");
    }

    #[test]
    fn role_deserializes_snake_case() {
        let r: Role = serde_json::from_str("\"root\"").unwrap();
        assert_eq!(r, Role::Root);
    }

    #[test]
    fn error_body_roundtrip() {
        let body = ErrorBody { code: "validation_failed".into(), message: "bad code".into() };
        let j = serde_json::to_string(&body).unwrap();
        let back: ErrorBody = serde_json::from_str(&j).unwrap();
        assert_eq!(body, back);
    }

    #[test]
    fn api_error_http_serializes_with_kind_tag() {
        let e = ApiError::Http {
            status: 401,
            code: "invalid_credentials".into(),
            message: "nope".into(),
        };
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"http\""), "got {j}");
        assert!(j.contains("\"status\":401"));
        assert!(j.contains("\"code\":\"invalid_credentials\""));
    }

    #[test]
    fn api_error_network_serializes() {
        let e = ApiError::Network("dns".into());
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"network\""));
    }

    #[test]
    fn api_error_refresh_failed_serializes() {
        let e = ApiError::RefreshFailed;
        let j = serde_json::to_string(&e).unwrap();
        assert!(j.contains("\"kind\":\"refresh_failed\""));
    }
}
```

- [ ] **Step 3: Create skeleton `src/http.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/http.rs` with this exact content (modules added in Tasks 4–10):

```rust
//! Outbound HTTP client for the aegis-server.
//!
//! Modules here know nothing about Tauri. The `commands/` layer adapts each
//! function to a `#[tauri::command]` shim.

pub mod config;
pub mod dto;

// Filled in by Tasks 4–10:
// pub mod auth;
// pub mod client;
// pub mod healthz;
// pub mod product;
// pub mod project;
// pub mod user;
// pub mod user_credential;
```

- [ ] **Step 4: Add `mod http;` to `lib.rs`**

Open `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. Add this line at the top of the file, before any other code:

```rust
mod http;
```

- [ ] **Step 5: Run the unit tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib
```

Expected: All tests pass. The new tests:

- `http::config::tests::login_is_no_auth`
- `http::config::tests::refresh_is_no_auth`
- `http::config::tests::user_list_needs_auth`
- `http::config::tests::method_mismatch_is_not_no_auth`
- `http::dto::tests::role_serializes_snake_case`
- `http::dto::tests::role_deserializes_snake_case`
- `http::dto::tests::error_body_roundtrip`
- `http::dto::tests::api_error_http_serializes_with_kind_tag`
- `http::dto::tests::api_error_network_serializes`
- `http::dto::tests::api_error_refresh_failed_serializes`

The pre-existing `greet` command still compiles.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/config.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/dto.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add http config + cross-resource DTOs"
```

---

### Task 3: OS identity wrapper

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/system.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs`

**Interfaces:**
- Produces:
  - `http::auth::login_domain` consumes `system::identity::current() -> Result<Identity, String>` (full signature is `(&self) -> Result<Identity, String>` for the trait-shape helper; the free function `current()` is what the resource modules call).
  - `Identity { domain: String, host_machine: String, sid: String, userid: String }`.
- Consumes: `windows_utils::get_user_info` on Windows only.

- [ ] **Step 1: Create `src/system/identity.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs` with this exact content:

```rust
//! Cross-platform wrapper for the OS-level identity tuple the
//! `loginDomain` command reads at request time. On Windows this calls
//! `windows_utils::get_user_info`; on non-Windows it returns a static
//! "not implemented" error so the rest of the crate still compiles.

/// Identity tuple that becomes `LoginDomainRequest { code, domain_name,
/// hostname, sid }` after the user fills in `code`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Identity {
    pub domain: String,
    pub host_machine: String,
    pub sid: String,
    pub userid: String,
}

/// Read the current OS identity. Returns `Err` on non-Windows targets so
/// callers (e.g. `http::auth::login_domain`) can translate to
/// `ApiError::NotImplemented`.
pub fn current() -> Result<Identity, String> {
    #[cfg(target_os = "windows")]
    {
        let info = windows_utils::get_user_info().map_err(|e| e.to_string())?;
        Ok(Identity {
            domain: info.domain,
            host_machine: info.host_machine,
            sid: info.sid,
            userid: info.userid,
        })
    }

    #[cfg(not(target_os = "windows"))]
    {
        Err("OS identity lookup requires Windows".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identity_fields_are_public_strings() {
        let id = Identity {
            domain: "corp.example".into(),
            host_machine: "ws-001".into(),
            sid: "S-1-5-21-...".into(),
            userid: "alice".into(),
        };
        assert_eq!(id.domain, "corp.example");
        assert_eq!(id.host_machine, "ws-001");
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn non_windows_returns_err() {
        let r = current();
        assert!(r.is_err());
        assert!(r.unwrap_err().contains("Windows"));
    }
}
```

- [ ] **Step 2: Create `src/system.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/system.rs` with this exact content:

```rust
//! Thin platform wrappers used by the HTTP layer.
pub mod identity;
```

- [ ] **Step 3: Add `mod system;` to `lib.rs`**

Open `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. Add the line below right after the existing `mod http;`:

```rust
mod system;
```

- [ ] **Step 4: Run the unit tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib system::
```

Expected: All tests pass. On Windows, the `current()` call will attempt to reach the OS and may succeed or fail depending on the test environment — both outcomes are tolerable, but `is_err()` test gates are filtered by `#[cfg(not(target_os = "windows"))]` so the failure is never surfaced as a panic on non-Windows CI.

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/system.rs \
        apps/desktop/aegis-desktop/src-tauri/src/system/identity.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add system/identity wrapper over windows-utils"
```

---

### Task 4: HttpClient + TokenStore trait + TauriStore / MemoryStore impls + retry logic

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs`

**Interfaces:**
- Produces:
  - `HttpClient::new(base_url: String, tokens: Arc<dyn TokenStore>) -> Self` — production wiring uses `TauriStore`; tests use `MemoryStore`.
  - `HttpClient::with_base_url(base_url: String, tokens: Arc<dyn TokenStore>) -> Self` — same constructor; named for symmetry in tests.
  - `HttpClient::request<TReq, TResp>(&self, method, path, body: Option<&TReq>) -> Result<TResp, ApiError>` where `TReq: Serialize + ?Sized`, `TResp: DeserializeOwned`.
  - `HttpClient::request_bytes<TReq>(&self, method, path, body: Option<&TReq>) -> Result<Vec<u8>, ApiError>` — used by `auth` to pull the raw body for token persistence.
  - `pub trait TokenStore: Send + Sync` with `access_token`, `refresh_token`, `set_access_token`, `set_refresh_token`, `clear`.
  - `pub struct TauriStore { store: Arc<tauri_plugin_store::Store<tauri::Wry>> }`.
  - `pub struct MemoryStore { inner: tokio::sync::Mutex<HashMap<String, String>> }` (used by `#[cfg(test)]` only, but exposed for completeness).
- Consumes: `http::config::is_no_auth`, `http::dto::ApiError`, `http::dto::ErrorBody`. Both produced by Task 2.

- [ ] **Step 1: Create `src/http/client.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs` with this exact content:

```rust
//! Outbound HTTP client with optional Bearer header and a single auto-refresh
//! retry on 401.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;

use super::config::is_no_auth;
use super::dto::{ApiError, ErrorBody};

/// Persistent storage for the access + refresh token pair. Two impls live
/// here: the prod `TauriStore` (backs onto `tauri_plugin_store::Store`) and
/// the test-only `MemoryStore` (`#[cfg(test)]`-gated).
#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn access_token(&self) -> Result<Option<String>, ApiError>;
    async fn refresh_token(&self) -> Result<Option<String>, ApiError>;
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError>;
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError>;
    async fn clear(&self) -> Result<(), ApiError>;
}

/// Production token store backed by `tauri_plugin_store::Store` writing to
/// the `auth.bin` file.
pub struct TauriStore {
    store: Arc<tauri_plugin_store::Store<tauri::Wry>>,
}

impl TauriStore {
    pub fn new(store: Arc<tauri_plugin_store::Store<tauri::Wry>>) -> Self {
        Self { store }
    }
}

#[async_trait]
impl TokenStore for TauriStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        self.store
            .get("access_token")
            .map(|v| v.as_str().map(|s| s.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))
    }

    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        self.store
            .get("refresh_token")
            .map(|v| v.as_str().map(|s| s.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))
    }

    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.store
            .set("access_token", serde_json::Value::String(value.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }

    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.store
            .set("refresh_token", serde_json::Value::String(value.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }

    async fn clear(&self) -> Result<(), ApiError> {
        self.store.delete("access_token").map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.delete("refresh_token").map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }
}

#[cfg(test)]
#[derive(Default, Debug)]
pub struct MemoryStore {
    inner: Mutex<HashMap<String, String>>,
}

#[cfg(test)]
impl MemoryStore {
    pub fn new() -> Self { Self::default() }
}

#[cfg(test)]
#[async_trait]
impl TokenStore for MemoryStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("access_token").cloned())
    }
    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("refresh_token").cloned())
    }
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("access_token".into(), value.into());
        Ok(())
    }
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("refresh_token".into(), value.into());
        Ok(())
    }
    async fn clear(&self) -> Result<(), ApiError> {
        self.inner.lock().await.clear();
        Ok(())
    }
}

/// The HTTP client. One instance per app, `.manage()`-d into Tauri state.
pub struct HttpClient {
    http: reqwest::Client,
    base_url: String,
    tokens: Arc<dyn TokenStore>,
    refresh_lock: Mutex<()>,
}

impl HttpClient {
    pub fn new(base_url: String, tokens: Arc<dyn TokenStore>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("aegis-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds");
        Self { http, base_url, tokens, refresh_lock: Mutex::new(()) }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    /// Type-safe request returning the parsed JSON body.
    pub async fn request<TReq, TResp>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<TResp, ApiError>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let bytes = self.request_bytes(method, path, body).await?;
        serde_json::from_slice::<TResp>(&bytes).map_err(|e| {
            ApiError::Http {
                status: 0,
                code: "decode_failed".into(),
                message: e.to_string(),
            }
        })
    }

    /// Raw-bytes request. Used by auth handlers that need to inspect the
    /// body for token fields, and used internally by the refresh retry path
    /// (which bypasses the typed `request` to avoid re-entering the
    /// refresh lock).
    pub async fn request_bytes<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<Vec<u8>, ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let needs_auth = !is_no_auth(method.as_str(), path);
        let token_opt = if needs_auth {
            self.tokens.access_token().await?
        } else {
            None
        };

        let mut attempted_token: Option<String> = None;
        let bytes = self.send_once(method.clone(), path, body, token_opt.clone()).await?;
        let status = self.last_status().await;

        // We don't track status across calls — so emulate: if the bytes
        // look like an ErrorBody and the request needed auth + had a
        // token, check it against the retry path by inferring from
        // stored `last_status` on the wiremock mock server. For prod, we
        // capture status inline below.
        let _ = status; // (placeholder; see send_with_status below)

        // The simpler implementation captures status in send_once:
        Ok(bytes)
    }

    async fn send_once<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
        token: Option<String>,
    ) -> Result<Vec<u8>, ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let url = self.url(path);
        let mut rb = self.http.request(method, &url);
        if let Some(t) = token {
            rb = rb.bearer_auth(t);
        }
        if let Some(b) = body {
            rb = rb.json(b);
        }
        let resp = rb.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if status.is_success() {
            Ok(bytes.to_vec())
        } else {
            // Try to parse as ErrorBody; fall back to status text.
            let parsed: Option<ErrorBody> = serde_json::from_slice(&bytes).ok();
            Err(match parsed {
                Some(b) => ApiError::Http { status: status.as_u16(), code: b.code, message: b.message },
                None => ApiError::Http {
                    status: status.as_u16(),
                    code: status.canonical_reason().unwrap_or("unknown").to_string(),
                    message: String::from_utf8_lossy(&bytes).into_owned(),
                },
            })
        }
    }

    async fn last_status(&self) -> u16 { 0 }

    /// Refresh helper. Constructs the request directly (does not go through
    /// `request`) because the refresh lock is held; recursing through
    /// `request` would deadlock.
    pub async fn refresh(&self) -> Result<(), ApiError> {
        let _guard = self.refresh_lock.lock().await;
        let refresh_token = self.tokens.refresh_token().await?
            .ok_or(ApiError::RefreshFailed)?;

        #[derive(Serialize)]
        struct Req<'a> { refresh_token: &'a str }
        #[derive(DeserializeOwned)]
        struct Resp { access_token: String }

        let url = self.url("/api/auth/refresh");
        let resp = self.http.post(&url)
            .json(&Req { refresh_token: &refresh_token })
            .send()
            .await?;
        let status = resp.status();
        let bytes = resp.bytes().await?;
        if !status.is_success() {
            let parsed: Option<ErrorBody> = serde_json::from_slice(&bytes).ok();
            let _ = parsed; // not relevant for refresh failure path
            self.tokens.clear().await?;
            return Err(ApiError::RefreshFailed);
        }
        let parsed: Resp = serde_json::from_slice(&bytes)
            .map_err(|_| ApiError::RefreshFailed)?;
        self.tokens.set_access_token(&parsed.access_token).await?;
        Ok(())
    }
}
```

Note on this draft: the `last_status` placeholder is unused and the retry path is inlined into the public `request_bytes` constructor below in the actual implementation. The cleaner final shape:

```rust
pub async fn request_bytes<TReq>(
    &self,
    method: reqwest::Method,
    path: &str,
    body: Option<&TReq>,
) -> Result<Vec<u8>, ApiError>
where
    TReq: Serialize + ?Sized,
{
    let needs_auth = !is_no_auth(method.as_str(), path);
    let first_token = if needs_auth { self.tokens.access_token().await? } else { None };

    let (status, bytes) = self.send_with_status(method.clone(), path, body, first_token.clone()).await?;

    if status.as_u16() == 401 && needs_auth && first_token.is_some() {
        // Retry path: refresh first, then re-send with the new token.
        let _guard = self.refresh_lock.lock().await;
        let new_token = self.tokens.access_token().await?;
        let token_for_retry = match new_token {
            Some(t) if t != first_token.as_deref().unwrap_or("") => t,
            _ => {
                // Either no token, or token didn't change → fetch refresh + retry
                self._refresh_with_lock().await?;
                self.tokens.access_token().await?
                    .ok_or(ApiError::RefreshFailed)?
            }
        };
        let (retry_status, retry_bytes) = self.send_with_status(method, path, body, Some(token_for_retry)).await?;
        if retry_status.is_success() {
            Ok(retry_bytes)
        } else {
            Err(parse_error(retry_status, retry_bytes))
        }
    } else if status.is_success() {
        Ok(bytes)
    } else {
        Err(parse_error(status, bytes))
    }
}

async fn _refresh_with_lock(&self) -> Result<(), ApiError> {
    let refresh_token = self.tokens.refresh_token().await?
        .ok_or(ApiError::RefreshFailed)?;
    #[derive(Serialize)]
    struct Req<'a> { refresh_token: &'a str }
    #[derive(DeserializeOwned)]
    struct Resp { access_token: String }
    let url = self.url("/api/auth/refresh");
    let resp = self.http.post(&url).json(&Req { refresh_token: &refresh_token }).send().await?;
    let status = resp.status();
    let bytes = resp.bytes().await?;
    if !status.is_success() {
        let _ = bytes;
        self.tokens.clear().await?;
        return Err(ApiError::RefreshFailed);
    }
    let parsed: Resp = serde_json::from_slice(&bytes).map_err(|_| ApiError::RefreshFailed)?;
    self.tokens.set_access_token(&parsed.access_token).await?;
    Ok(())
}

async fn send_with_status<TReq>(
    &self,
    method: reqwest::Method,
    path: &str,
    body: Option<&TReq>,
    token: Option<String>,
) -> Result<(reqwest::StatusCode, Vec<u8>), ApiError>
where
    TReq: Serialize + ?Sized,
{
    let url = self.url(path);
    let mut rb = self.http.request(method, &url);
    if let Some(t) = token { rb = rb.bearer_auth(t); }
    if let Some(b) = body { rb = rb.json(b); }
    let resp = rb.send().await?;
    let status = resp.status();
    let bytes = resp.bytes().await?.to_vec();
    Ok((status, bytes))
}

fn parse_error(status: reqwest::StatusCode, bytes: Vec<u8>) -> ApiError {
    let parsed: Option<ErrorBody> = serde_json::from_slice(&bytes).ok();
    match parsed {
        Some(b) => ApiError::Http { status: status.as_u16(), code: b.code, message: b.message },
        None => ApiError::Http {
            status: status.as_u16(),
            code: status.canonical_reason().unwrap_or("unknown").into(),
            message: String::from_utf8_lossy(&bytes).into_owned(),
        },
    }
}
```

Replace the `last_status` placeholder, the broken `request_bytes` body, and the standalone `send_once`/`refresh` with the three-method shape above. Final `client.rs` is:

```rust
//! Outbound HTTP client with optional Bearer header and a single auto-refresh
//! retry on 401.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::sync::Mutex;

use super::config::is_no_auth;
use super::dto::{ApiError, ErrorBody};

#[async_trait]
pub trait TokenStore: Send + Sync {
    async fn access_token(&self) -> Result<Option<String>, ApiError>;
    async fn refresh_token(&self) -> Result<Option<String>, ApiError>;
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError>;
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError>;
    async fn clear(&self) -> Result<(), ApiError>;
}

pub struct TauriStore {
    store: Arc<tauri_plugin_store::Store<tauri::Wry>>,
}

impl TauriStore {
    pub fn new(store: Arc<tauri_plugin_store::Store<tauri::Wry>>) -> Self { Self { store } }
}

#[async_trait]
impl TokenStore for TauriStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        self.store.get("access_token")
            .map(|v| v.as_str().map(|s| s.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))
    }
    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        self.store.get("refresh_token")
            .map(|v| v.as_str().map(|s| s.to_string()))
            .map_err(|e| ApiError::Store(e.to_string()))
    }
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.store.set("access_token", serde_json::Value::String(value.into()))
            .map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.store.set("refresh_token", serde_json::Value::String(value.into()))
            .map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }
    async fn clear(&self) -> Result<(), ApiError> {
        self.store.delete("access_token").map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.delete("refresh_token").map_err(|e| ApiError::Store(e.to_string()))?;
        self.store.save().map_err(|e| ApiError::Store(e.to_string()))
    }
}

#[cfg(test)]
#[derive(Default, Debug)]
pub struct MemoryStore { inner: Mutex<HashMap<String, String>> }

#[cfg(test)]
impl MemoryStore { pub fn new() -> Self { Self::default() } }

#[cfg(test)]
#[async_trait]
impl TokenStore for MemoryStore {
    async fn access_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("access_token").cloned())
    }
    async fn refresh_token(&self) -> Result<Option<String>, ApiError> {
        Ok(self.inner.lock().await.get("refresh_token").cloned())
    }
    async fn set_access_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("access_token".into(), value.into());
        Ok(())
    }
    async fn set_refresh_token(&self, value: &str) -> Result<(), ApiError> {
        self.inner.lock().await.insert("refresh_token".into(), value.into());
        Ok(())
    }
    async fn clear(&self) -> Result<(), ApiError> {
        self.inner.lock().await.clear();
        Ok(())
    }
}

pub struct HttpClient {
    http: reqwest::Client,
    base_url: String,
    tokens: Arc<dyn TokenStore>,
    refresh_lock: Mutex<()>,
}

impl HttpClient {
    pub fn new(base_url: String, tokens: Arc<dyn TokenStore>) -> Self {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .user_agent(concat!("aegis-desktop/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("reqwest client builds");
        Self { http, base_url, tokens, refresh_lock: Mutex::new(()) }
    }

    fn url(&self, path: &str) -> String {
        format!("{}{}", self.base_url.trim_end_matches('/'), path)
    }

    pub async fn request<TReq, TResp>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<TResp, ApiError>
    where
        TReq: Serialize + ?Sized,
        TResp: DeserializeOwned,
    {
        let bytes = self.request_bytes(method, path, body).await?;
        serde_json::from_slice::<TResp>(&bytes).map_err(|e| ApiError::Http {
            status: 0, code: "decode_failed".into(), message: e.to_string(),
        })
    }

    pub async fn request_bytes<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
    ) -> Result<Vec<u8>, ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let needs_auth = !is_no_auth(method.as_str(), path);
        let first_token = if needs_auth { self.tokens.access_token().await? } else { None };

        let (status, bytes) = self.send(method.clone(), path, body, first_token.clone()).await?;

        if status.as_u16() == 401 && needs_auth && first_token.is_some() {
            let _guard = self.refresh_lock.lock().await;
            let after_lock = self.tokens.access_token().await?;
            let token_for_retry = match after_lock {
                Some(t) if Some(&t) != first_token.as_ref() => t,
                _ => {
                    self.refresh_with_lock().await?;
                    self.tokens.access_token().await?.ok_or(ApiError::RefreshFailed)?
                }
            };
            let (retry_status, retry_bytes) = self.send(method, path, body, Some(token_for_retry)).await?;
            if retry_status.is_success() { Ok(retry_bytes) } else { Err(parse_error(retry_status, retry_bytes)) }
        } else if status.is_success() {
            Ok(bytes)
        } else {
            Err(parse_error(status, bytes))
        }
    }

    async fn refresh_with_lock(&self) -> Result<(), ApiError> {
        let refresh_token = self.tokens.refresh_token().await?
            .ok_or(ApiError::RefreshFailed)?;
        #[derive(Serialize)] struct Req<'a> { refresh_token: &'a str }
        #[derive(DeserializeOwned)] struct Resp { access_token: String }
        let url = self.url("/api/auth/refresh");
        let resp = self.http.post(&url).json(&Req { refresh_token: &refresh_token }).send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?.to_vec();
        if !status.is_success() {
            self.tokens.clear().await?;
            return Err(ApiError::RefreshFailed);
        }
        let parsed: Resp = serde_json::from_slice(&bytes).map_err(|_| {
            self.tokens.clear().await.ok();
            ApiError::RefreshFailed
        })?;
        self.tokens.set_access_token(&parsed.access_token).await?;
        Ok(())
    }

    async fn send<TReq>(
        &self,
        method: reqwest::Method,
        path: &str,
        body: Option<&TReq>,
        token: Option<String>,
    ) -> Result<(reqwest::StatusCode, Vec<u8>), ApiError>
    where
        TReq: Serialize + ?Sized,
    {
        let url = self.url(path);
        let mut rb = self.http.request(method, &url);
        if let Some(t) = token { rb = rb.bearer_auth(t); }
        if let Some(b) = body { rb = rb.json(b); }
        let resp = rb.send().await?;
        let status = resp.status();
        let bytes = resp.bytes().await?.to_vec();
        Ok((status, bytes))
    }
}

fn parse_error(status: reqwest::StatusCode, bytes: Vec<u8>) -> ApiError {
    let parsed: Option<ErrorBody> = serde_json::from_slice(&bytes).ok();
    match parsed {
        Some(b) => ApiError::Http { status: status.as_u16(), code: b.code, message: b.message },
        None => ApiError::Http {
            status: status.as_u16(),
            code: status.canonical_reason().unwrap_or("unknown").into(),
            message: String::from_utf8_lossy(&bytes).into_owned(),
        },
    }
}
```

- [ ] **Step 2: Wire `client` into `http.rs`**

Open `apps/desktop/aegis-desktop/src-tauri/src/http.rs`. Replace the contents with:

```rust
//! Outbound HTTP client for the aegis-server.
//!
//! Modules here know nothing about Tauri. The `commands/` layer adapts each
//! function to a `#[tauri::command]` shim.

pub mod auth;
pub mod client;
pub mod config;
pub mod dto;
pub mod healthz;
pub mod product;
pub mod project;
pub mod user;
pub mod user_credential;
```

The seven other module stubs (`auth`, `healthz`, `product`, `project`, `user`, `user_credential`) need to exist as empty files for `cargo check` to proceed. Create each as a minimal file:

For each resource stub, create `apps/desktop/aegis-desktop/src-tauri/src/http/<resource>.rs` with content:

```rust
// Filled in by Tasks 5–10.
```

Run:

```bash
touch apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs \
      apps/desktop/aegis-desktop/src-tauri/src/http/healthz.rs \
      apps/desktop/aegis-desktop/src-tauri/src/http/product.rs \
      apps/desktop/aegis-desktop/src-tauri/src/http/project.rs \
      apps/desktop/aegis-desktop/src-tauri/src/http/user.rs \
      apps/desktop/aegis-desktop/src-tauri/src/http/user_credential.rs
```

For Windows-style commands use:

```bash
cd d:/projects/rusty/aegis
for f in auth healthz product project user user_credential; do
  printf '// Filled in by Tasks 5–10.\n' > "apps/desktop/aegis-desktop/src-tauri/src/http/$f.rs"
done
```

- [ ] **Step 3: Add `cargo test --lib http::client`**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib http::client
```

Expected: All tests pass. Specifically:

- `client::tests::bearer_header_attached_on_protected_endpoint`
- `client::tests::bearer_header_absent_on_login`
- `client::tests::bearer_header_absent_on_user_credential`
- `client::tests::request_success_returns_parsed_body`
- `client::tests::request_404_returns_http_error_with_code`
- `client::tests::request_non_json_500_returns_status_text_code`
- `client::tests::network_failure_returns_network_error`
- `client::tests::auto_refresh_on_401_retries_with_new_token`
- `client::tests::refresh_failure_clears_store_and_returns_refresh_failed`
- `client::tests::concurrent_401s_share_one_refresh`
- `client::tests::memory_store_round_trips_tokens`
- `client::tests::api_error_serializes_kinds`

(Tests are added in Step 4 below. For this step, it's expected that `cargo test --lib http::client` produces a `no tests` message initially — that's fine; this step is purely "the module compiles.")

- [ ] **Step 4: Add the wiremock-based test block to `src/http/client.rs`**

Append this block to the bottom of `apps/desktop/aegis-desktop/src-tauri/src/http/client.rs` (above any existing `#[cfg(test)]` for `MemoryStore`; if they collide, merge them into one module):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use std::sync::Arc;
    use wiremock::matchers::{header, header_exists, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[derive(Serialize, Debug)]
    struct LoginReq { code: String, password: String }
    #[derive(Deserialize, Debug, PartialEq)]
    struct TokenPair { access_token: String, refresh_token: String }

    fn client_for(server: &MockServer, tokens: Arc<MemoryStore>) -> HttpClient {
        HttpClient::new(server.uri(), tokens)
    }

    #[tokio::test]
    async fn bearer_header_attached_on_protected_endpoint() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_AAA").await.unwrap();
        store.set_refresh_token("RT_AAA").await.unwrap();
        let m = Mock::given(method("GET"))
            .and(path("/api/user"))
            .and(header("authorization", "Bearer AT_AAA"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []})));
        server.register(m).await;
        let c = client_for(&server, store);
        let _: serde_json::Value = c.request(reqwest::Method::GET, "/api/user", None::<&()>).await.unwrap();
    }

    #[tokio::test]
    async fn bearer_header_absent_on_login() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let m = Mock::given(method("POST"))
            .and(path("/api/auth/login"))
            .and(header_exists("authorization").not())
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token": "AT", "refresh_token": "RT"})
            ));
        server.register(m).await;
        let c = client_for(&server, store);
        let bytes = c.request_bytes(
            reqwest::Method::POST, "/api/auth/login",
            Some(&LoginReq { code: "u".into(), password: "p".into() }),
        ).await.unwrap();
        let tp: TokenPair = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(tp.access_token, "AT");
        assert_eq!(tp.refresh_token, "RT");
    }

    #[tokio::test]
    async fn bearer_header_absent_on_user_credential() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_AAA").await.unwrap();
        let m = Mock::given(method("POST"))
            .and(path("/api/auth/user-credential"))
            .and(header_exists("authorization").not())
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({"user_code": "u", "user_name": "n", "role": "general", "active": true, "domain_name": "d", "hostname": "h", "sid": "s"})));
        server.register(m).await;
        let c = client_for(&server, store);
        let _: serde_json::Value = c.request(
            reqwest::Method::POST, "/api/auth/user-credential", None::<&()>,
        ).await.unwrap();
    }

    #[tokio::test]
    async fn request_404_returns_http_error_with_code() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let m = Mock::given(method("GET")).and(path("/api/user/foo"))
            .respond_with(ResponseTemplate::new(404).set_body_json(
                serde_json::json!({"code": "not_found", "message": "user foo"})
            ));
        server.register(m).await;
        let c = client_for(&server, store);
        let err = c.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user/foo", None).await.unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 404);
                assert_eq!(code, "not_found");
            }
            _ => panic!("expected Http error, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn request_non_json_500_returns_status_text_code() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        let m = Mock::given(method("GET")).and(path("/api/user"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal boom"));
        server.register(m).await;
        let c = client_for(&server, store);
        let err = c.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None).await.unwrap_err();
        match err {
            ApiError::Http { status, code, message } => {
                assert_eq!(status, 500);
                assert_eq!(code, "Internal Server Error");
                assert!(message.contains("internal boom"));
            }
            _ => panic!("expected Http, got {err:?}"),
        }
    }

    #[tokio::test]
    async fn network_failure_returns_network_error() {
        // Start a server, then drop it so the URI refuses connections.
        let server = MockServer::start().await;
        let uri = server.uri();
        drop(server);
        let store = Arc::new(MemoryStore::default());
        let c = HttpClient::new(uri, store);
        let err = c.request::<(), serde_json::Value>(reqwest::Method::GET, "/healthz", None).await.unwrap_err();
        assert!(matches!(err, ApiError::Network(_)), "got {err:?}");
    }

    #[tokio::test]
    async fn auto_refresh_on_401_retries_with_new_token() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        // Refresh endpoint always succeeds with a new access token.
        server.register(
            Mock::given(method("POST")).and(path("/api/auth/refresh"))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token": "AT_NEW"})
                ))
        ).await;

        // First call to GET /api/user returns 401; second returns 200.
        // We register both and let the retry see the 200 on the second hit.
        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .and(header("authorization", "Bearer AT_STALE"))
                .respond_with(ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"code": "token_verification_failed", "message": "expired"})
                ))
        ).await;
        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .and(header("authorization", "Bearer AT_NEW"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []})))
        ).await;

        let c = client_for(&server, store.clone());
        let v: serde_json::Value = c.request(reqwest::Method::GET, "/api/user", None::<&()>).await.unwrap();
        assert_eq!(v["users"].as_array().unwrap().len(), 0);
        // New token is persisted.
        assert_eq!(store.access_token().await.unwrap().as_deref(), Some("AT_NEW"));
    }

    #[tokio::test]
    async fn refresh_failure_clears_store_and_returns_refresh_failed() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT_DEAD").await.unwrap();

        server.register(
            Mock::given(method("POST")).and(path("/api/auth/refresh"))
                .respond_with(ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"code": "token_verification_failed", "message": "dead"})
                ))
        ).await;
        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .respond_with(ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"code": "token_verification_failed", "message": "expired"})
                ))
        ).await;

        let c = client_for(&server, store.clone());
        let err = c.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None).await.unwrap_err();
        assert!(matches!(err, ApiError::RefreshFailed));
        // Both tokens cleared.
        assert_eq!(store.access_token().await.unwrap(), None);
        assert_eq!(store.refresh_token().await.unwrap(), None);
    }

    #[tokio::test]
    async fn concurrent_401s_share_one_refresh() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT_STALE").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();

        // Only allow ONE refresh call. The second simultaneous 401 should
        // re-use the new token the first call wrote.
        let refresh_mock = Mock::given(method("POST")).and(path("/api/auth/refresh"))
            .respond_with(ResponseTemplate::new(200).set_body_json(
                serde_json::json!({"access_token": "AT_NEW"})
            ))
            .expect(1);
        server.register(refresh_mock).await;

        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .and(header("authorization", "Bearer AT_STALE"))
                .respond_with(ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"code": "token_verification_failed", "message": ""})
                ))
        ).await;
        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .and(header("authorization", "Bearer AT_NEW"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({"users": []})))
        ).await;

        let c1 = client_for(&server, store.clone());
        let c2 = client_for(&server, store.clone());
        let (r1, r2) = tokio::join!(
            c1.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None),
            c2.request::<(), serde_json::Value>(reqwest::Method::GET, "/api/user", None),
        );
        r1.unwrap();
        r2.unwrap();
    }

    #[tokio::test]
    async fn memory_store_round_trips_tokens() {
        let s = MemoryStore::default();
        assert_eq!(s.access_token().await.unwrap(), None);
        s.set_access_token("AT").await.unwrap();
        s.set_refresh_token("RT").await.unwrap();
        assert_eq!(s.access_token().await.unwrap().as_deref(), Some("AT"));
        assert_eq!(s.refresh_token().await.unwrap().as_deref(), Some("RT"));
        s.clear().await.unwrap();
        assert_eq!(s.access_token().await.unwrap(), None);
    }
}
```

Note: the `request` method needs to handle `body: None` correctly. The current signature passes `Option<&TReq>`; when `None`, no JSON body is added. The `serde_json::json!` macro would not work for empty bodies, so `body: None` is the right pattern.

- [ ] **Step 5: Run the wiremock tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib http::client
```

Expected: All 11 wiremock tests pass. If a test panics in the Bearer-header check, double-check the `Authorization` header name (`bearer_auth` in reqwest sets `authorization`, lowercase) and the test matcher uses `header("authorization", "...")`. If a test fails with `decode_failed`, inspect the bytes returned and ensure the request method matches.

- [ ] **Step 6: Run the full test suite**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib
```

Expected: All tests pass; `cargo check` clean.

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/client.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/healthz.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/product.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/project.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/user.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/user_credential.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs
git commit -m "feat(desktop): add HttpClient with auto-refresh + token store"
```

---

### Task 5: http::auth + commands::auth

**Files:**
- Create: `apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs` (replace stub from Task 4)
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands.rs`
- Create: `apps/desktop/aegis-desktop/src-tauri/src/commands/auth.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

**Interfaces:**
- Produces:
  - `http::auth::login(&HttpClient, body: LoginRequest) -> Result<(), ApiError>` — persists tokens.
  - `http::auth::login_domain(&HttpClient, code: &str) -> Result<(), ApiError>` — uses `system::identity::current()`.
  - `http::auth::refresh(&HttpClient) -> Result<(), ApiError>` — calls `/api/auth/refresh`, persists new access token.
  - `http::auth::logout(&HttpClient) -> Result<(), ApiError>` — server call, then `clear()`.
  - `#[tauri::command]` functions: `login`, `loginDomain`, `isLoggedIn`, `refresh`, `logout`.
- Consumes: `HttpClient` + `TokenStore` (T4), `system::identity` (T3).

- [ ] **Step 1: Replace `src/http/auth.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs` with:

```rust
//! Auth-flow HTTP functions: login, login-domain, refresh, logout.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;
use crate::system::identity;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginRequest { pub code: String, pub password: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest { pub refresh_token: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest { pub refresh_token: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessTokenResponse { pub access_token: String }

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogoutResponse {}

pub async fn login(c: &HttpClient, body: LoginRequest) -> Result<(), ApiError> {
    let bytes = c.request_bytes(reqwest::Method::POST, "/api/auth/login", Some(&body)).await?;
    let tp: TokenPairResponse = serde_json::from_slice(&bytes).map_err(|e| ApiError::Http {
        status: 0, code: "decode_failed".into(), message: e.to_string(),
    })?;
    c.tokens().set_access_token(&tp.access_token).await?;
    c.tokens().set_refresh_token(&tp.refresh_token).await?;
    Ok(())
}

pub async fn login_domain(c: &HttpClient, code: &str) -> Result<(), ApiError> {
    let id = identity::current().map_err(|_| ApiError::NotImplemented("loginDomain requires Windows"))?;
    let body = LoginDomainRequest {
        code: code.into(),
        domain_name: id.domain,
        hostname: id.host_machine,
        sid: id.sid,
    };
    let bytes = c.request_bytes(reqwest::Method::POST, "/api/auth/login-domain", Some(&body)).await?;
    let tp: TokenPairResponse = serde_json::from_slice(&bytes).map_err(|e| ApiError::Http {
        status: 0, code: "decode_failed".into(), message: e.to_string(),
    })?;
    c.tokens().set_access_token(&tp.access_token).await?;
    c.tokens().set_refresh_token(&tp.refresh_token).await?;
    Ok(())
}

pub async fn refresh(c: &HttpClient) -> Result<(), ApiError> {
    c.refresh().await
}

pub async fn logout(c: &HttpClient) -> Result<(), ApiError> {
    let rt = c.tokens().refresh_token().await?;
    if let Some(refresh_token) = rt {
        #[derive(Serialize)]
        struct Req<'a> { refresh_token: &'a str }
        let body = Req { refresh_token: &refresh_token };
        // Best-effort server logout; ignore network errors but still clear.
        let _ = c.request_bytes(reqwest::Method::POST, "/api/auth/logout", Some(&body)).await;
    }
    c.tokens().clear().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn login_persists_tokens() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server.register(
            Mock::given(method("POST")).and(path("/api/auth/login"))
                .and(body_json(serde_json::json!({"code": "u", "password": "p"})))
                .respond_with(ResponseTemplate::new(200).set_body_json(
                    serde_json::json!({"access_token": "AT", "refresh_token": "RT"})
                ))
        ).await;
        let c = HttpClient::new(server.uri(), store.clone());
        login(&c, LoginRequest { code: "u".into(), password: "p".into() }).await.unwrap();
        assert_eq!(store.access_token().await.unwrap().as_deref(), Some("AT"));
        assert_eq!(store.refresh_token().await.unwrap().as_deref(), Some("RT"));
    }

    #[tokio::test]
    async fn login_propagates_401() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server.register(
            Mock::given(method("POST")).and(path("/api/auth/login"))
                .respond_with(ResponseTemplate::new(401).set_body_json(
                    serde_json::json!({"code": "invalid_credentials", "message": "bad"})
                ))
        ).await;
        let c = HttpClient::new(server.uri(), store.clone());
        let err = login(&c, LoginRequest { code: "u".into(), password: "wrong".into() }).await.unwrap_err();
        match err {
            ApiError::Http { status, code, .. } => {
                assert_eq!(status, 401);
                assert_eq!(code, "invalid_credentials");
            }
            _ => panic!("got {err:?}"),
        }
        assert_eq!(store.access_token().await.unwrap(), None);
    }

    #[tokio::test]
    async fn logout_clears_tokens_even_if_server_unreachable() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        // Server doesn't register a mock; the request will 404/connection-refused.
        let c = HttpClient::new(server.uri().replace(":0", ":1").replace("127.0.0.1", "127.0.0.1"), store.clone());
        // Just exercise the local-clear path; the request may error, and we accept that.
        let r = logout(&c).await;
        // Whether the request succeeded or failed, the store must be cleared.
        let _ = r;
        assert_eq!(store.access_token().await.unwrap(), None);
        assert_eq!(store.refresh_token().await.unwrap(), None);
    }
}
```

Add this getter to `HttpClient` in `client.rs` (insert after the `pub fn new` block, before `fn url`):

```rust
pub fn tokens(&self) -> Arc<dyn TokenStore> {
    self.tokens.clone()
}
```

(`Arc::clone(&self.tokens)` works because `Arc<dyn TokenStore>` is `Clone`-able. Use the explicit form for clarity.)

- [ ] **Step 2: Create `src/commands.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands.rs` with this exact content:

```rust
//! Tauri command shims that delegate 1:1 to the `http` layer.
pub mod auth;
pub mod healthz;
pub mod product;
pub mod project;
pub mod user;
pub mod user_credential;
```

(Other resource modules are added by Tasks 6–10. If the file is empty during Task 5's intermediate step, the file doesn't compile — to keep `cargo check` valid throughout Tasks 5–10, every `commands/<resource>.rs` is created as a placeholder in Task 5 first, then expanded in its owning task. Apply the placeholder pattern:

```bash
cd d:/projects/rusty/aegis
for f in healthz product project user user_credential; do
  printf '// Filled in by Tasks 6–10.\n' > "apps/desktop/aegis-desktop/src-tauri/src/commands/$f.rs"
done
```

Then declare them in `src/commands.rs` and add `mod commands;` in `lib.rs`.)

- [ ] **Step 3: Create `src/commands/auth.rs`**

Create `apps/desktop/aegis-desktop/src-tauri/src/commands/auth.rs` with:

```rust
use tauri::State;

use crate::http::auth::{self, LoginRequest};
use crate::http::client::HttpClient;
use crate::http::dto::ApiError;

#[tauri::command]
pub async fn login(
    client: State<'_, HttpClient>,
    code: String,
    password: String,
) -> Result<(), ApiError> {
    auth::login(&client, LoginRequest { code, password }).await
}

#[tauri::command]
pub async fn login_domain(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<(), ApiError> {
    auth::login_domain(&client, &code).await
}

#[tauri::command]
pub async fn is_logged_in(
    client: State<'_, HttpClient>,
) -> Result<bool, ApiError> {
    Ok(client.tokens().access_token().await?.is_some())
}

#[tauri::command]
pub async fn refresh(client: State<'_, HttpClient>) -> Result<(), ApiError> {
    auth::refresh(&client).await
}

#[tauri::command]
pub async fn logout(client: State<'_, HttpClient>) -> Result<(), ApiError> {
    auth::logout(&client).await
}
```

- [ ] **Step 4: Wire `mod commands;` into `lib.rs`**

Open `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. After the existing `mod http;` and `mod system;` lines, add:

```rust
mod commands;
```

`lib.rs` now has three top-level `mod` declarations:

```rust
mod http;
mod system;
mod commands;
```

- [ ] **Step 5: Run the auth tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib http::auth
```

Expected: All 3 auth tests pass:

- `http::auth::tests::login_persists_tokens`
- `http::auth::tests::login_propagates_401`
- `http::auth::tests::logout_clears_tokens_even_if_server_unreachable`

- [ ] **Step 6: Confirm `cargo check` is clean**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo check
```

Expected: Clean compilation. The `commands::auth` module compiles because `commands.rs` declares all six modules (five still have placeholder content; they will be filled in Tasks 6–10).

- [ ] **Step 7: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/auth.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/auth.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/healthz.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/user_credential.rs \
        apps/desktop/aegis-desktop/src-tauri/src/lib.rs \
        apps/desktop/aegis-desktop/src-tauri/src/http/client.rs
git commit -m "feat(desktop): add login / loginDomain / refresh / logout + commands"
```

---

### Task 6: http::user_credential + commands::user_credential

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/user_credential.rs` (replace stub from Task 5)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/user_credential.rs`

**Interfaces:**
- Produces:
  - `http::user_credential::register(c, body) -> Result<RegisterUserResponse, ApiError>`.
  - `http::user_credential::update(c, body) -> Result<UserCredentialViewResponse, ApiError>`.
  - `#[tauri::command] registerUser`, `#[tauri::command] updateUserCredential`.
- Consumes: `HttpClient` (T4), `http::dto::ApiError` (T2).

- [ ] **Step 1: Replace `src/http/user_credential.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/http/user_credential.rs` with:

```rust
//! User-credential management: register (admin/root) + self-service rotation.

use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::{ApiError, Role};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegisterUserRequest {
    pub user_code: String,
    pub user_name: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
    pub password: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RegisterUserResponse {
    pub user_code: String,
    pub user_name: String,
    pub role: Role,
    pub active: bool,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdateUserCredentialRequest {
    pub user_code: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserCredentialViewResponse {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

pub async fn register(
    c: &HttpClient,
    body: RegisterUserRequest,
) -> Result<RegisterUserResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/auth/user-credential", Some(&body)).await
}

pub async fn update(
    c: &HttpClient,
    body: UpdateUserCredentialRequest,
) -> Result<UserCredentialViewResponse, ApiError> {
    c.request(reqwest::Method::PATCH, "/api/auth/user-credential", Some(&body)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn register_round_trips_role() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        let m = Mock::given(method("POST")).and(path("/api/auth/user-credential"))
            .and(body_json(serde_json::json!({
                "user_code": "u", "user_name": "n",
                "domain_name": "d", "hostname": "h", "sid": "s",
                "password": "p"
            })))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "user_code": "u", "user_name": "n", "role": "general", "active": true,
                "domain_name": "d", "hostname": "h", "sid": "s"
            })));
        server.register(m).await;
        let c = HttpClient::new(server.uri(), store);
        let resp = register(&c, RegisterUserRequest {
            user_code: "u".into(), user_name: "n".into(),
            domain_name: "d".into(), hostname: "h".into(), sid: "s".into(),
            password: "p".into(),
        }).await.unwrap();
        assert_eq!(resp.role, Role::General);
        assert!(resp.active);
    }

    #[tokio::test]
    async fn update_with_no_password_skips_field_in_json() {
        let body = UpdateUserCredentialRequest { user_code: "u".into(), password: None };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"user_code":"u"}"#);
    }
}
```

- [ ] **Step 2: Replace `src/commands/user_credential.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/commands/user_credential.rs` with:

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::user_credential::{
    self, RegisterUserRequest, RegisterUserResponse, UpdateUserCredentialRequest,
    UserCredentialViewResponse,
};

#[tauri::command]
pub async fn register_user(
    client: State<'_, HttpClient>,
    user_code: String,
    user_name: String,
    domain_name: String,
    hostname: String,
    sid: String,
    password: String,
) -> Result<RegisterUserResponse, ApiError> {
    user_credential::register(&client, RegisterUserRequest {
        user_code, user_name, domain_name, hostname, sid, password,
    }).await
}

#[tauri::command]
pub async fn update_user_credential(
    client: State<'_, HttpClient>,
    user_code: String,
    password: Option<String>,
) -> Result<UserCredentialViewResponse, ApiError> {
    user_credential::update(&client, UpdateUserCredentialRequest { user_code, password }).await
}
```

- [ ] **Step 3: Run the tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib http::user_credential
```

Expected: 2 tests pass (`register_round_trips_role`, `update_with_no_password_skips_field_in_json`).

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/user_credential.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/user_credential.rs
git commit -m "feat(desktop): add user-credential register/update + commands"
```

---

### Task 7: http::user + commands::user

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/user.rs` (replace stub)
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs`

**Interfaces:**
- Produces:
  - `http::user::{create, get_by_code, list, update}` — 4 functions.
  - `UserView`, `UserListResponse`, `CreateUserRequest`, `UpdateUserRequest` wire DTOs.
  - 4 `#[tauri::command]` shims.
- Consumes: `HttpClient`, `ApiError`, `Role`, `chrono::DateTime<chrono::Utc>` for the response.

- [ ] **Step 1: Replace `src/http/user.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/http/user.rs` with:

```rust
//! User CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::{ApiError, Role};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserListResponse {
    pub users: Vec<UserViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateUserRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<Role>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
}

pub async fn create(c: &HttpClient, body: CreateUserRequest) -> Result<UserViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/user", Some(&body)).await
}

pub async fn list(c: &HttpClient) -> Result<Vec<UserViewResponse>, ApiError> {
    let resp: UserListResponse = c.request(reqwest::Method::GET, "/api/user", None::<&()>).await?;
    Ok(resp.users)
}

pub async fn get_by_code(c: &HttpClient, code: &str) -> Result<UserViewResponse, ApiError> {
    c.request(reqwest::Method::GET, &format!("/api/user/{code}"), None::<&()>).await
}

pub async fn update(c: &HttpClient, code: &str, body: UpdateUserRequest) -> Result<UserViewResponse, ApiError> {
    c.request(reqwest::Method::PATCH, &format!("/api/user/{code}"), Some(&body)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn list_returns_users() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server.register(
            Mock::given(method("GET")).and(path("/api/user"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "users": [
                        {
                            "id": 1, "code": "a", "name": "Alice",
                            "role": "admin", "active": true,
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-02T00:00:00Z",
                        }
                    ]
                })))
        ).await;
        let c = HttpClient::new(server.uri(), store);
        let users = list(&c).await.unwrap();
        assert_eq!(users.len(), 1);
        assert_eq!(users[0].code, "a");
        assert_eq!(users[0].created_at, Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap());
    }

    #[test]
    fn update_request_skips_none_fields() {
        let body = UpdateUserRequest { name: Some("Alice".into()), ..Default::default() };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"name":"Alice"}"#);
    }
}
```

- [ ] **Step 2: Replace `src/commands/user.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs` with:

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::user::{self, CreateUserRequest, UpdateUserRequest, UserViewResponse};

#[tauri::command]
pub async fn create_user(
    client: State<'_, HttpClient>,
    code: String,
    name: String,
    role: crate::http::dto::Role,
) -> Result<UserViewResponse, ApiError> {
    user::create(&client, CreateUserRequest { code, name, role }).await
}

#[tauri::command]
pub async fn list_users(client: State<'_, HttpClient>) -> Result<Vec<UserViewResponse>, ApiError> {
    user::list(&client).await
}

#[tauri::command]
pub async fn get_user_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<UserViewResponse, ApiError> {
    user::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_user(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateUserRequest,
) -> Result<UserViewResponse, ApiError> {
    user::update(&client, &code, body).await
}
```

- [ ] **Step 3: Run the tests**

Run from `apps/desktop/aegis-desktop/src-tauri/`:

```bash
cargo test --lib http::user
```

Expected: 2 tests pass (`list_returns_users`, `update_request_skips_none_fields`).

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/user.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/user.rs
git commit -m "feat(desktop): add user CRUD + commands"
```

---

### Task 8: http::product + commands::product

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/product.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs`

**Interfaces:**
- 4 product CRUD functions + 4 commands.

- [ ] **Step 1: Replace `src/http/product.rs`**

```rust
//! Product CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProductViewResponse {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProductListResponse {
    pub products: Vec<ProductViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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

pub async fn create(c: &HttpClient, body: CreateProductRequest) -> Result<ProductViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/product", Some(&body)).await
}

pub async fn list(c: &HttpClient) -> Result<Vec<ProductViewResponse>, ApiError> {
    let resp: ProductListResponse = c.request(reqwest::Method::GET, "/api/product", None::<&()>).await?;
    Ok(resp.products)
}

pub async fn get_by_code(c: &HttpClient, code: &str) -> Result<ProductViewResponse, ApiError> {
    c.request(reqwest::Method::GET, &format!("/api/product/{code}"), None::<&()>).await
}

pub async fn update(c: &HttpClient, code: &str, body: UpdateProductRequest) -> Result<ProductViewResponse, ApiError> {
    c.request(reqwest::Method::PATCH, &format!("/api/product/{code}"), Some(&body)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn list_returns_products() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server.register(
            Mock::given(method("GET")).and(path("/api/product"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "products": [{
                        "id": 1, "code": "x", "name": "X",
                        "description": "", "active": true,
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-02T00:00:00Z"
                    }]
                })))
        ).await;
        let c = HttpClient::new(server.uri(), store);
        let products = list(&c).await.unwrap();
        assert_eq!(products.len(), 1);
        assert_eq!(products[0].code, "x");
    }

    #[test]
    fn update_skips_none() {
        let body = UpdateProductRequest { active: Some(false), ..Default::default() };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"active":false}"#);
    }
}
```

- [ ] **Step 2: Replace `src/commands/product.rs`**

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::product::{self, CreateProductRequest, UpdateProductRequest, ProductViewResponse};

#[tauri::command]
pub async fn create_product(
    client: State<'_, HttpClient>,
    code: String,
    name: String,
    description: String,
) -> Result<ProductViewResponse, ApiError> {
    product::create(&client, CreateProductRequest { code, name, description }).await
}

#[tauri::command]
pub async fn list_products(client: State<'_, HttpClient>) -> Result<Vec<ProductViewResponse>, ApiError> {
    product::list(&client).await
}

#[tauri::command]
pub async fn get_product_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<ProductViewResponse, ApiError> {
    product::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_product(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateProductRequest,
) -> Result<ProductViewResponse, ApiError> {
    product::update(&client, &code, body).await
}
```

- [ ] **Step 3: Run tests**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop/src-tauri
cargo test --lib http::product
```

Expected: 2 tests pass.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/product.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs
git commit -m "feat(desktop): add product CRUD + commands"
```

---

### Task 9: http::project + commands::project

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs`

**Interfaces:**
- 4 project CRUD functions + 4 commands.
- `ProjectMemberDataRequest { leaders: Vec<String>, workers: Vec<String> }` and `ProjectMemberViewResponse { leaders: Vec<UserSummary>, workers: Vec<UserSummary> }`.

- [ ] **Step 1: Replace `src/http/project.rs`**

```rust
//! Project CRUD.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::client::HttpClient;
use super::dto::ApiError;
use super::product::ProductViewResponse;

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectMemberDataRequest {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaders: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSummaryViewResponse {
    pub code: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectMemberViewResponse {
    pub leaders: Vec<UserSummaryViewResponse>,
    pub workers: Vec<UserSummaryViewResponse>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductViewResponse,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectListResponse {
    pub projects: Vec<ProjectViewResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub product_id: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
}

pub async fn create(c: &HttpClient, body: CreateProjectRequest) -> Result<ProjectViewResponse, ApiError> {
    c.request(reqwest::Method::POST, "/api/project", Some(&body)).await
}

pub async fn list(c: &HttpClient) -> Result<Vec<ProjectViewResponse>, ApiError> {
    let resp: ProjectListResponse = c.request(reqwest::Method::GET, "/api/project", None::<&()>).await?;
    Ok(resp.projects)
}

pub async fn get_by_code(c: &HttpClient, code: &str) -> Result<ProjectViewResponse, ApiError> {
    c.request(reqwest::Method::GET, &format!("/api/project/{code}"), None::<&()>).await
}

pub async fn update(c: &HttpClient, code: &str, body: UpdateProjectRequest) -> Result<ProjectViewResponse, ApiError> {
    c.request(reqwest::Method::PATCH, &format!("/api/project/{code}"), Some(&body)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn list_returns_projects() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        store.set_access_token("AT").await.unwrap();
        store.set_refresh_token("RT").await.unwrap();
        server.register(
            Mock::given(method("GET")).and(path("/api/project"))
                .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "projects": [{
                        "id": 1, "code": "p", "description": "",
                        "product": {
                            "id": 1, "code": "x", "name": "X", "description": "",
                            "active": true,
                            "created_at": "2026-01-01T00:00:00Z",
                            "updated_at": "2026-01-02T00:00:00Z"
                        },
                        "members": { "leaders": [], "workers": [] },
                        "unblind_members": { "leaders": [], "workers": [] },
                        "active": true,
                        "created_at": "2026-01-01T00:00:00Z",
                        "updated_at": "2026-01-02T00:00:00Z"
                    }]
                })))
        ).await;
        let c = HttpClient::new(server.uri(), store);
        let projects = list(&c).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].code, "p");
        assert_eq!(projects[0].product.code, "x");
    }

    #[test]
    fn update_skips_none_fields() {
        let body = UpdateProjectRequest { active: Some(false), ..Default::default() };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"active":false}"#);
    }

    #[test]
    fn project_member_data_request_omits_empty_arrays() {
        let body = ProjectMemberDataRequest::default();
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, "{}");
        let body = ProjectMemberDataRequest { leaders: vec!["a".into()], workers: vec![] };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"leaders":["a"]}"#);
    }
}
```

- [ ] **Step 2: Replace `src/commands/project.rs`**

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::project::{
    self, CreateProjectRequest, ProjectMemberDataRequest, ProjectViewResponse,
    UpdateProjectRequest,
};

#[tauri::command]
pub async fn create_project(
    client: State<'_, HttpClient>,
    code: String,
    description: String,
    product_id: i32,
    members: Option<ProjectMemberDataRequest>,
    unblind_members: Option<ProjectMemberDataRequest>,
) -> Result<ProjectViewResponse, ApiError> {
    project::create(&client, CreateProjectRequest {
        code, description, product_id, members, unblind_members,
    }).await
}

#[tauri::command]
pub async fn list_projects(client: State<'_, HttpClient>) -> Result<Vec<ProjectViewResponse>, ApiError> {
    project::list(&client).await
}

#[tauri::command]
pub async fn get_project_by_code(
    client: State<'_, HttpClient>,
    code: String,
) -> Result<ProjectViewResponse, ApiError> {
    project::get_by_code(&client, &code).await
}

#[tauri::command]
pub async fn update_project(
    client: State<'_, HttpClient>,
    code: String,
    body: UpdateProjectRequest,
) -> Result<ProjectViewResponse, ApiError> {
    project::update(&client, &code, body).await
}
```

- [ ] **Step 3: Run tests**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop/src-tauri
cargo test --lib http::project
```

Expected: 3 tests pass.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/project.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/project.rs
git commit -m "feat(desktop): add project CRUD + commands"
```

---

### Task 10: http::healthz + commands::healthz

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/healthz.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/healthz.rs`

**Interfaces:**
- `http::healthz::ping(&HttpClient) -> Result<String, ApiError>` returns `"ok"`.
- `#[tauri::command] healthz`.

- [ ] **Step 1: Replace `src/http/healthz.rs`**

```rust
//! `GET /healthz` — no auth, returns the server's "ok" probe.

use super::client::HttpClient;
use super::dto::ApiError;

pub async fn ping(c: &HttpClient) -> Result<String, ApiError> {
    let bytes = c.request_bytes(reqwest::Method::GET, "/healthz", None::<&()>).await?;
    String::from_utf8(bytes).map_err(|e| ApiError::Http {
        status: 0, code: "decode_failed".into(), message: e.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use wiremock::matchers::method;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::super::client::{HttpClient, MemoryStore};

    #[tokio::test]
    async fn ping_returns_ok() {
        let server = MockServer::start().await;
        let store = Arc::new(MemoryStore::default());
        server.register(
            Mock::given(method("GET")).and(path("/healthz"))
                .respond_with(ResponseTemplate::new(200)
                    .insert_header("content-type", "text/plain; charset=utf-8")
                    .set_body_string("ok"))
        ).await;
        let c = HttpClient::new(server.uri(), store);
        assert_eq!(ping(&c).await.unwrap(), "ok");
    }
}
```

- [ ] **Step 2: Replace `src/commands/healthz.rs`**

```rust
use tauri::State;

use crate::http::client::HttpClient;
use crate::http::dto::ApiError;
use crate::http::healthz;

#[tauri::command]
pub async fn healthz(client: State<'_, HttpClient>) -> Result<String, ApiError> {
    healthz::ping(&client).await
}
```

- [ ] **Step 3: Run tests**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop/src-tauri
cargo test --lib http::healthz
```

Expected: 1 test pass.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/http/healthz.rs \
        apps/desktop/aegis-desktop/src-tauri/src/commands/healthz.rs
git commit -m "feat(desktop): add healthz ping + command"
```

---

---

### Task 11: bootstrap (lib.rs + main.rs + invoke_handler)

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/main.rs`

**Interfaces:**
- Produces:
  - `pub fn run() -> Result<(), Box<dyn std::error::Error>>` — boots Tauri, opens `auth.bin`, registers the store, manages `HttpClient`, generates the full `invoke_handler!`.

- [ ] **Step 1: Replace `src/lib.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/lib.rs` with:

```rust
use std::sync::Arc;

use tauri::Manager;
use tauri_plugin_store::StoreExt;

mod commands;
mod http;
mod system;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            // auth
            commands::auth::login,
            commands::auth::login_domain,
            commands::auth::is_logged_in,
            commands::auth::refresh,
            commands::auth::logout,
            // user-credential
            commands::user_credential::register_user,
            commands::user_credential::update_user_credential,
            // user
            commands::user::create_user,
            commands::user::list_users,
            commands::user::get_user_by_code,
            commands::user::update_user,
            // product
            commands::product::create_product,
            commands::product::list_products,
            commands::product::get_product_by_code,
            commands::product::update_product,
            // project
            commands::project::create_project,
            commands::project::list_projects,
            commands::project::get_project_by_code,
            commands::project::update_project,
            // health
            commands::healthz::healthz,
        ])
        .setup(|app| {
            let store = app.store("auth.bin")
                .map_err(|e| format!("failed to open auth.bin store: {e}"))?;
            let tokens = Arc::new(http::client::TauriStore::new(store));
            let client = http::client::HttpClient::new(
                http::config::BASE_URL.to_string(),
                tokens,
            );
            app.manage(client);
            Ok(())
        })
        .build(tauri::generate_context!())?;

    app.run(tauri::generate_context!())?;
    Ok(())
}
```

- [ ] **Step 2: Replace `src/main.rs`**

Overwrite `apps/desktop/aegis-desktop/src-tauri/src/main.rs` with:

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    aegis_desktop_lib::run().expect("error while running tauri application");
}
```

(`main.rs` is essentially unchanged — only the call site gains `Result` propagation. If the previous content already matches, leave it.)

- [ ] **Step 3: Run cargo check + the full test suite**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop/src-tauri
cargo check
cargo test --lib
```

Expected: no errors, no warnings from the new code. Every test across all `http::*` modules passes.

- [ ] **Step 4: Build the app**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop/src-tauri
cargo build
```

Expected: `Finished` with no compile errors. (A `cargo build` of `src-tauri/` only builds the Rust binary, not the React frontend — that's a separate `pnpm tauri dev` command which is part of the smoke test in Task 14.)

- [ ] **Step 5: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src-tauri/src/lib.rs \
        apps/desktop/aegis-desktop/src-tauri/src/main.rs
git commit -m "feat(desktop): bootstrap HttpClient + register 20 commands"
```

---

### Task 12: TS wire-DTO type aliases

**Files:**
- Create: `apps/desktop/aegis-desktop/src/api/types.ts`

**Interfaces:**
- Produces: TS aliases matching every Rust wire DTO. Used by `src/api/index.ts` and any consumer that needs the structured types.

- [ ] **Step 1: Create `src/api/types.ts`**

Create `apps/desktop/aegis-desktop/src/api/types.ts` with this exact content:

```ts
// Wire-DTO mirrors. Hand-maintained — every shape matches the Rust DTO in
// `apps/desktop/aegis-desktop/src-tauri/src/http/*` 1:1. The server returns
// RFC 3339 timestamps as strings; we parse them with `Date.parse` where
// callers need a `Date`.

export type Role = "root" | "admin" | "general";

export interface ErrorBody {
  code: string;
  message: string;
}

// Mirrors `http::dto::ApiError` (`#[serde(tag = "kind", rename_all = "snake_case")]`).
export type ApiError =
  | { kind: "network"; 0: string }
  | { kind: "http"; status: number; code: string; message: string }
  | { kind: "refresh_failed" }
  | { kind: "not_implemented"; 0: string }
  | { kind: "store"; 0: string };

// Auth
export interface RegisterUserInput {
  userCode: string;
  userName: string;
  domainName: string;
  hostname: string;
  sid: string;
  password: string;
}
export interface RegisterUserResponse {
  user_code: string;
  user_name: string;
  role: Role;
  active: boolean;
  domain_name: string;
  hostname: string;
  sid: string;
}
export interface UserCredentialView {
  user_code: string;
  password_hash: string;
  token_version: number;
}
export interface UpdateUserCredentialInput { userCode: string; password?: string }

// User
export interface UserView {
  id: number;
  code: string;
  name: string;
  role: Role;
  active: boolean;
  created_at: string;
  updated_at: string;
}
export interface CreateUserInput { code: string; name: string; role: Role }
export interface UpdateUserBody { code?: string; name?: string; role?: Role; active?: boolean }

// Product
export interface ProductView {
  id: number;
  code: string;
  name: string;
  description: string;
  active: boolean;
  created_at: string;
  updated_at: string;
}
export interface CreateProductInput { code: string; name: string; description: string }
export interface UpdateProductBody { code?: string; name?: string; description?: string; active?: boolean }

// Project
export interface UserSummary { code: string; name: string }
export interface ProjectMembers {
  leaders?: string[];
  workers?: string[];
}
export interface ProjectMembersView { leaders: UserSummary[]; workers: UserSummary[] }
export interface ProjectView {
  id: number;
  code: string;
  description: string;
  product: ProductView;
  members: ProjectMembersView;
  unblind_members: ProjectMembersView;
  active: boolean;
  created_at: string;
  updated_at: string;
}
export interface CreateProjectInput {
  code: string;
  description: string;
  productId: number;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
}
export interface UpdateProjectBody {
  code?: string;
  description?: string;
  productId?: number;
  active?: boolean;
  members?: ProjectMembers;
  unblindMembers?: ProjectMembers;
}
```

Note: `ApiError`'s tuple form `{ kind: "network"; 0: string }` mirrors how serde serializes a single-field newtype variant — the field's contents end up at the synthetic key `"0"`. The frontend is encouraged to read the typed `Display` form for user-facing strings and switch on `kind` for control flow; the field name `"0"` is a serde newtype convention.

- [ ] **Step 2: Verify typecheck**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: No errors. The types compile and resolve.

- [ ] **Step 3: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/api/types.ts
git commit -m "feat(desktop): add api/types.ts wire-DTO aliases"
```

---

### Task 13: TS API wrapper + per-command tests

**Files:**
- Create: `apps/desktop/aegis-desktop/src/api/index.ts`
- Create: `apps/desktop/aegis-desktop/src/test/api.test.ts`

**Interfaces:**
- Produces:
  - `import { api } from "../api"` gives 20 typed wrapper functions.
  - 20 unit tests verifying each wrapper calls `invoke` with the right command name and argument shape.

- [ ] **Step 1: Create `src/api/index.ts`**

Create `apps/desktop/aegis-desktop/src/api/index.ts` with this exact content:

```ts
import { invoke } from "@tauri-apps/api/core";

import type {
  ApiError,
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  ProductView,
  ProjectView,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserView,
} from "./types";

export const api = {
  // auth
  login: (code: string, password: string): Promise<void> =>
    invoke<void>("login", { code, password }),
  loginDomain: (code: string): Promise<void> =>
    invoke<void>("loginDomain", { code }),
  isLoggedIn: (): Promise<boolean> => invoke<boolean>("isLoggedIn"),
  refresh: (): Promise<void> => invoke<void>("refresh"),
  logout: (): Promise<void> => invoke<void>("logout"),

  // user-credential
  registerUser: (input: RegisterUserInput): Promise<RegisterUserResponse> =>
    invoke<RegisterUserResponse>("registerUser", input),
  updateUserCredential: (
    input: UpdateUserCredentialInput,
  ): Promise<UserCredentialView> =>
    invoke<UserCredentialView>("updateUserCredential", input),

  // user
  createUser: (input: CreateUserInput): Promise<UserView> =>
    invoke<UserView>("createUser", input),
  listUsers: (): Promise<UserView[]> => invoke<UserView[]>("listUsers"),
  getUserByCode: (code: string): Promise<UserView> =>
    invoke<UserView>("getUserByCode", { code }),
  updateUser: (code: string, body: UpdateUserBody): Promise<UserView> =>
    invoke<UserView>("updateUser", { code, body }),

  // product
  createProduct: (input: CreateProductInput): Promise<ProductView> =>
    invoke<ProductView>("createProduct", input),
  listProducts: (): Promise<ProductView[]> => invoke<ProductView[]>("listProducts"),
  getProductByCode: (code: string): Promise<ProductView> =>
    invoke<ProductView>("getProductByCode", { code }),
  updateProduct: (code: string, body: UpdateProductBody): Promise<ProductView> =>
    invoke<ProductView>("updateProduct", { code, body }),

  // project
  createProject: (input: CreateProjectInput): Promise<ProjectView> =>
    invoke<ProjectView>("createProject", input),
  listProjects: (): Promise<ProjectView[]> => invoke<ProjectView[]>("listProjects"),
  getProjectByCode: (code: string): Promise<ProjectView> =>
    invoke<ProjectView>("getProjectByCode", { code }),
  updateProject: (code: string, body: UpdateProjectBody): Promise<ProjectView> =>
    invoke<ProjectView>("updateProject", { code, body }),

  // health
  healthz: (): Promise<string> => invoke<string>("healthz"),
} as const;

// Re-export `ApiError` and the wire types so consumers can `import { ApiError, UserView } from "../api";`.
export type { ApiError } from "./types";
export type {
  CreateProductInput,
  CreateProjectInput,
  CreateUserInput,
  ProductView,
  ProjectMembers,
  ProjectMembersView,
  ProjectView,
  Role,
  RegisterUserInput,
  RegisterUserResponse,
  UpdateProductBody,
  UpdateProjectBody,
  UpdateUserBody,
  UpdateUserCredentialInput,
  UserCredentialView,
  UserSummary,
  UserView,
} from "./types";
```

- [ ] **Step 2: Create `src/test/api.test.ts`**

Create `apps/desktop/aegis-desktop/src/test/api.test.ts` with this exact content:

```ts
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

import { api } from "../api";

const mockInvoke = invoke as unknown as ReturnType<typeof vi.fn>;

beforeEach(() => { mockInvoke.mockReset(); });
afterEach(() => { vi.restoreAllMocks(); });

describe("api wrappers", () => {
  it("login -> invoke('login', { code, password })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.login("alice", "secret");
    expect(mockInvoke).toHaveBeenCalledWith("login", { code: "alice", password: "secret" });
  });

  it("loginDomain -> invoke('loginDomain', { code })", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.loginDomain("alice");
    expect(mockInvoke).toHaveBeenCalledWith("loginDomain", { code: "alice" });
  });

  it("isLoggedIn -> invoke('isLoggedIn')", async () => {
    mockInvoke.mockResolvedValueOnce(true);
    await api.isLoggedIn();
    expect(mockInvoke).toHaveBeenCalledWith("isLoggedIn");
  });

  it("refresh -> invoke('refresh')", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.refresh();
    expect(mockInvoke).toHaveBeenCalledWith("refresh");
  });

  it("logout -> invoke('logout')", async () => {
    mockInvoke.mockResolvedValueOnce(undefined);
    await api.logout();
    expect(mockInvoke).toHaveBeenCalledWith("logout");
  });

  it("registerUser -> invoke('registerUser', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.registerUser({ userCode: "u", userName: "n", domainName: "d", hostname: "h", sid: "s", password: "p" });
    expect(mockInvoke).toHaveBeenCalledWith("registerUser", {
      userCode: "u", userName: "n", domainName: "d", hostname: "h", sid: "s", password: "p",
    });
  });

  it("updateUserCredential -> invoke('updateUserCredential', { userCode, password? })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUserCredential({ userCode: "u", password: "p" });
    expect(mockInvoke).toHaveBeenCalledWith("updateUserCredential", { userCode: "u", password: "p" });
  });

  it("createUser -> invoke('createUser', { code, name, role })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createUser({ code: "u", name: "Alice", role: "admin" });
    expect(mockInvoke).toHaveBeenCalledWith("createUser", { code: "u", name: "Alice", role: "admin" });
  });

  it("listUsers -> invoke('listUsers')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listUsers();
    expect(mockInvoke).toHaveBeenCalledWith("listUsers");
  });

  it("getUserByCode -> invoke('getUserByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getUserByCode("alice");
    expect(mockInvoke).toHaveBeenCalledWith("getUserByCode", { code: "alice" });
  });

  it("updateUser -> invoke('updateUser', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateUser("alice", { name: "Alicia" });
    expect(mockInvoke).toHaveBeenCalledWith("updateUser", { code: "alice", body: { name: "Alicia" } });
  });

  it("createProduct -> invoke('createProduct', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createProduct({ code: "p", name: "P", description: "" });
    expect(mockInvoke).toHaveBeenCalledWith("createProduct", { code: "p", name: "P", description: "" });
  });

  it("listProducts -> invoke('listProducts')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listProducts();
    expect(mockInvoke).toHaveBeenCalledWith("listProducts");
  });

  it("getProductByCode -> invoke('getProductByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getProductByCode("p");
    expect(mockInvoke).toHaveBeenCalledWith("getProductByCode", { code: "p" });
  });

  it("updateProduct -> invoke('updateProduct', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateProduct("p", { active: false });
    expect(mockInvoke).toHaveBeenCalledWith("updateProduct", { code: "p", body: { active: false } });
  });

  it("createProject -> invoke('createProject', input)", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.createProject({ code: "p", description: "", productId: 1 });
    expect(mockInvoke).toHaveBeenCalledWith("createProject", { code: "p", description: "", productId: 1 });
  });

  it("listProjects -> invoke('listProjects')", async () => {
    mockInvoke.mockResolvedValueOnce([]);
    await api.listProjects();
    expect(mockInvoke).toHaveBeenCalledWith("listProjects");
  });

  it("getProjectByCode -> invoke('getProjectByCode', { code })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.getProjectByCode("p");
    expect(mockInvoke).toHaveBeenCalledWith("getProjectByCode", { code: "p" });
  });

  it("updateProject -> invoke('updateProject', { code, body })", async () => {
    mockInvoke.mockResolvedValueOnce({});
    await api.updateProject("p", { active: false });
    expect(mockInvoke).toHaveBeenCalledWith("updateProject", { code: "p", body: { active: false } });
  });

  it("healthz -> invoke('healthz')", async () => {
    mockInvoke.mockResolvedValueOnce("ok");
    await api.healthz();
    expect(mockInvoke).toHaveBeenCalledWith("healthz");
  });
});
```

- [ ] **Step 3: Run the tests**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop
pnpm test
```

Expected: All 20 api tests pass. The `home.tsx` `greet` test still passes; the `routes/{__root,settings,index}` tests still pass.

- [ ] **Step 4: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/api/index.ts \
        apps/desktop/aegis-desktop/src/test/api.test.ts
git commit -m "feat(desktop): add typed api wrappers + per-command tests"
```

---

### Task 14: home.tsx login smoke test

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/pages/home.tsx`
- Modify: `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx` (existing test that calls `greet`)

**Interfaces:**
- The Home page gains a small login form exercising `api.login` + `api.isLoggedIn` as the first end-to-end smoke test for the new stack.

- [ ] **Step 1: Replace `src/pages/home.tsx`**

Overwrite `apps/desktop/aegis-desktop/src/pages/home.tsx` with:

```tsx
import { useState } from "react";
import { Box, Button, Stack, TextField, Typography } from "@aegis/ui/mui";
import { useI18n } from "@aegis/ui/i18n";

import { api } from "../api";

export function HomePage() {
  const { t } = useI18n();
  const [code, setCode] = useState("");
  const [password, setPassword] = useState("");
  const [loggedIn, setLoggedIn] = useState<boolean | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refreshLoginState() {
    try { setLoggedIn(await api.isLoggedIn()); }
    catch (e) { setError(String(e)); }
  }

  async function onLogin() {
    setError(null);
    try {
      await api.login(code, password);
      await refreshLoginState();
    } catch (e) { setError(String(e)); }
  }

  async function onLogout() {
    setError(null);
    try { await api.logout(); await refreshLoginState(); }
    catch (e) { setError(String(e)); }
  }

  return (
    <Box sx={{ p: 4 }}>
      <Typography variant="h4" gutterBottom>{t("home.heading")}</Typography>
      <Typography variant="body1" sx={{ mb: 3 }}>{t("home.welcome")}</Typography>

      <Stack direction="row" spacing={2} sx={{ alignItems: "center", mb: 2 }}>
        <TextField label="code" value={code} onChange={(e) => setCode(e.target.value)} size="small" />
        <TextField label="password" type="password" value={password} onChange={(e) => setPassword(e.target.value)} size="small" />
        <Button variant="contained" onClick={onLogin}>Login</Button>
        <Button variant="outlined" onClick={onLogout}>Logout</Button>
      </Stack>

      <Typography variant="body2">
        Logged in: {loggedIn === null ? "?" : String(loggedIn)}
      </Typography>
      {error && <Typography variant="body2" color="error">{error}</Typography>}
    </Box>
  );
}
```

Note: The existing test in `src/test/routes/index.test.tsx` mocks `invoke` and asserts on a `greet` button. With this page replacement, that test must be updated to match the new content. Update the test file in Step 2.

- [ ] **Step 2: Update `src/test/routes/index.test.tsx` to match the new home page**

Open `apps/desktop/aegis-desktop/src/test/routes/index.test.tsx`. Replace the existing content with:

```tsx
import { invoke } from "@tauri-apps/api/core";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";

import { AegisI18nProvider, AegisThemeProvider } from "@aegis/ui";
import { HomePage } from "../../pages/home";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));

beforeEach(() => { vi.restoreAllMocks(); vi.unstubAllGlobals(); });
afterEach(() => { vi.useRealTimers(); });

function renderHome() {
  return render(
    <AegisThemeProvider>
      <AegisI18nProvider defaultLocale="en">
        <HomePage />
      </AegisI18nProvider>
    </AegisThemeProvider>,
  );
}

describe("HomePage", () => {
  it("renders the login form and triggers invoke on submit", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(undefined); // login
    (invoke as unknown as ReturnType<typeof vi.fn>).mockResolvedValueOnce(true);     // isLoggedIn
    renderHome();
    await userEvent.type(screen.getByLabelText(/code/i), "alice");
    await userEvent.type(screen.getByLabelText(/password/i), "secret");
    await userEvent.click(screen.getByRole("button", { name: /login/i }));
    expect(invoke).toHaveBeenCalledWith("login", { code: "alice", password: "secret" });
    expect(invoke).toHaveBeenCalledWith("isLoggedIn");
  });

  it("displays an error message when the login call fails", async () => {
    (invoke as unknown as ReturnType<typeof vi.fn>).mockRejectedValueOnce(
      Object.assign(new Error("boom"), { kind: "http", status: 401, code: "invalid_credentials", message: "nope" }),
    );
    renderHome();
    await userEvent.type(screen.getByLabelText(/code/i), "alice");
    await userEvent.type(screen.getByLabelText(/password/i), "wrong");
    await userEvent.click(screen.getByRole("button", { name: /login/i }));
    expect(await screen.findByText(/invalid_credentials|nope|boom/)).toBeInTheDocument();
  });
});
```

If your test imports `renderInRouter`/`renderWithFullRouter` rather than `render`, adjust accordingly — the structural pattern (`AegisThemeProvider` → `AegisI18nProvider` → `HomePage`) is what matters.

- [ ] **Step 3: Run all frontend tests**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop
pnpm test
```

Expected: every test passes — the 20 `api.test.ts` tests, the 2 updated `HomePage` tests, and the unchanged `__root`/`settings`/`index` page tests.

- [ ] **Step 4: Run typecheck**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop
pnpm typecheck
```

Expected: clean.

- [ ] **Step 5: Manual smoke (sanity check, not a CI gate)**

```bash
cd d:/projects/rusty/aegis/apps/desktop/aegis-desktop
pnpm tauri dev
```

Expected: the Tauri window opens, the home page renders, and clicking `Login` against a running `aegis-server` updates `Logged in: true`. If no server is available, the page should still render without crashing — the failure surfaces as an error message rather than a blank screen.

- [ ] **Step 6: Commit**

```bash
cd d:/projects/rusty/aegis
git add apps/desktop/aegis-desktop/src/pages/home.tsx \
        apps/desktop/aegis-desktop/src/test/routes/index.test.tsx
git commit -m "feat(desktop): replace greet with login-form smoke test"
```

---

## Done

All 14 tasks complete. Final outcome:

- `src-tauri/` exposes 20 `#[tauri::command]` shims covering the full server catalog.
- Bearer tokens persist via `tauri_plugin_store`; auto-refresh on 401 with single-attempt retry.
- `windows-utils` powers `loginDomain` via a `cfg`-gated wrapper.
- Compile-time `AEGIS_SERVER_URL` (default `http://localhost:8080`).
- `src/api/index.ts` is the typed frontend surface; `src/test/api.test.ts` exhaustively covers each wrapper.
- Tests use `wiremock` (Rust) and `vi.mock("@tauri-apps/api/core", ...)` (TS), matching existing conventions.

Open items (out of scope for this plan):
- Cross-platform compile of `src-tauri/` is gated on `windows-utils`; non-Windows CI will need either skipping `src-tauri` or `#[cfg(target_os = "windows")]`-gating the dependency itself.
- The `user-credential` Bearer-exclusion is a known runtime discrepancy with the server (see `docs/superpowers/specs/2026-08-12-aegis-desktop-http-client-design.md` Open Risks); resolve via a server-side change when ready.


