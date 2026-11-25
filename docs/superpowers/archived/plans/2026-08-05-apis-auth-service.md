# apis AuthService Trait Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add the `apis::auth` module with an async `AuthService` trait and its supporting DTOs and error type, mirroring the layout and conventions of the existing `apis::user` module.

**Architecture:** Single new file `lib/crates/apis/src/auth.rs` housing all `auth` types and the trait. `pub mod auth;` added to `lib/crates/apis/src/lib.rs`. Existing compile-only test file `lib/crates/apis/tests/public_api.rs` gets one new block that exercises the trait through `Box<dyn AuthService>` and locks the DTO field types. No new crate dependencies.

**Tech Stack:** Rust 2024 edition; `async-trait` (already a workspace dep of `apis`); `thiserror` (already a workspace dep of `apis`).

## Global Constraints

- `apis` crate must remain independent of every other workspace crate — never add a `user`, `auth`, or any other internal crate as a `Cargo.toml` dependency.
- The `Role` enum already lives in `apis::user::Role`; reuse it via `use crate::user::Role;` in `auth.rs`. Do not introduce a second `Role` enum.
- Tokens (`access_token`, `refresh_token`) are plain `String`s. No newtype wrappers, no shared `Token` enum.
- Method parameters on the trait are `&str`; DTO struct fields are `String`. DTOs are owned-form for cross-boundary transport; the trait itself borrows.
- `AuthService: Send + Sync` so `Box<dyn AuthService>` is shared state in async servers.
- `#[async_trait]` matches the convention used by `apis::user::UserService` and `user::UserRepository`.
- `AuthApiError` keeps all seven variants (`Validation`, `NotFound`, `Inactive`, `InvalidCredentials`, `Signing`, `Verification`, `Repository`).
- `refresh` returns just the new access token (`Result<String, AuthApiError>`); the two `login_*` methods return `TokenPair`.
- `user::Role` is referenced from `auth.rs`, so `auth.rs` is only fully usable once `pub mod auth;` is in `lib.rs` and `pub mod user;` is too (it already is).

---

## File Structure

```text
lib/crates/apis/src/
  lib.rs                                # modify: add `pub mod auth;`
  auth.rs                               # create: all auth types + AuthService trait
lib/crates/apis/tests/
  public_api.rs                         # modify: append auth block (compile-only test)
```

No other files are touched. No `Cargo.toml` changes.

---

## Task 1: Wire `pub mod auth;` and create the `auth` module stub

**Files:**
- Modify: `lib/crates/apis/src/lib.rs`
- Create: `lib/crates/apis/src/auth.rs`

**Interfaces:**
- Consumes: nothing (this is the first task).
- Produces: `pub mod auth;` declared; `auth.rs` exists as an empty module so `cargo check -p apis` still passes before any types are filled in.

- [ ] **Step 1: Add `pub mod auth;` to `lib/crates/apis/src/lib.rs`**

Edit `lib/crates/apis/src/lib.rs` so the module list reads exactly:

```rust
//! `apis` workspace crate.
//!
//! Hosts outbound port traits that adapters (HTTP/gRPC handlers,
//! other backends) consume. Each trait is a self-contained
//! contract: this crate does not depend on any other workspace
//! crate, so any backend can implement the traits by adapting its
//! own types to the ones defined here.

pub mod auth;
pub mod user;
```

Preserve the leading doc comment. Do not change any other line.

- [ ] **Step 2: Create an empty `auth.rs`**

Create `lib/crates/apis/src/auth.rs` containing exactly this single line:

```rust
// `apis::auth` outbound port. See the module for the trait surface.
```

No `use`, no types, no trait. The file is a deliberate placeholder so `pub mod auth;` resolves without a "file not found" error.

- [ ] **Step 3: Verify the crate still compiles**

Run:

```bash
cargo check -p apis
```

Expected: `Finished` (or `Finished in <n>s`) with no errors and no warnings about an unresolved `auth` module.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/apis/src/lib.rs lib/crates/apis/src/auth.rs
git commit -m "feat(apis): scaffold auth module"
```

---

## Task 2: Add the supporting types to `auth.rs`

**Files:**
- Modify: `lib/crates/apis/src/auth.rs`

**Interfaces:**
- Consumes: `crate::user::Role` (already re-exported at `apis::user::Role`).
- Produces (these names and shapes become public API on `apis::auth::*`):
  - `pub struct TokenPair { pub access_token: String, pub refresh_token: String }` — derives `Debug, Clone, PartialEq, Eq`.
  - `pub struct AuthClaims { pub code: String, pub role: crate::user::Role, pub token_version: u32 }` — derives `Debug, Clone, PartialEq, Eq`.
  - `pub struct LoginWithPasswordRequest { pub code: String, pub password: String }` — derives `Debug, Clone`.
  - `pub struct LoginWithDomainUserInfoRequest { pub code: String, pub domain_name: String, pub hostname: String, pub sid: String }` — derives `Debug, Clone`.
  - `pub enum AuthApiError` — seven variants, exact set defined below.

- [ ] **Step 1: Replace the `auth.rs` body with the supporting types**

Overwrite `lib/crates/apis/src/auth.rs` with the following contents verbatim:

```rust
//! Outbound port for authentication.
//!
//! See [`AuthService`] for the trait surface. All supporting types
//! (`TokenPair`, `AuthClaims`, the login request DTOs, and
//! `AuthApiError`) are defined alongside the trait so a single
//! `use apis::auth::*;` brings the whole contract into scope.

use thiserror::Error;

use crate::user::Role;

/// Access + refresh token pair returned by the login methods.
///
/// `refresh` does not use `TokenPair` — it mints a new access token
/// only, returning the bare `String`. The login methods return both
/// freshly minted tokens so callers can hand them out together.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
}

/// Authenticated identity recovered from a verified access token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthClaims {
    pub code: String,
    pub role: Role,
    pub token_version: u32,
}

/// Input DTO for [`AuthService::login_with_password`].
#[derive(Debug, Clone)]
pub struct LoginWithPasswordRequest {
    pub code: String,
    pub password: String,
}

/// Input DTO for [`AuthService::login_with_domain_user_info`].
#[derive(Debug, Clone)]
pub struct LoginWithDomainUserInfoRequest {
    pub code: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}

/// Error surface returned by every [`AuthService`] method.
///
/// Adapters map backend-specific errors into this type at the
/// implementation boundary. The shape intentionally combines
/// validation, lookup, credential, and token concerns into a
/// single type so handlers can match exhaustively.
#[derive(Debug, Error)]
pub enum AuthApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("user not found")]
    NotFound,

    #[error("user is inactive")]
    Inactive,

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("token signing failed: {0}")]
    Signing(String),

    #[error("token verification failed: {0}")]
    Verification(String),

    #[error("repository error: {0}")]
    Repository(String),
}
```

Do not add the `AuthService` trait yet — that is Task 3.

- [ ] **Step 2: Verify the types compile**

Run:

```bash
cargo check -p apis
```

Expected: `Finished` with no errors and no warnings. `apis::auth::AuthApiError` is currently `dead_code` until the trait exists — `cargo check` will warn. That warning is expected and resolved in Task 3; do not silence it here. If any other warning appears, fix it before proceeding.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/apis/src/auth.rs
git commit -m "feat(apis): add auth supporting types"
```

---

## Task 3: Add the `AuthService` trait

**Files:**
- Modify: `lib/crates/apis/src/auth.rs`

**Interfaces:**
- Consumes: `TokenPair`, `AuthClaims`, `LoginWithPasswordRequest`, `LoginWithDomainUserInfoRequest`, `AuthApiError` (all defined in Task 2).
- Produces (public API on `apis::auth::*`):
  - `pub trait AuthService: Send + Sync` decorated with `#[async_trait::async_trait]` with exactly these five methods (signatures verbatim):
    - `async fn login_with_password(&self, code: &str, password: &str) -> Result<TokenPair, AuthApiError>;`
    - `async fn login_with_domain_user_info(&self, code: &str, domain_name: &str, hostname: &str, sid: &str) -> Result<TokenPair, AuthApiError>;`
    - `async fn logout(&self, code: &str) -> Result<(), AuthApiError>;`
    - `async fn verify(&self, access_token: &str) -> Result<AuthClaims, AuthApiError>;`
    - `async fn refresh(&self, refresh_token: &str) -> Result<String, AuthApiError>;`

- [ ] **Step 1: Append the `AuthService` trait to `auth.rs`**

Append to the bottom of `lib/crates/apis/src/auth.rs` (below the existing `AuthApiError` definition) the following block. Do not edit any earlier lines.

```rust
/// Outbound port for authentication.
///
/// `Send + Sync` so a `Box<dyn AuthService>` can be shared state in
/// an async server (axum, tarpc, etc.). Object-safe: no generic
/// methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer into this
/// contract, translating between backend-specific DTOs / errors
/// and the `apis` types defined above.
#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    /// Authenticate with a user code + password.
    ///
    /// On success mints a fresh access token and refresh token and
    /// returns them. Implementations check the password against
    /// the persisted hash and surface `InvalidCredentials` (not
    /// `NotFound`) for a code that exists with the wrong password.
    async fn login_with_password(
        &self,
        code: &str,
        password: &str,
    ) -> Result<TokenPair, AuthApiError>;

    /// Authenticate with Windows-domain user info (AD / NTLM style).
    ///
    /// `domain_name`, `hostname`, and `sid` identify the domain
    /// account. On success mints a fresh access token and refresh
    /// token and returns them. Implementations surface `NotFound`
    /// when no user maps to the supplied domain-identity triple.
    async fn login_with_domain_user_info(
        &self,
        code: &str,
        domain_name: &str,
        hostname: &str,
        sid: &str,
    ) -> Result<TokenPair, AuthApiError>;

    /// Invalidate any server-side session state for `code`.
    ///
    /// Returns `Ok(())` even if the user had no active session.
    /// Storage failures surface as `AuthApiError::Repository`.
    async fn logout(&self, code: &str) -> Result<(), AuthApiError>;

    /// Verify an access token and recover the identity it was minted for.
    ///
    /// Returns `AuthClaims` on success. Token-format, signature,
    /// and expiry failures all surface as
    /// `AuthApiError::Verification`.
    async fn verify(&self, access_token: &str) -> Result<AuthClaims, AuthApiError>;

    /// Exchange a still-valid refresh token for a brand-new access token.
    ///
    /// Returns the freshly minted access token as a `String`.
    /// Expired or tampered-with refresh tokens surface as
    /// `AuthApiError::Verification`. The refresh token itself is
    /// not rotated — callers keep using the same refresh token
    /// until it expires.
    async fn refresh(&self, refresh_token: &str) -> Result<String, AuthApiError>;
}
```

The trait uses `#[async_trait::async_trait]` (fully qualified) rather than importing the macro via `use async_trait::async_trait;`. This matches the style on this same file's sibling `apis::user::UserService`.

- [ ] **Step 2: Verify the trait compiles**

Run:

```bash
cargo check -p apis
```

Expected: `Finished` with no errors and no warnings. The `dead_code` warning that Task 2 surfaced should be gone — the trait now uses `AuthApiError`. If any other warning appears, fix it before proceeding.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/apis/src/auth.rs
git commit -m "feat(apis): add AuthService trait"
```

---

## Task 4: Extend `tests/public_api.rs` to cover `auth`

**Files:**
- Modify: `lib/crates/apis/tests/public_api.rs`

**Interfaces:**
- Consumes: `apis::auth::{AuthClaims, AuthApiError, AuthService, LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, TokenPair}` (the entire public surface defined in Tasks 2 and 3).
- Produces: a `FakeAuthService` struct that implements `AuthService` returning `todo!()` from every method, plus two compile-time assertions (`auth_service_is_object_safe`, `auth_service_is_send_sync`) following the same shape as the existing `user_service_*` tests already in the file.

- [ ] **Step 1: Append the auth block to `public_api.rs`**

Read the existing `lib/crates/apis/tests/public_api.rs` to confirm its current contents, then append (do not edit any earlier line) the following block at the bottom:

```rust
// -- apis::auth ---------------------------------------------------------

use apis::auth::{
    AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, TokenPair,
};

/// Every public type in `apis::auth` is nameable from the test.
#[test]
fn auth_public_types_are_nameable() {
    fn assert_pair(_: TokenPair) {}
    fn assert_claims(_: AuthClaims) {}
    fn assert_login_pw(_: LoginWithPasswordRequest) {}
    fn assert_login_domain(_: LoginWithDomainUserInfoRequest) {}
    fn assert_err(_: AuthApiError) {}

    // `TokenPair` is constructible field-by-field.
    assert_pair(TokenPair {
        access_token: "a".into(),
        refresh_token: "r".into(),
    });
    // `AuthClaims` is constructible field-by-field; `role` reuses
    // `apis::user::Role`.
    assert_claims(AuthClaims {
        code: "u1".into(),
        role: apis::user::Role::General,
        token_version: 0,
    });
    // Login request DTOs own their strings — that is the shape
    // adapters receive from outside the backend.
    assert_login_pw(LoginWithPasswordRequest {
        code: "u1".into(),
        password: "p".into(),
    });
    assert_login_domain(LoginWithDomainUserInfoRequest {
        code: "u1".into(),
        domain_name: "d".into(),
        hostname: "h".into(),
        sid: "s".into(),
    });

    // Touch every variant of the error type to keep it from being
    // dead-code-eliminated by the test build's analysis.
    let _: AuthApiError = AuthApiError::Validation("".into());
    let _: AuthApiError = AuthApiError::NotFound;
    let _: AuthApiError = AuthApiError::Inactive;
    let _: AuthApiError = AuthApiError::InvalidCredentials;
    let _: AuthApiError = AuthApiError::Signing("".into());
    let _: AuthApiError = AuthApiError::Verification("".into());
    let _: AuthApiError = AuthApiError::Repository("".into());
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's signature,
/// object-safety, and `Send + Sync` bounds. Each method returns
/// `todo!()` because the test only exercises the type system — never
/// the runtime behavior.
struct FakeAuthService;

#[async_trait::async_trait]
impl AuthService for FakeAuthService {
    async fn login_with_password(
        &self,
        _code: &str,
        _password: &str,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn login_with_domain_user_info(
        &self,
        _code: &str,
        _domain_name: &str,
        _hostname: &str,
        _sid: &str,
    ) -> Result<TokenPair, AuthApiError> {
        todo!()
    }
    async fn logout(&self, _code: &str) -> Result<(), AuthApiError> {
        todo!()
    }
    async fn verify(&self, _access_token: &str) -> Result<AuthClaims, AuthApiError> {
        todo!()
    }
    async fn refresh(&self, _refresh_token: &str) -> Result<String, AuthApiError> {
        todo!()
    }
}

/// `AuthService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn auth_service_is_object_safe() {
    let _boxed: Box<dyn AuthService> = Box::new(FakeAuthService);
}

/// `AuthService` requires `Send + Sync`, so a `Box<dyn AuthService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn auth_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn AuthService>>();
    assert_send_sync::<&FakeAuthService>();
}
```

Notes:

- The block uses fully-qualified `apis::auth::…` and `apis::user::Role::General` (no top-level `use` for those) so the import list at the top of the block is the only thing that scopes the test's view.
- The `FakeAuthService` mirrors `FakeUserService` already in the same file. The parallel naming keeps reviewers' eyes moving.
- `Cargo` will not actually invoke `todo!()` at runtime — the test calls are still inside `#[test]` functions whose return-type is `()`, so the futures are never awaited. If a later change ever replaces the casts with `.await`, `todo!()` will panic during test runs and the failure will be loud and obvious.

- [ ] **Step 2: Run the public-api tests**

Run:

```bash
cargo test -p apis
```

Expected output: all five existing tests pass (`public_types_are_nameable`, `user_service_is_object_safe`, `user_service_is_send_sync`, plus the two auth tests added in Step 1: `auth_public_types_are_nameable`, `auth_service_is_object_safe`, `auth_service_is_send_sync`). Total: 6 tests, all green.

If any test fails, the most likely cause is a typo in a method signature — re-check against Task 3's interface list and fix before committing.

- [ ] **Step 3: Run clippy to confirm no new lints**

Run:

```bash
cargo clippy -p apis --all-targets -- -D warnings
```

Expected: exit 0, no warnings. If clippy flags anything, fix it before proceeding.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/apis/tests/public_api.rs
git commit -m "test(apis): lock auth public-api surface and AuthService bounds"
```

---

## Task 5: Workspace-level sanity check

**Files:** none

- [ ] **Step 1: Build the full workspace**

Run:

```bash
cargo build --workspace
```

Expected: `Finished` with no errors. `apis` is a leaf crate, so this build should be a no-op for downstream crates — but it confirms nothing else broke.

- [ ] **Step 2: Run all workspace tests**

Run:

```bash
cargo test --workspace
```

Expected: every crate's tests pass. Specifically the `apis` crate should report 6 tests (3 existing + 3 new).

- [ ] **Step 3: Final commit if any tooling touched code**

Run `cargo fmt` if any formatting drift appeared:

```bash
cargo fmt -p apis
```

If anything was reformatted, commit:

```bash
git add lib/crates/apis
git commit -m "style(apis): cargo fmt"
```

---

## Self-Review

Spec coverage check:

- `crate layout` (one file `auth.rs`, `pub mod auth;` in `lib.rs`) — Task 1 covers it.
- `TokenPair`, `AuthClaims`, both request DTOs, `AuthApiError` — Task 2 covers them with exact field / variant shapes.
- `AuthService` trait with five methods, `Send + Sync`, `#[async_trait]`, `refresh` returning `Result<String, AuthApiError>` only — Task 3 covers it.
- `tests/public_api.rs` extended with the auth block — Task 4 covers it.
- Workspace integration sanity check — Task 5 covers it.

Placeholder scan: none. Every step has either exact code (Steps 1 of Tasks 1, 2, 3, 4) or an exact command with expected output (Step 2/3 of each task). The one place that describes work without code is the docstring comment block in Tasks 2 and 3, which is the actual code being added — not a description of code someone else should add.

Type consistency check:

- `login_with_password` / `login_with_domain_user_info` / `verify` / `refresh` / `logout` all use the parameter and return types defined in Tasks 2 and 3.
- `FakeAuthService`'s `impl AuthService` mirrors the trait signatures verbatim — same parameter names with leading underscore, same return types.
- `Role` is referenced only as `crate::user::Role` (in `auth.rs`) and `apis::user::Role` (in the test). Same type, same source. ✓
