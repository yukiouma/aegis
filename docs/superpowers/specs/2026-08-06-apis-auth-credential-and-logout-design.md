# apis AuthService — Credential CRUD & Refresh-Token Logout

## Goal

Extend the `apis::auth` port in two ways:

1. **Logout by refresh token, not by user code.** Switch `LogoutRequest` to carry the refresh token, drop the `LogoutResponse` DTO, and have `logout` return `Result<(), AuthApiError>`.
2. **Add a credential-management surface to `AuthService`.** Four new methods (find by code, create, update, remove) plus the supporting request / view DTOs.

Scope is the `apis` crate only — no `user`-crate domain type, no migration, no `AuthService` adapter against a real backend. The trait stays a self-contained contract; a follow-up task wires it to persistence.

## Crate layout

```text
lib/crates/apis/src/
  lib.rs        # unchanged
  auth.rs       # trait + DTOs (modified in place)
  user.rs       # unchanged
```

`lib/crates/apis/Cargo.toml` needs no new entries. `async-trait` and `thiserror` are already workspace dependencies here.

## Public API — changes to `auth.rs`

### Logout signature change

The current `LogoutRequest { code }` is replaced with `LogoutRequest { refresh_token }`. `LogoutResponse` is removed. The `logout` method's return type becomes `Result<(), AuthApiError>`.

```rust
/// Input DTO for [`AuthService::logout`].
#[derive(Debug, Clone)]
pub struct LogoutRequest {
    pub refresh_token: String,
}

// (LogoutResponse removed.)

// In the trait:
async fn logout(&self, req: LogoutRequest) -> Result<(), AuthApiError>;
```

The doc on `logout` is rewritten to: "Invalidate the session identified by `req.refresh_token`. The implementation looks up the token, removes any stored refresh-token entry, and returns `Ok(())`. Returns `Ok(())` even when the token had no active session (idempotent). A malformed or already-revoked refresh token surfaces as `AuthApiError::Verification`. Storage failures surface as `AuthApiError::Repository`."

### New error variant

`AuthApiError` gains `DuplicateCode(String)`, paralleling `UserApiError::DuplicateCode`. The wrapping string carries the offending `user_code` so callers can surface it.

```rust
#[derive(Debug, Error)]
pub enum AuthApiError {
    // ... existing variants unchanged ...

    #[error("user credential already exists: {0}")]
    DuplicateCode(String),
}
```

### New credential DTOs

```rust
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
```

### New `AuthService` methods

Added to the trait after `refresh` and before `logout`, grouped under a `// credential management` block comment so the file's sectioning stays predictable.

```rust
#[async_trait::async_trait]
pub trait AuthService: Send + Sync {
    // ... existing login_*, verify, refresh unchanged ...

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
    /// credential exists. An all-`None` `req` (other than
    /// `user_code`) is permitted and behaves as a read.
    async fn update_user_credential(
        &self,
        req: UpdateUserCredentialRequest,
    ) -> Result<UserCredentialView, AuthApiError>;

    /// Delete the credential row for `code`. Returns `NotFound` if
    /// no such credential exists.
    async fn remove_user_credential(
        &self,
        code: &str,
    ) -> Result<(), AuthApiError>;

    // -- session lifecycle --------------------------------------------

    /// (signature updated; see above)
    async fn logout(&self, req: LogoutRequest) -> Result<(), AuthApiError>;
}
```

Single-arg methods (`find_user_credential_by_code`, `remove_user_credential`) take `&str` to match `UserService::get_by_code` and `UserService`-style ergonomic call sites. Multi-arg methods take request DTOs to match the rest of `AuthService` (see the recent "wrap method parameters into request DTOs" commit).

## Module wiring

`lib/crates/apis/src/lib.rs` is unchanged. `pub mod auth;` already exposes the module.

## Testing

Extend `lib/crates/apis/tests/public_api.rs` to lock the new surface:

- Remove the `LogoutResponse` import and the `assert_logout_res` helper. Remove the `assert_logout_res(LogoutResponse { code: "u1".into() })` call.
- Update `FakeAuthService::logout` to return `Result<(), AuthApiError>`.
- Add imports for `CreateUserCredentialRequest`, `UpdateUserCredentialRequest`, `UserCredentialView`.
- Add field-by-field construction calls for each new DTO in the `auth_public_types_are_nameable` test.
- Add the four new `FakeAuthService` method stubs returning `todo!()`.
- Touch `AuthApiError::DuplicateCode("".into())` in the variant-reachability block.

No live I/O, no `#[ignore]`-gated integration tests. The compile-test discipline matches the rest of the crate.

## Out of scope

- A `UserCredential` domain type, repository port, in-memory facade, or PostgreSQL migration in the `user` crate.
- An `AuthService` adapter that wires the trait to a real backend.
- HTTP / gRPC handlers that call into the new credential methods.
- An admin "invalidate all sessions" entry point that mutates `token_version` directly. (The trait exposes `token_version` on the view; bumping it requires a future method that is not in this spec.)
- Any change to the `login_with_*` or `verify` semantics around `token_version`. The new methods only add surface; they do not change the existing flows.

## Workspace integration

`apis` is already a workspace member. No `Cargo.toml` changes are needed at the workspace level or in the `apis` crate (its `Cargo.toml` already has `async-trait` and `thiserror`).
