# apis::auth Credential CRUD & Refresh-Token Logout Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend `apis::auth` with credential-management methods and switch `logout` to take a refresh token instead of a user code.

**Architecture:** Single file change to `lib/crates/apis/src/auth.rs` plus a matching update to the compile-test in `lib/crates/apis/tests/public_api.rs`. New DTOs are co-located with the trait (one-module pattern, same as today). The trait stays a self-contained contract — no downstream changes to the `user` crate.

**Tech Stack:** Rust (edition 2024), `async-trait`, `thiserror`. No new dependencies. The test is a compile-only test that pins the public type surface and the trait's `Send + Sync` / object-safety bounds.

## Global Constraints

- The `apis` crate is a self-contained contract; it MUST NOT take a dependency on any other workspace crate.
- Type names in `apis::auth` follow the existing module convention: `*Request` for input DTOs, `*Response` for output DTOs, `*View` for safe projections, `AuthClaims` / `TokenPair` for the one-off payloads. Single-arg lookup methods take `&str`; multi-arg methods take a request DTO.
- Every new `AuthService` method must be `async`, object-safe, and keep the trait `Send + Sync`-able.
- The compile test in `lib/crates/apis/tests/public_api.rs` MUST keep passing after the change. It pins type names, field shapes, the trait's `Send + Sync` bound, and the trait's object-safety.
- No new entries in `lib/crates/apis/Cargo.toml`.
- No changes outside `lib/crates/apis/src/auth.rs` and `lib/crates/apis/tests/public_api.rs`.

---

## File Structure

This plan touches two files:

| File | Role | Change |
| --- | --- | --- |
| `lib/crates/apis/src/auth.rs` | Houses the `AuthService` trait and all its supporting DTOs / error enum. | In-place rewrite of the file. |
| `lib/crates/apis/tests/public_api.rs` | Compile-only test that pins the public type surface and the trait's `Send + Sync` / object-safety bounds. | Add new imports, new DTO-construction asserts, new `FakeAuthService` method stubs; update the existing `logout` / `LogoutRequest` / `LogoutResponse` asserts. |

No file is created; no file is split. The trait is small enough to keep in one file, and the test additions are small enough to keep the existing single-test-file structure.

---

## Task 1: Update `apis::auth` with credential CRUD and refresh-token logout

**Files:**
- Modify: `lib/crates/apis/src/auth.rs` (full rewrite — see Step 5)
- Modify: `lib/crates/apis/tests/public_api.rs` (test surface and `FakeAuthService` — see Steps 3 and 7)

**Interfaces (this task produces):**

The complete `apis::auth` surface after this task:

```rust
// Input DTOs
pub struct LoginWithPasswordRequest    { pub code: String, pub password: String }
pub struct LoginWithDomainUserInfoRequest {
    pub code: String, pub domain_name: String, pub hostname: String, pub sid: String
}
pub struct LogoutRequest               { pub refresh_token: String }            // CHANGED: code → refresh_token
pub struct VerifyRequest               { pub access_token: String }
pub struct RefreshRequest              { pub refresh_token: String }
pub struct CreateUserCredentialRequest { pub user_code: String, pub password_hash: String }   // NEW
pub struct UpdateUserCredentialRequest { pub user_code: String, pub password_hash: Option<String> } // NEW

// Output DTOs
pub struct TokenPair      { pub access_token: String, pub refresh_token: String }
pub struct AuthClaims     { pub code: String, pub role: Role, pub token_version: u32 }
pub struct LogoutResponse {}                                             // CHANGED: empty struct (was { code: String })
pub struct RefreshResponse { pub access_token: String }
pub struct UserCredentialView { pub user_code: String, pub password_hash: String, pub token_version: u32 } // NEW
pub struct RemoveUserCredentialResponse {}                               // NEW (empty struct)

// Error enum
pub enum AuthApiError {
    Validation(String), NotFound, Inactive, InvalidCredentials,
    Signing(String), Verification(String), Repository(String),
    DuplicateCode(String),    // NEW
}

// Trait
#[async_trait]
pub trait AuthService: Send + Sync {
    async fn login_with_password(&self, req: LoginWithPasswordRequest) -> Result<TokenPair, AuthApiError>;
    async fn login_with_domain_user_info(&self, req: LoginWithDomainUserInfoRequest) -> Result<TokenPair, AuthApiError>;
    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError>;
    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError>;
    // credential management
    async fn find_user_credential_by_code(&self, code: &str) -> Result<UserCredentialView, AuthApiError>; // NEW
    async fn create_user_credential(&self, req: CreateUserCredentialRequest) -> Result<UserCredentialView, AuthApiError>; // NEW
    async fn update_user_credential(&self, req: UpdateUserCredentialRequest) -> Result<UserCredentialView, AuthApiError>; // NEW
    async fn remove_user_credential(&self, code: &str) -> Result<RemoveUserCredentialResponse, AuthApiError>; // NEW
    // session lifecycle
    async fn logout(&self, req: LogoutRequest) -> Result<LogoutResponse, AuthApiError>; // CHANGED return
}
```

**Consumes:** the spec at `docs/superpowers/specs/2026-08-06-apis-auth-credential-and-logout-design.md`.

**Produces:** the trait + DTOs + error enum above, and the matching compile test that pins them.

---

- [ ] **Step 1: Confirm working tree is on `feat/apis_auth-user-credential`**

Run from the repo root:

```bash
git -C /root/coding/project/aegis status
git -C /root/coding/project/aegis branch --show-current
```

Expected: working tree is on branch `feat/apis_auth-user-credential` with no uncommitted changes in the apis crate (the spec doc commits are present, but `auth.rs` and `public_api.rs` are unchanged from `main`).

If you are on a different branch, do NOT switch — stop and ask the user. The plan is built for this branch.

- [ ] **Step 2: Verify the current test passes before any change**

Run from the repo root:

```bash
cd /root/coding/project/aegis && cargo test -p apis
```

Expected output ends with something like:

```
test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in ...
```

(The exact count of `passed` may differ; what matters is `0 failed` and that the test binary `tests::public_api` is among the passing targets.)

- [ ] **Step 3: Update the test file with the new imports, new DTO-construction asserts, and the changed `LogoutRequest` / `LogoutResponse` asserts (but do NOT update `FakeAuthService` yet)**

Open `lib/crates/apis/tests/public_api.rs` and apply the following edits.

**Edit 3a — Update the `apis::auth` import block (around line 92–96).** Replace the existing block:

```rust
use apis::auth::{
    AuthApiError, AuthClaims, AuthService, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
    TokenPair, VerifyRequest,
};
```

with:

```rust
use apis::auth::{
    AuthApiError, AuthClaims, AuthService, CreateUserCredentialRequest,
    LoginWithDomainUserInfoRequest, LoginWithPasswordRequest, LogoutRequest, LogoutResponse,
    RefreshRequest, RefreshResponse, RemoveUserCredentialResponse, TokenPair,
    UpdateUserCredentialRequest, UserCredentialView, VerifyRequest,
};
```

**Edit 3b — Update `auth_public_types_are_nameable` (around line 99–159).** Inside the test function:

- Change the closure `fn assert_logout_req(_: LogoutRequest) {}` to remain as-is.
- After `assert_login_domain(...)`, change:

  ```rust
  assert_logout_req(LogoutRequest { code: "u1".into() });
  ```

  to:

  ```rust
  assert_logout_req(LogoutRequest { refresh_token: "r".into() });
  ```

- After `assert_logout_res(LogoutResponse { code: "u1".into() });`, change the line:

  ```rust
  assert_logout_res(LogoutResponse { code: "u1".into() });
  ```

  to:

  ```rust
  assert_logout_res(LogoutResponse {});
  ```

- Just before the `let _: AuthApiError = AuthApiError::Validation(...)` block, add the following four new asserts and one new helper-closure (place the helpers alongside the other `fn assert_*` declarations, and the field-by-field constructions right before the `let _ = assert_err;` line):

  ```rust
      fn assert_create_cred(_: CreateUserCredentialRequest) {}
      fn assert_update_cred(_: UpdateUserCredentialRequest) {}
      fn assert_cred_view(_: UserCredentialView) {}
      fn assert_remove_cred_res(_: RemoveUserCredentialResponse) {}
  ```

  and right before `let _ = assert_err;`:

  ```rust
      assert_create_cred(CreateUserCredentialRequest {
          user_code: "u1".into(),
          password_hash: "h".into(),
      });
      assert_update_cred(UpdateUserCredentialRequest {
          user_code: "u1".into(),
          ..Default::default()
      });
      assert_cred_view(UserCredentialView {
          user_code: "u1".into(),
          password_hash: "h".into(),
          token_version: 0,
      });
      assert_remove_cred_res(RemoveUserCredentialResponse {});
  ```

- In the variant-reachability block, add a new line right after `let _: AuthApiError = AuthApiError::Repository("".into());`:

  ```rust
      let _: AuthApiError = AuthApiError::DuplicateCode("".into());
  ```

After this step the test file should still compile (the new types are imported and referenced, but the `FakeAuthService` still uses the OLD trait shape — see Step 7). At the end of this step the test file SHOULD NOT compile if you also updated the `LogoutRequest` / `LogoutResponse` construction, because the current trait still has `logout(&self, req: LogoutRequest) -> Result<LogoutResponse, AuthApiError>` (which matches the new DTO shapes, so this is fine). The `FakeAuthService::logout` body is unchanged for now, so the test will continue to compile.

Re-read the file end-to-end to make sure no half-applied edit was left behind (especially: every old `LogoutRequest { code ... }` and every old `LogoutResponse { code ... }` should now be updated).

- [ ] **Step 4: Run the test to verify it still passes after the test-file edits**

Run from the repo root:

```bash
cd /root/coding/project/aegis && cargo test -p apis
```

Expected: `0 failed`. The test still passes because the `FakeAuthService` in the test file still implements the OLD trait (which has not changed yet), and the new DTOs / variants are not yet defined — wait, that won't compile either.

Read the error output carefully. If the failure is `error[E0432]: unresolved import ... CreateUserCredentialRequest` (or any of the other new types), this is the EXPECTED outcome of Step 3 alone — the test references types the implementation does not yet expose. This is fine: the next step is the implementation.

If the failure is something else, stop and ask the user.

- [ ] **Step 5: Replace the contents of `lib/crates/apis/src/auth.rs` with the full new file**

Open `lib/crates/apis/src/auth.rs` and replace the entire file contents with the following:

```rust
//! Outbound port for authentication.
//!
//! See [`AuthService`] for the trait surface. All supporting types
//! (`TokenPair`, `AuthClaims`, the request / view / response DTOs,
//! and `AuthApiError`) are defined alongside the trait so a single
//! `use apis::auth::*;` brings the whole contract into scope.

use thiserror::Error;

use crate::user::Role;

/// Access + refresh token pair returned by the login methods.
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

/// Input DTO for [`AuthService::logout`].
#[derive(Debug, Clone)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

/// Input DTO for [`AuthService::verify`].
#[derive(Debug, Clone)]
pub struct VerifyRequest {
    pub access_token: String,
}

/// Input DTO for [`AuthService::refresh`].
#[derive(Debug, Clone)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

/// Input DTO for [`AuthService::create_user_credential`].
///
/// `token_version` is intentionally absent: the implementation picks
/// the initial value (typically `0`).
#[derive(Debug, Clone)]
pub struct CreateUserCredentialRequest {
    pub user_code: String,
    pub password_hash: String,
}

/// Input DTO for [`AuthService::update_user_credential`].
///
/// Only `password_hash` is mutable through this DTO. To change
/// `token_version` callers go through a future admin-facing API
/// (out of scope here).
#[derive(Debug, Clone, Default)]
pub struct UpdateUserCredentialRequest {
    pub user_code: String,
    pub password_hash: Option<String>,
}

/// Response DTO for [`AuthService::logout`].
///
/// Empty by design — a successful logout carries no payload. Kept
/// as a named type (rather than `()`) so the response shape is
/// explicit at the API boundary and can be extended later
/// without a breaking trait change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct LogoutResponse {}

/// Response DTO for [`AuthService::refresh`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefreshResponse {
    pub access_token: String,
}

/// Safe projection of a user's credential.
///
/// `password_hash` is always a hashed representation (Argon2 in the
/// canonical backend); the trait does not constrain the hashing
/// algorithm. `token_version` is read-only through this trait
/// surface — see [`CreateUserCredentialRequest`] and
/// [`UpdateUserCredentialRequest`] for what callers may set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserCredentialView {
    pub user_code: String,
    pub password_hash: String,
    pub token_version: u32,
}

/// Response DTO for [`AuthService::remove_user_credential`].
///
/// Empty by design — a successful removal carries no payload. Kept
/// as a named type (rather than `()`) so the response shape is
/// explicit at the API boundary and can be extended later
/// without a breaking trait change.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RemoveUserCredentialResponse {}

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

    #[error("user credential already exists: {0}")]
    DuplicateCode(String),
}

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
        req: LoginWithPasswordRequest,
    ) -> Result<TokenPair, AuthApiError>;

    /// Authenticate with Windows-domain user info (AD / NTLM style).
    ///
    /// `domain_name`, `hostname`, and `sid` identify the domain
    /// account. On success mints a fresh access token and refresh
    /// token and returns them. Implementations surface `NotFound`
    /// when no user maps to the supplied domain-identity triple.
    async fn login_with_domain_user_info(
        &self,
        req: LoginWithDomainUserInfoRequest,
    ) -> Result<TokenPair, AuthApiError>;

    /// Verify an access token and recover the identity it was minted for.
    ///
    /// Returns `AuthClaims` on success. Token-format, signature,
    /// and expiry failures all surface as
    /// `AuthApiError::Verification`.
    async fn verify(&self, req: VerifyRequest) -> Result<AuthClaims, AuthApiError>;

    /// Exchange a still-valid refresh token for a brand-new access token.
    ///
    /// Returns `RefreshResponse { access_token }` on success.
    /// Expired or tampered-with refresh tokens surface as
    /// `AuthApiError::Verification`. The refresh token itself is
    /// not rotated — callers keep using the same refresh token
    /// until it expires.
    async fn refresh(&self, req: RefreshRequest) -> Result<RefreshResponse, AuthApiError>;

    // -- credential management -----------------------------------------

    /// Look up the credential row attached to `code`. Returns
    /// `NotFound` if no credential exists for that code.
    async fn find_user_credential_by_code(
        &self,
        code: &str,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Persist a new credential row. The implementation picks the
    /// initial `token_version`. Returns `DuplicateCode(code)` if a
    /// credential already exists for that `user_code`.
    async fn create_user_credential(
        &self,
        req: CreateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Apply the optional fields on `req` to the credential
    /// identified by `req.user_code`. Returns `NotFound` if no such
    /// credential exists. A `req` whose only set field is
    /// `user_code` (every other field is `None`) is permitted and
    /// returns the unchanged credential view.
    async fn update_user_credential(
        &self,
        req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Delete the credential row for `code`. Returns `NotFound` if
    /// no such credential exists.
    async fn remove_user_credential(
        &self,
        code: &str,
    ) -> Result<RemoveUserCredentialResponse, AuthApiError>;

    // -- session lifecycle --------------------------------------------

    /// Invalidate the session identified by `req.refresh_token`.
    ///
    /// The implementation looks up the token, removes any stored
    /// refresh-token entry, and returns `Ok(LogoutResponse::default())`.
    /// Returns `Ok(...)` even when the token had no active session
    /// (idempotent). A malformed or already-revoked refresh token
    /// surfaces as `AuthApiError::Verification`. Storage failures
    /// surface as `AuthApiError::Repository`.
    async fn logout(
        &self,
        req: LogoutRequest,
    ) -> Result<LogoutResponse, AuthApiError>;
}
```

Verify the file is exactly the contents above (no trailing whitespace, one trailing newline). The previous version had a single trailing newline; preserve that.

- [ ] **Step 6: Run the test to confirm the test now fails on the `FakeAuthService` (expected)**

Run from the repo root:

```bash
cd /root/coding/project/aegis && cargo test -p apis
```

Expected: the build FAILS with errors of the form:

```
error[E0046]: not all trait items implemented, missing one of: find_user_credential_by_code, create_user_credential, update_user_credential, remove_user_credential
  --> lib/crates/apis/tests/public_api.rs
```

plus possibly errors on the now-removed `LogoutResponse { code: ... }` literal if any were left behind. Re-read the test file if so and remove them.

If the build fails for any OTHER reason, stop and ask the user. The only legitimate failure at this point is the missing `FakeAuthService` methods.

- [ ] **Step 7: Add the four new method stubs to `FakeAuthService` in the test file**

Open `lib/crates/apis/tests/public_api.rs` and find the `impl AuthService for FakeAuthService` block (around line 168). Apply two edits.

**Edit 7a — Update the `logout` method to return `LogoutResponse`.** Replace:

```rust
    async fn logout(
        &self,
        _req: LogoutRequest,
    ) -> Result<LogoutResponse, AuthApiError> {
        todo!()
    }
```

with:

```rust
    async fn logout(
        &self,
        _req: LogoutRequest,
    ) -> Result<LogoutResponse, AuthApiError> {
        todo!()
    }
```

(No change to `logout` itself — it already returns `Result<LogoutResponse, AuthApiError>`. This step is a no-op; it is here only to make the diff vs the previous file explicit. Skip if you can confirm by reading that the `logout` body already matches the new return type.)

**Edit 7b — Add the four new method stubs.** Find the end of the `impl` block (the line right before the closing `}` of `impl AuthService for FakeAuthService`). Add the following four methods just before that closing brace, in the same order they appear in the trait:

```rust
    async fn find_user_credential_by_code(
        &self,
        _code: &str,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn create_user_credential(
        &self,
        _req: CreateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn update_user_credential(
        &self,
        _req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError> {
        todo!()
    }
    async fn remove_user_credential(
        &self,
        _code: &str,
    ) -> Result<RemoveUserCredentialResponse, AuthApiError> {
        todo!()
    }
```

Re-read the entire `impl AuthService for FakeAuthService` block to confirm:

- `login_with_password` is unchanged.
- `login_with_domain_user_info` is unchanged.
- `verify` is unchanged.
- `refresh` is unchanged.
- `logout` is unchanged (already returns `Result<LogoutResponse, AuthApiError>`).
- The four new methods are added in the order they appear in the trait.

- [ ] **Step 8: Run the test to confirm it passes**

Run from the repo root:

```bash
cd /root/coding/project/aegis && cargo test -p apis
```

Expected output ends with:

```
test result: ok. N passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in ...
```

`N` should be the same total as before the change (no new test functions were added; only the existing `auth_public_types_are_nameable`, `auth_service_is_object_safe`, `auth_service_is_send_sync` tests are exercised with the new surface). The exact value of `N` is not important; `0 failed` is.

If the test fails, read the error. Likely causes:
- A typo in a method signature on `FakeAuthService` (e.g. wrong return type). Fix it.
- A missing import in the test file. Add it.
- A missing helper closure (`fn assert_create_cred` etc.) in the test function. Add it.

Re-run until the test passes.

- [ ] **Step 9: Confirm the public surface of `auth.rs` matches the spec**

Open `lib/crates/apis/src/auth.rs` and check, line by line, that every item the spec calls out is present:

- The five DTO renames / new types in the order documented in the spec: `TokenPair`, `AuthClaims`, `LoginWithPasswordRequest`, `LoginWithDomainUserInfoRequest`, `LogoutRequest`, `VerifyRequest`, `RefreshRequest`, `CreateUserCredentialRequest`, `UpdateUserCredentialRequest`, `LogoutResponse`, `RefreshResponse`, `UserCredentialView`, `RemoveUserCredentialResponse`.
- The `AuthApiError` enum with all 8 variants (`Validation`, `NotFound`, `Inactive`, `InvalidCredentials`, `Signing`, `Verification`, `Repository`, `DuplicateCode`).
- The `AuthService` trait with all 9 methods in order: `login_with_password`, `login_with_domain_user_info`, `verify`, `refresh`, `find_user_credential_by_code`, `create_user_credential`, `update_user_credential`, `remove_user_credential`, `logout`.
- The `// -- credential management` and `// -- session lifecycle` section comments are present.

If any item is missing or in the wrong order, fix it and re-run Step 8.

- [ ] **Step 10: Run a wider build to make sure nothing else in the workspace broke**

Run from the repo root:

```bash
cd /root/coding/project/aegis && cargo build --workspace
```

Expected: build succeeds with no errors. Warnings are OK; if there are warnings about unused imports or dead code, investigate them — they may indicate a typo in the test file or in `auth.rs`.

- [ ] **Step 11: Stage and commit**

Run from the repo root:

```bash
cd /root/coding/project/aegis && git add lib/crates/apis/src/auth.rs lib/crates/apis/tests/public_api.rs && git status
```

Expected: only the two files above are staged. Nothing else should be staged.

If anything else is staged, run `git restore --staged <file>` for each unwanted file and inspect it before discarding the change.

Once the staging area is clean, commit:

```bash
cd /root/coding/project/aegis && git commit -m "feat(apis): add credential CRUD and switch logout to refresh token

- LogoutRequest now carries refresh_token instead of user code;
  LogoutResponse is an empty struct (no payload).
- New AuthService methods: find_user_credential_by_code,
  create_user_credential, update_user_credential,
  remove_user_credential. token_version is exposed on the view
  but not on the create/update DTOs (read-only through this
  trait surface).
- New DTOs: UserCredentialView, CreateUserCredentialRequest,
  UpdateUserCredentialRequest, RemoveUserCredentialResponse.
- New AuthApiError variant: DuplicateCode(String).
- public_api.rs compile test updated to pin the new surface
  and the four new FakeAuthService method stubs.

Co-Authored-By: Claude <noreply@anthropic.com>"
```

Expected: one new commit on top of the spec commits already on the branch. Run `git log --oneline -5` to confirm the commit is at the top of `feat/apis_auth-user-credential`.

---

## Self-Review

**1. Spec coverage.** Going through the spec section by section:

| Spec section | Implemented in |
| --- | --- |
| "Logout signature change" (refresh_token, empty LogoutResponse) | Task 1 Steps 3b, 5, 7 |
| "New error variant" (`DuplicateCode`) | Task 1 Step 5 |
| "New credential DTOs" (View, Create, Update, Remove response) | Task 1 Step 5 |
| "New AuthService methods" (find, create, update, remove) | Task 1 Steps 5, 7 |
| "Module wiring" (no `lib.rs` change) | Task 1 Step 5 leaves `lib.rs` untouched |
| "Testing" (compile test exercises every new type) | Task 1 Steps 3, 7, 8 |
| "Out of scope" — explicitly NOT done (no user-crate, no migration, no adapter) | Plan only touches `apis/src/auth.rs` and `apis/tests/public_api.rs` |

No spec coverage gaps.

**2. Placeholder scan.** No "TBD", no "TODO", no "implement later", no "add appropriate error handling". Every step shows the exact code to write and the exact command to run. No step references types or functions not defined earlier in the same step or in the spec.

**3. Type consistency.** Cross-checked:

- `LogoutRequest { refresh_token: String }` is used consistently in the trait signature, in the test imports, and in the field-by-field test construction.
- `LogoutResponse {}` is consistent across the trait signature, the struct definition, the test helper, and the test field-by-field construction.
- `UserCredentialView { user_code, password_hash, token_version }` matches across the struct definition, the trait return types, and the test construction.
- `CreateUserCredentialRequest { user_code, password_hash }` matches across the struct definition (no `token_version`), the trait parameter, and the test construction.
- `UpdateUserCredentialRequest { user_code, password_hash: Option<String> }` matches across the struct definition, the trait parameter, and the test construction (`..Default::default()` for the all-`None` case).
- `RemoveUserCredentialResponse {}` matches across the struct definition, the trait return type, and the test construction.
- `AuthApiError::DuplicateCode(String)` is the same variant name in the enum, in the test reachability block, and in the trait doc for `create_user_credential`.
- The four trait methods appear in the same order in the trait, in the `FakeAuthService` impl, and in the spec.

No type mismatches.

**4. Other checks.**

- The `Send + Sync` and object-safety tests in `public_api.rs` (`auth_service_is_object_safe`, `auth_service_is_send_sync`) are not modified and still apply to the trait because the new methods preserve the same shape as the existing methods (async, `&self`, concrete types in and out).
- The crate's `Cargo.toml` is unchanged.
- The plan only touches the two files the spec called out.
