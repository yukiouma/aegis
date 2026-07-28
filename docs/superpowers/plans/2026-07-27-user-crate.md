# User Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a workspace library crate at `lib/crates/user` that exposes a SQLx/PostgreSQL-backed DDD user repository and asynchronous `UserUsecase` with password hashing and non-destructive deactivation.

**Architecture:** Use ports-and-adapters DDD. `domain` owns `User`, `Role`, validation, errors, and an async repository port; `usecase` owns commands, password hashing, and orchestration; `infrastructure` implements the port with SQLx and owns migrations. The crate root re-exports the consumer-facing API. Modules use modern Rust 2024 style: child files are declared alongside their parent `mod.rs`-free files.

**Tech Stack:** Rust 2024 edition, Cargo workspace, SQLx PostgreSQL runtime/Tokio, Tokio, Argon2, async-trait, thiserror, random salts from Argon2.

## Global Constraints

- Add `lib/crates/user` to the root Cargo workspace.
- Use the `2024` Rust edition for all newly added crates.
- Avoid `mod.rs`; use `src/<module>.rs` plus a `src/<module>/` directory of child files.
- Use asynchronous APIs returning typed `Result` values.
- Store roles as `root`, `admin`, and `general`.
- `code` is unique and required.
- Never expose or return the password hash from normal user outputs.
- Hash passwords with Argon2 and a cryptographically random salt in the usecase.
- Do not expose hard delete; deactivation sets `active = false` and retains the row.
- Include a SQLx migration for the `users` table.

---

### Task 1: Workspace and crate scaffolding

**Files:**
- Modify: `Cargo.toml`
- Create: `lib/crates/user/Cargo.toml`
- Create: `lib/crates/user/src/lib.rs`
- Create: `lib/crates/user/src/domain.rs`
- Create: `lib/crates/user/src/usecase.rs`
- Create: `lib/crates/user/src/infrastructure.rs`

**Interfaces:**
- Produces a compiling `user` crate and module boundaries for later tasks.
- The crate root will eventually export `User`, `Role`, usecase DTOs/errors, `UserUsecase`, and `UserRepo`.

- [ ] **Step 1: Add the member and dependency declarations**

Add `"lib/crates/user"` to the workspace members and declare package metadata with `edition = "2024"`. Use SQLx with `postgres`, `runtime-tokio`, and `macros`; Tokio with `macros` and `rt-multi-thread`; Argon2; `async-trait`; and `thiserror`.

- [ ] **Step 2: Add module declarations and re-exports**

Create `lib/crates/user/src/{domain,usecase,infrastructure}.rs` with empty module bodies. Declare them in `lib.rs` and re-export their eventual public types. Keep stubs minimal so the crate compiles before domain implementation.

- [ ] **Step 3: Verify scaffolding**

Run `cargo check -p user`. Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add Cargo.toml lib/crates/user
git commit -m "feat: scaffold user library crate"
```

### Task 2: Domain model, validation, errors, and repository port

**Files:**
- Create: `lib/crates/user/src/domain/user.rs`
- Create: `lib/crates/user/src/domain/role.rs`
- Create: `lib/crates/user/src/domain/error.rs`
- Create: `lib/crates/user/src/domain/repository.rs`
- Create: `lib/crates/user/src/domain/tests.rs`
- Modify: `lib/crates/user/src/domain.rs`

**Interfaces:**
- `pub struct User { pub id: i32, pub code: String, pub name: String, pub role: Role, pub active: bool, pub(crate) password: String }`.
- `pub enum Role { Root, Admin, General }`, with `as_str()` and `TryFrom<&str>`.
- `pub trait UserRepository: Send + Sync` with async `create`, `find_by_id`, `find_by_code`, `list`, `update`, and `deactivate`; no delete method.
- Domain errors cover empty code/name/password, invalid role, not found, duplicate code, and repository/domain conversion failures.

- [ ] **Step 1: Write failing domain tests**

Test that each role maps to and from its lowercase database value, unknown role strings fail, empty code/name are rejected, and a valid user passes validation. Assert that a user’s password is not exposed through its public projection/accessor.

- [ ] **Step 2: Run the focused tests and verify failure**

Run `cargo test -p user domain`. Expected: FAIL because domain types and validation are not implemented.

- [ ] **Step 3: Implement the domain types and port**

Replace the empty `domain.rs` with `mod user; mod role; mod error; mod repository;` and `#[cfg(test)] mod tests;`. Implement role conversion, user construction/validation, typed domain errors, and the async repository trait using `async-trait`. Use owned `User` values for repository results and an explicit update input that permits code/name/role/active/password changes while keeping plaintext password handling in the usecase.

- [ ] **Step 4: Run focused tests**

Run `cargo test -p user domain`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/domain.rs lib/crates/user/src/domain
git commit -m "feat: add user domain model and repository port"
```

### Task 3: Usecase commands and password hashing

**Files:**
- Create: `lib/crates/user/src/usecase/error.rs`
- Create: `lib/crates/user/src/usecase/commands.rs`
- Create: `lib/crates/user/src/usecase/user_usecase.rs`
- Create: `lib/crates/user/src/usecase/tests.rs`
- Modify: `lib/crates/user/src/usecase.rs`

**Interfaces:**
- `pub struct CreateUser { pub code: String, pub name: String, pub role: Role, pub password: String }`.
- `pub struct UpdateUser { pub id: i32, pub code: Option<String>, pub name: Option<String>, pub role: Option<Role>, pub active: Option<bool>, pub password: Option<String> }`.
- `pub struct UserUsecase<R: UserRepository> { repository: R }` with `pub fn new(repository: R) -> Self`.
- Async methods: `create`, `get_by_id`, `get_by_code`, `list`, `update`, and `deactivate`, returning users without password fields or hashes.

- [ ] **Step 1: Write failing usecase tests with a mock repository**

Cover create hashing, update password replacement hashing, retrieval/list projection without password, validation before repository calls, and deactivate setting `active` false. Verify the mock receives no plaintext password and that there is no hard-delete call.

- [ ] **Step 2: Run tests to verify failure**

Run `cargo test -p user usecase`. Expected: FAIL because the usecase and commands are absent.

- [ ] **Step 3: Implement usecase orchestration**

Replace the empty `usecase.rs` with `mod commands; mod error; mod user_usecase;` and `#[cfg(test)] mod tests;`. Use Argon2’s random salt generation and password hashing. Validate create/update inputs before repository calls. Convert the hashed password into the domain persistence input, map repository errors into usecase errors, and return a safe `UserView`/projection that omits `password`.

- [ ] **Step 4: Run focused tests**

Run `cargo test -p user usecase`. Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/user/src/usecase.rs lib/crates/user/src/usecase
git commit -m "feat: add user usecase and password hashing"
```

### Task 4: SQLx repository and migration

**Files:**
- Create: `lib/crates/user/src/infrastructure/user_repo.rs`
- Create: `lib/crates/user/src/infrastructure/row.rs`
- Create: `lib/crates/user/src/infrastructure/tests.rs`
- Modify: `lib/crates/user/src/infrastructure.rs`
- Create: `lib/crates/user/migrations/0001_create_users.sql`

**Interfaces:**
- `pub struct UserRepo { pool: sqlx::PgPool }`.
- `impl UserRepo { pub fn new(pool: PgPool) -> Self }`.
- Implement `UserRepository` with parameterized SQLx queries for create, find by ID/code, list, update, and deactivate.

- [ ] **Step 1: Write migration and mapping tests**

Test role-to-column values and row conversion for all valid roles plus an invalid role. Add a migration-content test asserting the users table includes all six required columns, a primary key, and a unique code constraint.

- [ ] **Step 2: Run focused tests to verify failure**

Run `cargo test -p user infrastructure`. Expected: FAIL because the repository and migration are absent.

- [ ] **Step 3: Implement migration**

Create `users` with `id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY`, unique non-null `code TEXT`, non-null `name TEXT`, constrained `role TEXT`, non-null `active BOOLEAN`, and non-null `password TEXT`. Add a check constraint limiting role values to `root`, `admin`, and `general`.

- [ ] **Step 4: Implement SQLx repository**

Replace the empty `infrastructure.rs` with `mod user_repo; mod row;` and `#[cfg(test)] mod tests;`. Use `sqlx::query_as!` or runtime `query_as` consistently with workspace build constraints. Map rows into `User`, convert role strings through `Role::try_from`, translate unique violations/not-found/database errors, and implement deactivation as an `UPDATE`, never `DELETE`.

- [ ] **Step 5: Run tests and check compilation**

Run `cargo test -p user` and `cargo check --workspace`. Expected: PASS. If offline SQLx macro metadata prevents compilation, use runtime query mapping or document the required `DATABASE_URL`/offline preparation rather than weakening repository behavior.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/user
git commit -m "feat: add postgres user repository and migration"
```

### Task 5: Public API integration and verification

**Files:**
- Modify: `lib/crates/user/src/lib.rs`
- Create: `lib/crates/user/tests/public_api.rs`
- Modify: `README.md` only if crate usage documentation is needed

**Interfaces:**
- Consumers can import `user::{UserRepo, UserUsecase, CreateUser, UpdateUser, Role, UserView}` and construct the requested dependency chain.

- [ ] **Step 1: Write the public API compile test**

Create a test that type-checks `let repo = UserRepo::new(pool); let usecase = UserUsecase::new(repo);` and imports the documented DTOs/role. Do not connect to PostgreSQL during this test.

- [ ] **Step 2: Complete re-exports and docs**

Re-export the public types, add rustdoc for constructors and operations, and ensure password-bearing internal fields are not public.

- [ ] **Step 3: Run all verification**

Run `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`, `cargo test --workspace`, and `cargo check --workspace`. Expected: all commands pass. Live PostgreSQL integration tests run only when configured with a database URL.

- [ ] **Step 4: Commit**

```bash
git add lib/crates/user README.md
git commit -m "feat: expose user crate public API"
```

## Self-review

- Spec coverage: workspace registration (Task 1), DDD layers (Tasks 2–4), public constructors/API (Task 5), role persistence (Tasks 2 and 4), password hashing (Task 3), no hard delete/deactivation (Tasks 3 and 4), migration (Task 4), and unit/integration verification (Tasks 2–5) are all covered.
- Edition/modernization: every newly added crate uses `edition = "2024"`, and module structure avoids `mod.rs`.
- Placeholder scan: no TBD/TODO or unspecified implementation steps are present.
- Type consistency: `UserRepository`, `UserRepo`, `UserUsecase<R>`, `CreateUser`, `UpdateUser`, and `UserView` are defined before their use by later tasks.
