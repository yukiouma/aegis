# apis UserService Trait Design

## Goal

Add the first port to the `apis` workspace crate: an async `UserService` trait that mirrors the `user` crate's `UserUsecase` surface, minus the soft-delete (`deactivate`) method. The `apis` crate must remain a self-contained contract — no dependency on the `user` crate — so any backend can implement the trait by adapting its own types to the `apis` types.

## Crate layout

The `apis` crate is currently a placeholder (`src/lib.rs` exposes a default `add` function). Replace it with a single new module:

```text
lib/crates/apis/src/
  lib.rs        # pub mod user;
  user.rs       # Role, UserApiError, UserView, CreateUserRequest, UpdateUserRequest, UserService
```

One file keeps the trait and its supporting types co-located, matching the existing crate's "tiny surface" character.

## Public API

`lib/crates/apis/src/user.rs` defines:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Root,
    Admin,
    General,
}

#[derive(Debug, Error)]
pub enum UserApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("user not found")]
    NotFound,

    #[error("user code already exists: {0}")]
    DuplicateCode(String),

    #[error("password hashing failed: {0}")]
    Hashing(String),

    #[error("repository error: {0}")]
    Repository(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

#[derive(Default)]
pub struct UpdateUserRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

#[async_trait]
pub trait UserService: Send + Sync {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError>;
    async fn get_by_id(&self, id: i32) -> Result<UserView, UserApiError>;
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError>;
    async fn list(&self) -> Result<Vec<UserView>, UserApiError>;
    async fn update(&self, req: UpdateUserRequest) -> Result<UserView, UserApiError>;
}
```

Notes:

- No `deactivate` method — the trait deliberately omits the soft-delete the `user` crate exposes.
- No `password` field on `CreateUserRequest` or `UpdateUserRequest` — password handling is a usecase-layer concern. Adapters (the future `user`-backed implementation, or any HTTP/gRPC handler) translate incoming requests into backend-specific DTOs that include a password.
- `UserView` excludes any password field by construction; the trait can never leak a hash.
- `Role` is a separate type from `user::Role`. Conversion between the two lives at the adapter boundary.
- `UserApiError` mirrors the shape of `user::UsecaseError` (`Validation`, `NotFound`, `DuplicateCode`, `Hashing`, `Repository`) without depending on it. The `From<DomainError>` shape used by `user` is not adopted — adapters map errors explicitly.
- `Send + Sync` on the trait so it can be held as shared state in axum/tarpc-style servers.
- `#[async_trait]` matches the convention already used by `user::UserRepository`.

## Module wiring

`lib/crates/apis/src/lib.rs` becomes:

```rust
pub mod user;
```

(The placeholder `add` function and its test are removed.)

## Dependencies

Update `lib/crates/apis/Cargo.toml`:

```toml
[dependencies]
async-trait = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
```

`chrono` is needed for `UserView::created_at` / `updated_at`. `thiserror` is already used by the `user` crate's error types; using the same workspace dep keeps the error message format consistent. `user` is intentionally **not** added — `apis` is a standalone contract.

## Testing

Add `lib/crates/apis/tests/public_api.rs`, a compile-only test mirroring `user`'s `tests/public_api.rs`:

- Assert `CreateUserRequest`, `UpdateUserRequest`, `UserView`, `Role`, `UserApiError` are nameable from the crate root via `apis::user::*`.
- Assert `UserService: Send + Sync` so it can back shared server state.
- Assert object-safety by holding `Box<dyn UserService>`.
- Lock the documented `async fn` signatures with a minimal in-test `impl UserService` that returns `todo!()` from each method, then exercise each method through `Box<dyn UserService>`. (`#[async_trait]` `async fn`s cannot be captured as raw `fn` pointers, so the locking is done by exercising the trait through a `dyn` reference rather than `fn`-pointer assignment.)

No live I/O, no `#[ignore]`-gated integration tests in this task — adapter-side integration tests belong with the concrete `UserService` implementation, which is out of scope here.

## Out of scope

- A concrete `UserService` implementation that adapts `user::UserUsecase` to these types.
- HTTP/gRPC handler code.
- Pagination on `list`.
- Authentication / password-verification entry point.

Those land in follow-up specs once the trait exists.

## Workspace integration

`apis` is already a member of the root workspace (`Cargo.toml` line 4). No workspace-level changes are required beyond the crate's own `Cargo.toml` update.