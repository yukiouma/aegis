# apis UserService Trait Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace the `apis` placeholder with a standalone contract exposing an async `UserService` trait plus its supporting types (`Role`, `UserApiError`, `UserView`, `CreateUserRequest`, `UpdateUserRequest`).

**Architecture:** Single new module `apis::user` holds the trait and all referenced types. `apis` depends on no other workspace crate; the `user` crate is excluded by design so any backend can implement the trait.

**Tech Stack:** Rust 2024 edition, `async-trait 0.1.91`, `chrono 0.4` (`clock` feature), `thiserror 2`.

## Global Constraints

- `apis` MUST NOT depend on the `user` crate (`Cargo.toml` has no `user` entry).
- `apis` MUST NOT depend on `sqlx`, `tokio`, `argon2`, or `rand_core` — those are `user`-crate concerns.
- `apis` MUST re-use workspace dependencies for `async-trait`, `chrono`, and `thiserror` (no version overrides).
- The `UserService` trait MUST be `Send + Sync` so it can back shared server state.
- The `UserService` trait MUST be object-safe (no generic methods, no `Self` in return position beyond `&self`).
- `CreateUserRequest` and `UpdateUserRequest` MUST NOT contain a `password` field — passwords stay in the usecase layer.
- `UserView` MUST NOT contain any password / hash field — the trait can never leak a hash.
- `Role` MUST mirror `user::Role` variants (`Root`, `Admin`, `General`).
- `UserApiError` variants MUST be: `Validation(String)`, `NotFound`, `DuplicateCode(String)`, `Hashing(String)`, `Repository(String)` — same shape as the `user` crate's combined error surface but a single type.

---

### Task 1: Crate wiring (Cargo.toml, lib.rs, empty user module)

**Files:**
- Modify: `lib/crates/apis/Cargo.toml`
- Modify: `lib/crates/apis/src/lib.rs`
- Create: `lib/crates/apis/src/user.rs`

**Step 1: Replace `lib/crates/apis/Cargo.toml`**

Open the file and replace the entire `[dependencies]` block so it reads:

```toml
[package]
name = "apis"
version = "0.1.0"
edition = "2024"

[dependencies]
async-trait = { workspace = true }
chrono = { workspace = true }
thiserror = { workspace = true }
```

No `[dev-dependencies]` are needed — the public-api test only references types already in scope.

**Step 2: Replace `lib/crates/apis/src/lib.rs`**

Open the file and replace its entire contents with:

```rust
//! `apis` workspace crate.
//!
//! Hosts outbound port traits that adapters (HTTP/gRPC handlers,
//! other backends) consume. Each trait is a self-contained
//! contract: this crate does not depend on any other workspace
//! crate, so any backend can implement the traits by adapting its
//! own types to the ones defined here.

pub mod user;
```

**Step 3: Create the `user` module stub**

Create `lib/crates/apis/src/user.rs` containing only the module-level doc comment and the types that the rest of this plan will flesh out. For now, leave it with just the doc comment so the build can succeed:

```rust
//! Outbound port for user lifecycle operations.
//!
//! See [`UserService`] for the trait surface. All supporting
//! types (`Role`, `UserApiError`, `UserView`, `CreateUserRequest`,
//! `UpdateUserRequest`) are defined alongside the trait so a
//! single `use apis::user::*;` brings the whole contract into
//! scope.
```

(No items yet — those land in Task 2.)

**Step 4: Verify the crate builds**

Run:

```bash
cargo build -p apis
```

Expected: success, no warnings beyond what `cargo build` normally emits for an empty module.

**Step 5: Commit**

```bash
git add lib/crates/apis/Cargo.toml lib/crates/apis/src/lib.rs lib/crates/apis/src/user.rs
git commit -m "feat(apis): wire crate for user module and add chrono/thiserror deps"
```

---

### Task 2: Implement `UserService` and supporting types

**Files:**
- Modify: `lib/crates/apis/src/user.rs`
- Create: `lib/crates/apis/tests/public_api.rs`

**Interfaces:**
- Consumes: nothing (this task is the producer of the contract).
- Produces: `apis::user::{Role, UserApiError, UserView, CreateUserRequest, UpdateUserRequest, UserService}`. Trait method shapes — see Step 3.

**Step 1: Write the failing compile-only test**

Create `lib/crates/apis/tests/public_api.rs`:

```rust
//! Public-API compile test for the `apis` crate.
//!
//! Does NOT run any I/O. Locks the documented trait surface and
//! the in-crate type names so a regression in `user.rs` is caught
//! at `cargo test -p apis` time.

use apis::user::{
    CreateUserRequest, Role, UpdateUserRequest, UserApiError, UserService, UserView,
};

/// Every public type in `apis::user` is nameable from the test.
#[test]
fn public_types_are_nameable() {
    fn assert_role(_: Role) {}
    fn assert_view(_: UserView) {}
    fn assert_create(_: CreateUserRequest) {}
    fn assert_update(_: UpdateUserRequest) {}
    fn assert_err(_: UserApiError) {}

    // `Role` is constructible from its variants.
    assert_role(Role::General);
    // `UserView` is constructible field-by-field.
    assert_view(UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
        updated_at: chrono::DateTime::from_timestamp(0, 0).unwrap(),
    });
    // `CreateUserRequest` has no `password` field — this is the
    // shape adapters receive from outside the backend.
    assert_create(CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
    });
    assert_update(UpdateUserRequest {
        id: 1,
        ..Default::default()
    });

    // Touch the error type to keep it from being dead-code-eliminated
    // by the test build's analysis.
    let _: UserApiError = UserApiError::NotFound;
    let _ = assert_err;
}

/// Minimal in-test implementation used to lock the trait's
/// signature, object-safety, and `Send + Sync` bounds. Each method
/// returns `todo!()` because the test only exercises the type
/// system — never the runtime behavior.
struct FakeUserService;

#[async_trait::async_trait]
impl UserService for FakeUserService {
    async fn create(&self, _req: CreateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_id(&self, _id: i32) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn get_by_code(&self, _code: &str) -> Result<UserView, UserApiError> {
        todo!()
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        todo!()
    }
    async fn update(&self, _req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        todo!()
    }
}

/// `UserService` is object-safe: it can be held behind a `Box<dyn …>`.
#[test]
fn user_service_is_object_safe() {
    let _boxed: Box<dyn UserService> = Box::new(FakeUserService);
}

/// `UserService` requires `Send + Sync`, so a `Box<dyn UserService>`
/// is itself `Send + Sync` and can be shared state in an async server.
#[test]
fn user_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<Box<dyn UserService>>();
    assert_send_sync::<&FakeUserService>();
}
```

**Step 2: Run the test to confirm it fails**

Run:

```bash
cargo test -p apis --test public_api
```

Expected: FAIL — the compiler complains that `apis::user::{CreateUserRequest, …, UserService}` do not exist (the module is still empty from Task 1).

**Step 3: Implement the types and trait in `user.rs`**

Replace the contents of `lib/crates/apis/src/user.rs` with:

```rust
//! Outbound port for user lifecycle operations.
//!
//! See [`UserService`] for the trait surface. All supporting
//! types (`Role`, `UserApiError`, `UserView`, `CreateUserRequest`,
//! `UpdateUserRequest`) are defined alongside the trait so a
//! single `use apis::user::*;` brings the whole contract into
//! scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Role of a user within the system.
///
/// Mirrors `user::Role` so adapters between the two crates can
/// convert losslessly. Kept independent here so `apis` does not
/// depend on the `user` crate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Role {
    Root,
    Admin,
    General,
}

/// Error surface returned by every [`UserService`] method.
///
/// Adapters map backend-specific errors (e.g. `user::UsecaseError`)
/// into this type at the implementation boundary. The shape
/// intentionally combines validation, lookup, and infrastructure
/// concerns into a single type so handlers can match exhaustively.
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

/// Safe projection of a user — no password / hash field, by
/// construction. This is what adapters hand back to whatever
/// consumes the API.
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

/// Input DTO for creating a user.
///
/// Deliberately omits `password` — the password-hashing policy
/// lives in the backend's usecase layer. Adapters receive this
/// shape from outside and translate it into a backend-specific
/// create DTO that includes the password.
pub struct CreateUserRequest {
    pub code: String,
    pub name: String,
    pub role: Role,
}

/// Input DTO for updating a user.
///
/// Every field except `id` is optional; only the fields that
/// actually changed need to be supplied. Same rationale as
/// [`CreateUserRequest`] for the omission of `password`.
#[derive(Default)]
pub struct UpdateUserRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub role: Option<Role>,
    pub active: Option<bool>,
}

/// Outbound port for user lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn UserService>` can be shared state
/// in an async server (axum, tarpc, etc.). Object-safe: no generic
/// methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `user::UserUsecase`) into this contract, translating between
/// backend-specific DTOs / errors and the `apis` types defined
/// above. The `password` field never appears on this trait's
/// surface.
#[async_trait]
pub trait UserService: Send + Sync {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError>;

    async fn get_by_id(&self, id: i32) -> Result<UserView, UserApiError>;

    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError>;

    async fn list(&self) -> Result<Vec<UserView>, UserApiError>;

    async fn update(&self, req: UpdateUserRequest) -> Result<UserView, UserApiError>;
}
```

**Step 4: Run the test to confirm it passes**

Run:

```bash
cargo test -p apis --test public_api
```

Expected: PASS, three tests (`public_types_are_nameable`, `user_service_is_object_safe`, `user_service_is_send_sync`) all green. No `todo!()` panic at runtime because none of the methods are actually called.

**Step 5: Run a full workspace build to make sure nothing else regressed**

Run:

```bash
cargo build --workspace
```

Expected: success. The `apis` crate is consumed by no other workspace crate yet, but the build confirms the new dependencies resolve everywhere.

**Step 6: Commit**

```bash
git add lib/crates/apis/src/user.rs lib/crates/apis/tests/public_api.rs
git commit -m "feat(apis): add UserService trait and supporting types"
```