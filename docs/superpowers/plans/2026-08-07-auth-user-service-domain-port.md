# Auth `UserService` Domain Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decouple the `auth` crate's `domain` and `usecase` layers from `apis::user::UserService` by introducing a new `crate::domain::UserService` port that exposes only `get_by_code`, and an `adapter::service::user::UserServiceImpl` that adapts the apis trait onto the domain port.

**Architecture:** Two new files in the auth crate (`domain/service.rs`, `adapter/service/user.rs`) plus minimal wiring changes (`domain.rs`, `adapter.rs`). Five existing files get mechanical edits: the trait imports flip from `apis::user::UserService` to `crate::domain::UserService`, the unit/integration test fakes are rewritten against the new trait, and two helper functions in `auth_usecase.rs` (`map_user_service_error`, `role_from_api`) are deleted because the new trait returns domain types directly.

**Tech Stack:** Rust 2024 edition, `async_trait` for trait objects, `Arc<dyn …>` for shared state, existing `thiserror` `DomainError` for error mapping. No new dependencies.

**Spec:** [docs/superpowers/specs/2026-08-07-auth-userservice-domain-port-design.md](../specs/2026-08-07-auth-userservice-domain-port-design.md)

## Global Constraints

- Auth crate uses ports-and-adapters DDD with three layers: `domain`, `usecase`, `adapter`. Each layer depends only on the one below it. After this refactor, `domain` and `usecase` MUST NOT import anything from `apis`.
- No `mod.rs`; each top-level module uses `src/<module>.rs` + `src/<module>/`. Terminal leaf modules are single files with no companion directory.
- All traits returned from public APIs that are held as `Arc<dyn …>` MUST be `Send + Sync`.
- Error mapping for `UserApiError → DomainError` is fixed: `NotFound → DomainError::NotFound`, all other variants → `DomainError::Repository(variant.to_string())`.
- Role mapping for `apis::user::Role → crate::domain::Role` is a 3-arm `match` (Root/Admin/General → Root/Admin/General). Use exactly the same body as the existing `role_from_api` in `auth_usecase.rs:387-393`.
- Commit messages follow the project's existing convention (`feat(auth):`, `refactor(auth):`, `test(auth):`).

---

### Task 1: Add the new domain port — `UserService` trait + `UserSummary` struct

**Files:**
- Create: `lib/crates/auth/src/domain/service.rs`
- Modify: `lib/crates/auth/src/domain.rs:1-17` (add `mod service;` + re-exports)

**Interfaces:**
- Produces (used in later tasks):
  - `crate::domain::UserSummary { code: String, active: bool, role: Role }` (derives `Debug, Clone, PartialEq, Eq`)
  - `crate::domain::UserService` trait with `async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>`, `Send + Sync`

- [ ] **Step 1: Create `lib/crates/auth/src/domain/service.rs`**

Write the new file:

```rust
//! Outbound port: look up a user's `active` state and `role` by code.
//!
//! `domain` defines this trait so the `usecase` layer never has to
//! reach into `apis::user::UserService` for these two facts. The
//! concrete adapter lives in
//! `crate::adapter::service::user::UserServiceImpl` and delegates
//! to `apis::user::UserService`.

use async_trait::async_trait;

use super::{DomainError, Role};

/// Minimal projection of a user — just the fields the auth usecase
/// needs to decide whether to mint tokens for `code`. The full user
/// record (id, name, timestamps, etc.) stays on the apis side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub code: String,
    pub active: bool,
    pub role: Role,
}

#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
}
```

- [ ] **Step 2: Wire the new module into `lib/crates/auth/src/domain.rs`**

Replace the entire current contents of `lib/crates/auth/src/domain.rs` with:

```rust
mod credentials;
mod domain_identity;
mod error;
mod repository;
mod role;
mod service;
mod token_version_cache;

#[cfg(test)]
mod tests;

pub use credentials::UserCredentials;
pub use domain_identity::DomainIdentity;
pub use error::DomainError;
pub use repository::{DomainIdentityRepository, UserCredentialsRepository};
pub use role::Role;
pub use service::{UserService, UserSummary};
pub use token_version_cache::TokenVersionCache;
```

- [ ] **Step 3: Verify the crate still compiles**

Run: `cargo build -p auth`
Expected: success. No behavior change yet — `UserService` / `UserSummary` are defined but no one calls them.

- [ ] **Step 4: Verify the existing tests still pass**

Run: `cargo test -p auth`
Expected: success. Same reason — nothing uses the new types yet.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/auth/src/domain/service.rs lib/crates/auth/src/domain.rs
git commit -m "feat(auth): add domain UserService port + UserSummary struct"
```

---

### Task 2: Add the new adapter — `UserServiceImpl` wrapping `apis::user::UserService`

**Files:**
- Create: `lib/crates/auth/src/adapter/service.rs`
- Create: `lib/crates/auth/src/adapter/service/user.rs`
- Modify: `lib/crates/auth/src/adapter.rs:7-13` (add `pub(crate) mod service;`)

**Interfaces:**
- Consumes:
  - `Arc<dyn apis::user::UserService>` (existing apis trait — do not modify)
  - `apis::user::Role`, `apis::user::UserApiError`
- Produces:
  - `crate::adapter::service::user::UserServiceImpl` with `pub fn new(inner: Arc<dyn apis::user::UserService>) -> Self` and impl of `crate::domain::UserService`

- [ ] **Step 1: Create `lib/crates/auth/src/adapter/service.rs`**

Write the new file:

```rust
pub(crate) mod user;
```

- [ ] **Step 2: Create `lib/crates/auth/src/adapter/service/user.rs`**

Write the new file:

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{Role as ApiRole, UserApiError, UserService as ApiUserService};
use crate::domain::{DomainError, Role, UserService, UserSummary};

/// Adapter that implements the domain `UserService` port on top of
/// the apis `UserService`. Delegates `get_by_code` to the inner
/// apis implementation and translates the apis types into the
/// domain equivalents.
pub struct UserServiceImpl {
    inner: Arc<dyn ApiUserService>,
}

impl UserServiceImpl {
    pub fn new(inner: Arc<dyn ApiUserService>) -> Self {
        Self { inner }
    }
}

fn map_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

fn map_error(err: UserApiError) -> DomainError {
    match err {
        UserApiError::NotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        let view = self.inner.get_by_code(code).await.map_err(map_error)?;
        Ok(UserSummary {
            code: view.code,
            active: view.active,
            role: map_role(view.role),
        })
    }
}
```

- [ ] **Step 3: Wire the new module into `lib/crates/auth/src/adapter.rs`**

Replace the entire current contents of `lib/crates/auth/src/adapter.rs` with:

```rust
//! Adapter layer.
//!
//! Houses the persistence adapters that implement the domain ports,
//! plus outbound-port adapters that adapt the usecase layer to
//! API-facing traits defined in other workspace crates.

pub(crate) mod cache;
pub(crate) mod facade;
pub(crate) mod persistence;
pub(crate) mod service;

pub use cache::in_memory::token_version::InMemoryTokenVersionCache;
pub use facade::in_memory::AuthServiceImpl;
pub use persistence::postgres::{DomainIdentityRepo, UserCredentialsRepo};
```

- [ ] **Step 4: Verify the crate still compiles and tests still pass**

Run: `cargo build -p auth && cargo test -p auth`
Expected: success. `UserServiceImpl` is defined but not yet wired into anything.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/auth/src/adapter/service.rs lib/crates/auth/src/adapter/service/user.rs lib/crates/auth/src/adapter.rs
git commit -m "feat(auth): add UserServiceImpl adapter over apis::user::UserService"
```

---

### Task 3: Migrate `auth_usecase.rs` to the new domain trait

**Files:**
- Modify: `lib/crates/auth/src/usecase/auth_usecase.rs:1-17` (imports), `:23-36` (`AuthUsecaseConfig`), `:72-80` (`AuthUsecase`), `:118-122, :166-170, :204-208, :239-243` (four call sites), `:380-393` (delete two helpers)

**Interfaces:**
- Consumes: `Arc<dyn crate::domain::UserService>` (from Task 1)
- Produces: `AuthUsecase` / `AuthUsecaseConfig` now hold `Arc<dyn crate::domain::UserService>` and consume `UserSummary` at call sites

- [ ] **Step 1: Update the imports at the top of `auth_usecase.rs`**

In `lib/crates/auth/src/usecase/auth_usecase.rs`, replace the current top block:

```rust
use std::sync::Arc;
use std::time::Duration;

use apis::user::{UserApiError, UserService};
use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, DomainIdentityRepository, TokenVersionCache, UserCredentials,
    UserCredentialsRepository,
};
```

with:

```rust
use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::domain::{
    DomainError, DomainIdentityRepository, TokenVersionCache, UserCredentials,
    UserCredentialsRepository, UserService,
};
```

- [ ] **Step 2: Update the `AuthUsecaseConfig.user_service` field type**

In the same file, in the `AuthUsecaseConfig` struct (around line 26), the field is currently:

```rust
    pub user_service: Arc<dyn UserService>,
```

It stays exactly that — the `UserService` it now refers to is the domain trait (via the import in Step 1). No change needed to the line itself; just confirm the line is present and unchanged. If your editor or the next refactor moved it, restore it to that exact form.

- [ ] **Step 3: Update the four call sites to use `summary` instead of `user` and drop `role_from_api`**

The four sites are at lines ~118-122 (in `login_with_password`), ~166-170 (in `login_with_domain_user_info`), ~204-208 (in `verify`), ~239-243 (in `refresh`). For each site, do the following two substitutions:

**Substitution A** — replace the binding and the active check. Current shape:

```rust
        let user = self
            .user_service
            .get_by_code(&cmd.code)
            .await
            .map_err(map_user_service_error)?;
        if !user.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }
```

becomes:

```rust
        let summary = self
            .user_service
            .get_by_code(&cmd.code)
            .await?;
        if !summary.active {
            return Err(UsecaseError::Repository(DomainError::Inactive));
        }
```

For the four sites, the `.map_err(map_user_service_error)` line is removed (the new trait returns `DomainError` which already converts via the `?` operator's `From<DomainError> for UsecaseError` impl in `usecase/error.rs`). The local `code` string passed to `get_by_code` varies per site (`&cmd.code`, `&claims.sub`) — preserve those exactly.

**Substitution B** — replace the role-from-api call. Current shape (example from `login_with_password`):

```rust
        let role = role_from_api(user.role);
        let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
```

becomes:

```rust
        let role = summary.role;
        let access = self.mint_access_token(&cmd.code, role, creds.token_version)?;
```

For `verify` (line ~213), the current shape is:

```rust
        let role = role_from_str(&claims.role)?;
```

That line is already taking the role from the JWT claim, NOT from the user service — leave it untouched. The role from `summary` is not used in `verify`'s happy path (only `summary.active` is checked).

For `refresh` (line ~248), the current shape is:

```rust
        let role = role_from_api(user.role);
        let access = self.mint_access_token(&claims.sub, role, current)?;
```

becomes:

```rust
        let role = summary.role;
        let access = self.mint_access_token(&claims.sub, role, current)?;
```

- [ ] **Step 4: Delete the two file-local helpers**

In the same file, delete the entire `map_user_service_error` function (current lines ~380-385) and the entire `role_from_api` function (current lines ~387-393). Both are now unused.

The file should end with just `role_from_str`, `creds_to_view`, and the doc-comments above them.

- [ ] **Step 5: Verify the crate builds**

Run: `cargo build -p auth 2>&1 | head -50`
Expected: build FAILS with errors in `src/usecase/tests.rs` and/or `tests/integration_persistence.rs` because the `FakeUserService` types there still implement `apis::user::UserService` instead of `crate::domain::UserService`. This is expected — Task 4 fixes it. Do NOT proceed past this step if the failure is in `auth_usecase.rs` itself.

- [ ] **Step 6: Commit the usecase migration**

```bash
git add lib/crates/auth/src/usecase/auth_usecase.rs
git commit -m "refactor(auth): migrate auth_usecase to domain UserService port"
```

---

### Task 4: Rewrite `FakeUserService` in `src/usecase/tests.rs` against the new domain trait

**Files:**
- Modify: `lib/crates/auth/src/usecase/tests.rs:7-25` (imports), `:146-189` (entire `FakeUserService` struct + impl + `seed` helper)

**Interfaces:**
- Consumes: `crate::domain::{Role, UserService, UserSummary}`, `crate::domain::UserService` trait
- Produces: `FakeUserService` that implements `crate::domain::UserService` and stores `HashMap<String, UserSummary>`. The `seed(&self, code: &str, role: Role, active: bool)` signature stays identical.

- [ ] **Step 1: Update the imports at the top of `src/usecase/tests.rs`**

Replace the current top of the file (lines 1-25):

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
    CreateUserRequest, Role as ApiRole, UpdateUserRequest, UserApiError, UserService, UserView,
};

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository,
};
use crate::usecase::commands::{
    AuthClaimsView, LoginWithDomainUserInfo, LoginWithPassword, Logout, RefreshAccessToken,
    TokenPairView, VerifyAccessToken,
};
use crate::usecase::{AuthUsecase, AuthUsecaseConfig, UsecaseError};
```

with:

```rust
//! Unit tests for `AuthUsecase`.
//!
//! Mock repos and a `FakeUserService` (implementing the domain
//! `UserService` port) stand in for the real adapters so the usecase
//! can be exercised without PostgreSQL.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, DomainIdentity, DomainIdentityRepository, Role, UserCredentials,
    UserCredentialsRepository, UserService, UserSummary,
};
use crate::usecase::commands::{
    AuthClaimsView, LoginWithDomainUserInfo, LoginWithPassword, Logout, RefreshAccessToken,
    TokenPairView, VerifyAccessToken,
};
use crate::usecase::{AuthUsecase, AuthUsecaseConfig, UsecaseError};
```

- [ ] **Step 2: Rewrite `FakeUserService`**

Replace the entire current `FakeUserService` block (lines 146-189) with:

```rust
#[derive(Clone, Default)]
pub struct FakeUserService {
    by_code: Arc<Mutex<HashMap<String, UserSummary>>>,
}

impl FakeUserService {
    pub fn seed(&self, code: &str, role: Role, active: bool) {
        let summary = UserSummary {
            code: code.to_string(),
            active,
            role,
        };
        self.by_code.lock().unwrap().insert(code.to_string(), summary);
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.by_code
            .lock()
            .unwrap()
            .get(code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
}
```

- [ ] **Step 3: Update `ApiRole::Admin` / `ApiRole::Root` call sites to use `Role`**

In the same file, there are many call sites of the form `users.seed("u1", ApiRole::Admin, true);` (or `ApiRole::Root`). Replace every occurrence of `ApiRole::Admin` with `Role::Admin`, and every occurrence of `ApiRole::Root` with `Role::Root`. The third argument (`true` / `false`) is unchanged.

The full list of lines to update, by the line numbers shown in the original file:

- `:238` — `ApiRole::Admin` → `Role::Admin`
- `:295` — `ApiRole::Admin` → `Role::Admin`
- `:359` — `ApiRole::Admin` → `Role::Admin`
- `:381` — `ApiRole::Admin` → `Role::Admin`
- `:411` — `ApiRole::Admin` → `Role::Admin`
- `:435` — `ApiRole::Admin` → `Role::Admin`
- `:464` — `ApiRole::Admin` → `Role::Admin`
- `:502` — `ApiRole::Admin` → `Role::Admin`
- `:567` — `ApiRole::Admin` → `Role::Admin`
- `:610` — `ApiRole::Admin` → `Role::Admin`
- `:635` — `ApiRole::Admin` → `Role::Admin`
- `:647` — `ApiRole::Admin` → `Role::Admin`
- `:666` — `ApiRole::Admin` → `Role::Admin`
- `:701` — `ApiRole::Admin` → `Role::Admin`
- `:726` — `ApiRole::Admin` → `Role::Admin`

A global `replace_all`-style edit works: `ApiRole::Admin` → `Role::Admin` and `ApiRole::Root` → `Role::Root`. Both are guaranteed to be unique to this file (the alias is gone after Step 1).

- [ ] **Step 4: Drop the now-unused `fixed_now` helper**

Search the file for `fixed_now`. The `seed` helper in `FakeUserService` (Step 2) no longer constructs a `UserView`, so the `let now = fixed_now();` call inside the old `seed` is gone. However, the file may still call `fixed_now()` elsewhere — search to confirm. If no other callers exist, delete the `fixed_now` function definition (around line 27-29):

```rust
fn fixed_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 7, 29, 0, 0, 0).unwrap()
}
```

If other callers do exist, leave `fixed_now` in place.

- [ ] **Step 5: Verify the crate builds and all unit tests pass**

Run: `cargo build -p auth && cargo test -p auth --lib`
Expected: build succeeds and all unit tests pass. This includes the usecase unit tests AND the in-memory facade tests in `src/adapter/facade/in_memory/tests.rs` (because they consume `FakeUserService` from `usecase::tests`).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/auth/src/usecase/tests.rs
git commit -m "test(auth): rewrite FakeUserService against domain UserService"
```

---

### Task 5: Rewrite the integration-test `FakeUserService` and update `public_api.rs`

**Files:**
- Modify: `lib/crates/auth/tests/integration_persistence.rs:23-29` (imports), `:230-268` (entire local `FakeUserService` struct + impl)
- Modify: `lib/crates/auth/tests/public_api.rs:10-16` (imports), `:58`, `:69` (replace `apis::user::UserService` with the new trait)

**Interfaces:**
- Consumes: `crate::domain::{UserService, UserSummary}`, `crate::domain::DomainError`
- Produces: integration-test `FakeUserService` implementing the domain trait; public_api test that locks the trait surface

- [ ] **Step 1: Update the imports in `tests/integration_persistence.rs`**

In `lib/crates/auth/tests/integration_persistence.rs`, replace the import block at lines 23-29:

```rust
use async_trait::async_trait;
use sqlx::PgPool;

use apis::user::{CreateUserRequest, UpdateUserRequest, UserApiError, UserService, UserView};
use auth::{
    AuthUsecase, AuthUsecaseConfig, DomainIdentity, DomainIdentityRepo, DomainIdentityRepository,
    UserCredentialsRepo, UserCredentialsRepository,
};
```

with:

```rust
use async_trait::async_trait;
use sqlx::PgPool;

use auth::{
    AuthUsecase, AuthUsecaseConfig, DomainError, DomainIdentity, DomainIdentityRepo,
    DomainIdentityRepository, UserCredentialsRepo, UserCredentialsRepository, UserService,
    UserSummary,
};
```

- [ ] **Step 2: Rewrite the local `FakeUserService` in `integration_persistence.rs`**

Replace the entire current `FakeUserService` block (lines 230-268) with:

```rust
/// Minimal fake `UserService` for the integration smoke test only.
pub struct FakeUserService {
    #[allow(dead_code)]
    by_code: Mutex<HashMap<String, UserSummary>>,
}

impl Default for FakeUserService {
    fn default() -> Self {
        Self::new()
    }
}

impl FakeUserService {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            by_code: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait]
impl UserService for FakeUserService {
    async fn get_by_code(&self, _code: &str) -> Result<UserSummary, DomainError> {
        Err(DomainError::NotFound)
    }
}
```

- [ ] **Step 3: Update `tests/public_api.rs`**

In `lib/crates/auth/tests/public_api.rs`, replace the import block at lines 10-16:

```rust
use apis::auth::AuthService;
use apis::user::UserService;
use auth::{
    AccessTokenView, AuthClaimsView, AuthServiceImpl, AuthUsecase, AuthUsecaseConfig,
    DomainIdentityRepo, DomainIdentityRepository, InMemoryTokenVersionCache, LogoutAck, Role,
    TokenPairView, TokenVersionCache, UserCredentialsRepo, UserCredentialsRepository,
};
```

with:

```rust
use apis::auth::AuthService;
use auth::{
    AccessTokenView, AuthClaimsView, AuthServiceImpl, AuthUsecase, AuthUsecaseConfig,
    DomainIdentityRepo, DomainIdentityRepository, InMemoryTokenVersionCache, LogoutAck, Role,
    TokenPairView, TokenVersionCache, UserCredentialsRepo, UserCredentialsRepository, UserService,
};
```

Then update the two lock-in references:

- Line 58: `let _: &Arc<dyn UserService> = &cfg.user_service;` — unchanged (the trait name is now `crate::domain::UserService`, but the local alias in the imports handles that).
- Line 69: `assert_user_service_is_send_sync::<Box<dyn UserService>>();` — unchanged for the same reason.

No other changes are needed in this file.

- [ ] **Step 4: Verify the integration tests and public-API compile test pass**

Run: `cargo test -p auth --test public_api`
Expected: success — the public API compile test passes against the new domain trait.

Run: `cargo test -p auth --lib`
Expected: success — same as Task 4's outcome; this is a sanity check that no further unit-level regressions snuck in.

Skip `--ignored` integration tests here — they require `AEGIS_DATABASE_URL` and the spec only requires that the file compiles cleanly. The `--ignored` smoke test will be exercised in Task 6 when a database is available; if the developer environment has one, run it now too:

Run (optional): `cargo test -p auth -- --ignored`
Expected: success — the smoke test `usecase_can_be_constructed_from_real_repos` constructs the usecase with the new `FakeUserService` and succeeds.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/auth/tests/integration_persistence.rs lib/crates/auth/tests/public_api.rs
git commit -m "test(auth): rewire integration fake + public-api test to domain UserService"
```

---

### Task 6: Final verification — no `apis::user` in `domain/` or `usecase/`, full test suite green

**Files:** none modified — verification only.

- [ ] **Step 1: Confirm no `apis::user` imports remain in the target layers**

Run: `grep -rn "apis::user" lib/crates/auth/src/domain lib/crates/auth/src/usecase`
Expected: no output (zero matches). The `domain` and `usecase` modules are now free of `apis::user` imports.

Run (informational): `grep -rn "apis::user" lib/crates/auth/src lib/crates/auth/tests`
Expected: the only remaining matches are in `src/adapter/service/user.rs` (the new adapter that intentionally delegates to the apis trait) and possibly in the doc-comment text of `src/usecase/tests.rs`. Any other match is a bug — investigate before merging.

- [ ] **Step 2: Run the full test suite**

Run: `cargo test -p auth`
Expected: success — all unit tests + integration compile test pass.

Run (optional): `cargo test -p auth -- --ignored` (requires `AEGIS_DATABASE_URL`)
Expected: success — the integration smoke test passes.

Run: `cargo test -p apis && cargo test -p user`
Expected: success — neither crate was touched by this refactor; their tests stay green.

- [ ] **Step 3: Final commit (no-op or doc-comment cleanup)**

If Step 1 surfaced a stray `apis::user` reference in a doc comment, fix it as a follow-up commit:

```bash
git add <affected file>
git commit -m "docs(auth): drop apis::user reference in doc comment"
```

If nothing changed, skip this step — there is no commit to make.

---

## Self-Review Notes (informational; pre-execution)

- **Spec coverage:** All six spec sections (New Domain Port, New Adapter, Usecase Changes, Test Wiring, Public API, What Stays Untouched) map to tasks. Task 1 ↔ New Domain Port; Task 2 ↔ New Adapter; Task 3 ↔ Usecase Changes; Tasks 4 + 5 ↔ Test Wiring; Task 5 ↔ Public API; Task 6 ↔ Untouched + cross-crate verification.
- **Placeholders:** No TBD/TODO/"implement later" anywhere. Every code step shows full code. Every command is concrete.
- **Type consistency:** `UserSummary { code: String, active: bool, role: Role }` is defined in Task 1 and used identically in Tasks 2, 4, 5. `UserService::get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>` is defined in Task 1 and matches the impl in Task 2 and the fakes in Tasks 4 and 5. `map_role` body in Task 2 matches the (deleted) `role_from_api` body. `map_error` body in Task 2 matches the (deleted) `map_user_service_error` body for the `NotFound` arm; other arms fall through to `DomainError::Repository(variant.to_string())`.
- **Compiles-after-each-task discipline:** Task 1 leaves the crate in a building state. Task 2 leaves it in a building state. Task 3 intentionally breaks the build (usecase uses the new trait, fakes still use the old) and expects Task 4 to fix it. Tasks 4-6 leave it in a building state. The Step 5 expected-failure in Task 3 is documented to prevent confusion.