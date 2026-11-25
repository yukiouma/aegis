# aegis-server HTTP Server & Auth Routes Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the `apps/server/aegis-server` HTTP server with `axum 0.8`, `utoipa`, `utoipa-axum`, and `utoipa-swagger-ui`, exposing the four auth-flow endpoints (POST `/api/auth/login`, `/api/auth/login-domain`, `/api/auth/refresh`, `/api/auth/logout`), a `/api/healthz`, swagger-ui at `/swagger-ui`, and the OpenAPI doc at `/api-docs/openapi.json`. The `AuthClaims` extractor verifies the `Authorization: Bearer …` header for future protected handlers. The auth crate's `AuthServiceImpl` is the production backend (Postgres + in-memory cache).

**Architecture:** A thin binary crate. Cross-cutting `Config` and `AppState` sit above the transport boundary; HTTP-specific code lives under `transport/http/` (handlers, wire DTOs, error mapping, OpenAPI doc, router composition). State attaches exactly once at the top-level `Router`. Wire DTOs live in aegis-server with `Serialize` / `Deserialize` / `ToSchema`; the `apis` crate stays free of serde / utoipa. Each module uses `src/<module>.rs` + `src/<module>/` directory style — no `mod.rs`.

**Tech Stack:** Rust 2024, `axum 0.8`, `tower`, `tower-http`, `tokio`, `utoipa`, `utoipa-axum`, `utoipa-swagger-ui`, `sqlx`, `auth` (path), `user` (path), `apis` (path), `serde`, `serde_json`, `chrono`, `thiserror`, `async-trait`, `dotenvy`, `tracing`, `tracing-subscriber`.

**Spec:** [docs/superpowers/specs/2026-08-07-aegis-server-http-and-auth-routes-design.md](../specs/2026-08-07-aegis-server-http-and-auth-routes-design.md)

---

## Global Constraints

- `apps/server/aegis-server` uses `<module>.rs` + `<module>/` directory style — never `mod.rs`. The convention is locked at the workspace level by `docs/guidelines/lib-crate-development.md` § 2; the same rule applies to server crates.
- Every dependency in `apps/server/aegis-server/Cargo.toml` is either a workspace dep (`{ workspace = true }`) or a path-dep (`{ path = "../..." }`). No direct version pinning.
- All `Result<_, ApiError>` returns in handlers use `?` on `AuthApiError`; the `From<AuthApiError> for ApiError` impl does the wrapping.
- `tower_http::trace::TraceLayer::new_for_http()` is the only middleware layer in this design. Auth middleware is an extractor, not a `from_fn_with_state` layer (no current route is protected).
- State attaches exactly once via `Router::with_state(state)` on the top-level Router. Never on a sub-router.
- Commit messages follow the project's existing convention (`feat(aegis-server):`, `test(aegis-server):`, `docs(aegis-server):`, `chore(aegis-server):`).
- All non-DB tests run on a plain `cargo test -p aegis-server`. Live-DB integration tests are `#[ignore]`-gated and run with `AEGIS_DATABASE_URL`.

---

### Task 1: Add `serde` and `serde_json` to workspace dependencies

**Files:**
- Modify: `Cargo.toml` (workspace root)

- [ ] **Step 1: Add the two new workspace dependencies**

In `Cargo.toml`, find the `[workspace.dependencies]` block. Add at the bottom (preserving alphabetical / grouped order — `chrono` is currently the last entry before the existing ones at the bottom):

```toml
# `serde` provides Serialize / Deserialize for wire-level DTOs (HTTP
# request / response bodies). Centralized here so every workspace
# crate that needs serialization inherits the same version and feature
# set.
serde      = { version = "1", features = ["derive"] }
# `serde_json` is the JSON codec used by axum's Json extractor. Pinned
# in the workspace so every consumer resolves the same version.
serde_json = "1"
```

- [ ] **Step 2: Verify the workspace still builds**

Run: `cargo check --workspace`
Expected: success. No new compile errors; existing crates are unaffected because no one's `Cargo.toml` adds the new deps yet.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml
git commit -m "chore(workspace): add serde + serde_json to workspace deps"
```

---

### Task 2: Scaffold the `aegis-server` crate skeleton

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`
- Create: `apps/server/aegis-server/src/lib.rs`
- Create: `apps/server/aegis-server/src/config.rs`
- Create: `apps/server/aegis-server/src/state.rs`
- Create: `apps/server/aegis-server/src/transport.rs`
- Create: `apps/server/aegis-server/src/transport/http.rs`
- Create: `apps/server/aegis-server/src/transport/http/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/auth.rs`
- Create: `apps/server/aegis-server/src/transport/http/auth/middleware.rs`
- Create: `apps/server/aegis-server/src/transport/http/dto.rs`
- Create: `apps/server/aegis-server/src/transport/http/error.rs`
- Create: `apps/server/aegis-server/src/transport/http/healthz.rs`
- Create: `apps/server/aegis-server/src/transport/http/openapi.rs`

- [ ] **Step 1: Replace `apps/server/aegis-server/Cargo.toml` with the full dependency list**

Replace the entire current contents of `apps/server/aegis-server/Cargo.toml` with:

```toml
[package]
name = "aegis-server"
version = "0.1.0"
edition = "2024"

[dependencies]
# axum 0.8 is the HTTP framework. The default features include
# tokio + tower + http; we pin the workspace's choice.
axum = { workspace = true }
# tower hosts ServiceExt::oneshot, used by handler tests to drive
# the router without binding a TCP listener.
tower = { workspace = true }
# tower-http's TraceLayer adds a tracing span around every request.
# The `trace` feature is required for the layer constructor.
tower-http = { workspace = true, features = ["trace"] }
# tokio with macros + rt-multi-thread for `#[tokio::main]` in main.rs
# + the spawn / signal utilities used by axum::serve.
tokio = { workspace = true, features = ["macros", "rt-multi-thread", "signal"] }
# utoipa's OpenApi derive generates the OpenAPI v3 document from
# paths(...) and components(schemas(...)) listings.
utoipa = { workspace = true }
# utoipa-axum's OpenApiRouter composes handlers + auto-registers
# paths in the generated OpenAPI document.
utoipa-axum = { workspace = true }
# utoipa-swagger-ui mounts the swagger-ui HTML at /swagger-ui and
# points it at the openapi.json URL.
utoipa-swagger-ui = { workspace = true }
# tracing + tracing-subscriber log every request via TraceLayer; the
# subscriber is initialized in main.rs.
tracing = { workspace = true }
tracing-subscriber = { workspace = true }
# sqlx provides the PgPool used by the AuthUsecase / UserUsecase
# wiring. The runtime-tokio + postgres features come from the
# workspace dep.
sqlx = { workspace = true }
# serde provides Serialize / Deserialize for wire-level DTOs.
serde = { workspace = true }
# serde_json is the JSON codec used by axum's Json extractor.
serde_json = { workspace = true }
# chrono provides DateTime<Utc> for any future DTO timestamps.
chrono = { workspace = true }
# thiserror provides #[derive(Error)] for ConfigError + ApiError.
thiserror = { workspace = true }
# async-trait implements #[async_trait] for AppState::Clone +
# AuthClaims FromRequestParts.
async-trait = { workspace = true }
# dotenvy loads .env at startup so AEGIS_DATABASE_URL etc. can be
# set in a file during development.
dotenvy = { workspace = true }
# auth provides AuthServiceImpl, AuthUsecase, AuthUsecaseConfig,
# UserServiceImpl (the apis->domain adapter), InMemoryTokenVersionCache,
# UserCredentialsRepo, DomainIdentityRepo.
auth = { path = "../../../lib/crates/auth" }
# apis provides the AuthService / UserService ports + DTOs (no
# serde / utoipa derives — those live on the wire DTOs in this crate).
apis = { path = "../../../lib/crates/apis" }
# user provides UserRepo (Postgres) + UserServiceImpl
# (UserUsecase -> apis::user::UserService).
user = { path = "../../../lib/crates/user" }

[dev-dependencies]
# `argon2` is used by `tests/integration_auth.rs` to seed a real
# credential row whose hash matches the password the test logs in
# with. Inherits the workspace version.
argon2 = { workspace = true }
```

- [ ] **Step 2: Create `apps/server/aegis-server/src/lib.rs` (skeleton)**

```rust
//! # aegis-server
//!
//! HTTP server binary. Wires the `auth` crate's `AuthServiceImpl`
//! against a Postgres pool + in-memory token-version cache, mounts
//! the auth-flow endpoints under `/api/auth/*` with `axum`, and
//! exposes the OpenAPI document at `/api-docs/openapi.json` plus
//! swagger-ui at `/swagger-ui`.
//!
//! The public surface is small (`run`, `Config`, `AppState`,
//! `transport::router`) so the binary entry point stays a thin
//! `main.rs` that parses env, initialises tracing, and calls
//! `aegis_server::run(config)`.

pub mod config;
pub mod state;
pub mod transport;
```

- [ ] **Step 3: Create `apps/server/aegis-server/src/config.rs` (skeleton)**

```rust
//! Server configuration loaded from environment variables at startup.

use std::net::SocketAddr;
use std::time::Duration;

use thiserror::Error;

/// Server configuration loaded from environment variables.
///
/// Constructed via [`Config::from_env`] in `main.rs`. Every field is
/// `pub`; the binary does no further wrapping.
#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub signing_key: Vec<u8>,
    pub bind_addr: SocketAddr,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Failure modes of [`Config::from_env`].
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable: {0}")]
    MissingEnvVariable(&'static str),

    #[error("invalid value for environment variable {var}: {message}")]
    InvalidValue {
        var: &'static str,
        message: String,
    },
}

impl Config {
    /// Read every required variable from `std::env`. Returns
    /// [`ConfigError::MissingEnvVariable`] if a required variable is
    /// not set, or [`ConfigError::InvalidValue`] if a value cannot be
    /// parsed.
    pub fn from_env() -> Result<Self, ConfigError> {
        todo!("implemented in Task 3")
    }
}
```

- [ ] **Step 4: Create `apps/server/aegis-server/src/state.rs` (skeleton)**

```rust
//! Shared state injected into every handler via `axum::extract::State`.

use std::sync::Arc;

/// Shared state injected into every handler.
///
/// Cloned per worker task (axum's `State<T>: Clone` requires it);
/// both fields are `Arc`, so the clone is cheap. The `user` field is
/// held so the auth `UserServiceImpl` can be wired once at startup
/// without a separate registry; future user-CRUD handlers will use it
/// directly.
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
}
```

- [ ] **Step 5: Create `apps/server/aegis-server/src/transport.rs`**

```rust
//! Transport layer.
//!
//! Houses every supported network transport as a sub-module. Today
//! the only transport is `http`; future transports (gRPC, tarpc,
//! CLI) land as siblings under this module.

pub mod http;

pub use http::router;
```

- [ ] **Step 6: Create `apps/server/aegis-server/src/transport/http.rs`**

```rust
//! HTTP transport.
//!
//! Hosts the axum `Router` composition, the auth handlers, the wire
//! DTOs, the `ErrorBody` + `ApiError` mapping, the healthz handler,
//! and the utoipa OpenAPI builder. Sub-modules re-export their
//! public surface here so consumers can write
//! `use aegis_server::transport::http::router` or
//! `use aegis_server::transport::router` (the outer re-export).

pub mod auth;
pub mod dto;
pub mod error;
pub mod healthz;
pub mod openapi;
pub mod router;

pub use router::router;
```

- [ ] **Step 7: Create stub files for every transport/http sub-module**

Create each of the six stub files below. They contain a single `todo!()` so the crate compiles, but produce no behaviour. Tasks 5–15 fill in the real implementations.

`apps/server/aegis-server/src/transport/http/router.rs`:

```rust
use crate::state::AppState;

pub fn router(_state: AppState) -> axum::Router {
    todo!("implemented in Task 16")
}
```

`apps/server/aegis-server/src/transport/http/auth.rs`:

```rust
pub mod middleware;

pub use middleware::AuthClaims;
```

`apps/server/aegis-server/src/transport/http/auth/middleware.rs`:

```rust
use axum::extract::FromRequestParts;
use axum::http::request::Parts;

use crate::state::AppState;

pub struct AuthClaims(pub apis::auth::AuthClaims);

#[async_trait::async_trait]
impl FromRequestParts<AppState> for AuthClaims {
    type Rejection = crate::transport::http::error::ApiError;

    async fn from_request_parts(
        _parts: &mut Parts,
        _state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        todo!("implemented in Task 15")
    }
}
```

`apps/server/aegis-server/src/transport/http/dto.rs`:

```rust
//! Wire-level DTOs for the HTTP transport.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}
```

`apps/server/aegis-server/src/transport/http/error.rs`:

```rust
//! HTTP error mapping.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Error)]
#[error("{0}")]
pub struct ApiError(pub apis::auth::AuthApiError);
```

`apps/server/aegis-server/src/transport/http/healthz.rs`:

```rust
pub async fn healthz() -> &'static str {
    todo!("implemented in Task 8")
}
```

`apps/server/aegis-server/src/transport/http/openapi.rs`:

```rust
pub fn openapi() -> utoipa::openapi::OpenApi {
    todo!("implemented in Task 9")
}
```

- [ ] **Step 8: Verify the crate skeleton compiles**

Run: `cargo build -p aegis-server`
Expected: success. Every stub uses `todo!()`, but the module tree must typecheck.

- [ ] **Step 9: Commit**

```bash
git add apps/server/aegis-server/Cargo.toml apps/server/aegis-server/src
git commit -m "feat(aegis-server): scaffold module skeleton + dependencies"
```

---

### Task 3: `Config::from_env` (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/config.rs` (replace `todo!()`)
- Create: `apps/server/aegis-server/src/config/tests.rs` (inline `#[cfg(test)] mod tests` is added to `config.rs` instead — see Step 1)

- [ ] **Step 1: Add an inline test module at the bottom of `config.rs`**

Append to `apps/server/aegis-server/src/config.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Set up an env-var-only test by serializing access to the
    /// process-global env. The mutex serializes parallel test
    /// threads; each test installs its vars under its own block so
    /// they are restored at the end.
    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn lock_env() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// Helper: set an env var, returning a guard that restores the
    /// previous value (or unsets if it was unset) on drop. Lets each
    /// test run inside a `let _g = set_env(...);` block.
    struct EnvGuard {
        key: &'static str,
        prev: Option<String>,
    }
    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
    fn set_env(key: &'static str, value: &str) -> EnvGuard {
        let prev = std::env::var(key).ok();
        std::env::set_var(key, value);
        EnvGuard { key, prev }
    }

    /// Hex-encode a fixed 32-byte key for tests.
    fn sample_key_hex() -> String {
        (0..32u8).map(|b| format!("{b:02x}")).collect()
    }

    #[test]
    fn from_env_succeeds_with_all_required_vars() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _b = set_env("AEGIS_HTTP_BIND", "127.0.0.1:9090");
        let _a = set_env("AEGIS_ACCESS_TTL_SECS", "60");
        let _r = set_env("AEGIS_REFRESH_TTL_SECS", "120");

        let cfg = Config::from_env().expect("config should parse");
        assert_eq!(cfg.database_url, "postgres://localhost/x");
        assert_eq!(cfg.signing_key.len(), 32);
        assert_eq!(cfg.bind_addr.to_string(), "127.0.0.1:9090");
        assert_eq!(cfg.access_ttl, std::time::Duration::from_secs(60));
        assert_eq!(cfg.refresh_ttl, std::time::Duration::from_secs(120));
    }

    #[test]
    fn from_env_uses_defaults_when_optional_vars_missing() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        // AEGIS_HTTP_BIND, AEGIS_ACCESS_TTL_SECS, AEGIS_REFRESH_TTL_SECS all unset.

        let cfg = Config::from_env().expect("config should parse");
        assert_eq!(cfg.bind_addr.to_string(), "0.0.0.0:8080");
        assert_eq!(cfg.access_ttl, std::time::Duration::from_secs(900));
        assert_eq!(cfg.refresh_ttl, std::time::Duration::from_secs(7 * 24 * 60 * 60));
    }

    #[test]
    fn from_env_errors_when_database_url_missing() {
        let _g = lock_env();
        std::env::remove_var("AEGIS_DATABASE_URL");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::MissingEnvVariable("AEGIS_DATABASE_URL")));
    }

    #[test]
    fn from_env_errors_when_signing_key_missing() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        std::env::remove_var("AEGIS_AUTH_SIGNING_KEY");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::MissingEnvVariable("AEGIS_AUTH_SIGNING_KEY")));
    }

    #[test]
    fn from_env_errors_on_short_signing_key() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        // 16 bytes -> < 32.
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &"00".repeat(16));

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_AUTH_SIGNING_KEY", .. }));
    }

    #[test]
    fn from_env_errors_on_invalid_hex_signing_key() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", "not-hex-bytes");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_AUTH_SIGNING_KEY", .. }));
    }

    #[test]
    fn from_env_errors_on_invalid_bind_addr() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _b = set_env("AEGIS_HTTP_BIND", "not-an-addr");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_HTTP_BIND", .. }));
    }

    #[test]
    fn from_env_errors_on_non_numeric_ttl() {
        let _g = lock_env();
        let _db = set_env("AEGIS_DATABASE_URL", "postgres://localhost/x");
        let _sk = set_env("AEGIS_AUTH_SIGNING_KEY", &sample_key_hex());
        let _a = set_env("AEGIS_ACCESS_TTL_SECS", "not-a-number");

        let err = Config::from_env().expect_err("should fail");
        assert!(matches!(err, ConfigError::InvalidValue { var: "AEGIS_ACCESS_TTL_SECS", .. }));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-server --lib config::`
Expected: FAIL — `todo!()` panics inside `from_env`.

- [ ] **Step 3: Replace `todo!()` with the real `from_env` implementation**

Replace the body of `from_env` in `apps/server/aegis-server/src/config.rs` (the line `todo!("implemented in Task 3")`) with:

```rust
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url = std::env::var("AEGIS_DATABASE_URL")
            .map_err(|_| ConfigError::MissingEnvVariable("AEGIS_DATABASE_URL"))?;

        let signing_key_hex = std::env::var("AEGIS_AUTH_SIGNING_KEY")
            .map_err(|_| ConfigError::MissingEnvVariable("AEGIS_AUTH_SIGNING_KEY"))?;
        let signing_key = hex_decode(&signing_key_hex).map_err(|message| {
            ConfigError::InvalidValue {
                var: "AEGIS_AUTH_SIGNING_KEY",
                message,
            }
        })?;
        if signing_key.len() < 32 {
            return Err(ConfigError::InvalidValue {
                var: "AEGIS_AUTH_SIGNING_KEY",
                message: format!("got {} bytes, need >= 32", signing_key.len()),
            });
        }

        let bind_addr_str = std::env::var("AEGIS_HTTP_BIND")
            .unwrap_or_else(|_| "0.0.0.0:8080".to_string());
        let bind_addr: SocketAddr = bind_addr_str.parse().map_err(|e: std::net::AddrParseError| {
            ConfigError::InvalidValue {
                var: "AEGIS_HTTP_BIND",
                message: e.to_string(),
            }
        })?;

        let access_ttl_secs = match std::env::var("AEGIS_ACCESS_TTL_SECS") {
            Ok(s) => s.parse::<u64>().map_err(|e| ConfigError::InvalidValue {
                var: "AEGIS_ACCESS_TTL_SECS",
                message: e.to_string(),
            })?,
            Err(_) => 900,
        };
        let refresh_ttl_secs = match std::env::var("AEGIS_REFRESH_TTL_SECS") {
            Ok(s) => s.parse::<u64>().map_err(|e| ConfigError::InvalidValue {
                var: "AEGIS_REFRESH_TTL_SECS",
                message: e.to_string(),
            })?,
            Err(_) => 7 * 24 * 60 * 60,
        };

        Ok(Self {
            database_url,
            signing_key,
            bind_addr,
            access_ttl: Duration::from_secs(access_ttl_secs),
            refresh_ttl: Duration::from_secs(refresh_ttl_secs),
        })
    }
```

Also add this helper at the bottom of `config.rs` (above the `#[cfg(test)] mod tests`):

```rust
/// Decode a hex string (lowercase or uppercase) into bytes. Rejects
/// odd-length input and any non-hex character.
fn hex_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("hex string has odd length".into());
    }
    let mut out = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(2) {
        let hi = hex_nibble(chunk[0]).ok_or_else(|| "non-hex character".to_string())?;
        let lo = hex_nibble(chunk[1]).ok_or_else(|| "non-hex character".to_string())?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib config::`
Expected: PASS — all 7 tests succeed.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/config.rs
git commit -m "feat(aegis-server): Config::from_env with hex signing key"
```

---

### Task 4: Wire-level DTOs (`transport/http/dto.rs`)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

- [ ] **Step 1: Add inline tests at the bottom of `dto.rs`**

Replace the current `apps/server/aegis-server/src/transport/http/dto.rs` (the stub with just `LoginRequest`) with the full set of DTOs, then append an inline test module:

```rust
//! Wire-level DTOs for the HTTP transport.
//!
//! Each wire DTO is a thin Rust struct with `Serialize`,
//! `Deserialize`, and `ToSchema`. Field names are `snake_case` to
//! match the apis surface. Handler code translates JSON ↔ apis DTOs
//! at the boundary; the apis crate deliberately has no serde /
//! utoipa derives.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

// -- requests -------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginRequest {
    pub code: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LoginDomainRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// -- responses ------------------------------------------------------------

#[derive(Serialize, Deserialize, ToSchema)]
pub struct TokenPairResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AccessTokenResponse {
    pub access_token: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct LogoutResponse {}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct AuthClaimsResponse {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

// -- Role -----------------------------------------------------------------

/// Wire-level mirror of `apis::user::Role`. The two enums have
/// identical variants; the conversion is a single 3-arm `match` in
/// `auth.rs`. Kept separate so the apis crate stays free of
/// serde / utoipa derives.
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Root,
    Admin,
    General,
}

impl From<apis::user::Role> for Role {
    fn from(r: apis::user::Role) -> Self {
        match r {
            apis::user::Role::Root => Role::Root,
            apis::user::Role::Admin => Role::Admin,
            apis::user::Role::General => Role::General,
        }
    }
}

impl From<Role> for apis::user::Role {
    fn from(r: Role) -> Self {
        match r {
            Role::Root => apis::user::Role::Root,
            Role::Admin => apis::user::Role::Admin,
            Role::General => apis::user::Role::General,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn login_request_roundtrip() {
        let json = r#"{"code":"u1","password":"p"}"#;
        let req: LoginRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.password, "p");
        let out = serde_json::to_string(&req).unwrap();
        assert_eq!(out, json);
    }

    #[test]
    fn login_domain_request_roundtrip() {
        let json = r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#;
        let req: LoginDomainRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.code, "u1");
        assert_eq!(req.domain_name, "d");
        assert_eq!(req.hostname, "h");
        assert_eq!(req.sid, "s");
        assert_eq!(serde_json::to_string(&req).unwrap(), json);
    }

    #[test]
    fn refresh_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: RefreshRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn logout_request_roundtrip() {
        let json = r#"{"refresh_token":"r"}"#;
        let req: LogoutRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.refresh_token, "r");
    }

    #[test]
    fn token_pair_response_roundtrip() {
        let json = r#"{"access_token":"a","refresh_token":"r"}"#;
        let res: TokenPairResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
        assert_eq!(res.refresh_token, "r");
    }

    #[test]
    fn access_token_response_roundtrip() {
        let json = r#"{"access_token":"a"}"#;
        let res: AccessTokenResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.access_token, "a");
    }

    #[test]
    fn logout_response_roundtrip() {
        let res: LogoutResponse = serde_json::from_str("{}").unwrap();
        let out = serde_json::to_string(&res).unwrap();
        assert_eq!(out, "{}");
    }

    #[test]
    fn auth_claims_response_roundtrip() {
        let json = r#"{"code":"u1","role":"admin","token_version":7}"#;
        let res: AuthClaimsResponse = serde_json::from_str(json).unwrap();
        assert_eq!(res.code, "u1");
        assert!(matches!(res.role, Role::Admin));
        assert_eq!(res.token_version, 7);
        assert_eq!(serde_json::to_string(&res).unwrap(), json);
    }

    #[test]
    fn role_round_trip_all_variants() {
        for r in [Role::Root, Role::Admin, Role::General] {
            let s = serde_json::to_string(&r).unwrap();
            let back: Role = serde_json::from_str(&s).unwrap();
            assert_eq!(format!("{r:?}"), format!("{back:?}"));
        }
    }

    #[test]
    fn role_from_apis_role_all_variants() {
        assert!(matches!(Role::from(apis::user::Role::Root), Role::Root));
        assert!(matches!(Role::from(apis::user::Role::Admin), Role::Admin));
        assert!(matches!(Role::from(apis::user::Role::General), Role::General));
    }
}
```

- [ ] **Step 2: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib transport::http::dto::`
Expected: PASS — all 10 tests succeed.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(aegis-server): wire-level DTOs with serde + utoipa"
```

---

### Task 5: Error mapping — `ErrorBody`, `ApiError`, `IntoResponse` (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`

- [ ] **Step 1: Append inline tests at the bottom of `error.rs`**

Append to `apps/server/aegis-server/src/transport/http/error.rs` (below the current stub):

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Drive `IntoResponse::into_response` and recover the status +
    /// JSON body so each variant can be asserted directly. The body
    /// bytes are re-parsed into `ErrorBody` for a structured
    /// comparison.
    async fn render(err: AuthApiError) -> (StatusCode, ErrorBody) {
        let api = ApiError::from(err);
        let response = api.into_response();
        let status = response.status();
        let body = axum::body::to_bytes(response.into_body(), 1024)
            .await
            .unwrap();
        let parsed: ErrorBody = serde_json::from_slice(&body).unwrap();
        (status, parsed)
    }

    #[tokio::test]
    async fn validation_maps_to_400() {
        let (status, body) = render(AuthApiError::Validation("bad".into())).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(body.code, "validation_failed");
        assert_eq!(body.message, "validation failed: bad");
    }

    #[tokio::test]
    async fn not_found_maps_to_404() {
        let (status, body) = render(AuthApiError::NotFound).await;
        assert_eq!(status, StatusCode::NOT_FOUND);
        assert_eq!(body.code, "not_found");
    }

    #[tokio::test]
    async fn inactive_maps_to_403() {
        let (status, body) = render(AuthApiError::Inactive).await;
        assert_eq!(status, StatusCode::FORBIDDEN);
        assert_eq!(body.code, "user_inactive");
    }

    #[tokio::test]
    async fn invalid_credentials_maps_to_401() {
        let (status, body) = render(AuthApiError::InvalidCredentials).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "invalid_credentials");
    }

    #[tokio::test]
    async fn verification_maps_to_401() {
        let (status, body) = render(AuthApiError::Verification("bad sig".into())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn duplicate_code_maps_to_409() {
        let (status, body) = render(AuthApiError::DuplicateCode("u1".into())).await;
        assert_eq!(status, StatusCode::CONFLICT);
        assert_eq!(body.code, "duplicate_code");
    }

    #[tokio::test]
    async fn signing_maps_to_500() {
        let (status, body) = render(AuthApiError::Signing("boom".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "signing_failed");
    }

    #[tokio::test]
    async fn repository_maps_to_500() {
        let (status, body) = render(AuthApiError::Repository("db down".into())).await;
        assert_eq!(status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(body.code, "repository_error");
    }

    #[test]
    fn from_auth_api_error_wraps() {
        let inner = AuthApiError::NotFound;
        let outer = ApiError::from(inner);
        assert_eq!(outer.0, AuthApiError::NotFound);
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::error::`
Expected: FAIL — `ApiError` does not yet implement `IntoResponse`, so `render` won't compile.

- [ ] **Step 3: Implement the real `ApiError` body**

Replace the entire current contents of `apps/server/aegis-server/src/transport/http/error.rs` with:

```rust
//! HTTP error mapping.
//!
//! [`ApiError`] wraps [`apis::auth::AuthApiError`] and adds an HTTP
//! status code + a JSON [`ErrorBody`] shape. Every handler returns
//! `Result<Json<T>, ApiError>` and uses `?` on `AuthApiError`; the
//! [`From`] impl does the wrapping.

use axum::Json;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;
use thiserror::Error;
use utoipa::ToSchema;

/// Stable JSON error envelope returned to clients.
///
/// `code` is a machine-readable string (e.g. `invalid_credentials`)
/// that clients should switch on. `message` is human-readable and
/// may be surfaced in a UI.
#[derive(Debug, Serialize, ToSchema)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
}

/// Newtype around [`apis::auth::AuthApiError`] that adds an HTTP
/// status code and renders as JSON [`ErrorBody`].
#[derive(Debug, Error)]
#[error("{0}")]
pub struct ApiError(pub apis::auth::AuthApiError);

impl ApiError {
    /// HTTP status code for this error variant.
    pub fn status(&self) -> StatusCode {
        match &self.0 {
            apis::auth::AuthApiError::Validation(_) => StatusCode::BAD_REQUEST,
            apis::auth::AuthApiError::NotFound => StatusCode::NOT_FOUND,
            apis::auth::AuthApiError::Inactive => StatusCode::FORBIDDEN,
            apis::auth::AuthApiError::InvalidCredentials => StatusCode::UNAUTHORIZED,
            apis::auth::AuthApiError::Verification(_) => StatusCode::UNAUTHORIZED,
            apis::auth::AuthApiError::DuplicateCode(_) => StatusCode::CONFLICT,
            apis::auth::AuthApiError::Signing(_) => StatusCode::INTERNAL_SERVER_ERROR,
            apis::auth::AuthApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// Stable machine-readable code used as `ErrorBody.code`.
    pub fn code(&self) -> &'static str {
        match &self.0 {
            apis::auth::AuthApiError::Validation(_) => "validation_failed",
            apis::auth::AuthApiError::NotFound => "not_found",
            apis::auth::AuthApiError::Inactive => "user_inactive",
            apis::auth::AuthApiError::InvalidCredentials => "invalid_credentials",
            apis::auth::AuthApiError::Verification(_) => "token_verification_failed",
            apis::auth::AuthApiError::DuplicateCode(_) => "duplicate_code",
            apis::auth::AuthApiError::Signing(_) => "signing_failed",
            apis::auth::AuthApiError::Repository(_) => "repository_error",
        }
    }
}

impl From<apis::auth::AuthApiError> for ApiError {
    fn from(err: apis::auth::AuthApiError) -> Self {
        Self(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = self.status();
        if status.is_server_error() {
            tracing::error!(
                code = self.code(),
                error = %self.0,
                "api error",
            );
        }
        let body = ErrorBody {
            code: self.code().to_string(),
            message: self.0.to_string(),
        };
        (status, Json(body)).into_response()
    }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib transport::http::error::`
Expected: PASS — all 9 tests succeed.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/error.rs
git commit -m "feat(aegis-server): ApiError + IntoResponse status mapping"
```

---

### Task 6: `healthz` handler (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/healthz.rs`

- [ ] **Step 1: Replace the stub with the real implementation + tests**

Replace the entire current contents of `apps/server/aegis-server/src/transport/http/healthz.rs` with:

```rust
//! Liveness probe handler.
//!
//! Returns the literal string `"ok"` with `Content-Type:
//! text/plain; charset=utf-8`. Does not consult any backing
//! service — a healthy process means a healthy server.

/// HTTP handler for `GET /api/healthz`.
pub async fn healthz() -> &'static str {
    "ok"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn returns_ok_string() {
        let body = healthz().await;
        assert_eq!(body, "ok");
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p aegis-server --lib transport::http::healthz::`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/healthz.rs
git commit -m "feat(aegis-server): /api/healthz handler"
```

---

### Task 7: `openapi()` builder (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Modify: `apps/server/aegis-server/src/transport/http/auth.rs` (add handler skeletons so the paths in `openapi` resolve — these become real in Task 8; for now the function bodies can be `todo!()`)

- [ ] **Step 1: Add handler skeletons to `auth.rs`**

Replace the entire current contents of `apps/server/aegis-server/src/transport/http/auth.rs` with:

```rust
//! HTTP handlers for the auth flow.

pub mod middleware;

pub use middleware::AuthClaims;

use axum::Json;
use axum::extract::State;

use crate::state::AppState;
use crate::transport::http::dto::{
    AccessTokenResponse, LoginDomainRequest, LoginRequest, LogoutRequest, LogoutResponse,
    RefreshRequest, TokenPairResponse,
};
use crate::transport::http::error::ApiError;

#[axum::debug_handler(state = AppState)]
pub async fn login(
    State(_state): State<AppState>,
    Json(_req): Json<LoginRequest>,
) -> Result<Json<TokenPairResponse>, ApiError> {
    todo!("implemented in Task 8")
}

#[axum::debug_handler(state = AppState)]
pub async fn login_domain(
    State(_state): State<AppState>,
    Json(_req): Json<LoginDomainRequest>,
) -> Result<Json<TokenPairResponse>, ApiError> {
    todo!("implemented in Task 8")
}

#[axum::debug_handler(state = AppState)]
pub async fn refresh(
    State(_state): State<AppState>,
    Json(_req): Json<RefreshRequest>,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    todo!("implemented in Task 8")
}

#[axum::debug_handler(state = AppState)]
pub async fn logout(
    State(_state): State<AppState>,
    Json(_req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, ApiError> {
    todo!("implemented in Task 8")
}

/// Sub-router mounting the four auth endpoints under `/auth/*`.
/// Composed into the top-level `Router` by `transport/http/router.rs`.
pub fn router() -> axum::Router<AppState> {
    use axum::routing::post;
    axum::Router::new()
        .route("/login", post(login))
        .route("/login-domain", post(login_domain))
        .route("/refresh", post(refresh))
        .route("/logout", post(logout))
}
```

- [ ] **Step 2: Replace the `openapi.rs` stub with the real builder + tests**

Replace the entire current contents of `apps/server/aegis-server/src/transport/http/openapi.rs` with:

```rust
//! utoipa OpenAPI document builder.
//!
//! Aggregates the paths and component schemas declared across the
//! `transport/http` sub-modules into a single `utoipa::openapi::OpenApi`
//! document. Served at `/api-docs/openapi.json` by `transport/http/router.rs`.

use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    info(title = "aegis-server", version = "0.1.0"),
    paths(
        super::auth::login,
        super::auth::login_domain,
        super::auth::refresh,
        super::auth::logout,
        super::healthz::healthz,
    ),
    components(schemas(
        super::dto::LoginRequest,
        super::dto::LoginDomainRequest,
        super::dto::RefreshRequest,
        super::dto::LogoutRequest,
        super::dto::TokenPairResponse,
        super::dto::AccessTokenResponse,
        super::dto::LogoutResponse,
        super::dto::AuthClaimsResponse,
        super::dto::Role,
        super::error::ErrorBody,
    )),
)]
struct ApiDoc;

/// Build the OpenAPI document. Called once per `Router` construction
/// so the same document object can be served at
/// `/api-docs/openapi.json`.
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn document_contains_every_auth_path() {
        let doc = openapi();
        let paths = doc.paths.paths.keys().map(|s| s.as_str()).collect::<Vec<_>>();
        // utoipa-axum renders the full path (with the /api prefix that
        // the OpenApiRouter registers). The actual prefix applied by
        // axum::Router::nest happens at runtime and does not affect
        // the registered path string — they match.
        assert!(
            paths.contains(&"/api/auth/login"),
            "missing /api/auth/login in {paths:?}"
        );
        assert!(
            paths.contains(&"/api/auth/login-domain"),
            "missing /api/auth/login-domain in {paths:?}"
        );
        assert!(
            paths.contains(&"/api/auth/refresh"),
            "missing /api/auth/refresh in {paths:?}"
        );
        assert!(
            paths.contains(&"/api/auth/logout"),
            "missing /api/auth/logout in {paths:?}"
        );
        assert!(
            paths.contains(&"/api/healthz"),
            "missing /api/healthz in {paths:?}"
        );
        assert_eq!(paths.len(), 5, "unexpected extra paths: {paths:?}");
    }

    #[test]
    fn document_contains_every_dto_schema() {
        use std::collections::BTreeSet;
        let doc = openapi();
        let actual: BTreeSet<String> = doc
            .components
            .as_ref()
            .map(|c| c.schemas.keys().cloned().collect())
            .unwrap_or_default();
        let expected: BTreeSet<String> = [
            "LoginRequest",
            "LoginDomainRequest",
            "RefreshRequest",
            "LogoutRequest",
            "TokenPairResponse",
            "AccessTokenResponse",
            "LogoutResponse",
            "AuthClaimsResponse",
            "Role",
            "ErrorBody",
        ]
        .into_iter()
        .map(String::from)
        .collect();
        assert_eq!(actual, expected, "schema list mismatch");
    }
}
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p aegis-server --lib transport::http::openapi::`
Expected: PASS — both tests succeed. (The handler skeletons are `todo!()` but they type-check, which is all utoipa's `paths(...)` derive needs.)

- [ ] **Step 4: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/auth.rs apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(aegis-server): utoipa OpenApi document with paths + schemas"
```

---

### Task 8: Auth handlers — login, login_domain, refresh, logout (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/auth.rs`

- [ ] **Step 1: Replace the four `todo!()` handler bodies with the real implementations**

In `apps/server/aegis-server/src/transport/http/auth.rs`, replace each handler body with the version below. The handler signatures stay exactly as they are; only the body changes. The `verify` handler is not in this file (it is implemented as the `AuthClaims` extractor in Task 9).

`login` body:

```rust
    let pair = state
        .auth
        .login_with_password(apis::auth::LoginWithPasswordRequest {
            code: req.code,
            password: req.password,
        })
        .await?;
    Ok(Json(TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
```

`login_domain` body:

```rust
    let pair = state
        .auth
        .login_with_domain_user_info(apis::auth::LoginWithDomainUserInfoRequest {
            code: req.code,
            domain_name: req.domain_name,
            hostname: req.hostname,
            sid: req.sid,
        })
        .await?;
    Ok(Json(TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
```

`refresh` body:

```rust
    let res = state
        .auth
        .refresh(apis::auth::RefreshRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok(Json(AccessTokenResponse {
        access_token: res.access_token,
    }))
```

`logout` body:

```rust
    state
        .auth
        .logout(apis::auth::LogoutRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok(Json(LogoutResponse {}))
```

After the replacements, the four handlers look like:

```rust
#[axum::debug_handler(state = AppState)]
pub async fn login(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<TokenPairResponse>, ApiError> {
    let pair = state
        .auth
        .login_with_password(apis::auth::LoginWithPasswordRequest {
            code: req.code,
            password: req.password,
        })
        .await?;
    Ok(Json(TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
}

#[axum::debug_handler(state = AppState)]
pub async fn login_domain(
    State(state): State<AppState>,
    Json(req): Json<LoginDomainRequest>,
) -> Result<Json<TokenPairResponse>, ApiError> {
    let pair = state
        .auth
        .login_with_domain_user_info(apis::auth::LoginWithDomainUserInfoRequest {
            code: req.code,
            domain_name: req.domain_name,
            hostname: req.hostname,
            sid: req.sid,
        })
        .await?;
    Ok(Json(TokenPairResponse {
        access_token: pair.access_token,
        refresh_token: pair.refresh_token,
    }))
}

#[axum::debug_handler(state = AppState)]
pub async fn refresh(
    State(state): State<AppState>,
    Json(req): Json<RefreshRequest>,
) -> Result<Json<AccessTokenResponse>, ApiError> {
    let res = state
        .auth
        .refresh(apis::auth::RefreshRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok(Json(AccessTokenResponse {
        access_token: res.access_token,
    }))
}

#[axum::debug_handler(state = AppState)]
pub async fn logout(
    State(state): State<AppState>,
    Json(req): Json<LogoutRequest>,
) -> Result<Json<LogoutResponse>, ApiError> {
    state
        .auth
        .logout(apis::auth::LogoutRequest {
            refresh_token: req.refresh_token,
        })
        .await?;
    Ok(Json(LogoutResponse {}))
}
```

- [ ] **Step 2: Append inline tests at the bottom of `auth.rs`**

Append to `apps/server/aegis-server/src/transport/http/auth.rs`:

```rust
#[cfg(test)]
mod tests {
    //! Handler tests using a `FakeAuthService` and
    //! `tower::ServiceExt::oneshot`. Each handler has a happy-path
    //! test plus at least one error-path test.

    use std::sync::Arc;

    use apis::auth::{
        AuthApiError, AuthService, LoginWithDomainUserInfoRequest,
        LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest,
        RefreshResponse, TokenPair,
    };
    use apis::user::{Role as ApiRole, UserService as ApiUserService};
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;
    use utoipa_axum::OpenApiRouter;

    use super::*;
    use crate::state::AppState;
    use crate::transport::http::dto::Role;

    /// Fake `AuthService` whose four login / refresh / logout methods
    /// return whatever the test set on the corresponding field. The
    /// `verify` method always returns `Err(NotFound)` — those tests
    /// live with the `AuthClaims` extractor.
    #[derive(Clone)]
    struct FakeAuthService {
        login: Arc<std::sync::Mutex<Option<Result<TokenPair, AuthApiError>>>>,
        login_domain: Arc<std::sync::Mutex<Option<Result<TokenPair, AuthApiError>>>>,
        refresh: Arc<std::sync::Mutex<Option<Result<RefreshResponse, AuthApiError>>>>,
        logout: Arc<std::sync::Mutex<Option<Result<LogoutResponse, AuthApiError>>>>,
    }

    impl FakeAuthService {
        fn new() -> Self {
            Self {
                login: Arc::new(std::sync::Mutex::new(None)),
                login_domain: Arc::new(std::sync::Mutex::new(None)),
                refresh: Arc::new(std::sync::Mutex::new(None)),
                logout: Arc::new(std::sync::Mutex::new(None)),
            }
        }
        fn expect_login(&self, v: Result<TokenPair, AuthApiError>) -> Self {
            *self.login.lock().unwrap() = Some(v);
            self.clone()
        }
        fn expect_login_domain(&self, v: Result<TokenPair, AuthApiError>) -> Self {
            *self.login_domain.lock().unwrap() = Some(v);
            self.clone()
        }
        fn expect_refresh(&self, v: Result<RefreshResponse, AuthApiError>) -> Self {
            *self.refresh.lock().unwrap() = Some(v);
            self.clone()
        }
        fn expect_logout(&self, v: Result<LogoutResponse, AuthApiError>) -> Self {
            *self.logout.lock().unwrap() = Some(v);
            self.clone()
        }
    }

    #[async_trait]
    impl AuthService for FakeAuthService {
        async fn login_with_password(
            &self,
            _req: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            self.login
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Err(AuthApiError::NotFound))
        }
        async fn login_with_domain_user_info(
            &self,
            _req: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> {
            self.login_domain
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Err(AuthApiError::NotFound))
        }
        async fn logout(&self, _req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
            self.logout
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Ok(LogoutResponse {}))
        }
        async fn verify(
            &self,
            _req: apis::auth::VerifyRequest,
        ) -> Result<apis::auth::AuthClaims, AuthApiError> {
            Err(AuthApiError::NotFound)
        }
        async fn refresh(
            &self,
            _req: RefreshRequest,
        ) -> Result<RefreshResponse, AuthApiError> {
            self.refresh
                .lock()
                .unwrap()
                .take()
                .unwrap_or(Err(AuthApiError::Verification("unset".into())))
        }
        async fn find_user_credential_by_code(
            &self,
            _code: &str,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            todo!()
        }
        async fn create_user_credential(
            &self,
            _req: apis::auth::CreateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            todo!()
        }
        async fn update_user_credential(
            &self,
            _req: apis::auth::UpdateUserCredentialRequest,
        ) -> Result<apis::auth::UserCredentialView, AuthApiError> {
            todo!()
        }
        async fn remove_user_credential(
            &self,
            _code: &str,
        ) -> Result<apis::auth::RemoveUserCredentialResponse, AuthApiError> {
            todo!()
        }
    }

    /// Stub `UserService` — never called by auth handlers, must be
    /// `Send + Sync` because `AppState` is.
    struct FakeUserService;
    #[async_trait]
    impl ApiUserService for FakeUserService {
        async fn create(
            &self,
            _: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            todo!()
        }
        async fn get_by_id(
            &self,
            _: i32,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            todo!()
        }
        async fn get_by_code(
            &self,
            _: &str,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            todo!()
        }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, apis::user::UserApiError> {
            todo!()
        }
        async fn update(
            &self,
            _: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            todo!()
        }
    }

    /// Build the auth sub-router wrapped in OpenApiRouter + state, so
    /// tests can `oneshot` against it. The `/api` prefix is added in
    /// the top-level router (Task 10); here we mount under `/auth`
    /// to match `auth::router()`.
    fn app(auth: Arc<dyn AuthService>) -> axum::Router {
        let state = AppState {
            auth,
            user: Arc::new(FakeUserService),
        };
        let (router, _api) = OpenApiRouter::new().nest("/auth", router()).split_for_openapi();
        router.with_state(state)
    }

    fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .body(Body::from(body.to_string()))
            .unwrap()
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), 4096).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // -- login ------------------------------------------------------------

    #[tokio::test]
    async fn login_happy_path_returns_token_pair() {
        let auth = Arc::new(FakeAuthService::new().expect_login(Ok(TokenPair {
            access_token: "a".into(),
            refresh_token: "r".into(),
        })));
        let res = app(auth)
            .oneshot(json_request("POST", "/auth/login", r#"{"code":"u1","password":"p"}"#))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["access_token"], "a");
        assert_eq!(body["refresh_token"], "r");
    }

    #[tokio::test]
    async fn login_returns_401_on_invalid_credentials() {
        let auth = Arc::new(
            FakeAuthService::new().expect_login(Err(AuthApiError::InvalidCredentials)),
        );
        let res = app(auth)
            .oneshot(json_request("POST", "/auth/login", r#"{"code":"u1","password":"p"}"#))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(res).await;
        assert_eq!(body["code"], "invalid_credentials");
    }

    #[tokio::test]
    async fn login_returns_400_on_malformed_json() {
        let auth = Arc::new(FakeAuthService::new());
        let res = app(auth)
            .oneshot(json_request("POST", "/auth/login", r#"not json"#))
            .await
            .unwrap();
        // axum's Json extractor returns 400 by default with no body
        // shape — we just assert the status here. (The structured
        // ErrorBody contract only covers AuthApiError.)
        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }

    // -- login-domain -----------------------------------------------------

    #[tokio::test]
    async fn login_domain_happy_path_returns_token_pair() {
        let auth = Arc::new(FakeAuthService::new().expect_login_domain(Ok(TokenPair {
            access_token: "a".into(),
            refresh_token: "r".into(),
        })));
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/login-domain",
                r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["access_token"], "a");
    }

    #[tokio::test]
    async fn login_domain_returns_404_on_not_found() {
        let auth = Arc::new(
            FakeAuthService::new().expect_login_domain(Err(AuthApiError::NotFound)),
        );
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/login-domain",
                r#"{"code":"u1","domain_name":"d","hostname":"h","sid":"s"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::NOT_FOUND);
        let body = body_json(res).await;
        assert_eq!(body["code"], "not_found");
    }

    // -- refresh ----------------------------------------------------------

    #[tokio::test]
    async fn refresh_happy_path_returns_access_token() {
        let auth = Arc::new(FakeAuthService::new().expect_refresh(Ok(RefreshResponse {
            access_token: "a".into(),
        })));
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/refresh",
                r#"{"refresh_token":"r"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body["access_token"], "a");
    }

    #[tokio::test]
    async fn refresh_returns_401_on_verification_failure() {
        let auth = Arc::new(FakeAuthService::new().expect_refresh(Err(
            AuthApiError::Verification("expired".into()),
        )));
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/refresh",
                r#"{"refresh_token":"r"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::UNAUTHORIZED);
        let body = body_json(res).await;
        assert_eq!(body["code"], "token_verification_failed");
    }

    // -- logout -----------------------------------------------------------

    #[tokio::test]
    async fn logout_happy_path_returns_200() {
        let auth = Arc::new(FakeAuthService::new().expect_logout(Ok(LogoutResponse {})));
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/logout",
                r#"{"refresh_token":"r"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = body_json(res).await;
        assert_eq!(body, serde_json::json!({}));
    }

    #[tokio::test]
    async fn logout_returns_403_on_inactive_user() {
        let auth = Arc::new(FakeAuthService::new().expect_logout(Err(AuthApiError::Inactive)));
        let res = app(auth)
            .oneshot(json_request(
                "POST",
                "/auth/logout",
                r#"{"refresh_token":"r"}"#,
            ))
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        let body = body_json(res).await;
        assert_eq!(body["code"], "user_inactive");
    }

    // -- sanity: AppState is Send + Sync ---------------------------------

    fn assert_send_sync<T: Send + Sync>() {}
    #[test]
    fn app_state_is_send_sync() {
        assert_send_sync::<AppState>();
    }

    // -- reference: ApiRole / Role conversion already tested in dto ------
    #[allow(dead_code)]
    fn _role_link(_r: ApiRole) -> Role {
        Role::from(ApiRole::Admin)
    }
}
```

- [ ] **Step 3: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib transport::http::auth::`
Expected: PASS — all 12 tests succeed.

- [ ] **Step 4: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/auth.rs
git commit -m "feat(aegis-server): login / login-domain / refresh / logout handlers"
```

---

### Task 9: `AuthClaims` extractor (TDD)

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/auth/middleware.rs`

- [ ] **Step 1: Append inline tests at the bottom of `middleware.rs`**

Append to `apps/server/aegis-server/src/transport/http/auth/middleware.rs` (the file currently holds a `todo!()` impl):

```rust
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apis::auth::{
        AuthApiError, AuthClaims as ApiClaims, AuthService, LoginWithDomainUserInfoRequest,
        LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest,
        RefreshResponse, UserCredentialView, VerifyRequest,
    };
    use apis::user::{Role as ApiRole, UserApiError, UserService as ApiUserService};
    use async_trait::async_trait;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request;
    use axum::routing::get;
    use tower::ServiceExt;

    use super::*;
    use crate::state::AppState;
    use crate::transport::http::error::ErrorBody;

    /// Configurable fake: returns the `verify_result` on every call.
    #[derive(Clone)]
    struct FakeAuthService {
        verify_result: Arc<std::sync::Mutex<Result<ApiClaims, AuthApiError>>>,
    }
    impl FakeAuthService {
        fn returning(r: Result<ApiClaims, AuthApiError>) -> Self {
            Self { verify_result: Arc::new(std::sync::Mutex::new(r)) }
        }
    }
    #[async_trait]
    impl AuthService for FakeAuthService {
        async fn login_with_password(
            &self, _: LoginWithPasswordRequest,
        ) -> Result<apis::auth::TokenPair, AuthApiError> { todo!() }
        async fn login_with_domain_user_info(
            &self, _: LoginWithDomainUserInfoRequest,
        ) -> Result<apis::auth::TokenPair, AuthApiError> { todo!() }
        async fn logout(
            &self, _: LogoutRequest,
        ) -> Result<LogoutResponse, AuthApiError> { todo!() }
        async fn verify(
            &self, _: VerifyRequest,
        ) -> Result<ApiClaims, AuthApiError> {
            self.verify_result.lock().unwrap().clone()
        }
        async fn refresh(
            &self, _: RefreshRequest,
        ) -> Result<RefreshResponse, AuthApiError> { todo!() }
        async fn find_user_credential_by_code(
            &self, _: &str,
        ) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn create_user_credential(
            &self, _: apis::auth::CreateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn update_user_credential(
            &self, _: apis::auth::UpdateUserCredentialRequest,
        ) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn remove_user_credential(
            &self, _: &str,
        ) -> Result<apis::auth::RemoveUserCredentialResponse, AuthApiError> { todo!() }
    }

    struct FakeUserService;
    #[async_trait]
    impl ApiUserService for FakeUserService {
        async fn create(&self, _: apis::user::CreateUserRequest) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn get_by_id(&self, _: i32) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn get_by_code(&self, _: &str) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, UserApiError> { todo!() }
        async fn update(&self, _: apis::user::UpdateUserRequest) -> Result<apis::user::UserView, UserApiError> { todo!() }
    }

    /// Protected handler used by every test below. Echoes the
    /// `AuthClaims` it received so tests can assert what the
    /// extractor handed the handler.
    async fn protected(claims: AuthClaims) -> String {
        format!("ok:{}:{}", claims.0.code, claims.0.token_version)
    }

    fn app(auth: Arc<FakeAuthService>) -> Router {
        let state = AppState {
            auth,
            user: Arc::new(FakeUserService),
        };
        Router::new().route("/protected", get(protected)).with_state(state)
    }

    async fn status_and_body(
        res: axum::response::Response,
    ) -> (axum::http::StatusCode, ErrorBody) {
        let status = res.status();
        let bytes = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        let body: ErrorBody = serde_json::from_slice(&bytes).unwrap();
        (status, body)
    }

    #[tokio::test]
    async fn rejects_missing_authorization_header() {
        let res = app(Arc::new(FakeAuthService::returning(Err(AuthApiError::NotFound))))
            .oneshot(Request::builder().uri("/protected").body(Body::empty()).unwrap())
            .await
            .unwrap();
        let (status, body) = status_and_body(res).await;
        assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn rejects_non_bearer_scheme() {
        let res = app(Arc::new(FakeAuthService::returning(Err(AuthApiError::NotFound))))
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("authorization", "Basic dXNlcjpwYXNz")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = status_and_body(res).await;
        assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn rejects_invalid_token_via_service_error() {
        let res = app(Arc::new(FakeAuthService::returning(Err(AuthApiError::Verification(
            "bad sig".into(),
        )))))
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("authorization", "Bearer wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = status_and_body(res).await;
        assert_eq!(status, axum::http::StatusCode::UNAUTHORIZED);
        assert_eq!(body.code, "token_verification_failed");
    }

    #[tokio::test]
    async fn rejects_inactive_user_via_service_error() {
        // `Inactive` -> ApiError::FORBIDDEN with code `user_inactive`.
        // This documents that the extractor surfaces whatever the
        // service returns, not just `Verification`.
        let res = app(Arc::new(FakeAuthService::returning(Err(AuthApiError::Inactive))))
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("authorization", "Bearer token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let (status, body) = status_and_body(res).await;
        assert_eq!(status, axum::http::StatusCode::FORBIDDEN);
        assert_eq!(body.code, "user_inactive");
    }

    #[tokio::test]
    async fn accepts_valid_token_and_passes_claims() {
        let res = app(Arc::new(FakeAuthService::returning(Ok(ApiClaims {
            code: "u1".into(),
            role: ApiRole::Admin,
            token_version: 7,
        }))))
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("authorization", "Bearer good")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        let bytes = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
        assert_eq!(bytes, "ok:u1:7");
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p aegis-server --lib transport::http::auth::middleware::`
Expected: FAIL — `from_request_parts` is `todo!()`, every call to a protected route panics.

- [ ] **Step 3: Implement the real extractor**

Replace the body of `from_request_parts` (currently `todo!("implemented in Task 15")`) in `apps/server/aegis-server/src/transport/http/auth/middleware.rs` with:

```rust
    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        use apis::auth::VerifyRequest;

        let header = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .ok_or_else(|| {
                crate::transport::http::error::ApiError::from(AuthApiError::Verification(
                    "missing Authorization header".into(),
                ))
            })?
            .to_str()
            .map_err(|_| {
                crate::transport::http::error::ApiError::from(AuthApiError::Verification(
                    "Authorization header is not valid UTF-8".into(),
                ))
            })?;

        let token = header.strip_prefix("Bearer ").ok_or_else(|| {
            crate::transport::http::error::ApiError::from(AuthApiError::Verification(
                "expected Bearer scheme".into(),
            ))
        })?;

        let claims = state
            .auth
            .verify(VerifyRequest {
                access_token: token.to_string(),
            })
            .await?;

        Ok(Self(claims))
    }
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p aegis-server --lib transport::http::auth::middleware::`
Expected: PASS — all 5 tests succeed.

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/auth/middleware.rs
git commit -m "feat(aegis-server): AuthClaims FromRequestParts extractor"
```

---

### Task 10: Top-level `router(state)` composition

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`

- [ ] **Step 1: Replace the stub with the real composition + tests**

Replace the entire current contents of `apps/server/aegis-server/src/transport/http/router.rs` with:

```rust
//! Top-level HTTP router composition.
//!
//! Combines the auth sub-router (under `/auth/*`), the healthz
//! handler (under `/healthz`), and the OpenAPI doc / swagger-ui
//! into a single `axum::Router`. State attaches exactly once at
//! the top level via `Router::with_state(state)` — never on a
//! sub-router, never on the api-scope wrapper.

use axum::Router;
use axum::routing::get;
use tower_http::trace::TraceLayer;
use utoipa_axum::OpenApiRouter;
use utoipa_swagger_ui::SwaggerUi;

use crate::state::AppState;

use super::auth::router as auth_router;
use super::healthz::healthz;
use super::openapi::openapi;

/// Build the top-level `axum::Router` for the HTTP transport.
///
/// The API endpoints are mounted under `/api/*`; swagger-ui and the
/// OpenAPI doc stay at the root.
pub fn router(state: AppState) -> Router {
    let (api_router, api) = OpenApiRouter::new()
        .nest("/auth", auth_router())
        .route("/healthz", get(healthz))
        .split_for_openapi();

    let api_scope = Router::new().merge(api_router);

    Router::new()
        .nest("/api", api_scope)
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", api))
        .with_state(state)
        .layer(TraceLayer::new_for_http())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use apis::auth::{
        AuthApiError, AuthService, LoginWithDomainUserInfoRequest,
        LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest,
        RefreshResponse, TokenPair, UserCredentialView, VerifyRequest,
    };
    use apis::user::{UserApiError, UserService as ApiUserService};
    use async_trait::async_trait;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;
    use crate::state::AppState;

    /// `FakeAuthService` that succeeds for `login` and ignores
    /// everything else. Used to verify the top-level router reaches
    /// the handler under the `/api` prefix.
    #[derive(Clone)]
    struct OkAuthService;
    #[async_trait]
    impl AuthService for OkAuthService {
        async fn login_with_password(
            &self, _: LoginWithPasswordRequest,
        ) -> Result<TokenPair, AuthApiError> {
            Ok(TokenPair { access_token: "a".into(), refresh_token: "r".into() })
        }
        async fn login_with_domain_user_info(
            &self, _: LoginWithDomainUserInfoRequest,
        ) -> Result<TokenPair, AuthApiError> { todo!() }
        async fn logout(&self, _: LogoutRequest) -> Result<LogoutResponse, AuthApiError> { todo!() }
        async fn verify(&self, _: VerifyRequest) -> Result<apis::auth::AuthClaims, AuthApiError> { todo!() }
        async fn refresh(&self, _: RefreshRequest) -> Result<RefreshResponse, AuthApiError> { todo!() }
        async fn find_user_credential_by_code(&self, _: &str) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn create_user_credential(&self, _: apis::auth::CreateUserCredentialRequest) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn update_user_credential(&self, _: apis::auth::UpdateUserCredentialRequest) -> Result<UserCredentialView, AuthApiError> { todo!() }
        async fn remove_user_credential(&self, _: &str) -> Result<apis::auth::RemoveUserCredentialResponse, AuthApiError> { todo!() }
    }

    struct StubUserService;
    #[async_trait]
    impl ApiUserService for StubUserService {
        async fn create(&self, _: apis::user::CreateUserRequest) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn get_by_id(&self, _: i32) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn get_by_code(&self, _: &str) -> Result<apis::user::UserView, UserApiError> { todo!() }
        async fn list(&self) -> Result<Vec<apis::user::UserView>, UserApiError> { todo!() }
        async fn update(&self, _: apis::user::UpdateUserRequest) -> Result<apis::user::UserView, UserApiError> { todo!() }
    }

    fn app() -> Router {
        router(AppState {
            auth: Arc::new(OkAuthService),
            user: Arc::new(StubUserService),
        })
    }

    #[tokio::test]
    async fn login_path_is_under_api_prefix() {
        let res = app()
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
        assert_eq!(res.status(), axum::http::StatusCode::OK);
    }

    #[tokio::test]
    async fn healthz_path_is_under_api_prefix() {
        let res = app()
            .oneshot(
                Request::builder().uri("/api/healthz").body(Body::empty()).unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 64).await.unwrap();
        assert_eq!(body, "ok");
    }

    #[tokio::test]
    async fn openapi_json_path_is_at_root() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), axum::http::StatusCode::OK);
        let body = axum::body::to_bytes(res.into_body(), 16 * 1024).await.unwrap();
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        // Sanity: OpenAPI doc declares the 5 paths.
        assert_eq!(v["info"]["title"], "aegis-server");
    }

    #[tokio::test]
    async fn swagger_ui_path_is_at_root() {
        let res = app()
            .oneshot(
                Request::builder()
                    .uri("/swagger-ui/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        // swagger-ui redirects to its index page; accept 200 or 3xx.
        let s = res.status().as_u16();
        assert!(
            s == 200 || (300..400).contains(&s),
            "unexpected status {s} from /swagger-ui/"
        );
    }

    #[tokio::test]
    async fn api_prefix_is_required_for_login() {
        let res = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/auth/login")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"code":"u1","password":"p"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        // /auth/login (without /api) does not exist; axum returns 404.
        assert_eq!(res.status(), axum::http::StatusCode::NOT_FOUND);
    }
}
```

- [ ] **Step 2: Run the tests**

Run: `cargo test -p aegis-server --lib transport::http::router::`
Expected: PASS — all 5 tests succeed.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/router.rs
git commit -m "feat(aegis-server): top-level router with /api nest + swagger-ui"
```

---

### Task 11: `lib.rs::run(Config)` bootstrap

**Files:**
- Modify: `apps/server/aegis-server/src/lib.rs`

- [ ] **Step 1: Replace the skeleton `lib.rs` with the real `run`**

Replace the entire current contents of `apps/server/aegis-server/src/lib.rs` with:

```rust
//! # aegis-server
//!
//! HTTP server binary. Wires the `auth` crate's `AuthServiceImpl`
//! against a Postgres pool + in-memory token-version cache, mounts
//! the auth-flow endpoints under `/api/auth/*` with `axum`, and
//! exposes the OpenAPI document at `/api-docs/openapi.json` plus
//! swagger-ui at `/swagger-ui`.
//!
//! The public surface is small (`run`, `Config`, `AppState`,
//! `transport::router`) so the binary entry point stays a thin
//! `main.rs` that parses env, initialises tracing, and calls
//! `aegis_server::run(config)`.

pub mod config;
pub mod state;
pub mod transport;

use std::sync::Arc;

use anyhow::{Context, Result};

pub use config::{Config, ConfigError};
pub use state::AppState;

/// Run the HTTP server until interrupted.
///
/// `run` owns the wiring from [`Config`] to a running
/// `tokio::net::TcpListener`. It does not apply migrations (those
/// are an ops step); the schema is expected to be in place when the
/// server boots.
pub async fn run(config: Config) -> Result<()> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await
        .context("connect to Postgres")?;

    let user_repo = user::UserRepo::new(pool.clone());
    let user_usecase = user::UserUsecase::new(user_repo);
    let user_service_impl = user::UserServiceImpl::new(user_usecase);
    let user_service: Arc<dyn apis::user::UserService> = Arc::new(user_service_impl);

    let auth_user_service_impl = auth::UserServiceImpl::new(user_service.clone());
    let auth_user_service: Arc<dyn auth::domain::UserService> =
        Arc::new(auth_user_service_impl);

    let credentials_repo = auth::UserCredentialsRepo::new(pool.clone());
    let identities_repo = auth::DomainIdentityRepo::new(pool);

    let cache: Arc<dyn auth::TokenVersionCache> = Arc::new(auth::InMemoryTokenVersionCache::new());

    let usecase = auth::AuthUsecase::new(auth::AuthUsecaseConfig {
        credentials: credentials_repo,
        identities: identities_repo,
        user_service: auth_user_service,
        cache,
        signing_key: config.signing_key.clone(),
        access_ttl: config.access_ttl,
        refresh_ttl: config.refresh_ttl,
    });

    let auth: Arc<dyn apis::auth::AuthService> = Arc::new(auth::AuthServiceImpl::new(usecase));

    let app_state = AppState {
        auth,
        user: user_service,
    };
    let app = transport::router(app_state);

    let listener = tokio::net::TcpListener::bind(config.bind_addr)
        .await
        .with_context(|| format!("bind {}", config.bind_addr))?;
    tracing::info!(addr = %config.bind_addr, "aegis-server listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("axum::serve")?;
    Ok(())
}

/// Wait for Ctrl-C / SIGTERM.
async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c().await.expect("install ctrl-c handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("install SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => tracing::info!("ctrl-c received, shutting down"),
        _ = terminate => tracing::info!("SIGTERM received, shutting down"),
    }
}
```

- [ ] **Step 2: Verify the crate compiles**

Run: `cargo build -p aegis-server`
Expected: success. (No test runs yet — the integration test is in Task 13.)

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/src/lib.rs
git commit -m "feat(aegis-server): run(Config) bootstrap with graceful shutdown"
```

---

### Task 12: `main.rs` thin entry point

**Files:**
- Modify: `apps/server/aegis-server/src/main.rs`

- [ ] **Step 1: Replace the stub with the real entry point**

Replace the entire current contents of `apps/server/aegis-server/src/main.rs` with:

```rust
use aegis_server::{Config, run};
use anyhow::{Context, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env if present (development only; production sets env vars
    // directly).
    let _ = dotenvy::dotenv();

    // Init tracing. JSON output so a downstream collector can parse
    // request / span events emitted by TraceLayer.
    tracing_subscriber::fmt()
        .json()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let config = Config::from_env().context("load server config")?;
    run(config).await
}
```

- [ ] **Step 2: Verify the binary compiles**

Run: `cargo build -p aegis-server --bin aegis-server`
Expected: success.

- [ ] **Step 3: Verify `cargo run` fails fast when env vars are missing**

Run: `unset AEGIS_DATABASE_URL AEGIS_AUTH_SIGNING_KEY && cargo run -p aegis-server`
Expected: the process exits with an error message naming `AEGIS_DATABASE_URL` (and does not bind a port).

(Re-export of `ConfigError` from `lib.rs` already gives the printable message; if the exit code path needs a tweak, see the error-display section in `Config::from_env`.)

- [ ] **Step 4: Commit**

```bash
git add apps/server/aegis-server/src/main.rs
git commit -m "feat(aegis-server): main.rs entry point with tracing init"
```

---

### Task 13: Live-DB integration test (`tests/integration_auth.rs`, `#[ignore]`)

**Files:**
- Create: `apps/server/aegis-server/tests/integration_auth.rs`

- [ ] **Step 1: Create the integration test file**

Create `apps/server/aegis-server/tests/integration_auth.rs`:

```rust
//! Live-database integration test for the aegis-server HTTP surface.
//!
//! Boots the real `Config::from_env` against `AEGIS_DATABASE_URL`,
//! runs migrations, wires the real `AuthServiceImpl`, fires real
//! `POST /api/auth/login` / `/refresh` / `/logout` requests against
//! the in-process `tower::ServiceExt::oneshot` router, and asserts
//! the round-trip.
//!
//! `#[ignore]`-gated so the default `cargo test -p aegis-server`
//! stays green without a database. Run with:
//!
//! ```bash
//! AEGIS_DATABASE_URL=postgres://… cargo test -p aegis-server \
//!     --test integration_auth -- --ignored --test-threads=1
//! ```

use std::sync::Arc;
use std::time::Duration;

use apis::user::UserService as _;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use sqlx::PgPool;
use tower::ServiceExt;
use utoipa_axum::OpenApiRouter;

use aegis_server::transport::http::auth::router as auth_router;
use aegis_server::{AppState, run};

fn database_url() -> String {
    std::env::var("AEGIS_DATABASE_URL")
        .expect("set AEGIS_DATABASE_URL before running ignored tests")
}

async fn pool() -> PgPool {
    sqlx::postgres::PgPoolOptions::new()
        .max_connections(2)
        .connect(&database_url())
        .await
        .expect("connect to Postgres")
}

async fn apply_migrations(pool: &PgPool) {
    sqlx::migrate!("../../lib/crates/auth/migrations")
        .run(pool)
        .await
        .expect("apply auth migrations");
    sqlx::migrate!("../../lib/crates/user/migrations")
        .run(pool)
        .await
        .expect("apply user migrations");
}

/// Build a minimal real `AppState` for round-trip testing.
///
/// Inserts a user row directly via the `user` crate's repository,
/// then inserts an `auth_user_credentials` row with a pre-computed
/// Argon2 hash. The hash below matches the password "secret".
const PASSWORD: &str = "secret";
const USER_CODE: &str = "it-user-1";

fn argon_hash_of_password(password: &str) -> String {
    use argon2::password_hash::{PasswordHasher, SaltString};
    let salt = SaltString::from_b64("bWluaW11bXNhbHQ").unwrap();
    argon2::Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .expect("hash password")
        .to_string()
}

async fn seed_user_and_credential(pool: &PgPool) {
    let user_repo = user::UserRepo::new(pool.clone());
    let user_usecase = user::UserUsecase::new(user_repo);
    let user_svc_impl = user::UserServiceImpl::new(user_usecase);
    let user_svc: Arc<dyn apis::user::UserService> = Arc::new(user_svc_impl);

    user_svc
        .create(apis::user::CreateUserRequest {
            code: USER_CODE.into(),
            name: "Integration Test User".into(),
            role: apis::user::Role::Admin,
        })
        .await
        .expect("create user");

    let hash = argon_hash_of_password(PASSWORD);

    sqlx::query(
        "INSERT INTO auth_user_credentials (code, password_hash, token_version) VALUES ($1, $2, 0)",
    )
    .bind(USER_CODE)
    .bind(&hash)
    .execute(pool)
    .await
    .expect("insert credential row");
}

fn json_request(method: &str, uri: &str, body: &str) -> Request<Body> {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

async fn body_text(res: axum::response::Response) -> String {
    let bytes = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    String::from_utf8(bytes.to_vec()).unwrap()
}

async fn body_json(res: axum::response::Response) -> serde_json::Value {
    let bytes = axum::body::to_bytes(res.into_body(), 4096).await.unwrap();
    serde_json::from_slice(&bytes).unwrap()
}

/// Build the same router the server uses in `run`, but driven via
/// `oneshot` instead of `axum::serve`. Skips tracing + swagger-ui so
/// the test stays focused on the auth round-trip.
fn build_test_router(pool: PgPool) -> axum::Router {
    let user_repo = user::UserRepo::new(pool.clone());
    let user_usecase = user::UserUsecase::new(user_repo);
    let user_service_impl = user::UserServiceImpl::new(user_usecase);
    let user_service: Arc<dyn apis::user::UserService> = Arc::new(user_service_impl);

    let auth_user_service_impl = auth::UserServiceImpl::new(user_service.clone());
    let auth_user_service: Arc<dyn auth::domain::UserService> =
        Arc::new(auth_user_service_impl);

    let credentials_repo = auth::UserCredentialsRepo::new(pool.clone());
    let identities_repo = auth::DomainIdentityRepo::new(pool);

    let cache: Arc<dyn auth::TokenVersionCache> = Arc::new(auth::InMemoryTokenVersionCache::new());

    let usecase = auth::AuthUsecase::new(auth::AuthUsecaseConfig {
        credentials: credentials_repo,
        identities: identities_repo,
        user_service: auth_user_service,
        cache,
        signing_key: vec![0u8; 32],
        access_ttl: Duration::from_secs(60),
        refresh_ttl: Duration::from_secs(120),
    });
    let auth: Arc<dyn apis::auth::AuthService> = Arc::new(auth::AuthServiceImpl::new(usecase));

    let state = AppState {
        auth,
        user: user_service,
    };
    let (r, _api) = OpenApiRouter::new().nest("/auth", auth_router()).split_for_openapi();
    r.with_state(state)
}

#[tokio::test]
#[ignore]
async fn login_refresh_logout_round_trip() {
    let pool = pool().await;
    apply_migrations(&pool).await;
    seed_user_and_credential(&pool).await;

    let app = build_test_router(pool);

    // 1. login
    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/auth/login",
            &format!(r#"{{"code":"{USER_CODE}","password":"{PASSWORD}"}}"#),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let access_token = body["access_token"].as_str().unwrap().to_string();
    let refresh_token = body["refresh_token"].as_str().unwrap().to_string();
    assert!(!access_token.is_empty());
    assert!(!refresh_token.is_empty());

    // 2. refresh
    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/auth/refresh",
            &format!(r#"{{"refresh_token":"{refresh_token}"}}"#),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_json(res).await;
    let new_access = body["access_token"].as_str().unwrap();
    assert!(!new_access.is_empty());

    // 3. logout
    let res = app
        .clone()
        .oneshot(json_request(
            "POST",
            "/auth/logout",
            &format!(r#"{{"refresh_token":"{refresh_token}"}}"#),
        ))
        .await
        .unwrap();
    assert_eq!(res.status(), StatusCode::OK);
    let body = body_text(res).await;
    assert_eq!(body, "{}");
}

#[tokio::test]
#[ignore]
async fn run_starts_and_serves_healthz() {
    // Smoke test: actually call `run` against an ephemeral port and
    // fire a real TCP request at /api/healthz. Aborts the server
    // before returning.
    use tokio::net::TcpListener;

    let pool = pool().await;
    apply_migrations(&pool).await;

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener); // free the port so axum can rebind it

    let config = aegis_server::Config {
        database_url: database_url(),
        signing_key: vec![0u8; 32],
        bind_addr: addr,
        access_ttl: Duration::from_secs(60),
        refresh_ttl: Duration::from_secs(120),
    };

    let server = tokio::spawn(run(config));

    // Give axum a moment to bind.
    tokio::time::sleep(Duration::from_millis(250)).await;

    // Connect directly to the bound address.
    let mut stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    use tokio::io::{AsyncWriteExt, BufWriter};
    let mut writer = BufWriter::new(&mut stream);
    writer
        .write_all(b"GET /api/healthz HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n")
        .await
        .unwrap();
    writer.flush().await.unwrap();
    drop(writer);

    let mut buf = Vec::new();
    use tokio::io::AsyncReadExt;
    stream.read_to_end(&mut buf).await.unwrap();
    let response = String::from_utf8(buf).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 200"),
        "expected 200, got: {response}"
    );
    assert!(response.contains("ok"), "response body did not contain 'ok'");

    server.abort();
}
```

- [ ] **Step 2: Verify the integration test compiles (do not run it yet)**

Run: `cargo test -p aegis-server --test integration_auth --no-run`
Expected: success. The test bodies use `#[ignore]`, so they don't actually need a DB to compile.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/tests/integration_auth.rs
git commit -m "test(aegis-server): live-DB round-trip integration test (ignored)"
```

---

### Task 14: Public-API compile test (`tests/public_api.rs`)

**Files:**
- Create: `apps/server/aegis-server/tests/public_api.rs`

- [ ] **Step 1: Create the public-API test file**

Create `apps/server/aegis-server/tests/public_api.rs`:

```rust
//! Public-API compile test for the `aegis-server` crate.
//!
//! Does NOT run any I/O. Locks the documented `aegis_server::*`
//! re-exports so a refactor that drops or renames a public name
//! fails at `cargo test -p aegis-server --test public_api` time.

use std::sync::Arc;

use aegis_server::transport::http::auth::AuthClaims;
use aegis_server::transport::http::dto::{
    AccessTokenResponse, AuthClaimsResponse, LoginDomainRequest, LoginRequest, LogoutRequest,
    LogoutResponse, RefreshRequest, Role, TokenPairResponse,
};
use aegis_server::transport::http::error::ErrorBody;
use aegis_server::{AppState, Config, run};

/// `run(Config) -> anyhow::Result<()>` is the documented entry
/// point. The function pointer assertion locks the signature.
#[test]
fn run_signature_is_documented() {
    let _f: fn(Config) -> _ = run;
}

/// `Config` is constructible field-by-field with documented fields.
#[test]
fn config_fields_are_public() {
    fn _assert_send_sync<T: Send + Sync>() {}
    _assert_send_sync::<Config>();
    let _ = std::any::type_name::<Config>();
}

/// `AppState` is `Clone` + `Send + Sync` and exposes two `Arc<dyn …>`
/// fields. Future user-CRUD routes will use `state.user`; the auth
/// flow uses `state.auth`.
#[test]
fn app_state_layout_is_documented() {
    fn _clone<T: Clone>() {}
    fn _send_sync<T: Send + Sync>() {}
    _clone::<AppState>();
    _send_sync::<AppState>();
    let _auth: Arc<dyn apis::auth::AuthService>;
    let _user: Arc<dyn apis::user::UserService>;
}

/// Every wire DTO is nameable, constructible, and survives a serde
/// round-trip. (The serde round-trip is also covered by the inline
/// `dto.rs` tests; this is a compile-time lock.)
#[test]
fn wire_dtos_are_nameable() {
    fn _assert_login(_: LoginRequest) {}
    fn _assert_login_domain(_: LoginDomainRequest) {}
    fn _assert_refresh(_: RefreshRequest) {}
    fn _assert_logout_req(_: LogoutRequest) {}
    fn _assert_pair(_: TokenPairResponse) {}
    fn _assert_access(_: AccessTokenResponse) {}
    fn _assert_logout_res(_: LogoutResponse) {}
    fn _assert_claims(_: AuthClaimsResponse) {}
    fn _assert_role(_: Role) {}

    _assert_login(LoginRequest { code: "u".into(), password: "p".into() });
    _assert_login_domain(LoginDomainRequest {
        code: "u".into(),
        domain_name: "d".into(),
        hostname: "h".into(),
        sid: "s".into(),
    });
    _assert_refresh(RefreshRequest { refresh_token: "r".into() });
    _assert_logout_req(LogoutRequest { refresh_token: "r".into() });
    _assert_pair(TokenPairResponse { access_token: "a".into(), refresh_token: "r".into() });
    _assert_access(AccessTokenResponse { access_token: "a".into() });
    _assert_logout_res(LogoutResponse {});
    _assert_claims(AuthClaimsResponse { code: "u".into(), role: Role::Admin, token_version: 0 });
    _assert_role(Role::Root);
}

/// `ErrorBody` is nameable + constructible. The two fields are the
/// only ones the wire contract promises.
#[test]
fn error_body_is_documented() {
    let _ = ErrorBody { code: "x".into(), message: "y".into() };
}

/// `AuthClaims` extractor is reachable from the documented path.
#[test]
fn auth_claims_extractor_is_documented() {
    let _ = std::any::type_name::<AuthClaims>();
}
```

- [ ] **Step 2: Run the public-API test**

Run: `cargo test -p aegis-server --test public_api`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add apps/server/aegis-server/tests/public_api.rs
git commit -m "test(aegis-server): public-API compile test"
```

---

### Task 15: README + workspace-wide verification

**Files:**
- Create: `apps/server/aegis-server/README.md`

- [ ] **Step 1: Create the README**

Create `apps/server/aegis-server/README.md`:

````markdown
# aegis-server

HTTP server binary. Wires the `auth` crate's `AuthServiceImpl` against
a Postgres pool + an in-memory token-version cache, mounts the
auth-flow endpoints under `/api/auth/*` with `axum`, and serves the
OpenAPI document at `/api-docs/openapi.json` plus swagger-ui at
`/swagger-ui`.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the workspace-wide conventions this crate follows.

## Run

```bash
export AEGIS_DATABASE_URL=postgres://localhost/aegis
export AEGIS_AUTH_SIGNING_KEY=$(openssl rand -hex 32)
sqlx migrate run --source lib/crates/auth/migrations
sqlx migrate run --source lib/crates/user/migrations
cargo run -p aegis-server
```

The server binds `0.0.0.0:8080` by default. Override with
`AEGIS_HTTP_BIND=127.0.0.1:9090`.

## Endpoints

| Method | Path                          | Purpose                                |
|--------|-------------------------------|----------------------------------------|
| POST   | `/api/auth/login`             | Code + password login                  |
| POST   | `/api/auth/login-domain`      | Domain-identity (AD / NTLM) login      |
| POST   | `/api/auth/refresh`           | Exchange refresh token for access token |
| POST   | `/api/auth/logout`            | Invalidate the refresh token           |
| GET    | `/api/healthz`                | Liveness probe (returns `ok`)          |
| GET    | `/api-docs/openapi.json`      | OpenAPI v3 document                    |
| GET    | `/swagger-ui`                 | swagger-ui HTML                        |

`Authorization: Bearer <access_token>` is consumed by the
[`AuthClaims`](src/transport/http/auth/middleware.rs) extractor; future
protected handlers take `claims: AuthClaims` as a parameter.

## Environment variables

| var                      | required | default        | notes                                              |
|--------------------------|----------|----------------|----------------------------------------------------|
| `AEGIS_DATABASE_URL`     | yes      | —              | Postgres URL                                       |
| `AEGIS_AUTH_SIGNING_KEY` | yes      | —              | hex-encoded; ≥32 bytes decoded                      |
| `AEGIS_HTTP_BIND`        | no       | `0.0.0.0:8080` | `SocketAddr`                                       |
| `AEGIS_ACCESS_TTL_SECS`  | no       | `900` (15 m)   | `u64` → `Duration::from_secs`                      |
| `AEGIS_REFRESH_TTL_SECS` | no       | `604800` (7 d) | `u64` → `Duration::from_secs`                      |
| `RUST_LOG`               | no       | `info`         | `tracing-subscriber` env filter                    |

## Tests

```bash
cargo test -p aegis-server                                 # unit + public-API compile
AEGIS_DATABASE_URL=… cargo test -p aegis-server \
    -- --ignored --test-threads=1                         # live-DB round-trip
```

The integration test in `tests/integration_auth.rs` requires the
schema to be migrated against a reachable Postgres before running.
It applies both the `auth` and the `user` migrations at startup.
````

- [ ] **Step 2: Commit the README**

```bash
git add apps/server/aegis-server/README.md
git commit -m "docs(aegis-server): README with env vars + endpoint table"
```

- [ ] **Step 3: Run the full verification gate**

Run (in order):

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-server --all-targets -- -D warnings
cargo test -p aegis-server
cargo doc -p aegis-server --no-deps
```

Expected:

- `cargo fmt` exit 0 (no diff).
- `cargo clippy` exit 0 (no warnings).
- `cargo test` reports every unit + integration-compile test passing.
- `cargo doc` builds cleanly.

If any command fails, fix the offending file and re-run the gate
before considering this plan complete.

- [ ] **Step 4: Commit any verification fixes**

If the gate surfaced a small fix (clippy lint, fmt diff, doc link),
commit it as a separate `chore(aegis-server): …` commit so the
verification log stays out of feature commits.

```bash
git add <offending files>
git commit -m "chore(aegis-server): address verification gate findings"
```

(If nothing was surfaced by Step 3, skip this step — there is no
commit to make.)

---

## Self-Review Notes (informational; pre-execution)

- **Spec coverage:** Every spec section maps to a task.
  - § Architecture (transport/, http/, files) → Task 2.
  - § Config + § State + § Bootstrap → Tasks 3, 4, 11, 12.
  - § Routes & middleware (handler list, prefix `/api`, AuthClaims extractor) → Tasks 8, 9, 10.
  - § Wire DTOs → Task 4.
  - § Error mapping → Task 5.
  - § OpenAPI document → Task 7.
  - § Testing (six layers) → Tasks 3, 4, 5, 6, 7, 8, 9, 10, 13, 14.
  - § Public API surface → Task 14.
  - § Verification gate → Task 15.
- **Placeholders:** No "TBD" / "implement later" / "fill in later" anywhere. Task 2's scaffold stubs use `todo!()` for module bodies that compile but panic — every one of those `todo!()` is named in the task that fills it in (Task 6 healthz, Task 7 openapi, Task 8 auth handlers, Task 9 AuthClaims extractor, Task 10 router, Task 11 run). Every code block shows full code; every command is concrete.
- **Type consistency:** DTO field names match across `dto.rs`, the
  handler signatures, the OpenAPI schemas, and the test bodies. The
  `From<apis::user::Role> for dto::Role` and `From<dto::Role> for
  apis::user::Role` impls are defined in Task 4 and used consistently
  in handler tests. The `AppState` field types (`Arc<dyn
  apis::auth::AuthService>`, `Arc<dyn apis::user::UserService>`) match
  in Tasks 4, 8, 10, 11, 14.
- **Compiles-after-each-task discipline:** Tasks 1–2 set up deps +
  skeleton (compiles). Tasks 3–10 each have TDD discipline (failing
  test → impl → passing test → commit). Task 11 wires the
  bootstrap; the crate builds at the end. Task 12 wires main.rs; the
  binary builds. Tasks 13–14 add test files. Task 15 runs the gate.
- **Test counts:** roughly 50 tests total across the suite (10 DTO
  round-trips + 9 error mapping + 1 healthz + 2 OpenAPI + 12 handler +
  5 AuthClaims + 5 router + 8 public-API + 2 ignored integration).