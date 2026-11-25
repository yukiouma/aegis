# User-Service Facade Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `user::adapter::facade::in_memory::UserServiceImpl<R>`, an implementation of `apis::user::UserService` that adapts `UserUsecase<R>` to the API contract, plus unit tests with an in-memory fake repository.

**Architecture:** A thin facade struct wraps a `UserUsecase<R>` and translates the per-call request/response/error shapes between the `apis` crate and the `user` crate. The conversion lives inline in each trait method, with a single `From<UsecaseError> for UserApiError` impl. The type is generic over `R: UserRepository`, mirroring `UserUsecase<R>`. Tests use an in-memory `UserRepository` so no PostgreSQL connection is required.

**Tech Stack:** Rust (edition 2024), `async-trait`, `thiserror`, `chrono::DateTime<Utc>`, `std::sync::Mutex` + `std::sync::atomic::AtomicI32` for the in-memory fake.

## File Structure

New files:

- `lib/crates/user/src/adapter/facade.rs` — module declaration; `pub use in_memory::UserServiceImpl;`
- `lib/crates/user/src/adapter/facade/in_memory.rs` — `UserServiceImpl<R>`, the `UserService` impl, role helpers, `From<UsecaseError> for UserApiError`, and the `#[cfg(test)] mod tests;` declaration.
- `lib/crates/user/src/adapter/facade/in_memory/tests.rs` — in-memory `UserRepository` fake and per-method tests.

Modified files:

- `lib/crates/user/Cargo.toml` — add `apis = { path = "../apis" }`.
- `lib/crates/user/src/adapter.rs` — add `mod facade;` (above `mod persistence;`).
- `lib/crates/user/src/lib.rs` — add `pub use adapter::UserServiceImpl;` next to the existing `UserRepo` / `UserUsecase` re-exports.

## Global Constraints

- **Module visibility**: `in_memory` is `pub` only inside `facade`; only `UserServiceImpl` reaches the outside world via re-export. Mirrors how `postgres` is structured today (the `postgres` child is `pub`, only `UserRepo` reaches the outside).
- **No new external dependencies** — the `apis` path dep is the only addition. The fake repo uses `std::sync::Mutex` + `std::sync::atomic::AtomicI32` (no tokio `sync` feature, no parking_lot).
- **No password / hashing logic** — `apis::user::CreateUserRequest` has no `password` field; `UserApiError::Hashing` is unreachable from this implementation and is not produced here.
- **Role mapping is exhaustive**: `apis::user::Role` ↔ `user::domain::Role` cover exactly the same three variants. Use helper `fn`s (orphan rule blocks bidirectional `From` impls across crates).
- **Doc comments on every public type** match the project's existing tone (sentence case, why-then-what, references to neighbours).

---

### Task 1: Add the `apis` path dependency to the `user` crate

**Files:**
- Modify: `lib/crates/user/Cargo.toml`

The `user` crate must depend on `apis` so `in_memory.rs` can import `apis::user::*`. We use a path dep because both crates live in the same workspace.

- [ ] **Step 1: Edit `lib/crates/user/Cargo.toml`**

Add `apis = { path = "../apis" }` to `[dependencies]`. Insert it after the existing `chrono` line:

```toml
[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `chrono` provides the `DateTime<Utc>` type used by the domain
# `User` and the usecase `UserView` to carry the `created_at` /
# `updated_at` columns surfaced by the repository. The `clock` feature
# keeps the binary small (no local time zones) while still enabling
# `NOW()`-style integration through `chrono::Utc`.
chrono = { workspace = true }
# `apis` provides the outbound `UserService` port the facade
# implements. Path-dep because both crates share the workspace.
apis = { path = "../apis" }
```

- [ ] **Step 2: Verify the build still succeeds**

Run: `cargo build -p user`
Expected: `Compiling user v0.1.0` and `Finished` with no errors.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/user/Cargo.toml
git commit -m "build(user): depend on apis for the UserService facade"
```

---

### Task 2: Create the facade module skeleton

**Files:**
- Create: `lib/crates/user/src/adapter/facade.rs`
- Create: `lib/crates/user/src/adapter/facade/in_memory.rs`
- Modify: `lib/crates/user/src/adapter.rs`

The skeleton establishes the module tree and wires it into the adapter layer. `in_memory.rs` is empty for now; the next task fills it in.

- [ ] **Step 1: Create `lib/crates/user/src/adapter/facade.rs`**

```rust
//! Outbound-port adapters.
//!
//! Adapters in this sub-module implement API-facing traits defined
//! in other workspace crates (today: `apis::user::UserService`).
//! Each backend lives under its own child module so a second port
//! (e.g. a future gRPC facade) is purely additive.

mod in_memory;

pub use in_memory::UserServiceImpl;
```

- [ ] **Step 2: Create `lib/crates/user/src/adapter/facade/in_memory.rs`**

```rust
//! In-memory `UserService` adapter.
//!
//! Hosts `UserServiceImpl<R>`, the implementation of
//! `apis::user::UserService` that adapts `user::UserUsecase` to the
//! API contract. Behaviour is exercised by `tests`, which wires the
//! adapter on top of an in-memory `UserRepository` so no live
//! PostgreSQL connection is required.
```

- [ ] **Step 3: Edit `lib/crates/user/src/adapter.rs` to wire the new module**

Replace the existing body of `lib/crates/user/src/adapter.rs` with:

```rust
//! Adapter layer.
//!
//! Houses the persistence adapters that implement the
//! `UserRepository` port defined in the domain layer, plus outbound
//! port adapters (e.g. the `UserService` facade) that adapt the
//! usecase layer to API-facing traits defined in other workspace
//! crates.
//!
//! Storage-specific implementations live under
//! `persistence/<backend>/`. At the moment only the PostgreSQL
//! backend exists; the layer boundary re-exports `UserRepo` so
//! external callers can name it via the crate root
//! (`user::UserRepo`). API-facing adapters live under `facade/`.

mod facade;
mod persistence;

pub use facade::UserServiceImpl;
pub use persistence::postgres::UserRepo;
```

- [ ] **Step 4: Verify the build**

Run: `cargo build -p user`
Expected: `Finished` with no errors. The empty `in_memory` module compiles because it has no items.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/adapter.rs lib/crates/user/src/adapter/facade.rs lib/crates/user/src/adapter/facade/in_memory.rs
git commit -m "feat(user): scaffold adapter::facade::in_memory module"
```

---

### Task 3: Add `UserServiceImpl<R>`, the `From` impl, role helpers, and the in-memory fake repository

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs`
- Create: `lib/crates/user/src/adapter/facade/in_memory/tests.rs`

This task sets up everything except the per-trait-method bodies: the type, its constructor, the role conversion helpers, the `From<UsecaseError> for UserApiError` impl, and an in-memory `UserRepository` fake used by every subsequent test. A single smoke test asserts the scaffolding compiles and runs.

- [ ] **Step 1: Replace the body of `lib/crates/user/src/adapter/facade/in_memory.rs` with**

```rust
//! In-memory `UserService` adapter.
//!
//! Hosts `UserServiceImpl<R>`, the implementation of
//! `apis::user::UserService` that adapts `user::UserUsecase` to the
//! API contract. Behaviour is exercised by `tests`, which wires the
//! adapter on top of an in-memory `UserRepository` so no live
//! PostgreSQL connection is required.

use async_trait::async_trait;

use apis::user::{
    CreateUserRequest, UpdateUserRequest, UserApiError, UserService, UserView,
};
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, Role, UserRepository};
use crate::usecase::{CreateUser, UpdateUser, UsecaseError, UserUsecase};

/// Adapter that implements [`UserService`] on top of a
/// [`UserUsecase`].
///
/// Generic over the persistence port (`R: UserRepository`) so the
/// adapter can be exercised against in-memory fakes in tests and
/// against the PostgreSQL-backed [`UserRepo`](crate::UserRepo) in
/// production. Translation between `apis::user::*` and
/// `user::usecase::*` happens inline in each trait method.
pub struct UserServiceImpl<R: UserRepository> {
    usecase: UserUsecase<R>,
}

impl<R: UserRepository> UserServiceImpl<R> {
    /// Build a new `UserServiceImpl` wrapping the supplied usecase.
    pub fn new(usecase: UserUsecase<R>) -> Self {
        Self { usecase }
    }
}

/// Map the API's `Role` into the domain's `Role`. The two enums
/// share the same three variants; the match is exhaustive and the
/// compiler enforces it on either side.
fn to_internal_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

/// Inverse of [`to_internal_role`].
fn from_internal_role(r: Role) -> ApiRole {
    match r {
        Role::Root => ApiRole::Root,
        Role::Admin => ApiRole::Admin,
        Role::General => ApiRole::General,
    }
}

/// Translate a [`UsecaseError`] into the API's [`UserApiError`].
///
/// `UsecaseError::Validation` only ever wraps the validation-only
/// `DomainError` variants; the `unreachable!` arm in the
/// `Repository` branch documents that fact and would fire if a
/// future change ever broke the invariant.
impl From<UsecaseError> for UserApiError {
    fn from(err: UsecaseError) -> Self {
        match err {
            UsecaseError::Validation(domain) => UserApiError::Validation(domain.to_string()),
            UsecaseError::Repository(domain) => match domain {
                DomainError::NotFound => UserApiError::NotFound,
                DomainError::DuplicateCode(code) => UserApiError::DuplicateCode(code),
                DomainError::Repository(msg) => UserApiError::Repository(msg),
                DomainError::EmptyCode
                | DomainError::EmptyName
                | DomainError::InvalidRole(_) => unreachable!(
                    "domain validation errors are only produced as UsecaseError::Validation"
                ),
            },
        }
    }
}

#[cfg(test)]
mod tests;
```

- [ ] **Step 2: Create `lib/crates/user/src/adapter/facade/in_memory/tests.rs`**

```rust
//! Unit tests for `UserServiceImpl`.
//!
//! Wires the adapter on top of an in-memory `UserRepository` so the
//! behaviour is exercised without touching PostgreSQL.

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use apis::user::UserService;
use apis::user::Role as ApiRole;

use crate::domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};
use crate::usecase::UserUsecase;

use super::UserServiceImpl;

/// Fixed `DateTime<Utc>` returned by the fake repository for every
/// row it creates. Keeps the assertions readable.
fn epoch() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap()
}

/// In-memory `UserRepository` used by the facade tests.
///
/// `std::sync::Mutex` is sufficient because the async methods never
/// hold the lock across an `.await`. `AtomicI32` for `next_id`
/// avoids mutating the same byte as `users` from different threads.
#[derive(Default)]
struct InMemoryRepo {
    users: Mutex<Vec<User>>,
    next_id: AtomicI32,
}

impl InMemoryRepo {
    fn new() -> Self {
        Self {
            next_id: AtomicI32::new(1),
            ..Self::default()
        }
    }
}

#[async_trait]
impl UserRepository for InMemoryRepo {
    async fn create(&self, input: UserNew) -> Result<User, DomainError> {
        // Reject duplicate codes first so the caller can distinguish
        // collisions from id exhaustion.
        {
            let users = self.users.lock().unwrap();
            if users.iter().any(|u| u.code == input.code) {
                return Err(DomainError::DuplicateCode(input.code));
            }
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let now = epoch();
        let user = User::for_repository(
            id,
            input.code,
            input.name,
            input.role,
            input.active,
            now,
            now,
        );
        self.users.lock().unwrap().push(user.clone());
        Ok(user)
    }

    async fn find_by_id(&self, id: i32) -> Result<User, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn find_by_code(&self, code: &str) -> Result<User, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }

    async fn list(&self) -> Result<Vec<User>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }

    async fn update(&self, input: UserUpdate) -> Result<User, DomainError> {
        let mut users = self.users.lock().unwrap();
        let user = users
            .iter_mut()
            .find(|u| u.id == input.id)
            .ok_or(DomainError::NotFound)?;
        if let Some(ref new_code) = input.code {
            if users
                .iter()
                .any(|u| u.code == *new_code && u.id != input.id)
            {
                return Err(DomainError::DuplicateCode(new_code.clone()));
            }
            user.code = new_code.clone();
        }
        if let Some(ref new_name) = input.name {
            user.name = new_name.clone();
        }
        if let Some(new_role) = input.role {
            user.role = new_role;
        }
        if let Some(new_active) = input.active {
            user.active = new_active;
        }
        Ok(user.clone())
    }
}

/// Build a `UserServiceImpl` wired on top of `InMemoryRepo`.
fn service() -> UserServiceImpl<InMemoryRepo> {
    UserServiceImpl::new(UserUsecase::new(InMemoryRepo::new()))
}

/// Smoke test: the adapter can be constructed. Per-method
/// behaviour is covered by the per-method tasks that follow.
#[tokio::test]
async fn user_service_impl_can_be_constructed() {
    let _service = service();
}
```

- [ ] **Step 3: Verify the scaffolding builds and the smoke test passes**

Run: `cargo test -p user --lib adapter::facade::in_memory`
Expected: `1 passed` (the smoke test).

- [ ] **Step 4: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): scaffold UserServiceImpl with From impl and in-memory fake repo"
```

---

### Task 4: Implement `create()` (TDD)

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs` — add the `create` method to the `impl UserService for UserServiceImpl<R>` block.
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs` — add the test cases.

The facade's `create` converts a `CreateUserRequest` into a `CreateUser` usecase command, calls `usecase.create`, then projects the resulting `UserView` into the API's `UserView` shape.

- [ ] **Step 1: Write the failing tests**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs` (before the final `}` of the file):

```rust
#[tokio::test]
async fn create_returns_view_with_assigned_id_and_active_true() {
    let svc = service();
    let view = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::Admin,
        })
        .await
        .unwrap();
    assert_eq!(view.id, 1);
    assert_eq!(view.code, "u1");
    assert_eq!(view.name, "Alice");
    assert_eq!(view.role, ApiRole::Admin);
    assert!(view.active);
    assert_eq!(view.created_at, epoch());
    assert_eq!(view.updated_at, epoch());
}

#[tokio::test]
async fn create_rejects_empty_code_with_validation() {
    let svc = service();
    let err = svc
        .create(apis::user::CreateUserRequest {
            code: "  ".into(),
            name: "Alice".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::Validation(_)));
}

#[tokio::test]
async fn create_rejects_duplicate_code() {
    let svc = service();
    svc.create(apis::user::CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: ApiRole::General,
    })
    .await
    .unwrap();
    let err = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Bob".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::user::UserApiError::DuplicateCode(ref c) if c == "u1"
    ));
}
```

- [ ] **Step 2: Run the new tests and verify they fail**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::create`
Expected: compile error (`create` not implemented on `UserServiceImpl`) — this is the expected "fail" for the TDD cycle.

- [ ] **Step 3: Add the `UserService` impl skeleton and the `create` method to `lib/crates/user/src/adapter/facade/in_memory.rs`**

Insert this block immediately above the `#[cfg(test)] mod tests;` line:

```rust
#[async_trait]
impl<R: UserRepository> UserService for UserServiceImpl<R> {
    async fn create(&self, req: CreateUserRequest) -> Result<UserView, UserApiError> {
        let cmd = CreateUser {
            code: req.code,
            name: req.name,
            role: to_internal_role(req.role),
        };
        let view = self.usecase.create(cmd).await?;
        Ok(UserView {
            id: view.id,
            code: view.code,
            name: view.name,
            role: from_internal_role(view.role),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        })
    }
}
```

- [ ] **Step 4: Run the new tests and verify they pass**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::create`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): implement UserService::create on UserServiceImpl"
```

---

### Task 5: Implement `get_by_id()` (TDD)

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs` — extend the `UserService` impl.
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs` — add the tests.

`get_by_id` is the simplest method: the input shape is a primitive `i32`, the output is the same `UserView` projection `create` already uses. Factor the projection into a private helper `fn user_view_from_internal(view: crate::usecase::UserView) -> UserView` to keep the per-method bodies uniform.

- [ ] **Step 1: Refactor: extract the `UserView` projection**

Replace the `Ok(UserView { ... })` block inside `create` with:

```rust
        let view = self.usecase.create(cmd).await?;
        Ok(user_view_from_internal(view))
```

Add the helper just above the `From<UsecaseError>` impl block:

```rust
/// Project the usecase-layer `UserView` into the API-layer
/// `UserView`. Field-for-field because the two structs are kept
/// identical by design.
fn user_view_from_internal(view: crate::usecase::UserView) -> UserView {
    UserView {
        id: view.id,
        code: view.code,
        name: view.name,
        role: from_internal_role(view.role),
        active: view.active,
        created_at: view.created_at,
        updated_at: view.updated_at,
    }
}
```

- [ ] **Step 2: Write the failing tests**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn get_by_id_returns_seeded_user() {
    let svc = service();
    let created = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::Admin,
        })
        .await
        .unwrap();
    let fetched = svc.get_by_id(created.id).await.unwrap();
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn get_by_id_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc.get_by_id(999).await.unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::NotFound));
}
```

- [ ] **Step 3: Run the new tests and verify they fail to compile**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::get_by_id`
Expected: compile error (`get_by_id` not implemented).

- [ ] **Step 4: Add `get_by_id` to the `UserService` impl**

Inside the existing `impl<R: UserRepository> UserService for UserServiceImpl<R>`, add a new method (after `create`):

```rust
    async fn get_by_id(&self, id: i32) -> Result<UserView, UserApiError> {
        let view = self.usecase.get_by_id(id).await?;
        Ok(user_view_from_internal(view))
    }
```

- [ ] **Step 5: Run the tests and verify they pass**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::get_by_id`
Expected: 2 passed.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): implement UserService::get_by_id on UserServiceImpl"
```

---

### Task 6: Implement `get_by_code()` (TDD)

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs`
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs`

`get_by_code` is identical in shape to `get_by_id` but takes `&str`. The usecase already validates non-empty codes; we just forward.

- [ ] **Step 1: Write the failing tests**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn get_by_code_returns_seeded_user() {
    let svc = service();
    let created = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::Admin,
        })
        .await
        .unwrap();
    let fetched = svc.get_by_code("u1").await.unwrap();
    assert_eq!(fetched, created);
}

#[tokio::test]
async fn get_by_code_returns_not_found_for_unknown_code() {
    let svc = service();
    let err = svc.get_by_code("ghost").await.unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::NotFound));
}

#[tokio::test]
async fn get_by_code_rejects_empty_code_with_validation() {
    let svc = service();
    let err = svc.get_by_code("   ").await.unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::Validation(_)));
}
```

- [ ] **Step 2: Run the new tests and verify they fail to compile**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::get_by_code`
Expected: compile error (`get_by_code` not implemented).

- [ ] **Step 3: Add `get_by_code` to the `UserService` impl**

Inside the existing `impl<R: UserRepository> UserService for UserServiceImpl<R>`, add a new method (after `get_by_id`):

```rust
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        let view = self.usecase.get_by_code(code).await?;
        Ok(user_view_from_internal(view))
    }
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::get_by_code`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): implement UserService::get_by_code on UserServiceImpl"
```

---

### Task 7: Implement `list()` (TDD)

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs`
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs`

`list` projects each `Vec<UserView>` element through `user_view_from_internal`.

- [ ] **Step 1: Write the failing test**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn list_returns_all_seeded_users_in_insertion_order() {
    let svc = service();
    for (code, name) in [("u1", "Alice"), ("u2", "Bob"), ("u3", "Carol")] {
        svc.create(apis::user::CreateUserRequest {
            code: code.into(),
            name: name.into(),
            role: ApiRole::General,
        })
        .await
        .unwrap();
    }
    let list = svc.list().await.unwrap();
    assert_eq!(list.len(), 3);
    assert_eq!(list[0].code, "u1");
    assert_eq!(list[1].code, "u2");
    assert_eq!(list[2].code, "u3");
}

#[tokio::test]
async fn list_returns_empty_vec_when_no_users_exist() {
    let svc = service();
    let list = svc.list().await.unwrap();
    assert!(list.is_empty());
}
```

- [ ] **Step 2: Run the new tests and verify they fail to compile**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::list`
Expected: compile error (`list` not implemented).

- [ ] **Step 3: Add `list` to the `UserService` impl**

Inside the existing `impl<R: UserRepository> UserService for UserServiceImpl<R>`, add a new method (after `get_by_code`):

```rust
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        let views = self.usecase.list().await?;
        Ok(views.into_iter().map(user_view_from_internal).collect())
    }
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::list`
Expected: 2 passed.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): implement UserService::list on UserServiceImpl"
```

---

### Task 8: Implement `update()` (TDD)

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory.rs`
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs`

`update` converts the optional `ApiRole` on `UpdateUserRequest` to the internal `Role` (via `to_internal_role`/`Option::map`), forwards, and projects the result.

- [ ] **Step 1: Write the failing tests**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn update_applies_supplied_fields_and_returns_view() {
    let svc = service();
    let created = svc
        .create(apis::user::CreateUserRequest {
            code: "u1".into(),
            name: "Alice".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap();
    let updated = svc
        .update(apis::user::UpdateUserRequest {
            id: created.id,
            name: Some("Alicia".into()),
            role: Some(ApiRole::Admin),
            active: Some(false),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(updated.id, created.id);
    assert_eq!(updated.code, "u1");
    assert_eq!(updated.name, "Alicia");
    assert_eq!(updated.role, ApiRole::Admin);
    assert!(!updated.active);
}

#[tokio::test]
async fn update_returns_not_found_for_unknown_id() {
    let svc = service();
    let err = svc
        .update(apis::user::UpdateUserRequest {
            id: 999,
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(err, apis::user::UserApiError::NotFound));
}

#[tokio::test]
async fn update_rejects_duplicate_code() {
    let svc = service();
    svc.create(apis::user::CreateUserRequest {
        code: "u1".into(),
        name: "Alice".into(),
        role: ApiRole::General,
    })
    .await
    .unwrap();
    let second = svc
        .create(apis::user::CreateUserRequest {
            code: "u2".into(),
            name: "Bob".into(),
            role: ApiRole::General,
        })
        .await
        .unwrap();
    let err = svc
        .update(apis::user::UpdateUserRequest {
            id: second.id,
            code: Some("u1".into()),
            ..Default::default()
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::user::UserApiError::DuplicateCode(ref c) if c == "u1"
    ));
}
```

- [ ] **Step 2: Run the new tests and verify they fail to compile**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::update`
Expected: compile error (`update` not implemented).

- [ ] **Step 3: Add `update` to the `UserService` impl**

Inside the existing `impl<R: UserRepository> UserService for UserServiceImpl<R>`, add a new method (after `list`):

```rust
    async fn update(&self, req: UpdateUserRequest) -> Result<UserView, UserApiError> {
        let cmd = UpdateUser {
            id: req.id,
            code: req.code,
            name: req.name,
            role: req.role.map(to_internal_role),
            active: req.active,
        };
        let view = self.usecase.update(cmd).await?;
        Ok(user_view_from_internal(view))
    }
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test -p user --lib adapter::facade::in_memory::tests::update`
Expected: 3 passed.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/adapter/facade/in_memory.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): implement UserService::update on UserServiceImpl"
```

---

### Task 9: Add object-safety and `Send + Sync` assertions, re-export `UserServiceImpl`, run the full verification

**Files:**
- Modify: `lib/crates/user/src/adapter/facade/in_memory/tests.rs` — append object-safety and `Send + Sync` assertions.
- Modify: `lib/crates/user/src/lib.rs` — re-export `UserServiceImpl` at the crate root.

These final pieces lock in the trait-level invariants the `apis` crate's own tests already check, plus the crate-root re-export that consumers will use.

- [ ] **Step 1: Append the object-safety and `Send + Sync` tests**

Append to `lib/crates/user/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn user_service_impl_is_object_safe() {
    let svc = service();
    let _boxed: Box<dyn UserService> = Box::new(svc);
}

#[tokio::test]
async fn user_service_impl_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<UserServiceImpl<InMemoryRepo>>();
    assert_send_sync::<Box<dyn UserService>>();
}
```

- [ ] **Step 2: Run the new tests**

Run: `cargo test -p user --lib adapter::facade::in_memory`
Expected: all tests in the module pass (smoke + per-method + object-safety + Send/Sync = 14 tests).

- [ ] **Step 3: Re-export `UserServiceImpl` from the crate root**

Edit `lib/crates/user/src/lib.rs`. In the final `pub use` block, add a line for the facade adapter next to `UserRepo`:

```rust
// Re-exports for the documented public surface.
//
// `UserRepo` is the SQLx-backed repository implementation that
// consumers wire into a `UserUsecase`. `User` and `UserRepository` are
// re-exported alongside it so consumers who only depend on the port
// can name the trait at the crate root. The error types and input
// DTOs (`UsecaseError`, `DomainError`, `UserNew`, `UserUpdate`) are
// re-exported so consumers can `match` on them and construct
// repository inputs without reaching into the internal modules.
// `UserServiceImpl` is the `apis::user::UserService` adapter that
// sits on top of `UserUsecase` so an API consumer (e.g. an axum
// router) can depend on the trait without touching the persistence
// or usecase modules directly.
pub use adapter::{UserRepo, UserServiceImpl};
pub use domain::{DomainError, Role, User, UserNew, UserRepository, UserUpdate};
pub use usecase::{CreateUser, UpdateUser, UsecaseError, UserUsecase, UserView};
```

- [ ] **Step 4: Verify the full crate compiles**

Run: `cargo build -p user`
Expected: `Finished` with no errors.

- [ ] **Step 5: Run the full crate test suite**

Run: `cargo test -p user`
Expected: all tests pass — the existing public-API and integration tests, plus the new `adapter::facade::in_memory` tests.

- [ ] **Step 6: Verify the sibling `apis` crate still compiles**

Run: `cargo test -p apis`
Expected: all tests pass (no regression in the trait contract).

- [ ] **Step 7: Commit**

```bash
git add lib/crates/user/src/lib.rs lib/crates/user/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(user): re-export UserServiceImpl and lock in object-safety / Send+Sync"
```

---

## Self-Review

**Spec coverage:**

| Spec requirement | Task |
|---|---|
| `UserServiceImpl<R>` struct + constructor | Task 3 |
| `From<UsecaseError> for UserApiError` mapping | Task 3 |
| Role helpers (both directions) | Task 3 |
| `create` impl with `From` error conversion | Task 4 |
| `get_by_id` impl | Task 5 |
| `get_by_code` impl | Task 6 |
| `list` impl | Task 7 |
| `update` impl | Task 8 |
| Object-safety / `Send + Sync` assertions | Task 9 |
| `apis` path dep in `Cargo.toml` | Task 1 |
| Module skeleton (`facade.rs` + `in_memory.rs` + `adapter.rs` wire) | Task 2 |
| In-memory fake `UserRepository` | Task 3 |
| Re-export from `adapter.rs` | Task 2 |
| Re-export from crate root | Task 9 |
| `cargo build -p user` succeeds | Tasks 1, 2, 3, 9 |
| `cargo test -p user` passes | Task 9 |
| `cargo test -p apis` still passes | Task 9 |

All spec requirements covered. No gaps.

**Placeholder scan:** No "TBD" / "TODO" / "implement later" / "similar to Task N" in any step. Every code block is complete.

**Type consistency:** `UserServiceImpl<R>` is referenced consistently across all tasks. `user_view_from_internal` is introduced in Task 5 and used unchanged in Tasks 5–8. `to_internal_role` / `from_internal_role` are introduced in Task 3 and used unchanged. Method signatures on `UserService` match the `apis::user::UserService` trait verbatim.