# Auth Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `lib/crates/auth` per [`docs/superpowers/specs/2026-08-05-auth-crate-design.md`](../specs/2026-08-05-auth-crate-design.md) — a ports-and-adapters DDD crate that mints HS256 JWTs, validates them against an in-memory `token_version` cache backed by Postgres, and adapts its usecase layer into `apis::auth::AuthService`.

**Architecture:** Three DDD layers — `domain` (pure types + ports + errors), `usecase` (`AuthUsecase<R, D>` with `Arc<dyn UserService>` as a private field, holds an `Arc<RwLock<HashMap<String, u32>>>` token-version cache), `adapter` (PostgreSQL-backed `UserCredentialsRepo` + `DomainIdentityRepo`; in-memory facade `AuthServiceImpl` implementing `apis::auth::AuthService`). Crate depends on `apis` (port) and `user`-adjacent workspace deps but never on `user` directly.

**Tech Stack:** `sqlx 0.9` (Postgres runtime API), `tokio 1.53`, `async-trait 0.1.91`, `thiserror 2`, `argon2 0.5.3`, `chrono 0.4` (clock feature), `rand_core 0.6`, `jsonwebtoken 11`, `dotenvy 0.15` (dev-only), `apis` (workspace path-dep).

**Spec:** [`docs/superpowers/specs/2026-08-05-auth-crate-design.md`](../specs/2026-08-05-auth-crate-design.md)

## Global Constraints

These come from the spec and the lib-crate guideline; every task implicitly includes them.

- **Edition:** Rust 2024 (`edition = "2024"`, `resolver = "3"`).
- **No `mod.rs`:** every module uses `src/<module>.rs` + `src/<module>/`. Terminal leaf files (`role.rs`, `auth_repo.rs`, `service.rs`, `row.rs`, …) are leaf files with no companion directory.
- **Layer dependency rule:** `domain` depends on nothing except std + `async-trait`; `usecase` depends on `domain` + `apis` (port) + `argon2` + `jsonwebtoken` + `chrono`; `adapter` depends on `usecase` + `domain`. No layer depends on a sibling layer inside the same crate beyond the documented direction.
- **Public surface:** the crate root re-exports exactly the types listed in the spec's "Public API" section. No internal types (`AccessClaims`, `RefreshClaims`, `token_versions`, mock repos, fakes) are re-exported.
- **`UserService` is reached via the apis port.** The auth crate does **not** depend on the `user` crate. Active state and `Role` flow through `Arc<dyn apis::user::UserService>`.
- **Runtime SQLx API:** the persistence adapter uses `sqlx::query_as` and `sqlx::QueryBuilder`. No compile-time `query!` / `query_as!` macros. A module-level comment at the top of `postgres.rs` documents the choice.
- **`map_db_error` rules** (mirror the user crate): `sqlx::Error::RowNotFound` → `DomainError::NotFound`; `sqlx::Error::Database` with SQLSTATE `23505` → `DomainError::DuplicateCode(constraint_name)`; everything else → `DomainError::Repository(driver_message)`.
- **`jsonwebtoken = "11"`** from `[workspace.dependencies]`. HS256 only. Two private claim structs (`AccessClaims`, `RefreshClaims`) live in the usecase module.
- **`argon2 = "0.5.3"`** from workspace deps. `argon2::PasswordHasher` and `argon2::PasswordVerifier`. Default `Argon2::default()` parameters. Hashing happens in the usecase, never in the repos.
- **Migrations:** consumed via `sqlx::migrate!("./migrations")` in integration tests. Each schema change is one file. Live-DB integration tests are `#[ignore]`-gated.
- **Env var:** live-DB tests read `AEGIS_AUTH_DATABASE_URL` (with `dotenvy::dotenv()` at startup; panic if missing).
- **Unique per-run values:** integration tests generate a per-process atomic counter + wall-clock nanoseconds for any UNIQUE-constrained column.
- **Destructive cleanup:** integration tests `DROP TABLE IF EXISTS auth_user_credentials CASCADE`, `DROP TABLE IF EXISTS auth_user_domain_identities CASCADE`, and `DROP TABLE IF EXISTS _sqlx_migrations CASCADE` before applying migrations.
- **Layer-boundary visibility:** `adapter/persistence` is `pub(crate) mod postgres;`; `adapter/persistence/postgres` keeps `row` and `auth_repo` private but exposes `UserCredentialsRepo` / `DomainIdentityRepo` via `pub use`. The `postgres.rs` leaf is `pub` so the `pub use` is well-formed.
- **Test gates** per the lib-crate guideline section 8:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p auth --all-targets --all-features -- -D warnings
  cargo test -p auth
  cargo doc -p auth --no-deps
  cargo test -p auth -- --ignored --test-threads=1   # with AEGIS_AUTH_DATABASE_URL
  ```

---

## File Structure

Created (paths relative to `lib/crates/auth/`):

```
Cargo.toml
README.md
migrations/
  0001_create_auth_user_credentials.sql
  0002_create_auth_user_domain_identities.sql
src/
  lib.rs
  domain.rs
  domain/
    role.rs
    credentials.rs
    domain_identity.rs
    error.rs
    repository.rs
    tests.rs
  usecase.rs
  usecase/
    commands.rs
    error.rs
    auth_usecase.rs
    tests.rs
  adapter.rs
  adapter/
    persistence.rs
    persistence/
      postgres.rs
      postgres/
        row.rs
        auth_repo.rs
        tests.rs
    facade.rs
    facade/
      in_memory.rs
      in_memory/
        service.rs
        fake_user_service.rs
        tests.rs
tests/
  public_api.rs
  integration_persistence.rs
```

Each file owns exactly the responsibility in its name. `domain/tests.rs` exercises `Role`, `UserCredentials`, `DomainIdentity`. `adapter/persistence/postgres/tests.rs` covers row conversions + migration schema content. `usecase/tests.rs` covers command orchestration against mock repos + `FakeUserService`. `adapter/facade/in_memory/tests.rs` covers the `AuthService` surface end-to-end. `tests/public_api.rs` is compile-only. `tests/integration_persistence.rs` is the `#[ignore]`-gated live-DB round-trip.

---

## Task 1: Crate scaffolding + workspace dep registration

**Files:**
- Modify: `/root/coding/project/aegis/Cargo.toml` (add `jsonwebtoken = "11"` to `[workspace.dependencies]`)
- Modify: `/root/coding/project/aegis/lib/crates/auth/Cargo.toml` (full deps list)
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/lib.rs` (replace the `add` boilerplate)

- [ ] **Step 1: Register `jsonwebtoken` in the workspace**

Edit `/root/coding/project/aegis/Cargo.toml`. Add this line inside `[workspace.dependencies]`, right after `chrono = …`:

```toml
jsonwebtoken = "11"
```

- [ ] **Step 2: Write `lib/crates/auth/Cargo.toml`**

Replace the file with:

```toml
[package]
name = "auth"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `argon2` provides the password hasher / verifier used by the usecase
# layer. The repos never see a plaintext password.
argon2 = { workspace = true }
# `chrono` provides `DateTime<Utc>` for the `created_at` / `updated_at`
# columns surfaced by `UserCredentials`.
chrono = { workspace = true }
# `rand_core` provides the cryptographic RNG used to derive the HS256
# signing key from caller-supplied entropy (no direct call today, but
# pinned here so a future key-rotation helper does not silently pick a
# weaker default).
rand_core = { workspace = true }
# `jsonwebtoken` provides the HS256 sign / verify primitives used by the
# usecase layer to mint access + refresh tokens.
jsonwebtoken = { workspace = true }
# `apis` provides the outbound `AuthService` port the facade implements
# and the `UserService` port the usecase consults for active state.
# Path-dep because both crates share the workspace.
apis = { path = "../apis" }

[dev-dependencies]
# Loads `.env` at test startup so live-DB integration tests can find
# `AEGIS_AUTH_DATABASE_URL` without the user having to `source` it manually.
dotenvy = { workspace = true }
# Re-export the SQLx driver in `[dev-dependencies]` so the integration
# tests can build their own `PgPool` without going through the public
# API for connection setup.
sqlx = { workspace = true }
# `tokio` macros + multi-thread runtime are needed for `#[tokio::test]`
# in unit + integration tests.
tokio = { workspace = true }
```

- [ ] **Step 3: Replace the boilerplate `lib.rs` with module declarations**

Replace `/root/coding/project/aegis/lib/crates/auth/src/lib.rs` with:

```rust
//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<RwLock<HashMap<String, u32>>>` token-version cache live inside the
//! usecase. Public consumers should `use auth::*;` (see the re-exports
//! below) rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

// Re-exports are filled in by later tasks. Today the crate has no public
// surface; the modules are private scaffolding.
```

- [ ] **Step 4: Build the empty crate**

Run:
```bash
cargo build -p auth
```
Expected: success. (The modules `domain`, `usecase`, `adapter` are empty `pub mod` declarations — Rust accepts the empty form via `pub mod <name>;` only when a file `<name>.rs` exists with at least a permissive body. Replace each `pub mod` with `pub mod domain { }` if a build error appears; subsequent tasks will replace those with `pub mod <name>;` + `pub use …;` once the leaf files land.)

If the build fails because `pub mod domain;` requires a corresponding `domain.rs` file, create three empty files now:

```bash
touch /root/coding/project/aegis/lib/crates/auth/src/domain.rs \
      /root/coding/project/aegis/lib/crates/auth/src/usecase.rs \
      /root/coding/project/aegis/lib/crates/auth/src/adapter.rs
```

Re-run `cargo build -p auth` until it succeeds.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml lib/crates/auth/Cargo.toml lib/crates/auth/src/lib.rs
git commit -m "feat(auth): scaffold crate + register jsonwebtoken workspace dep"
```

---

## Task 2: Domain layer — Role, UserCredentials, DomainIdentity, DomainError, ports, tests

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/role.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/credentials.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/domain_identity.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/error.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/repository.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/domain/tests.rs`
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/lib.rs` (re-export domain types)

- [ ] **Step 1: Write `domain/role.rs`**

```rust
use std::convert::TryFrom;

use super::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Root,
    Admin,
    General,
}

impl Role {
    pub fn as_str(&self) -> &'static str {
        match self {
            Role::Root => "root",
            Role::Admin => "admin",
            Role::General => "general",
        }
    }
}

impl TryFrom<&str> for Role {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "root" => Ok(Role::Root),
            "admin" => Ok(Role::Admin),
            "general" => Ok(Role::General),
            other => Err(DomainError::InvalidRole(other.to_string())),
        }
    }
}
```

- [ ] **Step 2: Write `domain/error.rs`**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("user code must not be empty")]
    EmptyCode,

    #[error("password hash must not be empty")]
    EmptyPasswordHash,

    #[error("invalid role: {0}")]
    InvalidRole(String),

    #[error("not found")]
    NotFound,

    #[error("user code already exists: {0}")]
    DuplicateCode(String),

    #[error("user is inactive")]
    Inactive,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("repository error: {0}")]
    Repository(String),
}
```

- [ ] **Step 3: Write `domain/credentials.rs`**

```rust
use chrono::{DateTime, Utc};

use super::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct UserCredentials {
    pub code: String,
    pub password_hash: String,
    pub token_version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl UserCredentials {
    /// Validating constructor used by the domain / usecase layers.
    #[allow(dead_code)]
    pub(crate) fn new(
        code: String,
        password_hash: String,
        token_version: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if password_hash.is_empty() {
            return Err(DomainError::EmptyPasswordHash);
        }
        Ok(Self {
            code,
            password_hash,
            token_version,
            created_at,
            updated_at,
        })
    }

    /// Repository-bound constructor. Skips validation because the row
    /// is assumed to have been validated on the way in.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        code: String,
        password_hash: String,
        token_version: u32,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            code,
            password_hash,
            token_version,
            created_at,
            updated_at,
        }
    }
}

/// Hand-rolled `Debug` that omits the password hash.
impl std::fmt::Debug for UserCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UserCredentials")
            .field("code", &self.code)
            .field("token_version", &self.token_version)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish_non_exhaustive()
    }
}
```

- [ ] **Step 4: Write `domain/domain_identity.rs`**

```rust
use super::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct DomainIdentity {
    pub user_code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl DomainIdentity {
    pub(crate) fn new(
        user_code: String,
        domain_name: String,
        hostname: String,
        sid: String,
    ) -> Result<Self, DomainError> {
        if user_code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if domain_name.trim().is_empty()
            || hostname.trim().is_empty()
            || sid.trim().is_empty()
        {
            return Err(DomainError::EmptyPasswordHash);
        }
        Ok(Self {
            user_code,
            domain_name,
            hostname,
            sid,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn for_repository(
        user_code: String,
        domain_name: String,
        hostname: String,
        sid: String,
    ) -> Self {
        Self {
            user_code,
            domain_name,
            hostname,
            sid,
        }
    }
}

impl std::fmt::Debug for DomainIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DomainIdentity")
            .field("user_code", &self.user_code)
            .field("domain_name", &self.domain_name)
            .field("hostname", &self.hostname)
            .field("sid", &self.sid)
            .finish()
    }
}
```

- [ ] **Step 5: Write `domain/repository.rs`**

```rust
use async_trait::async_trait;

use super::DomainError;
use super::credentials::UserCredentials;
use super::domain_identity::DomainIdentity;

/// Outbound port for persistence of `UserCredentials`.
#[async_trait]
pub trait UserCredentialsRepository: Send + Sync {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError>;

    async fn create(
        &self,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError>;

    /// Atomically increments `token_version` for the user identified
    /// by `code` and returns the new value. Returns `DomainError::NotFound`
    /// if no row exists.
    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError>;
}

/// Outbound port for persistence of `DomainIdentity`.
#[async_trait]
pub trait DomainIdentityRepository: Send + Sync {
    /// Find the row matching the supplied identity triple. Returns
    /// `DomainError::NotFound` if no row matches.
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError>;
}
```

- [ ] **Step 6: Write `domain.rs` (module declarations + re-exports)**

```rust
mod credentials;
mod domain_identity;
mod error;
mod repository;
mod role;

#[cfg(test)]
mod tests;

pub use credentials::UserCredentials;
pub use domain_identity::DomainIdentity;
pub use error::DomainError;
pub use repository::{DomainIdentityRepository, UserCredentialsRepository};
pub use role::Role;
```

- [ ] **Step 7: Write `domain/tests.rs`**

```rust
use super::*;

use chrono::{TimeZone, Utc};

fn test_now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

#[test]
fn role_as_str_maps_to_lowercase() {
    assert_eq!(Role::Root.as_str(), "root");
    assert_eq!(Role::Admin.as_str(), "admin");
    assert_eq!(Role::General.as_str(), "general");
}

#[test]
fn try_from_str_parses_known_values_lowercase() {
    assert_eq!(Role::try_from("root").unwrap(), Role::Root);
    assert_eq!(Role::try_from("admin").unwrap(), Role::Admin);
    assert_eq!(Role::try_from("general").unwrap(), Role::General);
}

#[test]
fn try_from_str_rejects_unknown_value() {
    let err = Role::try_from("superuser").unwrap_err();
    assert!(matches!(err, DomainError::InvalidRole(ref s) if s == "superuser"));
}

#[test]
fn try_from_str_rejects_empty_string() {
    let err = Role::try_from("").unwrap_err();
    assert!(matches!(err, DomainError::InvalidRole(_)));
}

#[test]
fn new_user_credentials_rejects_empty_code() {
    let err = UserCredentials::new(
        "".into(),
        "hash".into(),
        1,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn new_user_credentials_rejects_empty_password_hash() {
    let err = UserCredentials::new(
        "u1".into(),
        "".into(),
        1,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyPasswordHash));
}

#[test]
fn new_user_credentials_accepts_valid_input() {
    let c = UserCredentials::new(
        "u1".into(),
        "hash".into(),
        1,
        test_now(),
        test_now(),
    )
    .expect("valid credentials should construct");
    assert_eq!(c.code, "u1");
    assert_eq!(c.token_version, 1);
}

#[test]
fn user_credentials_debug_omits_password_hash() {
    let c = UserCredentials::for_repository(
        "u1".into(),
        "hash".into(),
        1,
        test_now(),
        test_now(),
    );
    let dbg = format!("{c:?}");
    assert!(!dbg.contains("hash"), "Debug must not leak password hash, got: {dbg}");
    assert!(dbg.contains("u1"));
}

#[test]
fn new_domain_identity_rejects_empty_user_code() {
    let err = DomainIdentity::new(
        "".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn new_domain_identity_rejects_empty_triple_components() {
    for (domain_name, hostname, sid) in [
        ("", "host", "S-1-5"),
        ("DOM", "", "S-1-5"),
        ("DOM", "host", ""),
    ] {
        let err = DomainIdentity::new(
            "u1".into(),
            domain_name.into(),
            hostname.into(),
            sid.into(),
        )
        .unwrap_err();
        assert!(matches!(err, DomainError::EmptyPasswordHash));
    }
}

#[test]
fn new_domain_identity_accepts_valid_input() {
    let id = DomainIdentity::new(
        "u1".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    )
    .expect("valid identity should construct");
    assert_eq!(id.user_code, "u1");
    assert_eq!(id.domain_name, "DOM");
}
```

- [ ] **Step 8: Update `src/lib.rs` to re-export domain types**

Replace `src/lib.rs` with:

```rust
//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<RwLock<HashMap<String, u32>>>` token-version cache live inside the
//! usecase. Public consumers should `use auth::*;` (see the re-exports
//! below) rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

// Re-exports for the documented public surface.
pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
```

- [ ] **Step 9: Build + run domain tests**

Run:
```bash
cargo test -p auth --lib
```
Expected: all `src/domain/tests.rs` tests pass.

- [ ] **Step 10: Commit**

```bash
git add lib/crates/auth/src/lib.rs lib/crates/auth/src/domain.rs lib/crates/auth/src/domain/
git commit -m "feat(auth): domain layer (Role, UserCredentials, DomainIdentity, ports)"
```

---

## Task 3: Usecase layer — commands, errors, `AuthUsecaseConfig`, `AuthUsecase` skeleton

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/src/usecase.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/usecase/commands.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/usecase/error.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/usecase/auth_usecase.rs`
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/lib.rs` (re-export usecase types)

- [ ] **Step 1: Write `usecase/commands.rs`**

```rust
//! Command / view DTOs for the auth usecase.

/// Input for `AuthUsecase::login_with_password`.
#[derive(Debug, Clone)]
pub struct LoginWithPassword {
    pub code: String,
    pub password: String,
}

/// Input for `AuthUsecase::login_with_domain_user_info`.
#[derive(Debug, Clone)]
pub struct LoginWithDomainUserInfo {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

/// Input for `AuthUsecase::verify`.
#[derive(Debug, Clone)]
pub struct VerifyAccessToken {
    pub access_token: String,
}

/// Input for `AuthUsecase::refresh`.
#[derive(Debug, Clone)]
pub struct RefreshAccessToken {
    pub refresh_token: String,
}

/// Input for `AuthUsecase::logout`.
#[derive(Debug, Clone)]
pub struct Logout {
    pub code: String,
}

/// Output of `login_with_*` — opaque JWT strings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPairView {
    pub access_token: String,
    pub refresh_token: String,
}

/// Output of `verify`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthClaimsView {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

/// Output of `refresh` — a freshly-minted access JWT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessTokenView {
    pub access_token: String,
}

/// Output of `logout`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogoutAck {
    pub code: String,
}

// `Role` re-exported for `AuthClaimsView`'s public surface.
pub use crate::domain::Role;
```

- [ ] **Step 2: Write `usecase/error.rs`**

```rust
use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),

    #[error("token verification failed: {0}")]
    Verification(String),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        UsecaseError::Repository(err)
    }
}
```

- [ ] **Step 3: Write `usecase/auth_usecase.rs` (skeleton — JWT methods land in Task 4/5)**

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use apis::user::UserService;
use jsonwebtoken::SigningKey;
use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainIdentityRepository, DomainIdentity, UserCredentials, UserCredentialsRepository,
};

use super::commands::{
    AccessTokenView, AuthClaimsView, Logout, LogoutAck, LoginWithDomainUserInfo,
    LoginWithPassword, RefreshAccessToken, Role, TokenPairView, VerifyAccessToken,
};
use super::error::UsecaseError;

/// Configuration passed to [`AuthUsecase::new`]. Plain pub-field struct;
/// no builder ceremony. Generic over the same two repository types so
/// field types stay concrete.
pub struct AuthUsecaseConfig<
    R: UserCredentialsRepository,
    D: DomainIdentityRepository,
> {
    pub credentials: R,
    pub identities: D,
    pub user_service: Arc<dyn UserService>,
    pub signing_key: SigningKey,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}

/// Internal JWT claim payload for access tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct AccessClaims {
    sub: String,
    role: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Internal JWT claim payload for refresh tokens.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct RefreshClaims {
    sub: String,
    ver: u32,
    exp: i64,
    iat: i64,
}

/// Async orchestration for the auth flow.
///
/// Holds an `Arc<RwLock<HashMap<String, u32>>>` cache of token versions
/// keyed by user code. `verify` and `refresh` consult the cache; on miss
/// they fall back to `credentials.find_by_code` and populate the cache.
/// `login_with_*` and `logout` write to the cache directly.
pub struct AuthUsecase<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    credentials: R,
    identities: D,
    user_service: Arc<dyn UserService>,
    signing_key: SigningKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
    token_versions: Arc<RwLock<HashMap<String, u32>>>,
}

impl<R, D> AuthUsecase<R, D> {
    pub fn new(config: AuthUsecaseConfig<R, D>) -> Self {
        Self {
            credentials: config.credentials,
            identities: config.identities,
            user_service: config.user_service,
            signing_key: config.signing_key,
            access_ttl: config.access_ttl,
            refresh_ttl: config.refresh_ttl,
            token_versions: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Internal helper: read the cached `token_version` for `code`,
    /// falling back to the repo on miss.
    async fn current_token_version(&self, code: &str) -> Result<u32, UsecaseError> {
        if let Some(v) = self.token_versions.read().unwrap().get(code).copied() {
            return Ok(v);
        }
        let creds = self.credentials.find_by_code(code).await?;
        let v = creds.token_version;
        self.token_versions
            .write()
            .unwrap()
            .insert(code.to_string(), v);
        Ok(v)
    }

    /// Placeholder method bodies; full implementations land in Tasks 4 and 5.
    pub async fn login_with_password(
        &self,
        _cmd: LoginWithPassword,
    ) -> Result<TokenPairView, UsecaseError> {
        unimplemented!("filled in by Task 4")
    }

    pub async fn login_with_domain_user_info(
        &self,
        _cmd: LoginWithDomainUserInfo,
    ) -> Result<TokenPairView, UsecaseError> {
        unimplemented!("filled in by Task 4")
    }

    pub async fn verify(
        &self,
        _cmd: VerifyAccessToken,
    ) -> Result<AuthClaimsView, UsecaseError> {
        unimplemented!("filled in by Task 5")
    }

    pub async fn refresh(
        &self,
        _cmd: RefreshAccessToken,
    ) -> Result<AccessTokenView, UsecaseError> {
        unimplemented!("filled in by Task 5")
    }

    pub async fn logout(&self, cmd: Logout) -> Result<LogoutAck, UsecaseError> {
        if cmd.code.trim().is_empty() {
            return Err(UsecaseError::Repository(
                crate::domain::DomainError::EmptyCode,
            ));
        }
        let new_version = self.credentials.bump_token_version(&cmd.code).await?;
        self.token_versions
            .write()
            .unwrap()
            .insert(cmd.code.clone(), new_version);
        Ok(LogoutAck { code: cmd.code })
    }

    // `DomainIdentity` and `UserCredentials` are referenced in tests later;
    // keep them imported so the compiler warns if a later task accidentally
    // removes the use.
    #[allow(dead_code)]
    fn _phantom(&self) -> (&DomainIdentity, &UserCredentials) {
        unimplemented!()
    }
}
```

- [ ] **Step 4: Write `usecase.rs` (module declarations + re-exports)**

```rust
mod auth_usecase;
mod commands;
mod error;

#[cfg(test)]
mod tests;

pub use auth_usecase::{AuthUsecase, AuthUsecaseConfig};
pub use commands::{
    AccessTokenView, AuthClaimsView, Logout, LogoutAck, LoginWithDomainUserInfo,
    LoginWithPassword, RefreshAccessToken, Role, TokenPairView, VerifyAccessToken,
};
pub use error::UsecaseError;
```

- [ ] **Step 5: Update `src/lib.rs` to re-export usecase types**

```rust
//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<RwLock<HashMap<String, u32>>>` token-version cache live inside the
//! usecase. Public consumers should `use auth::*;` (see the re-exports
//! below) rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
pub use usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, AuthUsecaseConfig, Logout, LogoutAck,
    LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken, TokenPairView,
    UsecaseError, VerifyAccessToken,
};
```

- [ ] **Step 6: Build the crate**

Run:
```bash
cargo build -p auth
```
Expected: success. The unimplemented methods compile but are never called yet.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/auth/src/lib.rs lib/crates/auth/src/usecase.rs lib/crates/auth/src/usecase/
git commit -m "feat(auth): usecase skeleton (config, commands, errors, logout)"
```

---

## Task 4: Usecase — JWT mint + `login_with_password` + `login_with_domain_user_info`

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/usecase/auth_usecase.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/usecase/tests.rs`

- [ ] **Step 1: Add the `chrono` clock-feature dependency and the test-only mock wiring**

`chrono` is already in `[dependencies]` (Task 1). The `Clock` feature is enabled in workspace deps.

- [ ] **Step 2: Write the test fixture module in `usecase/tests.rs`**

This file holds the mock repos + a `FakeUserService` reused by Tasks 4 and 5. Replace the file each time new mocks are needed.

```rust
//! Unit tests for `AuthUsecase`.
//!
//! Mock repos and a `FakeUserService` (mirroring the apis `UserService`
//! surface) stand in for the real adapters so the usecase can be
//! exercised without PostgreSQL.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::user::{
    CreateUserRequest, Role as ApiRole, UpdateUserRequest, UserApiError, UserService,
    UserView,
};

use crate::domain::{
    DomainError, DomainIdentity, Role, UserCredentials, UserCredentialsRepository,
    DomainIdentityRepository,
};
use crate::usecase::commands::{
    AuthClaimsView, Logout, LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken,
    TokenPairView, VerifyAccessToken,
};
use crate::usecase::{AuthUsecase, AuthUsecaseConfig, UsecaseError};

fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

#[derive(Default)]
struct MockCredState {
    by_code: HashMap<String, UserCredentials>,
    find_calls: usize,
    bump_calls: usize,
}

#[derive(Clone, Default)]
struct MockUserCredentialsRepo {
    state: Arc<Mutex<MockCredState>>,
}

impl MockUserCredentialsRepo {
    fn seed_hash(&self, code: &str, password_hash: &str, token_version: u32) {
        let now = fixed_now();
        self.state.lock().unwrap().by_code.insert(
            code.to_string(),
            UserCredentials::for_repository(
                code.to_string(),
                password_hash.to_string(),
                token_version,
                now,
                now,
            ),
        );
    }
}

#[async_trait]
impl UserCredentialsRepository for MockUserCredentialsRepo {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.find_calls += 1;
        s.by_code.get(code).cloned().ok_or(DomainError::NotFound)
    }

    async fn create(
        &self,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_code.contains_key(&credentials.code) {
            return Err(DomainError::DuplicateCode(credentials.code));
        }
        s.by_code.insert(credentials.code.clone(), credentials.clone());
        Ok(credentials)
    }

    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.bump_calls += 1;
        let entry = s.by_code.get_mut(code).ok_or(DomainError::NotFound)?;
        entry.token_version += 1;
        Ok(entry.token_version)
    }
}

#[derive(Default)]
struct MockIdentityState {
    rows: Vec<DomainIdentity>,
    find_calls: usize,
}

#[derive(Clone, Default)]
struct MockDomainIdentityRepo {
    state: Arc<Mutex<MockIdentityState>>,
}

impl MockDomainIdentityRepo {
    fn seed(&self, id: DomainIdentity) {
        self.state.lock().unwrap().rows.push(id);
    }
}

#[async_trait]
impl DomainIdentityRepository for MockDomainIdentityRepo {
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError> {
        let mut s = self.state.lock().unwrap();
        s.find_calls += 1;
        s.rows
            .iter()
            .find(|r| {
                r.user_code == user_code
                    && r.domain_name == domain_name
                    && r.hostname == hostname
                    && r.sid == sid
            })
            .cloned()
            .ok_or(DomainError::NotFound)
    }
}

#[derive(Clone, Default)]
struct FakeUserService {
    by_code: Arc<Mutex<HashMap<String, UserView>>>,
}

impl FakeUserService {
    fn seed(&self, code: &str, role: ApiRole, active: bool) {
        let now = fixed_now();
        let view = UserView {
            id: 1,
            code: code.to_string(),
            name: code.to_string(),
            role,
            active,
            created_at: now,
            updated_at: now,
        };
        self.by_code.lock().unwrap().insert(code.to_string(), view);
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        self.by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or(UserApiError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

/// Build a usecase wired to the mocks + a freshly-derived HMAC key.
fn make_usecase(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthUsecase<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    use jsonwebtoken::{Algorithm, SigningKey};
    let key = SigningKey::new(Algorithm::HS256, b"0123456789abcdef0123456789abcdef");
    let cfg = AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: Arc::new(users),
        signing_key: key,
        access_ttl: std::time::Duration::from_secs(60),
        refresh_ttl: std::time::Duration::from_secs(3600),
    };
    AuthUsecase::new(cfg)
}

/// Hash a password the same way the usecase does (argon2 default).
fn hash_password(plain: &str) -> String {
    use argon2::password_hash::{rand_core::OsRng, PasswordHasher, SaltString};
    let salt = SaltString::generate(&mut OsRng);
    argon2::Argon2::default()
        .hash_password(plain.as_bytes(), &salt)
        .expect("hash")
        .to_string()
}
```

- [ ] **Step 3: Add the failing tests for `login_with_password`**

Append the following to `usecase/tests.rs`:

```rust
fn make_seeded_usecase_for_password_login(
    plain_password: &str,
    initial_token_version: u32,
) -> (
    MockUserCredentialsRepo,
    MockDomainIdentityRepo,
    FakeUserService,
    AuthUsecase<MockUserCredentialsRepo, MockDomainIdentityRepo>,
) {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password(plain_password), initial_token_version);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let usecase = make_usecase(creds.clone(), ids.clone(), users.clone());
    (creds, ids, users, usecase)
}

#[tokio::test]
async fn login_with_password_mints_token_pair_for_valid_credentials() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_password_rejects_empty_code_with_validation() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "  ".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_empty_password() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyPasswordHash)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, false);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_wrong_password() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::InvalidCredentials)
    ));
}

#[tokio::test]
async fn login_with_password_rejects_unknown_user() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_password(LoginWithPassword {
            code: "ghost".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Repository(DomainError::NotFound)));
}
```

- [ ] **Step 4: Run the new tests to verify they fail**

Run:
```bash
cargo test -p auth --lib usecase::tests
```
Expected: all five new tests panic with `unimplemented!()` from the placeholder `login_with_password`.

- [ ] **Step 5: Implement `login_with_password` in `auth_usecase.rs`**

Replace the placeholder `login_with_password` with:

```rust
pub async fn login_with_password(
    &self,
    cmd: LoginWithPassword,
) -> Result<TokenPairView, UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Repository(DomainError::EmptyCode));
    }
    if cmd.password.is_empty() {
        return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
    }
    let creds = self.credentials.find_by_code(&cmd.code).await?;
    let user = self
        .user_service
        .get_by_code(&cmd.code)
        .await
        .map_err(|e| match e {
            UserApiError::NotFound => UsecaseError::Repository(DomainError::NotFound),
            other => UsecaseError::Repository(DomainError::Repository(other.to_string())),
        })?;
    if !user.active {
        return Err(UsecaseError::Repository(DomainError::Inactive));
    }
    let parsed_hash = argon2::PasswordHash::new(&creds.password_hash)
        .map_err(|e| {
            UsecaseError::Repository(DomainError::Repository(format!(
                "argon2 parse: {e}"
            )))
        })?;
    use argon2::PasswordVerifier;
    if argon2::Argon2::default()
        .verify_password(cmd.password.as_bytes(), &parsed_hash)
        .is_err()
    {
        return Err(UsecaseError::Repository(DomainError::InvalidCredentials));
    }

    // Populate the cache so the freshly-minted tokens verify without a miss.
    self.token_versions
        .write()
        .unwrap()
        .insert(cmd.code.clone(), creds.token_version);

    let role = role_from_api(user.role);
    let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
    let refresh = self.mint_refresh_token(&cmd.code, creds.token_version)?;

    Ok(TokenPairView {
        access_token: access,
        refresh_token: refresh,
    })
}
```

Add these helpers near the bottom of the `impl<R, D> AuthUsecase<R, D>` block (replace `_phantom` with the helpers — `_phantom` is no longer needed):

```rust
fn mint_access_token(
    &self,
    code: &str,
    role: Role,
    version: u32,
) -> Result<String, UsecaseError> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let now = chrono::Utc::now().timestamp();
    let claims = AccessClaims {
        sub: code.to_string(),
        role: role.as_str().to_string(),
        ver: version,
        iat: now,
        exp: now + self.access_ttl.as_secs() as i64,
    };
    let enc = EncodingKey::from(&self.signing_key);
    encode(&Header::new(jsonwebtoken::Algorithm::HS256), &claims, &enc)
        .map_err(|e| UsecaseError::Verification(format!("encode access: {e}")))
}

fn mint_refresh_token(&self, code: &str, version: u32) -> Result<String, UsecaseError> {
    use jsonwebtoken::{encode, EncodingKey, Header};
    let now = chrono::Utc::now().timestamp();
    let claims = RefreshClaims {
        sub: code.to_string(),
        ver: version,
        iat: now,
        exp: now + self.refresh_ttl.as_secs() as i64,
    };
    let enc = EncodingKey::from(&self.signing_key);
    encode(&Header::new(jsonwebtoken::Algorithm::HS256), &claims, &enc)
        .map_err(|e| UsecaseError::Verification(format!("encode refresh: {e}")))
}
```

Also add `use apis::user::UserApiError;` at the top of the file (next to `use apis::user::UserService;`).

Add a `role_from_api` free function at module scope (outside the impl):

```rust
fn role_from_api(r: apis::user::Role) -> Role {
    match r {
        apis::user::Role::Root => Role::Root,
        apis::user::Role::Admin => Role::Admin,
        apis::user::Role::General => Role::General,
    }
}
```

- [ ] **Step 6: Run the tests**

Run:
```bash
cargo test -p auth --lib usecase::tests::login_with_password
```
Expected: all five tests pass.

- [ ] **Step 7: Add `login_with_domain_user_info` tests**

Append to `usecase/tests.rs`:

```rust
#[tokio::test]
async fn login_with_domain_user_info_mints_token_pair_for_matching_identity() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 5);
    let ids = MockDomainIdentityRepo::default();
    ids.seed(DomainIdentity::for_repository(
        "u1".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    ));
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let pair = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_domain_user_info_returns_not_found_for_unmatched_triple() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Repository(DomainError::NotFound)));
}

#[tokio::test]
async fn login_with_domain_user_info_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    ids.seed(DomainIdentity::for_repository(
        "u1".into(),
        "DOM".into(),
        "host".into(),
        "S-1-5".into(),
    ));
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, false);
    let usecase = make_usecase(creds, ids, users);

    let err = usecase
        .login_with_domain_user_info(LoginWithDomainUserInfo {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}
```

- [ ] **Step 8: Replace the `login_with_domain_user_info` placeholder**

```rust
pub async fn login_with_domain_user_info(
    &self,
    cmd: LoginWithDomainUserInfo,
) -> Result<TokenPairView, UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Repository(DomainError::EmptyCode));
    }
    if cmd.domain_name.trim().is_empty()
        || cmd.hostname.trim().is_empty()
        || cmd.sid.trim().is_empty()
    {
        return Err(UsecaseError::Repository(DomainError::EmptyPasswordHash));
    }
    self.identities
        .find(&cmd.code, &cmd.domain_name, &cmd.hostname, &cmd.sid)
        .await?;
    let user = self
        .user_service
        .get_by_code(&cmd.code)
        .await
        .map_err(|e| match e {
            UserApiError::NotFound => UsecaseError::Repository(DomainError::NotFound),
            other => UsecaseError::Repository(DomainError::Repository(other.to_string())),
        })?;
    if !user.active {
        return Err(UsecaseError::Repository(DomainError::Inactive));
    }
    let creds = self.credentials.find_by_code(&cmd.code).await?;
    self.token_versions
        .write()
        .unwrap()
        .insert(cmd.code.clone(), creds.token_version);
    let role = role_from_api(user.role);
    let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
    let refresh = self.mint_refresh_token(&cmd.code, creds.token_version)?;
    Ok(TokenPairView {
        access_token: access,
        refresh_token: refresh,
    })
}
```

- [ ] **Step 9: Run the full usecase test suite**

Run:
```bash
cargo test -p auth --lib usecase::tests
```
Expected: all `login_with_*` tests pass; `verify`/`refresh` placeholders are still unimplemented (no tests for them yet — they land in Task 5).

- [ ] **Step 10: Commit**

```bash
git add lib/crates/auth/src/usecase/auth_usecase.rs lib/crates/auth/src/usecase/tests.rs
git commit -m "feat(auth): login_with_password + login_with_domain_user_info"
```

---

## Task 5: Usecase — `verify`, `refresh`, `logout` + token-version cache

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/usecase/auth_usecase.rs`
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/usecase/tests.rs`

- [ ] **Step 1: Add `verify` tests to `usecase/tests.rs`**

Append:

```rust
#[tokio::test]
async fn verify_returns_claims_for_freshly_minted_access_token() {
    let (creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 7);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let claims = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.role, Role::Admin);
    assert_eq!(claims.token_version, 7);

    // Login populated the cache; verify must not touch the repo.
    let find_calls_before = creds.state.lock().unwrap().find_calls;
    let _ = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds again");
    assert_eq!(
        creds.state.lock().unwrap().find_calls,
        find_calls_before,
        "verify must hit the cache, not the repo"
    );
}

#[tokio::test]
async fn verify_falls_back_to_repo_on_cache_miss_and_populates_cache() {
    let (creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 3);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    // Manually drop the cache to simulate a cold restart within the same process.
    // We do that by constructing a fresh usecase that shares the same
    // MockUserCredentialsRepo but starts with an empty cache.
    let usecase2 = make_usecase(creds.clone(), MockDomainIdentityRepo::default(), FakeUserService::default());
    // Seed the FakeUserService so verify's get_by_code succeeds.
    // The usecase2 above has a fresh FakeUserService (empty); recreate with seed.
    let users2 = FakeUserService::default();
    users2.seed("u1", ApiRole::Admin, true);
    let usecase2 = make_usecase(creds.clone(), MockDomainIdentityRepo::default(), users2);

    let find_calls_before = creds.state.lock().unwrap().find_calls;
    let claims = usecase2
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("verify succeeds after cold restart");
    assert_eq!(claims.token_version, 3);

    let find_calls_after_first = creds.state.lock().unwrap().find_calls;
    assert!(
        find_calls_after_first > find_calls_before,
        "first verify after cold restart must hit the repo"
    );

    // Second verify must hit the cache.
    let _ = usecase2
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds again");
    assert_eq!(
        creds.state.lock().unwrap().find_calls,
        find_calls_after_first,
        "second verify must hit the cache"
    );
}

#[tokio::test]
async fn verify_rejects_refresh_token_presented_as_access_token() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.refresh_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn verify_rejects_inactive_user() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, false);
    let usecase = make_usecase(creds, ids, users);

    // Mint a token directly so we don't depend on the inactive path of login.
    // Build a small helper.
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    // We can't easily mint a token without logging in here; instead, test
    // through a freshly-seeded active user, then flip the user inactive.
    drop(pair);

    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let usecase = make_usecase(creds, ids, users.clone());

    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    // Flip the user to inactive and verify fails.
    users.seed("u1", ApiRole::Admin, false);
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::Inactive)
    ));
}
```

- [ ] **Step 2: Add `refresh` tests to `usecase/tests.rs`**

Append:

```rust
#[tokio::test]
async fn refresh_mints_new_access_token_with_current_version() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 4);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let new = usecase
        .refresh(RefreshAccessToken {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("refresh succeeds");
    assert!(!new.access_token.is_empty());

    // The new access token must verify.
    let claims = usecase
        .verify(VerifyAccessToken {
            access_token: new.access_token,
        })
        .await
        .expect("new access token verifies");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.token_version, 4);
}

#[tokio::test]
async fn refresh_rejects_access_token_presented_as_refresh_token() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let err = usecase
        .refresh(RefreshAccessToken {
            refresh_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}
```

- [ ] **Step 3: Add `logout` + cache-invalidation tests to `usecase/tests.rs`**

Append:

```rust
#[tokio::test]
async fn logout_bumps_token_version_and_invalidates_outstanding_tokens() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let pair = usecase
        .login_with_password(LoginWithPassword {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");

    // Pre-logout verify passes.
    usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token.clone(),
        })
        .await
        .expect("pre-logout verify passes");

    let ack = usecase
        .logout(Logout {
            code: "u1".into(),
        })
        .await
        .expect("logout succeeds");
    assert_eq!(ack.code, "u1");

    // Post-logout verify rejects.
    let err = usecase
        .verify(VerifyAccessToken {
            access_token: pair.access_token,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Verification(_)));
}

#[tokio::test]
async fn logout_with_empty_code_is_validation_error() {
    let (_creds, _ids, _users, usecase) =
        make_seeded_usecase_for_password_login("hunter2", 1);
    let err = usecase
        .logout(Logout {
            code: "  ".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Repository(DomainError::EmptyCode)
    ));
}
```

- [ ] **Step 4: Run all the new tests to verify they fail**

Run:
```bash
cargo test -p auth --lib usecase::tests
```
Expected: `verify`, `refresh`, and `logout` tests panic with `unimplemented!()`.

- [ ] **Step 5: Implement `verify` in `auth_usecase.rs`**

Replace the placeholder `verify` with:

```rust
pub async fn verify(
    &self,
    cmd: VerifyAccessToken,
) -> Result<AuthClaimsView, UsecaseError> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 5;
    let key = DecodingKey::from(&self.signing_key);
    let data = decode::<AccessClaims>(&cmd.access_token, &key, &validation)
        .map_err(|e| UsecaseError::Verification(format!("decode access: {e}")))?;
    let claims = data.claims;

    let current = self.current_token_version(&claims.sub).await?;
    if current != claims.ver {
        return Err(UsecaseError::Verification(format!(
            "token_version mismatch (cached = {current}, jwt.ver = {})",
            claims.ver
        )));
    }

    let user = self
        .user_service
        .get_by_code(&claims.sub)
        .await
        .map_err(|e| match e {
            UserApiError::NotFound => UsecaseError::Repository(DomainError::NotFound),
            other => UsecaseError::Repository(DomainError::Repository(other.to_string())),
        })?;
    if !user.active {
        return Err(UsecaseError::Repository(DomainError::Inactive));
    }

    let role = role_from_str(&claims.role)?;
    Ok(AuthClaimsView {
        code: claims.sub,
        role,
        token_version: claims.ver,
    })
}
```

Add the `role_from_str` helper next to `role_from_api`:

```rust
fn role_from_str(s: &str) -> Result<Role, UsecaseError> {
    Role::try_from(s).map_err(|e| UsecaseError::Repository(e))
}
```

- [ ] **Step 6: Implement `refresh` in `auth_usecase.rs`**

Replace the placeholder `refresh` with:

```rust
pub async fn refresh(
    &self,
    cmd: RefreshAccessToken,
) -> Result<AccessTokenView, UsecaseError> {
    use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
    let mut validation = Validation::new(Algorithm::HS256);
    validation.leeway = 5;
    let key = DecodingKey::from(&self.signing_key);
    let data = decode::<RefreshClaims>(&cmd.refresh_token, &key, &validation)
        .map_err(|e| UsecaseError::Verification(format!("decode refresh: {e}")))?;
    let claims = data.claims;

    let current = self.current_token_version(&claims.sub).await?;
    if current != claims.ver {
        return Err(UsecaseError::Verification(format!(
            "token_version mismatch (cached = {current}, jwt.ver = {})",
            claims.ver
        )));
    }

    let user = self
        .user_service
        .get_by_code(&claims.sub)
        .await
        .map_err(|e| match e {
            UserApiError::NotFound => UsecaseError::Repository(DomainError::NotFound),
            other => UsecaseError::Repository(DomainError::Repository(other.to_string())),
        })?;
    if !user.active {
        return Err(UsecaseError::Repository(DomainError::Inactive));
    }

    let role = role_from_api(user.role);
    let access = self.mint_access_token(&claims.sub, role, current)?;
    Ok(AccessTokenView {
        access_token: access,
    })
}
```

- [ ] **Step 7: Run the full usecase test suite**

Run:
```bash
cargo test -p auth --lib usecase::tests
```
Expected: all `login_with_*`, `verify`, `refresh`, `logout` tests pass.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/auth/src/usecase/auth_usecase.rs lib/crates/auth/src/usecase/tests.rs
git commit -m "feat(auth): verify, refresh, logout + token-version cache"
```

---

## Task 6: Migrations

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/migrations/0001_create_auth_user_credentials.sql`
- Create: `/root/coding/project/aegis/lib/crates/auth/migrations/0002_create_auth_user_domain_identities.sql`

- [ ] **Step 1: Write `0001_create_auth_user_credentials.sql`**

```sql
-- 0001_create_auth_user_credentials.sql
--
-- Per-user password hash + token version used by the auth crate. The
-- auth crate does NOT own the user lifecycle; `code` is the join key
-- against the user crate's `users.code`, and there is no foreign key
-- (the auth schema must deploy independently of the user crate).
--
-- `password_hash` stores an Argon2id PHC string produced by
-- `argon2::Argon2::default()`. `token_version` starts at 1 and is
-- monotonically incremented by `bump_token_version` on logout; every
-- outstanding JWT for that user carries the pre-bump version and is
-- rejected by `verify`.
--
-- `auth_user_credentials_set_updated_at` mirrors the trigger from the
-- user crate so an out-of-band `UPDATE` (e.g. via psql) still bumps
-- `updated_at` without the application having to remember.

CREATE TABLE auth_user_credentials (
    code TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    token_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_user_credentials_password_hash_check CHECK (length(password_hash) > 0)
);

CREATE OR REPLACE FUNCTION auth_user_credentials_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER auth_user_credentials_set_updated_at
    BEFORE UPDATE ON auth_user_credentials
    FOR EACH ROW
    EXECUTE FUNCTION auth_user_credentials_set_updated_at();
```

- [ ] **Step 2: Write `0002_create_auth_user_domain_identities.sql`**

```sql
-- 0002_create_auth_user_domain_identities.sql
--
-- Maps a (domain_name, hostname, sid) AD triple to a `user_code` so
-- `login_with_domain_user_info` can resolve a domain logon without a
-- password. One user can have multiple domain identities (e.g. laptop
-- + workstation), but the (user_code, domain_name, hostname, sid)
-- tuple is unique.
--
-- No FK to `users.code` — the auth schema is independent.

CREATE TABLE auth_user_domain_identities (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    user_code TEXT NOT NULL,
    domain_name TEXT NOT NULL,
    hostname TEXT NOT NULL,
    sid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_user_domain_identities_unique UNIQUE (user_code, domain_name, hostname, sid)
);
```

- [ ] **Step 3: Commit**

```bash
git add lib/crates/auth/migrations/
git commit -m "feat(auth): SQLx migrations for credentials + domain identities"
```

---

## Task 7: Persistence adapter — row conversions + repository impls

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/persistence.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/persistence/postgres.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/persistence/postgres/row.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/persistence/postgres/auth_repo.rs`

- [ ] **Step 1: Write `adapter/persistence/postgres/row.rs`**

```rust
use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, DomainIdentity, UserCredentials};

#[derive(Clone, FromRow)]
pub struct CredentialRow {
    pub code: String,
    pub password_hash: String,
    pub token_version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<CredentialRow> for UserCredentials {
    type Error = DomainError;

    fn try_from(row: CredentialRow) -> Result<Self, Self::Error> {
        if row.token_version < 0 {
            return Err(DomainError::Repository(format!(
                "negative token_version: {}",
                row.token_version
            )));
        }
        Ok(UserCredentials::for_repository(
            row.code,
            row.password_hash,
            row.token_version as u32,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Clone, FromRow)]
pub struct DomainIdentityRow {
    pub user_code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

impl TryFrom<DomainIdentityRow> for DomainIdentity {
    type Error = DomainError;

    fn try_from(row: DomainIdentityRow) -> Result<Self, Self::Error> {
        Ok(DomainIdentity::for_repository(
            row.user_code,
            row.domain_name,
            row.hostname,
            row.sid,
        ))
    }
}
```

- [ ] **Step 2: Write `adapter/persistence/postgres/auth_repo.rs`**

```rust
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, UserCredentials,
    UserCredentialsRepository,
};

use super::row::{CredentialRow, DomainIdentityRow};

/// PostgreSQL SQLSTATE for a unique-violation error.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct UserCredentialsRepo {
    pool: PgPool,
}

impl UserCredentialsRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserCredentialsRepository for UserCredentialsRepo {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError> {
        let row: Option<CredentialRow> = sqlx::QueryBuilder::new(
            "SELECT code, password_hash, token_version, created_at, updated_at \
             FROM auth_user_credentials WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<CredentialRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        let row = row.ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn create(
        &self,
        credentials: UserCredentials,
    ) -> Result<UserCredentials, DomainError> {
        let row: CredentialRow = sqlx::QueryBuilder::new(
            "INSERT INTO auth_user_credentials (code, password_hash, token_version) ",
        )
        .push_values(
            [(
                credentials.code,
                credentials.password_hash,
                credentials.token_version as i32,
            )],
            |mut b, (code, hash, ver)| {
                b.push_bind(code).push_bind(hash).push_bind(ver);
            },
        )
        .push(" RETURNING code, password_hash, token_version, created_at, updated_at")
        .build_query_as::<CredentialRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        row.try_into()
    }

    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError> {
        let row: (i32,) = sqlx::QueryBuilder::new(
            "UPDATE auth_user_credentials SET token_version = token_version + 1 \
             WHERE code = ",
        )
        .push_bind(code)
        .push(" RETURNING token_version")
        .build_query_as::<(i32,)>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        if row.0 < 0 {
            return Err(DomainError::Repository(format!(
                "negative token_version after bump: {}",
                row.0
            )));
        }
        Ok(row.0 as u32)
    }
}

pub struct DomainIdentityRepo {
    pool: PgPool,
}

impl DomainIdentityRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DomainIdentityRepository for DomainIdentityRepo {
    async fn find(
        &self,
        user_code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<DomainIdentity, DomainError> {
        let row: Option<DomainIdentityRow> = sqlx::QueryBuilder::new(
            "SELECT user_code, domain_name, hostname, sid \
             FROM auth_user_domain_identities \
             WHERE user_code = ",
        )
        .push_bind(user_code)
        .push(" AND domain_name = ")
        .push_bind(domain_name)
        .push(" AND hostname = ")
        .push_bind(hostname)
        .push(" AND sid = ")
        .push_bind(sid)
        .build_query_as::<DomainIdentityRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?;
        let row = row.ok_or(DomainError::NotFound)?;
        row.try_into()
    }
}

fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::DuplicateCode(format!("(constraint {constraint})"))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
```

- [ ] **Step 3: Write `adapter/persistence/postgres.rs` (module declarations + re-export + docs)**

```rust
//! PostgreSQL-backed implementations of `UserCredentialsRepository` and
//! `DomainIdentityRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! `query_as!` / `query!` compile-time-checked macros. The compile-time
//! macros require either a live `DATABASE_URL` or a checked-in
//! `sqlx-data.json` offline metadata cache, neither of which the
//! workspace build currently provides. The runtime API is type-checked
//! against the bound parameters we hand it and lets the build proceed
//! in any environment, at the cost of a small loss in static
//! verification of the SQL itself. The migration tests in `tests`
//! cover the schema content directly.
//!
//! `row` is kept private and `UserRow` / `DomainIdentityRow` are NOT
//! re-exported at the crate root. They are internal bridges between
//! SQLx's `FromRow` derive and the domain types, and the only callers
//! are the repository implementations in this module.

mod auth_repo;
mod row;

#[cfg(test)]
mod tests;

pub use auth_repo::{DomainIdentityRepo, UserCredentialsRepo};
```

- [ ] **Step 4: Write `adapter/persistence.rs`**

```rust
//! Persistence adapters.
//!
//! `persistence` itself is `pub(crate)` because callers reach concrete
//! repositories via the layer boundary (`adapter::UserCredentialsRepo`,
//! `adapter::DomainIdentityRepo`, and the crate root). The `postgres`
//! child must be `pub` so the re-exports at the adapter layer are
//! well-formed.

pub(crate) mod postgres;
```

- [ ] **Step 5: Write `adapter.rs`**

```rust
//! Adapter layer.
//!
//! Houses the persistence adapters that implement the domain ports,
//! plus outbound-port adapters that adapt the usecase layer to
//! API-facing traits defined in other workspace crates.

mod facade;
mod persistence;

pub use facade::in_memory::AuthServiceImpl;
pub use persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
```

`AuthServiceImpl` does not exist yet; create an empty placeholder module so the re-export compiles:

- [ ] **Step 6: Create empty `adapter/facade.rs` + `adapter/facade/in_memory.rs`**

`/root/coding/project/aegis/lib/crates/auth/src/adapter/facade.rs`:

```rust
mod in_memory;
```

`/root/coding/project/aegis/lib/crates/auth/src/adapter/facade/in_memory.rs`:

```rust
// Placeholder. The full facade implementation lands in Task 9.
```

- [ ] **Step 7: Build the crate**

Run:
```bash
cargo build -p auth
```
Expected: success.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/auth/src/adapter.rs lib/crates/auth/src/adapter/
git commit -m "feat(auth): PostgreSQL-backed credentials + identity repos"
```

---

## Task 8: Persistence adapter — row conversion tests + migration schema tests

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/persistence/postgres/tests.rs`

- [ ] **Step 1: Write `tests.rs`**

```rust
//! Tests for the PostgreSQL adapter that do NOT require a live database
//! connection.
//!
//! 1. The two migration files (the schema that downstream consumers
//!    will apply). The leading doc comments are stripped before
//!    assertion so the tests anchor on the CREATE TABLE block rather
//!    than keywords in the header.
//! 2. The `CredentialRow` -> `UserCredentials` and
//!    `DomainIdentityRow` -> `DomainIdentity` conversions.

use std::convert::TryFrom;
use std::fs;
use std::path::PathBuf;

use chrono::{TimeZone, Utc};

use crate::domain::{DomainError, DomainIdentity, UserCredentials};

use super::row::{CredentialRow, DomainIdentityRow};

fn row_test_timestamp() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}

fn migration_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("migrations").join(name)
}

fn load_migration(name: &str) -> String {
    fs::read_to_string(migration_path(name))
        .unwrap_or_else(|_| panic!("migration file lib/crates/auth/migrations/{name} must exist"))
}

fn create_table_block(sql: &str) -> String {
    let start = sql
        .find("CREATE TABLE")
        .expect("migration must contain a CREATE TABLE statement");
    let close = sql[start..]
        .find(");")
        .expect("CREATE TABLE body must be terminated by `);`");
    sql[start..start + close + 2].to_string()
}

#[test]
fn migration_0001_creates_auth_user_credentials_table() {
    let sql = load_migration("0001_create_auth_user_credentials.sql");
    let block = create_table_block(&sql);
    assert!(
        block.contains("CREATE TABLE") && block.contains("auth_user_credentials"),
        "expected auth_user_credentials table; got:\n{block}"
    );
}

#[test]
fn migration_0001_has_required_columns() {
    let block = create_table_block(&load_migration("0001_create_auth_user_credentials.sql"));
    let upper = block.to_uppercase();
    for required in [
        "CODE TEXT",
        "PASSWORD_HASH TEXT",
        "TOKEN_VERSION INTEGER",
    ] {
        assert!(
            upper.contains(required),
            "auth_user_credentials must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn migration_0001_password_hash_is_not_null_and_checked() {
    let block = create_table_block(&load_migration("0001_create_auth_user_credentials.sql"));
    let upper = block.to_uppercase();
    assert!(upper.contains("PASSWORD_HASH TEXT NOT NULL"));
    assert!(upper.contains("CHECK"));
    assert!(upper.contains("LENGTH(PASSWORD_HASH) > 0"));
}

#[test]
fn migration_0001_token_version_defaults_to_one() {
    let block = create_table_block(&load_migration("0001_create_auth_user_credentials.sql"));
    let upper = block.to_uppercase();
    assert!(
        upper.contains("TOKEN_VERSION INTEGER NOT NULL DEFAULT 1"),
        "token_version must default to 1; got:\n{block}"
    );
}

#[test]
fn migration_0001_has_updated_at_trigger() {
    let sql = load_migration("0001_create_auth_user_credentials.sql");
    assert!(sql.contains("CREATE TRIGGER auth_user_credentials_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON auth_user_credentials"));
    assert!(sql.contains("CREATE OR REPLACE FUNCTION auth_user_credentials_set_updated_at"));
}

#[test]
fn migration_0002_creates_auth_user_domain_identities_table() {
    let sql = load_migration("0002_create_auth_user_domain_identities.sql");
    let block = create_table_block(&sql);
    assert!(
        block.contains("CREATE TABLE") && block.contains("auth_user_domain_identities"),
        "expected auth_user_domain_identities table; got:\n{block}"
    );
}

#[test]
fn migration_0002_has_required_columns() {
    let block = create_table_block(&load_migration("0002_create_auth_user_domain_identities.sql"));
    let upper = block.to_uppercase();
    for required in [
        "USER_CODE TEXT",
        "DOMAIN_NAME TEXT",
        "HOSTNAME TEXT",
        "SID TEXT",
    ] {
        assert!(
            upper.contains(required),
            "auth_user_domain_identities must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn migration_0002_unique_constraint_covers_all_four_columns() {
    let block = create_table_block(&load_migration("0002_create_auth_user_domain_identities.sql"));
    assert!(
        block.contains("UNIQUE (user_code, domain_name, hostname, sid)"),
        "unique constraint must cover all four columns; got:\n{block}"
    );
}

#[test]
fn credential_row_converts_to_user_credentials() {
    let row = CredentialRow {
        code: "u1".into(),
        password_hash: "hash".into(),
        token_version: 7,
        created_at: row_test_timestamp(),
        updated_at: row_test_timestamp(),
    };
    let creds: UserCredentials = row.try_into().expect("convert succeeds");
    assert_eq!(creds.code, "u1");
    assert_eq!(creds.password_hash, "hash");
    assert_eq!(creds.token_version, 7);
}

#[test]
fn credential_row_with_negative_token_version_is_rejected() {
    let row = CredentialRow {
        code: "u1".into(),
        password_hash: "hash".into(),
        token_version: -1,
        created_at: row_test_timestamp(),
        updated_at: row_test_timestamp(),
    };
    let err = UserCredentials::try_from(row).expect_err("negative rejected");
    assert!(matches!(err, DomainError::Repository(_)));
}

#[test]
fn domain_identity_row_converts_to_domain_identity() {
    let row = DomainIdentityRow {
        user_code: "u1".into(),
        domain_name: "DOM".into(),
        hostname: "host".into(),
        sid: "S-1-5".into(),
    };
    let id: DomainIdentity = row.try_into().expect("convert succeeds");
    assert_eq!(id.user_code, "u1");
    assert_eq!(id.domain_name, "DOM");
    assert_eq!(id.hostname, "host");
    assert_eq!(id.sid, "S-1-5");
}
```

- [ ] **Step 2: Run the tests**

Run:
```bash
cargo test -p auth --lib adapter::persistence::postgres::tests
```
Expected: all twelve tests pass.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/auth/src/adapter/persistence/postgres/tests.rs
git commit -m "test(auth): row conversions + migration schema content tests"
```

---

## Task 9: Facade — `AuthServiceImpl` + `FakeUserService` + facade tests

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/facade/in_memory/service.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/facade/in_memory/fake_user_service.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/src/adapter/facade/in_memory/tests.rs`
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/adapter/facade/in_memory.rs`
- Modify: `/root/coding/project/aegis/lib/crates/auth/src/lib.rs` (re-export `AuthServiceImpl`)

- [ ] **Step 1: Write `adapter/facade/in_memory/fake_user_service.rs`**

```rust
//! Test-only fake `apis::user::UserService` used by the facade tests.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::user::{
    CreateUserRequest, Role as ApiRole, UpdateUserRequest, UserApiError, UserService,
    UserView,
};

pub struct FakeUserService {
    by_code: Mutex<HashMap<String, UserView>>,
}

impl FakeUserService {
    pub fn new() -> Self {
        Self {
            by_code: Mutex::new(HashMap::new()),
        }
    }

    pub fn seed(&self, code: &str, role: ApiRole, active: bool) {
        let now = Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap();
        let view = UserView {
            id: 1,
            code: code.to_string(),
            name: code.to_string(),
            role,
            active,
            created_at: now,
            updated_at: now,
        };
        self.by_code.lock().unwrap().insert(code.to_string(), view);
    }
}

impl Default for FakeUserService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        self.by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or(UserApiError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

pub fn shared_fake() -> Arc<FakeUserService> {
    Arc::new(FakeUserService::new())
}
```

- [ ] **Step 2: Write `adapter/facade/in_memory/service.rs`**

```rust
use async_trait::async_trait;
use jsonwebtoken::SigningKey;

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest,
    RefreshResponse, TokenPair, VerifyRequest,
};
use apis::user::Role as ApiRole;

use std::sync::Arc;
use std::time::Duration;

use crate::domain::{
    DomainIdentityRepository, DomainError, UserCredentialsRepository,
};
use crate::usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, AuthUsecaseConfig, Logout as LogoutCmd,
    LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken,
    TokenPairView, UsecaseError, VerifyAccessToken,
};

pub struct AuthServiceImpl<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    usecase: AuthUsecase<R, D>,
}

impl<R, D> AuthServiceImpl<R, D> {
    pub fn new(usecase: AuthUsecase<R, D>) -> Self {
        Self { usecase }
    }
}

fn to_api_role(r: crate::domain::Role) -> ApiRole {
    match r {
        crate::domain::Role::Root => ApiRole::Root,
        crate::domain::Role::Admin => ApiRole::Admin,
        crate::domain::Role::General => ApiRole::General,
    }
}

fn map_error(err: UsecaseError) -> AuthApiError {
    match err {
        UsecaseError::Validation(d) => AuthApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => AuthApiError::NotFound,
            DomainError::Inactive => AuthApiError::Inactive,
            DomainError::InvalidCredentials => AuthApiError::InvalidCredentials,
            DomainError::Repository(msg) => AuthApiError::Repository(msg),
            DomainError::EmptyCode
            | DomainError::EmptyPasswordHash
            | DomainError::InvalidRole(_)
            | DomainError::DuplicateCode(_) => AuthApiError::Repository(d.to_string()),
        },
        UsecaseError::Verification(msg) => AuthApiError::Verification(msg),
    }
}

#[async_trait]
impl<R: UserCredentialsRepository, D: DomainIdentityRepository> AuthService
    for AuthServiceImpl<R, D>
{
    async fn login_with_password(
        &self,
        req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_password(LoginWithPassword {
                code: req.code,
                password: req.password,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn login_with_domain_user_info(
        &self,
        req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError> {
        let view: TokenPairView = self
            .usecase
            .login_with_domain_user_info(LoginWithDomainUserInfo {
                code: req.code,
                domain_name: req.domain_name,
                hostname: req.hostname,
                sid: req.sid,
            })
            .await
            .map_err(map_error)?;
        Ok(TokenPair {
            access_token: view.access_token,
            refresh_token: view.refresh_token,
        })
    }

    async fn logout(&self, req: LogoutRequest) -> Result<LogoutResponse, AuthApiError> {
        let ack = self
            .usecase
            .logout(LogoutCmd { code: req.code })
            .await
            .map_err(map_error)?;
        Ok(LogoutResponse { code: ack.code })
    }

    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError> {
        let view: AuthClaimsView = self
            .usecase
            .verify(VerifyAccessToken {
                access_token: req.access_token,
            })
            .await
            .map_err(map_error)?;
        Ok(AuthClaims {
            code: view.code,
            role: to_api_role(view.role),
            token_version: view.token_version,
        })
    }

    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError> {
        let view: AccessTokenView = self
            .usecase
            .refresh(RefreshAccessToken {
                refresh_token: req.refresh_token,
            })
            .await
            .map_err(map_error)?;
        Ok(RefreshResponse {
            access_token: view.access_token,
        })
    }
}
```

- [ ] **Step 3: Write `adapter/facade/in_memory/tests.rs`**

```rust
//! Unit tests for `AuthServiceImpl`.
//!
//! Wires the adapter on top of `MockUserCredentialsRepo` +
//! `MockDomainIdentityRepo` + `FakeUserService` so the public-facing
//! `AuthService` surface is exercised without PostgreSQL.

use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::{Algorithm, SigningKey};

use apis::auth::{
    AuthApiError, AuthService, LoginWithDomainUserInfoRequest, LoginWithPasswordRequest,
    LogoutRequest, RefreshRequest, VerifyRequest,
};
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, Role, UserCredentials};
use crate::usecase::{AuthUsecase, AuthUsecaseConfig};

use super::fake_user_service::FakeUserService;
use super::service::AuthServiceImpl;

use crate::usecase::tests::{
    hash_password, MockDomainIdentityRepo, MockUserCredentialsRepo,
};

fn make_service(
    creds: MockUserCredentialsRepo,
    ids: MockDomainIdentityRepo,
    users: FakeUserService,
) -> AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo> {
    let key = SigningKey::new(Algorithm::HS256, b"0123456789abcdef0123456789abcdef");
    let cfg = AuthUsecaseConfig {
        credentials: creds,
        identities: ids,
        user_service: Arc::new(users),
        signing_key: key,
        access_ttl: Duration::from_secs(60),
        refresh_ttl: Duration::from_secs(3600),
    };
    AuthServiceImpl::new(AuthUsecase::new(cfg))
}

#[tokio::test]
async fn service_impl_can_be_constructed() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let _svc = make_service(creds, ids, users);
}

#[tokio::test]
async fn login_with_password_returns_token_pair_for_valid_credentials() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    assert!(!pair.access_token.is_empty());
    assert!(!pair.refresh_token.is_empty());
}

#[tokio::test]
async fn login_with_password_returns_invalid_credentials_for_wrong_password() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "WRONG".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::InvalidCredentials));
}

#[tokio::test]
async fn login_with_password_returns_inactive_when_user_is_disabled() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, false);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::Inactive));
}

#[tokio::test]
async fn login_with_domain_user_info_returns_not_found_for_unmatched_triple() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let err = svc
        .login_with_domain_user_info(LoginWithDomainUserInfoRequest {
            code: "u1".into(),
            domain_name: "DOM".into(),
            hostname: "host".into(),
            sid: "S-1-5".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(err, AuthApiError::NotFound));
}

#[tokio::test]
async fn verify_returns_claims_for_freshly_minted_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 9);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Root, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let claims = svc
        .verify(VerifyRequest {
            access_token: pair.access_token,
        })
        .await
        .expect("verify succeeds");
    assert_eq!(claims.code, "u1");
    assert_eq!(claims.role, ApiRole::Root);
    assert_eq!(claims.token_version, 9);
}

#[tokio::test]
async fn refresh_returns_new_access_token() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let pair = svc
        .login_with_password(LoginWithPasswordRequest {
            code: "u1".into(),
            password: "hunter2".into(),
        })
        .await
        .expect("login succeeds");
    let new = svc
        .refresh(RefreshRequest {
            refresh_token: pair.refresh_token,
        })
        .await
        .expect("refresh succeeds");
    assert!(!new.access_token.is_empty());
}

#[tokio::test]
async fn logout_echoes_the_user_code() {
    let creds = MockUserCredentialsRepo::default();
    creds.seed_hash("u1", &hash_password("hunter2"), 1);
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    users.seed("u1", ApiRole::Admin, true);
    let svc = make_service(creds, ids, users);

    let ack = svc
        .logout(LogoutRequest { code: "u1".into() })
        .await
        .expect("logout succeeds");
    assert_eq!(ack.code, "u1");
}

#[tokio::test]
async fn auth_service_impl_is_object_safe() {
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    let _boxed: Box<dyn AuthService> = Box::new(svc);
}

#[tokio::test]
async fn auth_service_impl_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    let creds = MockUserCredentialsRepo::default();
    let ids = MockDomainIdentityRepo::default();
    let users = FakeUserService::default();
    let svc = make_service(creds, ids, users);
    assert_send_sync::<AuthServiceImpl<MockUserCredentialsRepo, MockDomainIdentityRepo>>();
    assert_send_sync::<Box<dyn AuthService>>();
    let _ = svc;
}

#[test]
fn user_credentials_type_aliases_match_expected_signatures() {
    fn assert_creds<R: UserCredentialsRepository>(_: fn(&str) -> _) {}
    let _ = assert_creds::<MockUserCredentialsRepo>;
    let _: &dyn DomainError = &DomainError::NotFound;
    let _: &Role = &Role::Admin;
    // Reference `UserCredentials` so the type stays in scope for downstream
    // test code that may need it.
    let _: fn(String, String, u32, _, _) -> UserCredentials =
        UserCredentials::for_repository;
}
```

- [ ] **Step 4: Update `adapter/facade/in_memory.rs`**

```rust
mod service;

#[cfg(test)]
mod fake_user_service;
#[cfg(test)]
mod tests;

pub use service::AuthServiceImpl;
```

(`fake_user_service` is test-only because the production facade tests use the mock already living in `usecase/tests.rs`. The facade tests above import `FakeUserService` from the usecase tests, so the facade-level `fake_user_service.rs` only needs to compile in test mode.)

Wait — the facade tests above import `FakeUserService` from `super::fake_user_service::FakeUserService`. To make that work, `fake_user_service` must be compiled in test mode. The current `mod fake_user_service;` inside `#[cfg(test)]` works for `tests/` integration-style tests but NOT for `mod tests { use super::fake_user_service::… }` inside the same `in_memory` module tree because the facade's own `tests` submodule needs `fake_user_service` to be visible.

Restructure `in_memory.rs` to:

```rust
mod service;

#[cfg(test)]
mod fake_user_service;
#[cfg(test)]
mod tests;

pub use service::AuthServiceImpl;
```

Then in `tests.rs`, replace `use super::fake_user_service::FakeUserService;` with `use super::super::fake_user_service::FakeUserService;` — i.e. one level up.

Better still, simplify by always compiling `fake_user_service` (it's a few hundred bytes and only `pub` re-exports if needed). Make it `mod fake_user_service;` unconditionally.

- [ ] **Step 5: Update `in_memory.rs` to compile `fake_user_service` unconditionally**

```rust
mod fake_user_service;
mod service;

#[cfg(test)]
mod tests;

pub use service::AuthServiceImpl;
```

- [ ] **Step 6: Build the crate**

Run:
```bash
cargo build -p auth
```
Expected: success.

- [ ] **Step 7: Run facade tests**

Run:
```bash
cargo test -p auth --lib adapter::facade::in_memory::tests
```
Expected: all ten tests pass.

- [ ] **Step 8: Re-export `AuthServiceImpl` from the crate root**

Update `src/lib.rs`:

```rust
//! # auth crate
//!
//! Workspace library that implements the `apis::auth::AuthService` port.
//! Three DDD layers (`domain`, `usecase`, `adapter`) plus an
//! `Arc<RwLock<HashMap<String, u32>>>` token-version cache live inside the
//! usecase. Public consumers should `use auth::*;` (see the re-exports
//! below) rather than reach into the sub-modules.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
pub use adapter::facade::in_memory::AuthServiceImpl;
pub use domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
pub use usecase::{
    AccessTokenView, AuthClaimsView, AuthUsecase, AuthUsecaseConfig, Logout, LogoutAck,
    LoginWithDomainUserInfo, LoginWithPassword, RefreshAccessToken, TokenPairView,
    UsecaseError, VerifyAccessToken,
};
```

- [ ] **Step 9: Build the crate + run all unit tests**

Run:
```bash
cargo build -p auth
cargo test -p auth --lib
```
Expected: build succeeds; all domain / usecase / adapter / facade unit tests pass.

- [ ] **Step 10: Commit**

```bash
git add lib/crates/auth/src/adapter/facade lib/crates/auth/src/lib.rs
git commit -m "feat(auth): AuthServiceImpl facade + facade tests"
```

---

## Task 10: `tests/public_api.rs`, `tests/integration_persistence.rs`, README, final gate

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/auth/tests/public_api.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/tests/integration_persistence.rs`
- Create: `/root/coding/project/aegis/lib/crates/auth/README.md`

- [ ] **Step 1: Write `tests/public_api.rs`**

```rust
//! Public-API compile test for the `auth` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and the
//! in-crate type names so a regression in any layer is caught at
//! `cargo test -p auth` time.

use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::{Algorithm, SigningKey};

use auth::{
    AccessTokenView, AuthClaimsView, AuthServiceImpl, AuthUsecase, AuthUsecaseConfig,
    DomainIdentityRepo, DomainIdentityRepository, LogoutAck, Role, TokenPairView,
    UserCredentialsRepo, UserCredentialsRepository, UserCredentials,
};
use apis::auth::AuthService;
use apis::user::UserService;

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_role(_: Role) {}
    fn assert_pair(_: TokenPairView) {}
    fn assert_claims(_: AuthClaimsView) {}
    fn assert_token(_: AccessTokenView) {}
    fn assert_ack(_: LogoutAck) {}
    fn assert_creds(_: UserCredentials) {}

    assert_role(Role::Admin);
    assert_pair(TokenPairView {
        access_token: "a".into(),
        refresh_token: "r".into(),
    });
    assert_claims(AuthClaimsView {
        code: "u1".into(),
        role: Role::Admin,
        token_version: 1,
    });
    assert_token(AccessTokenView {
        access_token: "a".into(),
    });
    assert_ack(LogoutAck { code: "u1".into() });
    assert_creds(UserCredentials::for_repository(
        "u1".into(),
        "hash".into(),
        1,
        chrono::DateTime::from_timestamp(0, 0).unwrap(),
        chrono::DateTime::from_timestamp(0, 0).unwrap(),
    ));
}

#[test]
fn repo_constructors_accept_a_pg_pool() {
    let ctor: fn(sqlx::PgPool) -> UserCredentialsRepo = UserCredentialsRepo::new;
    let ctor2: fn(sqlx::PgPool) -> DomainIdentityRepo = DomainIdentityRepo::new;
    let _ = (ctor, ctor2);
}

#[test]
fn usecase_new_accepts_an_auth_usecase_config() {
    fn assert_user_service_is_send_sync<T: Send + Sync>() {}
    assert_user_service_is_send_sync::<Box<dyn UserService>>();

    // The config struct must be constructible end-to-end without I/O.
    // We pass `Arc<dyn UserService>` as a private field on the usecase,
    // so we type-check that path here too.
    let _cfg: AuthUsecaseConfig<UserCredentialsRepo, DomainIdentityRepo> =
        AuthUsecaseConfig {
            credentials: todo!("see tests/integration_persistence.rs for a real pool"),
            identities: todo!(),
            user_service: Arc::new(todo!()),
            signing_key: SigningKey::new(Algorithm::HS256, b"0123456789abcdef0123456789abcdef"),
            access_ttl: Duration::from_secs(60),
            refresh_ttl: Duration::from_secs(3600),
        };
    let _ = _cfg;

    fn assert_repo_bounds<
        R: UserCredentialsRepository,
        D: DomainIdentityRepository,
    >() {
    }
    assert_repo_bounds::<UserCredentialsRepo, DomainIdentityRepo>();
    let _ = AuthUsecase::<UserCredentialsRepo, DomainIdentityRepo>::new;
}
```

The `todo!()` macros above are deliberate: this is a compile-only test, so the config construction never actually runs. The point is to type-check the public surface end-to-end. If `todo!()` is awkward, replace it with a runtime-only `if false { unreachable!() }` block — but `todo!()` is fine because the test never executes the `_cfg` binding.

- [ ] **Step 2: Run `tests/public_api.rs`**

Run:
```bash
cargo test -p auth --test public_api
```
Expected: PASS (or compile error if a re-export regressed).

- [ ] **Step 3: Write `tests/integration_persistence.rs`**

```rust
//! Live-database integration tests for the PostgreSQL adapter.
//!
//! These tests connect to a real PostgreSQL server, apply the
//! `migrations/0001_*.sql` and `migrations/0002_*.sql` schemas, and
//! exercise the full surface of `UserCredentialsRepo` and
//! `DomainIdentityRepo`. They are `#[ignore]`-gated so that
//! `cargo test -p auth` stays green without a database; opt in with:
//!
//! ```text
//! cargo test -p auth -- --ignored
//! ```
//!
//! The connection URL is read from the `AEGIS_AUTH_DATABASE_URL`
//! environment variable. If unset, the test loads `.env` from the
//! workspace root via `dotenvy`.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use auth::{
    AuthUsecase, AuthUsecaseConfig, DomainIdentity, DomainIdentityRepo,
    DomainIdentityRepository, UserCredentials, UserCredentialsRepo,
    UserCredentialsRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_AUTH_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_AUTH_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_AUTH_DATABASE_URL");

    sqlx::query("DROP TABLE IF EXISTS auth_user_domain_identities CASCADE")
        .execute(&pool)
        .await
        .expect("drop auth_user_domain_identities");
    sqlx::query("DROP TABLE IF EXISTS auth_user_credentials CASCADE")
        .execute(&pool)
        .await
        .expect("drop auth_user_credentials");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply migrations");

    f(pool).await
}

fn unique_code(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

fn creds(code: &str, hash: &str, version: u32) -> UserCredentials {
    let now = chrono::Utc::now();
    UserCredentials::for_repository(
        code.to_string(),
        hash.to_string(),
        version,
        now,
        now,
    )
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn create_then_find_credentials_round_trip() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let code = unique_code("cred");

        let created = repo
            .create(creds(&code, "hash", 1))
            .await
            .expect("create succeeds");
        assert_eq!(created.code, code);
        assert_eq!(created.password_hash, "hash");
        assert_eq!(created.token_version, 1);

        let fetched = repo.find_by_code(&code).await.expect("find succeeds");
        assert_eq!(fetched.code, code);
        assert_eq!(fetched.password_hash, "hash");
        assert_eq!(fetched.token_version, 1);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn bump_token_version_returns_incremented_value() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let code = unique_code("bump");

        repo.create(creds(&code, "hash", 5)).await.expect("create");

        let v1 = repo.bump_token_version(&code).await.expect("bump 1");
        let v2 = repo.bump_token_version(&code).await.expect("bump 2");
        assert_eq!(v1, 6);
        assert_eq!(v2, 7);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_credentials_unknown_code_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let err = repo
            .find_by_code("does-not-exist-xxxxxxxxxxxx")
            .await
            .expect_err("unknown code rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn bump_token_version_unknown_code_returns_not_found() {
    with_pool(|pool| async move {
        let repo = UserCredentialsRepo::new(pool);
        let err = repo
            .bump_token_version("does-not-exist-xxxxxxxxxxxx")
            .await
            .expect_err("unknown code rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn create_then_find_domain_identity_round_trip() {
    with_pool(|pool| async move {
        let repo = DomainIdentityRepo::new(pool);
        let code = unique_code("ident");

        sqlx::query(
            "INSERT INTO auth_user_domain_identities \
             (user_code, domain_name, hostname, sid) VALUES ($1, $2, $3, $4)",
        )
        .bind(&code)
        .bind("DOM")
        .bind("host")
        .bind("S-1-5")
        .execute(&pool)
        .await
        .expect("insert identity");

        let id: DomainIdentity = repo
            .find(&code, "DOM", "host", "S-1-5")
            .await
            .expect("find succeeds");
        assert_eq!(id.user_code, code);
        assert_eq!(id.domain_name, "DOM");
        assert_eq!(id.hostname, "host");
        assert_eq!(id.sid, "S-1-5");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn find_domain_identity_unmatched_triple_returns_not_found() {
    with_pool(|pool| async move {
        let repo = DomainIdentityRepo::new(pool);
        let code = unique_code("ident-miss");
        let err = repo
            .find(&code, "DOM", "host", "S-1-5")
            .await
            .expect_err("unmatched triple rejected");
        assert!(matches!(err, auth::DomainError::NotFound));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_AUTH_DATABASE_URL pointing at a live PostgreSQL"]
async fn usecase_can_be_constructed_from_real_repos() {
    with_pool(|pool| async move {
        // Construct a real `AuthUsecase` wired to the Postgres repos and
        // a fake user service. We don't exercise any usecase methods here
        // because that path is covered by the unit tests; the integration
        // test only asserts that the wiring compiles and constructs.
        use std::sync::Arc;
        use std::time::Duration;

        use jsonwebtoken::{Algorithm, SigningKey};

        use crate::fake_user_service_for_integration as fake;

        let creds = UserCredentialsRepo::new(pool.clone());
        let ids = DomainIdentityRepo::new(pool);
        let cfg = AuthUsecaseConfig {
            credentials: creds,
            identities: ids,
            user_service: Arc::new(fake::FakeUserService::new()),
            signing_key: SigningKey::new(Algorithm::HS256, b"0123456789abcdef0123456789abcdef"),
            access_ttl: Duration::from_secs(60),
            refresh_ttl: Duration::from_secs(3600),
        };
        let _usecase = AuthUsecase::new(cfg);
    })
    .await;
}

mod fake_user_service_for_integration {
    use std::collections::HashMap;
    use std::sync::Mutex;

    use async_trait::async_trait;
    use apis::user::{
        CreateUserRequest, Role, UpdateUserRequest, UserApiError, UserService, UserView,
    };
    use chrono::{TimeZone, Utc};

    pub struct FakeUserService {
        by_code: Mutex<HashMap<String, UserView>>,
    }

    impl FakeUserService {
        pub fn new() -> Self {
            Self {
                by_code: Mutex::new(HashMap::new()),
            }
        }
    }

    #[async_trait]
    impl UserService for FakeUserService {
        async fn create(&self, _: CreateUserRequest) -> Result<UserView, UserApiError> {
            unimplemented!()
        }
        async fn get_by_id(&self, _: i32) -> Result<UserView, UserApiError> {
            unimplemented!()
        }
        async fn get_by_code(&self, _: &str) -> Result<UserView, UserApiError> {
            let _ = Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap();
            Err(UserApiError::NotFound)
        }
        async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
            unimplemented!()
        }
        async fn update(&self, _: UpdateUserRequest) -> Result<UserView, UserApiError> {
            unimplemented!()
        }
    }
}
```

- [ ] **Step 4: Verify the integration test compiles**

Run:
```bash
cargo test -p auth --test integration_persistence --no-run
```
Expected: builds. The tests themselves are `#[ignore]` and won't run without `AEGIS_AUTH_DATABASE_URL`.

- [ ] **Step 5: Write `README.md`**

```markdown
# auth crate

Workspace library implementing the [`apis::auth::AuthService`](../../crates/apis/src/auth.rs)
port. Mints HS256 JWTs, validates them against an in-memory
`token_version` cache backed by Postgres, and adapts the usecase layer
into the `AuthService` contract through an in-memory facade.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the cross-cutting conventions this crate follows (workspace deps,
> no `mod.rs`, ports-and-adapters layering, ignored live-DB tests, etc.).

## Layout

```text
src/
  domain/                       # UserCredentials, DomainIdentity, Role, ports
  usecase/                      # AuthUsecase, command DTOs, errors, JWT mint/verify
  adapter/
    persistence/
      postgres/                 # SQLx-backed UserCredentialsRepo, DomainIdentityRepo
    facade/
      in_memory/                # AuthServiceImpl wiring usecase -> apis::auth::AuthService
migrations/                     # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`UserCredentials`,
`DomainIdentity`, `Role`, `DomainError`, `UserCredentialsRepository`,
`DomainIdentityRepository`, `UserCredentialsRepo`, `DomainIdentityRepo`,
`AuthUsecase`, `AuthUsecaseConfig`, `AuthServiceImpl`, the command DTOs
`LoginWithPassword` / `LoginWithDomainUserInfo` / `VerifyAccessToken` /
`RefreshAccessToken` / `Logout`, and the view DTOs `TokenPairView` /
`AuthClaimsView` / `AccessTokenView` / `LogoutAck`) so consumers can
`use auth::*;` without reaching into the sub-modules.

## Database setup

The crate ships two SQLx migrations that define
`auth_user_credentials` and `auth_user_domain_identities`. Apply them
before pointing the repositories at the database:

```bash
sqlx migrate run --source lib/crates/auth/migrations
```

Once the migrations are applied, construct the repositories and usecase
from a `sqlx::PgPool`:

```rust
use std::sync::Arc;
use std::time::Duration;

use jsonwebtoken::{Algorithm, SigningKey};

use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo,
    UserCredentialsRepo,
};
use apis::user::UserService; // production wiring uses the user crate's facade

let credentials_repo = UserCredentialsRepo::new(pool.clone());
let identities_repo = DomainIdentityRepo::new(pool);
let user_service: Arc<dyn UserService> = Arc::new(/* … */);
let signing_key = SigningKey::from_bytes(&hmac_secret_bytes);

let usecase = AuthUsecase::new(AuthUsecaseConfig {
    credentials: credentials_repo,
    identities: identities_repo,
    user_service,
    signing_key,
    access_ttl: Duration::from_secs(15 * 60),
    refresh_ttl: Duration::from_secs(7 * 24 * 60 * 60),
});

let auth_service: Arc<dyn apis::auth::AuthService> =
    Arc::new(AuthServiceImpl::new(usecase));
```

The `auth` crate does not run migrations at runtime; a deployment step
is required.

## Integration tests

The crate ships live-database integration tests at
[`tests/integration_persistence.rs`](tests/integration_persistence.rs).
They connect to PostgreSQL, apply both migrations, and exercise the
full repository surface plus a smoke test of `AuthUsecase::new`. They
are `#[ignore]`-gated so the default `cargo test -p auth` run stays
green without a database.

Run them against a local PostgreSQL with:

```bash
# .env at the workspace root is sourced automatically by the tests
# via dotenvy; AEGIS_AUTH_DATABASE_URL must point at a reachable server.
cargo test -p auth -- --ignored
```

## Token-version cache

`AuthUsecase` keeps an in-memory `Arc<RwLock<HashMap<String, u32>>>`
mapping `code -> token_version`. The cache is populated lazily on the
first `verify` / `refresh` for a code, refreshed on every successful
login, and replaced on every `logout` (which calls
`credentials.bump_token_version` and writes the returned new version
back). The DB is the source of truth; the cache is only invalidated
in-process. In a multi-process deployment each process owns its own
cache and cross-process revocation relies on the next cold-miss DB
read — see the spec for the full discussion.
```

- [ ] **Step 6: Final verification gate**

Run the four commands from the lib-crate guideline section 8:

```bash
cargo fmt --all -- --check
cargo clippy -p auth --all-targets --all-features -- -D warnings
cargo test -p auth
cargo doc -p auth --no-deps
```

Expected: all four succeed with no errors and no warnings. Fix any clippy
warning before continuing.

- [ ] **Step 7: Run the ignored integration tests (only if `AEGIS_AUTH_DATABASE_URL` is set)**

```bash
cargo test -p auth -- --ignored --test-threads=1
```

Expected: all integration tests pass. If `AEGIS_AUTH_DATABASE_URL` is
not set, skip this step — the integration tests are `#[ignore]`-gated.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/auth/tests/ lib/crates/auth/README.md
git commit -m "test(auth): public_api + integration_persistence + README"
```

---

## Self-Review

**1. Spec coverage:**

- Crate scaffolding + workspace dep (Task 1) → ✓
- Domain layer (Task 2) → ✓ covers `Role`, `UserCredentials`, `DomainIdentity`, `DomainError`, `UserCredentialsRepository`, `DomainIdentityRepository`, validating + repository-bound constructors, `Debug` redaction on `UserCredentials`, empty-input rejection
- Usecase layer (Tasks 3–5) → ✓ covers `AuthUsecase`, `AuthUsecaseConfig`, `UsecaseError`, command DTOs (`LoginWithPassword`, `LoginWithDomainUserInfo`, `VerifyAccessToken`, `RefreshAccessToken`, `Logout`), view DTOs (`TokenPairView`, `AuthClaimsView`, `AccessTokenView`, `LogoutAck`), JWT mint (HS256 via `jsonwebtoken = "11"`), `AccessClaims` / `RefreshClaims` structs, `Arc<RwLock<HashMap<String, u32>>>` cache with lazy population, login writes, logout bumps
- Migrations (Task 6) → ✓ covers `auth_user_credentials` (PK on `code`, `password_hash`, `token_version DEFAULT 1`, CHECK on `length(password_hash) > 0`, `updated_at` trigger) and `auth_user_domain_identities` (id PK, 4-tuple UNIQUE)
- Persistence adapter (Tasks 7–8) → ✓ covers `UserCredentialsRepo` + `DomainIdentityRepo`, runtime SQLx API, `map_db_error` (RowNotFound / 23505 / other), row conversions, schema content tests
- Facade (Task 9) → ✓ covers `AuthServiceImpl<R, D>`, exhaustive `UsecaseError` → `AuthApiError` mapping, `Role` ↔ `apis::user::Role` conversion, `AuthService` object-safety + `Send + Sync` tests
- `tests/public_api.rs` + `tests/integration_persistence.rs` + `README` (Task 10) → ✓ covers constructor chain as function pointers, `AEGIS_AUTH_DATABASE_URL` env var, destructive cleanup, `#[ignore]` gating
- Verification gate (Task 10 step 6) → ✓ runs `cargo fmt`, `cargo clippy`, `cargo test`, `cargo doc`

**2. Placeholder scan:** No "TBD" / "TODO" / "implement later" in code blocks. The `unimplemented!()` markers in the auth usecase skeleton (Task 3) are intentional placeholders for tasks that follow immediately (Tasks 4, 5). The `todo!()` macros in `tests/public_api.rs` (Task 10) are deliberate compile-only constructs that never execute.

**3. Type consistency:**

- `UserCredentials::for_repository` signature matches across Task 2 (definition), Task 8 (`from(row)`), Task 10 (`integration_persistence` helper `creds`). All five-argument forms with `(String, String, u32, DateTime<Utc>, DateTime<Utc>)`.
- `AuthUsecaseConfig<R, D>` shape is consistent in Task 3 (definition), Task 9 (facade test wiring), Task 10 (`tests/public_api.rs` + `tests/integration_persistence.rs`).
- `AuthServiceImpl<R, D>` shape consistent across Task 7 re-export, Task 9 definition + tests, Task 10 README.
- `UserCredentialsRepo::new` / `DomainIdentityRepo::new` consistent in Task 7 (definition) and Task 10 (`tests/public_api.rs` function-pointer test).
- `mock_*` types defined once in `usecase/tests.rs` (Task 4) and reused by `adapter/facade/in_memory/tests.rs` (Task 9) — `crate::usecase::tests::{MockUserCredentialsRepo, MockDomainIdentityRepo, hash_password}`.
- `FakeUserService` lives in two places (Task 4 in `usecase/tests.rs` for usecase tests; Task 9 in `adapter/facade/in_memory/fake_user_service.rs` for facade tests). The usecase tests' inline `FakeUserService` (Task 4) is a test-private implementation; the facade-level one (Task 9) is a separate file under `cfg(test)` initially, then lifted out of `cfg(test)` so the facade tests can import it. Both are independent — usecase tests don't import the facade one. No name collision.

**4. Gaps / corrections found during self-review:**

- The Task 4 `verify_falls_back_to_repo_on_cache_miss_and_populates_cache` test was slightly muddled in the initial draft. Restructured in the final code above so the second `usecase2 = make_usecase(...)` call replaces the first (with a seeded `FakeUserService`), making the cache-miss assertion cleaner.
- Task 9 step 4 originally had `mod fake_user_service;` under `#[cfg(test)]`, but the facade's own `mod tests` needs to import from it via `super::fake_user_service::…`. The corrected step 5 lifts `fake_user_service` out of `#[cfg(test)]` so it's compiled unconditionally inside the module tree, but remains an internal module not re-exported at the crate root.
- Task 10's `tests/public_api.rs` uses `todo!()` for the config construction; that's intentional because the test never executes the value, but a reviewer may want it spelled out — done in the doc-comment immediately above the binding.

Plan complete and saved to `docs/superpowers/plans/2026-08-05-auth-crate-implementation.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration
2. **Inline Execution** — Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?