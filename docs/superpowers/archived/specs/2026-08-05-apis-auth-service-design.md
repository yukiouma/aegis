# apis AuthService Trait Design

## Goal

Add a second port to the `apis` workspace crate: an async `AuthService` trait that defines the authentication surface (login, verify, refresh, logout). Mirrors the layout and conventions established by the existing `apis::user` module — same file co-location pattern, same `Send + Sync` bound, same `#[async_trait]` style, same compile-test discipline.

The `apis` crate stays a self-contained contract; no dependency on `user` is added. The trait reuses `apis::user::Role` for the `AuthClaims` payload because a parallel `auth::Role` enum would force a second adapter-side conversion with no benefit.

## Crate layout

```text
lib/crates/apis/src/
  lib.rs        # pub mod auth; pub mod user;   (added line)
  auth.rs       # AuthClaims, TokenPair, AuthApiError,
                # LoginWithPasswordRequest, LoginWithDomainUserInfoRequest,
                # AuthService
  user.rs       # (unchanged)
```

## Public API

`lib/crates/apis/src/auth.rs` defines:

```rust
use async_trait::async_trait;
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

/// Outbound port for authentication.
///
/// `Send + Sync` so a `Box<dyn AuthService>` can be shared state in an
/// async server (axum, tarpc, etc.). Object-safe: no generic methods,
/// no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g. an
/// `auth::AuthUsecase` against the `user::UserUsecase`) into this
/// contract, translating between backend-specific DTOs / errors and
/// the `apis` types defined above.
#[async_trait]
pub trait AuthService: Send + Sync {
    /// Authenticate with a user code + password.
    ///
    /// On success mints a fresh access token and refresh token and
    /// returns them. Implementations check the password against the
    /// persisted hash and surface `InvalidCredentials` (not
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
    /// token and returns them. Implementations surface `NotFound` when
    /// no user maps to the supplied domain-identity triple.
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
    /// Returns `AuthClaims` on success. Token-format, signature, and
    /// expiry failures all surface as `AuthApiError::Verification`.
    async fn verify(&self, access_token: &str) -> Result<AuthClaims, AuthApiError>;

    /// Exchange a still-valid refresh token for a brand-new access token.
    ///
    /// Returns the freshly minted access token as a `String`.
    /// Expired or tampered-with refresh tokens surface as
    /// `AuthApiError::Verification`. The refresh token itself is not
    /// rotated — callers keep using the same refresh token until it
    /// expires.
    async fn refresh(&self, refresh_token: &str) -> Result<String, AuthApiError>;
}
```

Notes:

- Tokens are plain `String`s. The trait does not commit to JWT, PASETO, or any other wire format — signing is an implementation concern.
- `refresh` returns just the new access token; the refresh token itself is not rotated. Login methods still return `TokenPair` (both tokens fresh).
- Method parameters are `&str`; DTO struct fields are `String`. The DTOs are the owned-form for cross-boundary transport (HTTP / gRPC adapters), and the trait itself borrows because callers frequently already own an allocation they can lend.
- `AuthApiError` variants cover every failure mode the methods can produce. No `From<DomainError>` blanket conversion — adapters map backend errors explicitly, matching the convention used by `apis::user::UserApiError`.
- `AuthClaims::role` reuses `apis::user::Role`. No second `Role` enum is introduced; the conversion story is the same one already documented for `apis::user`.
- `Send + Sync` and `#[async_trait]` mirror `apis::user::UserService` exactly.

## Module wiring

`lib/crates/apis/src/lib.rs` becomes:

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

No other changes to `lib.rs`.

## Dependencies

`lib/crates/apis/Cargo.toml` needs no new entries. `async-trait` and `thiserror` are already workspace dependencies in this crate (they appear in the `apis` `Cargo.toml` for the existing `user` module).

## Testing

Extend `lib/crates/apis/tests/public_api.rs` with one more block for `auth`:

- Assert `AuthClaims`, `TokenPair`, `LoginWithPasswordRequest`, `LoginWithDomainUserInfoRequest`, `AuthApiError` are nameable from the crate root via `apis::auth::*`.
- Construct each DTO / payload field-by-field with literal `String`s / `&str`s to lock the field names and types.
- Assert `AuthService: Send + Sync` by way of `fn assert_send_sync<T: Send + Sync>()`.
- Assert object-safety by holding `Box<dyn AuthService>`.
- Lock the five `async fn` signatures with a minimal `impl AuthService for FakeAuthService` that returns `todo!()` from each method, then exercise each method through `Box<dyn AuthService>` (same pattern as `FakeUserService` further up in the file).
- Touch every `AuthApiError` variant to keep it from being dead-code-eliminated.

No live I/O, no `#[ignore]`-gated integration tests in this task — adapter-side integration tests belong with the concrete `AuthService` implementation, which is out of scope here.

## Out of scope

- A concrete `AuthService` implementation that adapts a backend usecase layer (auth or user) to this trait.
- HTTP / gRPC handler code that calls into `AuthService`.
- Token-storage strategy (in-memory revocation list, Redis, DB).
- Password-hashing policy. The trait does not constrain it.
- Refreshing the refresh token. The trait only mints a new access token on `refresh`; callers continue to use the same refresh token until it expires.

## Workspace integration

`apis` is already a workspace member (`Cargo.toml` line 4). No workspace-level changes are required beyond the file additions described above.
