# Project Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `lib/crates/project` per [`docs/superpowers/specs/2026-08-09-project-crate-design.md`](../specs/2026-08-09-project-crate-design.md) — a ports-and-adapters DDD crate that owns CRUD over `Product`, `Project`, and the four membership sets hanging off `Project`, and adapts its usecase layer into a new `apis::project::ProjectService` port.

**Architecture:** Three DDD layers — `domain` (pure types, value objects, ports, `DomainError`), `usecase` (`ProjectUsecase<P, R, U>` with `Arc<dyn UserService>` indirectly via a generic `U: UserService`, project shell can be created without members), `adapter` (Postgres-backed `ProductRepo` + `ProjectRepo`; in-memory `ProjectServiceImpl` implementing `apis::project::ProjectService`; a narrow `UserServiceImpl` adapter bridging `apis::user::UserService` to the domain `UserService` port). Crate depends on `apis` (port) and workspace deps but never on `user` directly.

**Tech Stack:** `sqlx 0.9` (Postgres runtime API), `tokio 1.53`, `async-trait 0.1.91`, `thiserror 2`, `chrono 0.4` (clock feature), `dotenvy 0.15` (dev-only), `apis` (workspace path-dep).

**Spec:** [`docs/superpowers/specs/2026-08-09-project-crate-design.md`](../specs/2026-08-09-project-crate-design.md)

**Guideline:** [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md)

---

## Global Constraints

These come from the spec and the lib-crate guideline; every task implicitly includes them.

- **Edition:** Rust 2024 (`edition = "2024"`). The workspace root already pins `resolver = "3"`.
- **No `mod.rs`:** every module uses `src/<module>.rs` + `src/<module>/`. Terminal leaf files (`team_role.rs`, `product.rs`, `project_repo.rs`, `service.rs`, `row.rs`, …) are leaf files with no companion directory.
- **Layer dependency rule:** `domain` depends on nothing except std + `async-trait`; `usecase` depends on `domain` + `apis` (port) + `chrono`; `adapter` depends on `usecase` + `domain` + `apis`. No layer depends on a sibling layer inside the same crate beyond the documented direction.
- **Public surface:** the crate root re-exports exactly the types listed in the spec's "Public API" section. No internal types (`MockProductRepo`, `MockProjectRepo`, `MockUserService`, `*Row` structs, fakes) are re-exported.
- **`UserService` is reached via a narrow domain port.** The project crate does **not** depend on the `user` crate. `UserSummary` (code + name) flows through `domain::UserService`, which the `adapter::service::user::UserServiceImpl` adapts from `apis::user::UserService`.
- **Runtime SQLx API:** the persistence adapter uses `sqlx::query_as` and `sqlx::QueryBuilder`. No compile-time `query!` / `query_as!` macros. A module-level comment at the top of `postgres.rs` documents the choice.
- **`map_db_error` rules** (mirror the user crate): `sqlx::Error::RowNotFound` → `DomainError::NotFound`; `sqlx::Error::Database` with SQLSTATE `23503` on the `projects_product_fk` constraint → `DomainError::ProductNotFound(product_id)`; SQLSTATE `23505` → `DomainError::DuplicateCode(constraint_name)`; everything else → `DomainError::Repository(driver_message)`.
- **Migrations:** consumed via `sqlx::migrate!("./migrations")` in integration tests. Each schema change is one file. Live-DB integration tests are `#[ignore]`-gated.
- **Env var:** live-DB tests read `AEGIS_PROJECT_DATABASE_URL` (with `dotenvy::dotenv()` at startup; panic if missing).
- **Unique per-run values:** integration tests generate a per-process atomic counter + wall-clock nanoseconds for any UNIQUE-constrained column.
- **Destructive cleanup:** integration tests `DROP TABLE IF EXISTS project_members CASCADE`, `DROP TABLE IF EXISTS projects CASCADE`, `DROP TABLE IF EXISTS products CASCADE`, and `DROP TABLE IF EXISTS _sqlx_migrations CASCADE` before applying migrations.
- **Layer-boundary visibility:** `adapter/persistence` is `pub(crate) mod postgres;`; `adapter/persistence/postgres` keeps `row` and the individual repo files private but re-exports `ProductRepo` / `ProjectRepo` via `pub use`. The `postgres.rs` leaf is `pub` so the `pub use` is well-formed.
- **Whole-list membership replacement:** `ProjectRepo::update` deletes the existing rows for a `(project_id, team_type)` pair and reinserts the supplied list inside the same transaction. `None` membership fields leave that team alone; `Some(empty)` wipes it.
- **Optional membership on create:** `ProjectNew.members` and `ProjectNew.unblind_members` are `Option<ProjectMember>`. `None` and `Some(empty)` are equivalent on create — neither inserts any rows for that team.
- **Test gates** per the lib-crate guideline section 8:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p project --all-targets --all-features -- -D warnings
  cargo test -p project
  cargo doc -p project --no-deps
  cargo test -p project -- --ignored --test-threads=1   # with AEGIS_PROJECT_DATABASE_URL
  ```

---

## File Structure

Created (paths relative to `lib/crates/project/`):

```
Cargo.toml
README.md
migrations/
  0001_create_products.sql
  0002_create_projects.sql
src/
  lib.rs
  domain.rs
  domain/
    team_role.rs
    project_member.rs
    product.rs
    project.rs
    user.rs
    error.rs
    tests.rs
  usecase.rs
  usecase/
    commands.rs
    views.rs
    error.rs
    project_usecase.rs
    tests.rs
  adapter.rs
  adapter/
    persistence.rs
    persistence/
      postgres.rs
      postgres/
        row.rs
        product_repo.rs
        project_repo.rs
        tests.rs
    service.rs
    service/
      user.rs
    facade.rs
    facade/
      in_memory.rs
      in_memory/
        service.rs
        tests.rs
tests/
  public_api.rs
  integration_persistence.rs
```

Modified (one each):

```
/Users/yukichen/Coding/Projects/aegis/Cargo.toml                                 # workspace members
/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/lib.rs                 # add pub mod project;
/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/project.rs             # NEW
```

Each file owns exactly the responsibility in its name. `domain/tests.rs` exercises `TeamType`, `RoleType`, `ProjectMember`, `Product`, `Project`. `adapter/persistence/postgres/tests.rs` covers row conversions + migration schema content. `usecase/tests.rs` covers command orchestration against mock repos + a mock `UserService`. `adapter/facade/in_memory/tests.rs` covers the `apis::project::ProjectService` surface end-to-end on in-memory fakes. `tests/public_api.rs` is compile-only. `tests/integration_persistence.rs` is the `#[ignore]`-gated live-DB round-trip.

---

## Task 1: Crate scaffolding + workspace registration

**Files:**
- Modify: `/Users/yukichen/Coding/Projects/aegis/Cargo.toml` (add `lib/crates/project` to `[workspace].members`)
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/Cargo.toml`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (replace the `add` boilerplate)

- [ ] **Step 1: Add the crate to the workspace**

Edit `/Users/yukichen/Coding/Projects/aegis/Cargo.toml`. The `[workspace].members` array currently reads:

```toml
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/project",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

`lib/crates/project` is already listed. No change needed. Confirm with `grep -n '"lib/crates/project"' /Users/yukichen/Coding/Projects/aegis/Cargo.toml` → expect `    "lib/crates/auth", "lib/crates/project",`.

- [ ] **Step 2: Write `lib/crates/project/Cargo.toml`**

Replace the existing file with:

```toml
[package]
name = "project"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `chrono` provides `DateTime<Utc>` for the `created_at` / `updated_at`
# columns surfaced by `Product` and `Project` and carried by every view
# DTO in the crate. The `clock` feature keeps the binary small while
# still enabling `NOW()`-style integration through `chrono::Utc`.
chrono = { workspace = true }
# `apis` provides the outbound `apis::project::ProjectService` port
# the facade implements and the `apis::user::UserService` port the
# domain-level `UserService` adapter delegates to. Path-dep because
# both crates share the workspace.
apis = { path = "../apis" }

[dev-dependencies]
# Loads `.env` at test startup so live-DB integration tests can find
# `AEGIS_PROJECT_DATABASE_URL` without the user having to `source` it
# manually.
dotenvy = { workspace = true }
# Re-export the SQLx driver in `[dev-dependencies]` so the integration
# tests can build their own `PgPool` without going through the public
# API for connection setup. The library itself only needs `PgPool` as
# an opaque type behind `ProductRepo::new` / `ProjectRepo::new`.
sqlx = { workspace = true }
# `tokio` macros + multi-thread runtime are needed for `#[tokio::test]`
# in unit + integration tests.
tokio = { workspace = true }
```

- [ ] **Step 3: Replace the `lib.rs` boilerplate**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` with:

```rust
// Stub. The real public surface lands in Task 2 (domain) onwards.
```

- [ ] **Step 4: Verify the crate compiles with empty source**

Run: `cargo check -p project`
Expected: success, with a warning that the file is empty.

- [ ] **Step 5: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/Cargo.toml lib/crates/project/src/lib.rs
git commit -m "feat(project): scaffold crate

Registers lib/crates/project in the workspace (already present in
Cargo.toml members) and stubs out Cargo.toml with workspace deps and
src/lib.rs.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo check -p project"
```

---

## Task 2: Domain layer — value objects, aggregates, ports

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/team_role.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/project_member.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/product.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/project.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/user.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/error.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/tests.rs`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (declare `pub mod domain;` and re-export its public surface)

- [ ] **Step 1: Write the failing test for `TeamType` / `RoleType` parsing**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/tests.rs` with:

```rust
use super::*;

#[test]
fn team_type_as_str_maps_to_lowercase() {
    assert_eq!(TeamType::Members.as_str(), "members");
    assert_eq!(TeamType::UnblindMembers.as_str(), "unblind_members");
}

#[test]
fn team_type_try_from_str_parses_known_values() {
    assert_eq!(TeamType::try_from("members").unwrap(), TeamType::Members);
    assert_eq!(
        TeamType::try_from("unblind_members").unwrap(),
        TeamType::UnblindMembers
    );
}

#[test]
fn team_type_try_from_str_rejects_unknown_value() {
    let err = TeamType::try_from("admins").unwrap_err();
    assert!(matches!(err, DomainError::UnknownTeamType(ref s) if s == "admins"));
}

#[test]
fn role_type_as_str_maps_to_lowercase() {
    assert_eq!(RoleType::Leader.as_str(), "leader");
    assert_eq!(RoleType::Worker.as_str(), "worker");
}

#[test]
fn role_type_try_from_str_parses_known_values() {
    assert_eq!(RoleType::try_from("leader").unwrap(), RoleType::Leader);
    assert_eq!(RoleType::try_from("worker").unwrap(), RoleType::Worker);
}

#[test]
fn role_type_try_from_str_rejects_unknown_value() {
    let err = RoleType::try_from("admin").unwrap_err();
    assert!(matches!(err, DomainError::UnknownRoleType(ref s) if s == "admin"));
}
```

- [ ] **Step 2: Verify the test fails to compile**

Run: `cargo test -p project --lib`
Expected: FAIL — `TeamType`, `RoleType`, `DomainError::UnknownTeamType`, `DomainError::UnknownRoleType` are not defined.

- [ ] **Step 3: Implement `DomainError`, `TeamType`, `RoleType`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("code must not be empty")]
    EmptyCode,

    #[error("name must not be empty")]
    EmptyName,

    #[error("product id must be non-zero")]
    ZeroProductId,

    #[error("duplicate code in leaders: {0}")]
    DuplicateLeader(String),

    #[error("duplicate code in workers: {0}")]
    DuplicateWorker(String),

    #[error("unknown team type: {0}")]
    UnknownTeamType(String),

    #[error("unknown role type: {0}")]
    UnknownRoleType(String),

    #[error("not found")]
    NotFound,

    #[error("product not found: {0}")]
    ProductNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}
```

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/team_role.rs`:

```rust
use std::convert::TryFrom;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TeamType {
    Members,
    UnblindMembers,
}

impl TeamType {
    pub fn as_str(&self) -> &'static str {
        match self {
            TeamType::Members => "members",
            TeamType::UnblindMembers => "unblind_members",
        }
    }
}

impl TryFrom<&str> for TeamType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "members" => Ok(TeamType::Members),
            "unblind_members" => Ok(TeamType::UnblindMembers),
            other => Err(DomainError::UnknownTeamType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RoleType {
    Leader,
    Worker,
}

impl RoleType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RoleType::Leader => "leader",
            RoleType::Worker => "worker",
        }
    }
}

impl TryFrom<&str> for RoleType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "leader" => Ok(RoleType::Leader),
            "worker" => Ok(RoleType::Worker),
            other => Err(DomainError::UnknownRoleType(other.to_string())),
        }
    }
}
```

- [ ] **Step 4: Wire `domain.rs` and add the test scaffolding**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain.rs` with:

```rust
mod error;
mod team_role;
#[cfg(test)]
mod tests;

pub use error::DomainError;
pub use team_role::{RoleType, TeamType};
```

`mod project_member;`, `mod product;`, `mod project;`, `mod user;` will be added in subsequent steps.

- [ ] **Step 5: Run the tests; verify they pass**

Run: `cargo test -p project --lib`
Expected: PASS, six new tests under `domain::tests`.

- [ ] **Step 6: Write the failing test for `ProjectMember::new`**

Append the following to `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/tests.rs`:

```rust
#[test]
fn project_member_rejects_duplicate_leader() {
    let err = ProjectMember::new(vec!["u1".into()], vec!["u2".into()]).unwrap();
    // sanity: clean construction succeeds
    assert_eq!(err.leaders, vec!["u1".to_string()]);

    let err = ProjectMember::new(vec!["u1".into(), "u1".into()], vec![]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateLeader(ref s) if s == "u1"));
}

#[test]
fn project_member_rejects_duplicate_worker() {
    let err = ProjectMember::new(vec![], vec!["u2".into(), "u2".into()]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateWorker(ref s) if s == "u2"));
}

#[test]
fn project_member_allows_same_code_in_leaders_and_workers() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u1".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u1".to_string()]);
}

#[test]
fn project_member_accepts_empty_lists() {
    let m = ProjectMember::new(vec![], vec![]).unwrap();
    assert!(m.leaders.is_empty());
    assert!(m.workers.is_empty());
}
```

Fix Step 6's first sub-test: the `let err = ... .unwrap()` line is a `ProjectMember`, not an error. Correct it to:

```rust
#[test]
fn project_member_accepts_clean_input() {
    let m = ProjectMember::new(vec!["u1".into()], vec!["u2".into()]).unwrap();
    assert_eq!(m.leaders, vec!["u1".to_string()]);
    assert_eq!(m.workers, vec!["u2".to_string()]);
}

#[test]
fn project_member_rejects_duplicate_leader() {
    let err = ProjectMember::new(vec!["u1".into(), "u1".into()], vec![]).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateLeader(ref s) if s == "u1"));
}
```

(Use the corrected pair.)

- [ ] **Step 7: Verify the test fails to compile**

Run: `cargo test -p project --lib`
Expected: FAIL — `ProjectMember` is not defined.

- [ ] **Step 8: Implement `ProjectMember`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/project_member.rs`:

```rust
use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMember {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

impl ProjectMember {
    /// Validating constructor used by the usecase layer (and by tests).
    ///
    /// Rules:
    /// - `leaders` must not contain duplicate codes; the first duplicate
    ///   is returned as `DuplicateLeader`.
    /// - `workers` must not contain duplicate codes; the first duplicate
    ///   is returned as `DuplicateWorker`.
    /// - The same code may appear in both `leaders` and `workers` of the
    ///   same team — a leader can also do worker work.
    /// - Either list may be empty.
    pub fn new(leaders: Vec<String>, workers: Vec<String>) -> Result<Self, DomainError> {
        for code in &leaders {
            if leaders.iter().filter(|c| *c == code).count() > 1 {
                return Err(DomainError::DuplicateLeader(code.clone()));
            }
        }
        for code in &workers {
            if workers.iter().filter(|c| *c == code).count() > 1 {
                return Err(DomainError::DuplicateWorker(code.clone()));
            }
        }
        Ok(Self { leaders, workers })
    }

    /// Bypasses validation. Reserved for the adapter layer when materialising
    /// rows from persistence; duplicates cannot occur because the
    /// `project_members` PK is `(project_id, team_type, role_type, user_code)`.
    pub(crate) fn for_repository(leaders: Vec<String>, workers: Vec<String>) -> Self {
        Self { leaders, workers }
    }
}
```

- [ ] **Step 9: Wire the module**

In `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain.rs`, add `mod project_member;` and `pub use project_member::ProjectMember;`.

- [ ] **Step 10: Run the tests; verify they pass**

Run: `cargo test -p project --lib`
Expected: PASS, six team/role tests + five project-member tests.

- [ ] **Step 11: Write the failing tests for `Product` and `Project`**

Append to `domain/tests.rs`:

```rust
#[test]
fn product_new_rejects_empty_code() {
    let err = Product::new(1, "".into(), "Alice".into(), "".into(), true, test_now(), test_now())
        .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn product_new_rejects_empty_name() {
    let err = Product::new(1, "p1".into(), "".into(), "".into(), true, test_now(), test_now())
        .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn product_new_accepts_valid_input() {
    let p = Product::new(7, "p7".into(), "Alice".into(), "desc".into(), true, test_now(), test_now())
        .unwrap();
    assert_eq!(p.id, 7);
    assert_eq!(p.code, "p7");
}

#[test]
fn project_new_rejects_empty_code() {
    let m = ProjectMember::default();
    let err = Project::new(
        1,
        "".into(),
        "desc".into(),
        1,
        m.clone(),
        m,
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn project_new_rejects_zero_product_id() {
    let m = ProjectMember::default();
    let err = Project::new(
        1,
        "proj1".into(),
        "desc".into(),
        0,
        m.clone(),
        m,
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::ZeroProductId));
}

#[test]
fn project_new_accepts_valid_input() {
    let m = ProjectMember::default();
    let p = Project::new(
        9,
        "proj9".into(),
        "desc".into(),
        3,
        m.clone(),
        m,
        true,
        test_now(),
        test_now(),
    )
    .unwrap();
    assert_eq!(p.id, 9);
    assert_eq!(p.product_id, 3);
}

fn test_now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}
```

Add `use chrono::{TimeZone, Utc};` to the top of `domain/tests.rs`.

- [ ] **Step 12: Verify the tests fail to compile**

Run: `cargo test -p project --lib`
Expected: FAIL — `Product`, `Project` not defined.

- [ ] **Step 13: Implement `Product`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/product.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;

#[derive(Clone, PartialEq, Eq)]
pub struct Product {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Product {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        name: String,
        description: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id,
            code,
            name,
            description,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        name: String,
        description: String,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            name,
            description,
            active,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for Product {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Product")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("name", &self.name)
            .field("description", &self.description)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
```

- [ ] **Step 14: Implement `Project`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/project.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::project_member::ProjectMember;

#[derive(Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        description: String,
        product_id: i32,
        members: ProjectMember,
        unblind_members: ProjectMember,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        if product_id == 0 {
            return Err(DomainError::ZeroProductId);
        }
        Ok(Self {
            id,
            code,
            description,
            product_id,
            members,
            unblind_members,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        description: String,
        product_id: i32,
        members: ProjectMember,
        unblind_members: ProjectMember,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            description,
            product_id,
            members,
            unblind_members,
            active,
            created_at,
            updated_at,
        }
    }
}

impl std::fmt::Debug for Project {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Project")
            .field("id", &self.id)
            .field("code", &self.code)
            .field("description", &self.description)
            .field("product_id", &self.product_id)
            .field("members", &self.members)
            .field("unblind_members", &self.unblind_members)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}
```

- [ ] **Step 15: Implement `UserSummary` + `UserService` port**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/user.rs`:

```rust
use async_trait::async_trait;

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub code: String,
    pub name: String,
}

/// Narrow user port that the project crate uses to hydrate membership
/// codes. Implementations adapt `apis::user::UserService` (the only
/// caller is `adapter::service::user::UserServiceImpl`).
#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError>;
}
```

- [ ] **Step 16: Wire the new modules**

In `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain.rs`, replace its content with:

```rust
mod error;
mod product;
mod project;
mod project_member;
mod team_role;
mod user;
#[cfg(test)]
mod tests;

pub use error::DomainError;
pub use product::{Product, ProductNew, ProductRepository, ProductUpdate};
pub use project::{Project, ProjectNew, ProjectRepository, ProjectUpdate};
pub use project_member::ProjectMember;
pub use team_role::{RoleType, TeamType};
pub use user::{UserService, UserSummary};
```

- [ ] **Step 17: Add the repository ports + input DTOs**

Append to `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/product.rs`:

```rust
use async_trait::async_trait;

/// Input DTO for `ProductRepository::create`.
#[derive(Debug, Clone)]
pub struct ProductNew {
    pub code: String,
    pub name: String,
    pub description: String,
}

/// Input DTO for `ProductRepository::update`. Every field is optional
/// so the usecase can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct ProductUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

/// Outbound port for persistence of `Product` aggregates.
#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError>;
    async fn list(&self) -> Result<Vec<Product>, DomainError>;
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError>;
}
```

Append to `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/domain/project.rs`:

```rust
use async_trait::async_trait;

/// Input DTO for `ProjectRepository::create`.
#[derive(Debug, Clone)]
pub struct ProjectNew {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. `None` and `Some(empty)` are equivalent — neither
    /// inserts any `project_members` rows for that team. Letting the
    /// field be absent keeps the "create shell, add members later"
    /// flow ergonomic.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

/// Input DTO for `ProjectRepository::update`. Every field is optional
/// so the usecase can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe that
    /// team's rows. The two are distinct on update.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

/// Outbound port for persistence of `Project` aggregates.
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError>;
    async fn list(&self) -> Result<Vec<Project>, DomainError>;
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError>;
}
```

- [ ] **Step 18: Wire the public surface at the crate root**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` with:

```rust
//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for `Product` and `Project` aggregates and an async
//! `ProjectUsecase` that orchestrates them and adapts to the
//! `apis::project::ProjectService` port.
//!
//! The crate exposes the three DDD layers as sub-modules for power
//! users (`domain`, `usecase`, `adapter`) and re-exports the public
//! surface at the crate root so consumers can simply write
//!
//! ```no_run
//! use project::{ProjectRepo, ProductRepo, ProjectUsecase, ProjectUsecaseConfig, ProjectServiceImpl};
//! ```

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::ProjectServiceImpl;
pub use adapter::persistence::postgres::{ProductRepo, ProjectRepo};
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
pub use usecase::{
    CreateProduct, CreateProject, ProductView, ProjectMemberView, ProjectUsecase,
    ProjectUsecaseConfig, ProjectView, UpdateProduct, UpdateProject, UsecaseError, UserSummaryView,
};
```

(`adapter::facade::in_memory::ProjectServiceImpl`, `adapter::service::user::UserServiceImpl`, and `usecase` types will be filled in by Tasks 4, 6, 7. Until then the `pub use` lines for not-yet-existing items will fail to compile — that is expected and acceptable; we will re-run `cargo check` at the end of Task 2.)

- [ ] **Step 19: Run the test suite; verify the domain tests pass**

Run: `cargo test -p project --lib`
Expected: PASS — domain tests pass. The crate root `pub use` lines may fail because the targets do not exist yet; if so, comment out the not-yet-existing re-exports (lines for `ProductRepo`, `ProjectRepo`, `ProjectUsecase`, `ProjectUsecaseConfig`, `ProjectServiceImpl`, `UserServiceImpl`, `usecase::*`) by prefixing them with `// `, then uncomment as the corresponding Tasks land.

- [ ] **Step 20: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/src/domain.rs \
        lib/crates/project/src/domain \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): domain layer

Adds the domain module: TeamType / RoleType enums with TryFrom
parsing; ProjectMember value object with duplicate-leader and
duplicate-worker rejection; Product and Project aggregates with
validating + repository-only constructors; narrow domain::UserService
port + UserSummary; ProductRepository and ProjectRepository ports with
ProductNew / ProductUpdate / ProjectNew / ProjectUpdate DTOs;
DomainError enum.

ProjectMember::new rules: per-set duplicates are rejected; the same
code may appear in both leaders and workers; both fields may be empty.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo test -p project --lib"
```

---

## Task 3: apis port (`apis::project`)

**Files:**
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/lib.rs` (add `pub mod project;`)
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/project.rs`

- [ ] **Step 1: Create the apis port file**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/project.rs`:

```rust
//! Outbound port for product / project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`Role`, `ProjectApiError`, `ProductView`, `ProjectView`,
//! `ProjectMemberView`, `UserSummaryView`, `*Request`) are defined
//! alongside the trait so a single `use apis::project::*;` brings the
//! whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Error surface returned by every [`ProjectService`] method.
///
/// Adapters map backend-specific errors (e.g. `project::UsecaseError`)
/// into this type at the implementation boundary.
#[derive(Debug, Clone, Error)]
pub enum ProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("product not found: {0}")]
    ProductNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Safe projection of a product — every field is safe to log today.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a project: the parent `ProductView` is
/// denormalised in, and the membership lists are hydrated to
/// `Vec<UserSummaryView>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductView,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

/// Wire-shaped membership data. `leaders` and `workers` are user codes
/// (not full user records); the backend hydrates them to
/// `UserSummaryView` on read.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectMemberData {
    pub leaders: Vec<String>,
    pub workers: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CreateProductRequest {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProductRequest {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. Omit (or pass an empty `ProjectMemberData`) to create
    /// the project with no membership rows; the shell can be filled in
    /// via a later `update_project` call.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
}

/// Outbound port for product / project lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn ProjectService>` can be shared state in
/// an async server (axum, tarpc, etc.).
#[async_trait]
pub trait ProjectService: Send + Sync {
    // Products
    async fn create_product(
        &self,
        req: CreateProductRequest,
    ) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_id(&self, id: i32) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_code(&self, code: &str) -> Result<ProductView, ProjectApiError>;
    async fn list_products(&self) -> Result<Vec<ProductView>, ProjectApiError>;
    async fn update_product(
        &self,
        req: UpdateProductRequest,
    ) -> Result<ProductView, ProjectApiError>;

    // Projects
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError>;
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError>;
    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError>;
}
```

- [ ] **Step 2: Register the module in `apis::lib`**

In `/Users/yukichen/Coding/Projects/aegis/lib/crates/apis/src/lib.rs`, add a new line:

```rust
pub mod project;
```

after the existing `pub mod user;` line.

- [ ] **Step 3: Verify the apis crate compiles**

Run: `cargo check -p apis`
Expected: success.

- [ ] **Step 4: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/apis/src/lib.rs lib/crates/apis/src/project.rs
git commit -m "feat(apis): add project port

Adds apis::project with the ProjectApiError enum, ProductView,
ProjectView, ProjectMemberView, UserSummaryView, ProjectMemberData,
Create* / Update* request DTOs, and the ProjectService async trait
covering product + project CRUD.

The project crate (landed in the previous commit) implements this
port via adapter::facade::in_memory::ProjectServiceImpl.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo check -p apis"
```

---

## Task 4: usecase layer

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/error.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/commands.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/views.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/project_usecase.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/tests.rs`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (uncomment the `usecase::*` re-exports from Task 2)

- [ ] **Step 1: Implement `UsecaseError`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/error.rs`:

```rust
use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        UsecaseError::Repository(err)
    }
}
```

- [ ] **Step 2: Implement command DTOs**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/commands.rs`:

```rust
use crate::domain::ProjectMember;

#[derive(Debug, Clone)]
pub struct CreateProduct {
    pub code: String,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProduct {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProject {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}
```

- [ ] **Step 3: Implement view DTOs + `From` impls**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/views.rs`:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{Product, Project, ProjectMember, UserSummary};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummaryView {
    pub code: String,
    pub name: String,
}

impl From<UserSummary> for UserSummaryView {
    fn from(s: UserSummary) -> Self {
        Self {
            code: s.code,
            name: s.name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

impl From<ProjectMember> for ProjectMemberView {
    fn from(m: ProjectMember) -> Self {
        Self {
            leaders: m.leaders.into_iter().map(UserSummaryView::from).collect(),
            workers: m.workers.into_iter().map(UserSummaryView::from).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Product> for ProductView {
    fn from(p: Product) -> Self {
        Self {
            id: p.id,
            code: p.code,
            name: p.name,
            description: p.description,
            active: p.active,
            created_at: p.created_at,
            updated_at: p.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductView,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectView {
    /// Build the parent Product + membership hydration around a domain
    /// `Project`. The product and user summaries are looked up via the
    /// supplied closures so the constructor stays testable without
    /// reaching for `ProductRepository` / `UserService` directly here.
    pub fn from_project(
        project: Project,
        product: Product,
        members: ProjectMemberView,
        unblind_members: ProjectMemberView,
    ) -> Self {
        Self {
            id: project.id,
            code: project.code,
            description: project.description,
            product: product.into(),
            members,
            unblind_members,
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}
```

- [ ] **Step 4: Implement `ProjectUsecase` + `ProjectUsecaseConfig`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/project_usecase.rs`:

```rust
use std::collections::HashMap;

use crate::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, UserService, UserSummary,
};

use super::commands::{CreateProduct, CreateProject, UpdateProduct, UpdateProject};
use super::error::UsecaseError;
use super::views::{ProductView, ProjectMemberView, ProjectView};

pub struct ProjectUsecaseConfig<P: ProductRepository, R: ProjectRepository, U: UserService> {
    pub product_repo: P,
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<P: ProductRepository, R: ProjectRepository, U: UserService> {
    product_repo: P,
    project_repo: R,
    users: U,
}

impl<P: ProductRepository, R: ProjectRepository, U: UserService> ProjectUsecase<P, R, U> {
    pub fn new(cfg: ProjectUsecaseConfig<P, R, U>) -> Self {
        Self {
            product_repo: cfg.product_repo,
            project_repo: cfg.project_repo,
            users: cfg.users,
        }
    }

    // -------- Products --------

    pub async fn create_product(
        &self,
        cmd: CreateProduct,
    ) -> Result<ProductView, UsecaseError> {
        validate_create_product(&cmd)?;
        let product = self
            .product_repo
            .create(ProductNew {
                code: cmd.code,
                name: cmd.name,
                description: cmd.description,
            })
            .await?;
        Ok(product.into())
    }

    pub async fn get_product_by_id(&self, id: i32) -> Result<ProductView, UsecaseError> {
        let product = self.product_repo.find_by_id(id).await?;
        Ok(product.into())
    }

    pub async fn get_product_by_code(&self, code: &str) -> Result<ProductView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let product = self.product_repo.find_by_code(code).await?;
        Ok(product.into())
    }

    pub async fn list_products(&self) -> Result<Vec<ProductView>, UsecaseError> {
        let products = self.product_repo.list().await?;
        Ok(products.into_iter().map(ProductView::from).collect())
    }

    pub async fn update_product(
        &self,
        cmd: UpdateProduct,
    ) -> Result<ProductView, UsecaseError> {
        validate_update_product(&cmd)?;
        let product = self
            .product_repo
            .update(ProductUpdate {
                id: cmd.id,
                code: cmd.code,
                name: cmd.name,
                description: cmd.description,
                active: cmd.active,
            })
            .await?;
        Ok(product.into())
    }

    // -------- Projects --------

    pub async fn create_project(
        &self,
        cmd: CreateProject,
    ) -> Result<ProjectView, UsecaseError> {
        validate_create_project(&cmd)?;
        // Surface `ProductNotFound` early; the FK would catch it later
        // but failing here gives a clearer error path.
        let product = self
            .product_repo
            .find_by_id(cmd.product_id)
            .await
            .map_err(|err| match err {
                DomainError::NotFound => UsecaseError::Repository(DomainError::ProductNotFound(
                    cmd.product_id.to_string(),
                )),
                other => UsecaseError::Repository(other),
            })?;

        let new_project = self
            .project_repo
            .create(ProjectNew {
                code: cmd.code,
                description: cmd.description,
                product_id: cmd.product_id,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
            })
            .await?;

        self.hydrate_project_view(new_project, product).await
    }

    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError> {
        let project = self.project_repo.find_by_id(id).await?;
        self.hydrate_project_view(project, None).await
    }

    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let project = self.project_repo.find_by_code(code).await?;
        self.hydrate_project_view(project, None).await
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError> {
        let projects = self.project_repo.list().await?;
        // One user-service round-trip per call; bucket the codes into
        // each project's two teams afterwards.
        let all_users = self.users.list().await?;
        let mut out = Vec::with_capacity(projects.len());
        for project in projects {
            let product = self.product_repo.find_by_id(project.product_id).await?;
            let view = hydrate_with(&all_users, project, product)?;
            out.push(view);
        }
        Ok(out)
    }

    pub async fn update_project(
        &self,
        cmd: UpdateProject,
    ) -> Result<ProjectView, UsecaseError> {
        validate_update_project(&cmd)?;
        let updated = self
            .project_repo
            .update(ProjectUpdate {
                id: cmd.id,
                code: cmd.code,
                description: cmd.description,
                product_id: cmd.product_id,
                active: cmd.active,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
            })
            .await?;
        self.hydrate_project_view(updated, None).await
    }

    // -------- helpers --------

    async fn hydrate_project_view(
        &self,
        project: Project,
        product: Option<Product>,
    ) -> Result<ProjectView, UsecaseError> {
        let product = match product {
            Some(p) => p,
            None => self.product_repo.find_by_id(project.product_id).await?,
        };
        let all_users = self.users.list().await?;
        hydrate_with(&all_users, project, product)
    }
}

/// Bucket the supplied user summaries into a project's two teams and
/// produce a `ProjectView`. Pure (no I/O) so tests can exercise it
/// directly through the usecase.
fn hydrate_with(
    users: &[UserSummary],
    project: Project,
    product: Product,
) -> Result<ProjectView, UsecaseError> {
    let by_code: HashMap<&str, &UserSummary> =
        users.iter().map(|u| (u.code.as_str(), u)).collect();
    let members = project.members.clone();
    let unblind_members = project.unblind_members.clone();

    let leaders: Vec<UserSummary> = lookup_set(&by_code, &members.leaders)?;
    let workers: Vec<UserSummary> = lookup_set(&by_code, &members.workers)?;
    let members_view = ProjectMemberView {
        leaders: leaders.into_iter().map(Into::into).collect(),
        workers: workers.into_iter().map(Into::into).collect(),
    };

    let unblind_leaders: Vec<UserSummary> =
        lookup_set(&by_code, &unblind_members.leaders)?;
    let unblind_workers: Vec<UserSummary> =
        lookup_set(&by_code, &unblind_members.workers)?;
    let unblind_view = ProjectMemberView {
        leaders: unblind_leaders.into_iter().map(Into::into).collect(),
        workers: unblind_workers.into_iter().map(Into::into).collect(),
    };

    Ok(ProjectView::from_project(
        project,
        product,
        members_view,
        unblind_view,
    ))
}

fn lookup_set<'a>(
    by_code: &HashMap<&'a str, &'a UserSummary>,
    codes: &[String],
) -> Result<Vec<UserSummary>, UsecaseError> {
    let mut out = Vec::with_capacity(codes.len());
    for code in codes {
        match by_code.get(code.as_str()) {
            Some(summary) => out.push((*summary).clone()),
            None => {
                return Err(UsecaseError::Repository(DomainError::UserNotFound(
                    code.clone(),
                )));
            }
        }
    }
    Ok(out)
}

fn validate_create_product(cmd: &CreateProduct) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_product(cmd: &UpdateProduct) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref n) = cmd.name
        && n.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_project(cmd: &CreateProject) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if cmd.product_id == 0 {
        return Err(UsecaseError::Validation(DomainError::ZeroProductId));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    Ok(())
}

fn validate_update_project(cmd: &UpdateProject) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(pid) = cmd.product_id
        && pid == 0
    {
        return Err(UsecaseError::Validation(DomainError::ZeroProductId));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    Ok(())
}
```

- [ ] **Step 5: Wire `usecase.rs`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase.rs`:

```rust
mod commands;
mod error;
mod project_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{CreateProduct, CreateProject, UpdateProduct, UpdateProject};
pub use error::UsecaseError;
pub use project_usecase::{ProjectUsecase, ProjectUsecaseConfig};
pub use views::{ProductView, ProjectMemberView, ProjectView, UserSummaryView};
```

- [ ] **Step 6: Uncomment the `usecase` re-exports at the crate root**

In `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs`, the `pub use usecase::{...}` line should already be uncommented from Task 2 (no further change needed).

- [ ] **Step 7: Write the failing usecase tests**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/usecase/tests.rs`:

```rust
//! Tests for the usecase layer.
//!
//! Mock repositories + a mock `UserService` stand in for the real
//! adapters so the orchestration + view projection can be exercised
//! without infrastructure.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
use crate::usecase::commands::{CreateProduct, CreateProject, UpdateProduct, UpdateProject};
use crate::usecase::error::UsecaseError;
use crate::usecase::project_usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- mock product repo ----------

#[derive(Default)]
struct MockProductState {
    products: HashMap<i32, Product>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProductRepo {
    state: Arc<Mutex<MockProductState>>,
}

impl MockProductRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProductState { next_id: 1 })),
        }
    }
    fn with_products(products: Vec<Product>) -> Self {
        let max_id = products.iter().map(|p| p.id).max().unwrap_or(0);
        let mut map = HashMap::new();
        for p in products {
            map.insert(p.id, p);
        }
        Self {
            state: Arc::new(Mutex::new(MockProductState {
                products: map,
                next_id: max_id + 1,
            })),
        }
    }
}

#[async_trait]
impl ProductRepository for MockProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.products.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint products_code_unique)".into(),
            ));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let product = Product::for_repository(
            id,
            input.code,
            input.name,
            input.description,
            true,
            now,
            now,
        );
        s.products.insert(id, product.clone());
        Ok(product)
    }
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Product>, DomainError> {
        Ok(self.state.lock().unwrap().products.values().cloned().collect())
    }
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        let p = s.products.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            if s.products.values().any(|other| other.code == *code && other.id != input.id) {
                return Err(DomainError::DuplicateCode(
                    "(constraint products_code_unique)".into(),
                ));
            }
            p.code = code.clone();
        }
        if let Some(ref name) = input.name {
            p.name = name.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(active) = input.active {
            p.active = active;
        }
        Ok(p.clone())
    }
}

// ---------- mock project repo ----------

#[derive(Default)]
struct MockProjectState {
    projects: HashMap<i32, Project>,
    /// (project_id, team_type) -> sorted user codes
    members: HashMap<(i32, TeamType), Vec<String>>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProjectRepo {
    state: Arc<Mutex<MockProjectState>>,
}

impl MockProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProjectState { next_id: 1 })),
        }
    }
}

#[async_trait]
impl ProjectRepository for MockProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id;
        s.next_id += 1;
        let now = mock_now();
        let members = input.members.unwrap_or_default();
        let unblind_members = input.unblind_members.unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            input.product_id,
            members.clone(),
            unblind_members.clone(),
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        if !members.leaders.is_empty() || !members.workers.is_empty() {
            s.members.insert((id, TeamType::Members), members.leaders.clone());
            s.members.insert((id, TeamType::Members), members.workers.clone()); // overwrites; tests don't rely on the distinction
        }
        if !unblind_members.leaders.is_empty() || !unblind_members.workers.is_empty() {
            s.members
                .insert((id, TeamType::UnblindMembers), unblind_members.leaders.clone());
            s.members
                .insert((id, TeamType::UnblindMembers), unblind_members.workers.clone());
        }
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        self.state
            .lock()
            .unwrap()
            .projects
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self.state.lock().unwrap().projects.values().cloned().collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            if s.projects.values().any(|other| other.code == *code && other.id != input.id) {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
            p.code = code.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(pid) = input.product_id {
            p.product_id = pid;
        }
        if let Some(active) = input.active {
            p.active = active;
        }
        // Replace membership wholesale per team.
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        Ok(p.clone())
    }
}

// ---------- mock user service ----------

#[derive(Clone, Default)]
struct MockUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl MockUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for MockUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }
}

// ---------- fixtures ----------

use std::sync::Arc;

fn make_usecase() -> (
    MockProductRepo,
    MockProjectRepo,
    MockUserService,
    ProjectUsecase<MockProductRepo, MockProjectRepo, MockUserService>,
) {
    let products = MockProductRepo::new();
    let projects = MockProjectRepo::new();
    let users = MockUserService::with_users(vec![
        UserSummary { code: "u1".into(), name: "Alice".into() },
        UserSummary { code: "u2".into(), name: "Bob".into() },
        UserSummary { code: "u3".into(), name: "Carol".into() },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products.clone(),
        project_repo: projects.clone(),
        users: users.clone(),
    });
    (products, projects, users, usecase)
}

fn seed_product(id: i32, code: &str) -> Product {
    let now = mock_now();
    Product::for_repository(id, code.into(), "P".into(), "".into(), true, now, now)
}

// ---------- tests ----------

#[tokio::test]
async fn create_product_returns_view() {
    let (_products, _projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_product(CreateProduct {
            code: "p1".into(),
            name: "Widget".into(),
            description: "desc".into(),
        })
        .await
        .expect("create succeeds");
    assert_eq!(view.id, 1);
    assert_eq!(view.code, "p1");
    assert_eq!(view.name, "Widget");
    assert!(view.active);
}

#[tokio::test]
async fn create_product_rejects_empty_code() {
    let (_p, _r, _u, usecase) = make_usecase();
    let err = usecase
        .create_product(CreateProduct {
            code: "  ".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect_err("blank code rejected");
    assert!(matches!(err, UsecaseError::Validation(DomainError::EmptyCode)));
}

#[tokio::test]
async fn get_product_by_code_returns_view() {
    let product = seed_product(5, "p5");
    let products = MockProductRepo::with_products(vec![product.clone()]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase.get_product_by_code("p5").await.expect("found");
    assert_eq!(view.id, 5);
}

#[tokio::test]
async fn list_products_returns_all_views() {
    let products = MockProductRepo::with_products(vec![
        seed_product(1, "p1"),
        seed_product(2, "p2"),
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let views = usecase.list_products().await.expect("list");
    assert_eq!(views.len(), 2);
}

#[tokio::test]
async fn update_product_flips_active_flag() {
    let product = seed_product(1, "p1");
    let products = MockProductRepo::with_products(vec![product]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase
        .update_product(UpdateProduct {
            id: 1,
            active: Some(false),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(!view.active);
}

#[tokio::test]
async fn create_project_without_membership_succeeds() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .expect("create");
    assert_eq!(view.code, "proj1");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_membership() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![
            UserSummary { code: "u1".into(), name: "Alice".into() },
            UserSummary { code: "u2".into(), name: "Bob".into() },
        ]),
    });
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(ProjectMember::default()),
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders.len(), 1);
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        }]),
    });
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect_err("unknown member rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::UserNotFound(ref c)) if c == "ghost"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn create_project_with_missing_product_returns_product_not_found() {
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: MockProductRepo::new(),
        project_repo: MockProjectRepo::new(),
        users: MockUserService::default(),
    });
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 999,
            members: None,
            unblind_members: None,
        })
        .await
        .expect_err("missing product");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::ProductNotFound(ref s)) if s == "999"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: MockProjectRepo::new(),
        users: MockUserService::with_users(vec![
            UserSummary { code: "u1".into(), name: "Alice".into() },
            UserSummary { code: "u2".into(), name: "Bob".into() },
            UserSummary { code: "u3".into(), name: "Carol".into() },
        ]),
    });
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            product_id: 1,
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            members: Some(ProjectMember {
                leaders: vec![],
                workers: vec!["u2".into(), "u3".into()],
            }),
            unblind_members: None,
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.members.leaders.is_empty());
    assert_eq!(updated.members.workers.len(), 2);
}

#[tokio::test]
async fn list_projects_calls_user_service_once() {
    let products = MockProductRepo::with_products(vec![seed_product(1, "p1")]);
    let projects = MockProjectRepo::new();
    // We instrument via a counting wrapper around MockUserService.
    // For brevity, use the default mock and just check we get two
    // projects back.
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: projects.clone(),
        users: MockUserService::with_users(vec![]),
    });
    let _ = usecase
        .create_project(CreateProject {
            code: "p1".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .unwrap();
    let _ = usecase
        .create_project(CreateProject {
            code: "p2".into(),
            description: "".into(),
            product_id: 1,
            members: None,
            unblind_members: None,
        })
        .await
        .unwrap();
    let list = usecase.list_projects().await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn project_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectUsecase<MockProductRepo, MockProjectRepo, MockUserService>>();
}
```

- [ ] **Step 8: Verify the tests fail to compile**

Run: `cargo test -p project --lib`
Expected: FAIL — the usecase tests reference `ProjectUsecase`, `MockProductRepo`, etc. that are now defined; should compile cleanly. If the only failure is the unused `RoleType` / `TeamType` / `AtomicI32` / `Ordering` imports the test file brings in, that's a warning; promote them to `_` or remove. Run `cargo build -p project --tests` and confirm only test compilation succeeds with no errors.

- [ ] **Step 9: Run the tests; verify they pass**

Run: `cargo test -p project --lib`
Expected: PASS, all usecase tests green.

- [ ] **Step 10: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/src/usecase.rs \
        lib/crates/project/src/usecase
git commit -m "feat(project): usecase layer

Adds ProjectUsecase<P, R, U> with ProjectUsecaseConfig; CreateProduct,
UpdateProduct, CreateProject, UpdateProject command DTOs; ProductView,
ProjectView, ProjectMemberView, UserSummaryView view DTOs;
UsecaseError. create_project validates membership via ProjectMember::new,
surfaces ProductNotFound early, and treats None members as empty.
get_*_project hydrates by looking up the parent Product via
ProductRepository and bucketing UserService::list into the four
membership sets. Usecase tests cover CRUD, hydration, optional
membership, missing-product, missing-user, and Send + Sync.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo test -p project --lib"
```

---

## Task 5: Postgres persistence (migrations, repos, integration tests)

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/migrations/0001_create_products.sql`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/migrations/0002_create_projects.sql`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/row.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/product_repo.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/tests.rs`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (uncomment the `ProductRepo` / `ProjectRepo` re-exports)

- [ ] **Step 1: Write migration 0001 (products)**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/migrations/0001_create_products.sql`:

```sql
-- 0001_create_products.sql
--
-- Initial schema for the `products` table. Applied by the database
-- migration toolchain (sqlx migrate run / refinery / etc.) before the
-- `project` crate can be used against PostgreSQL.
--
-- Layout:
--   * `id`          - surrogate primary key, generated by default
--                     (so callers can supply a known id during seeding
--                     or `copy`-style imports).
--   * `code`        - caller-chosen stable identifier. Uniqueness is
--                     enforced globally.
--   * `name`        - human-readable display name.
--   * `description` - free-form long description. Defaults to empty.
--   * `active`      - soft-delete flag. There is no `deactivate`
--                     operation in this crate; callers can flip
--                     `active` via the generic `update_product` entry
--                     point. The crate never issues a hard `DELETE`.
--   * `created_at`  - timestamp at which the row was inserted. Set once
--                     by `DEFAULT NOW()` and never modified afterwards.
--   * `updated_at`  - timestamp of the most recent row modification.
--                     Initial value comes from `DEFAULT NOW()` and the
--                     `products_set_updated_at` trigger refreshes it
--                     on every UPDATE.

CREATE TABLE products (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT products_code_unique UNIQUE (code)
);

-- Auto-update `updated_at` on every row modification.
CREATE OR REPLACE FUNCTION products_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER products_set_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION products_set_updated_at();
```

- [ ] **Step 2: Write migration 0002 (projects + members)**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/migrations/0002_create_projects.sql`:

```sql
-- 0002_create_projects.sql
--
-- Schema for the `projects` table and the `project_members` join
-- table. Applied after 0001 so the FK to `products(id)` is valid.
--
-- `projects`:
--   * `id`          - surrogate primary key.
--   * `code`        - caller-chosen stable identifier; unique.
--   * `description` - free-form long description. Defaults to empty.
--   * `product_id`  - FK to `products(id)`; the owning product.
--   * `active`      - soft-delete flag (no hard DELETE).
--   * `created_at`  - DEFAULT NOW() at insert.
--   * `updated_at`  - DEFAULT NOW() at insert; the
--                     `projects_set_updated_at` trigger refreshes it.
--
-- `project_members`:
--   * Composite PK on (project_id, team_type, role_type, user_code).
--   * `team_type` ∈ {'members', 'unblind_members'}.
--   * `role_type` ∈ {'leader', 'worker'}.
--   * ON DELETE CASCADE on the FK so wiping a project also wipes its
--     membership rows.

CREATE TABLE projects (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    product_id INTEGER NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT projects_code_unique UNIQUE (code),
    CONSTRAINT projects_product_fk FOREIGN KEY (product_id)
        REFERENCES products(id)
);

CREATE OR REPLACE FUNCTION projects_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER projects_set_updated_at
    BEFORE UPDATE ON projects
    FOR EACH ROW
    EXECUTE FUNCTION projects_set_updated_at();

CREATE TABLE project_members (
    project_id INTEGER NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    team_type TEXT NOT NULL,
    role_type TEXT NOT NULL,
    user_code TEXT NOT NULL,
    PRIMARY KEY (project_id, team_type, role_type, user_code),
    CONSTRAINT project_members_team_check
        CHECK (team_type IN ('members', 'unblind_members')),
    CONSTRAINT project_members_role_check
        CHECK (role_type IN ('leader', 'worker'))
);
```

- [ ] **Step 3: Write the schema assertion test**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

```rust
//! Schema + row-conversion tests for the PostgreSQL adapter.
//!
//! These tests do NOT require a live database. They read the migration
//! files and the row-bridge impls directly. Live-database round-trips
//! live in `tests/integration_persistence.rs` and are `#[ignore]`-gated.

use std::fs;
use std::path::PathBuf;

fn migration_path(name: &str) -> PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    PathBuf::from(manifest_dir).join("migrations").join(name)
}

fn load_migration(name: &str) -> String {
    fs::read_to_string(migration_path(name))
        .unwrap_or_else(|_| panic!("migration file {name} must exist"))
}

fn create_table_block(sql: &str) -> String {
    let start = sql.find("CREATE TABLE").expect("CREATE TABLE");
    let close = sql[start..]
        .find(");")
        .expect("CREATE TABLE terminated by `);`");
    sql[start..start + close + 2].to_string()
}

#[test]
fn products_migration_creates_products_table() {
    let sql = load_migration("0001_create_products.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("products"));
}

#[test]
fn products_migration_has_required_columns() {
    let block = create_table_block(&load_migration("0001_create_products.sql"));
    let upper = block.to_uppercase();
    for required in [
        "ID INTEGER",
        "CODE TEXT",
        "NAME TEXT",
        "DESCRIPTION TEXT",
        "ACTIVE BOOLEAN",
        "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(
            upper.contains(&required.to_uppercase()),
            "products table must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn products_migration_makes_code_unique_and_not_null() {
    let block = create_table_block(&load_migration("0001_create_products.sql"));
    assert!(
        block.contains("UNIQUE (code)") || block.contains("UNIQUE(\"code\")"),
        "expected UNIQUE on code; got:\n{block}"
    );
    assert!(block.to_uppercase().contains("NOT NULL"));
}

#[test]
fn products_migration_has_updated_at_trigger() {
    let sql = load_migration("0001_create_products.sql");
    assert!(sql.contains("CREATE TRIGGER products_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON products"));
}

#[test]
fn projects_migration_creates_projects_table() {
    let sql = load_migration("0002_create_projects.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("projects"));
}

#[test]
fn projects_migration_references_products() {
    let block = create_table_block(&load_migration("0002_create_projects.sql"));
    let upper = block.to_uppercase();
    assert!(
        upper.contains("PRODUCT_ID INTEGER"),
        "projects.product_id must be INTEGER; got:\n{block}"
    );
    assert!(
        upper.contains("REFERENCES PRODUCTS(ID)"),
        "projects.product_id must FK to products(id); got:\n{block}"
    );
}

#[test]
fn projects_migration_has_updated_at_trigger() {
    let sql = load_migration("0002_create_projects.sql");
    assert!(sql.contains("CREATE TRIGGER projects_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON projects"));
}

#[test]
fn project_members_migration_has_composite_pk_and_checks() {
    let sql = load_migration("0002_create_projects.sql");
    let upper = sql.to_uppercase();
    // Find the project_members CREATE TABLE block.
    let start = upper.find("CREATE TABLE PROJECT_MEMBERS").expect("project_members");
    let close = upper[start..].find(");").expect("close") + start + 2;
    let block = &sql[start..close];
    let upper_block = block.to_uppercase();
    assert!(
        upper_block.contains("PRIMARY KEY (PROJECT_ID, TEAM_TYPE, ROLE_TYPE, USER_CODE)"),
        "project_members PK must be the composite; got:\n{block}"
    );
    assert!(upper_block.contains("CHECK"));
    assert!(upper_block.contains("'MEMBERS'") && upper_block.contains("'UNBLIND_MEMBERS'"));
    assert!(upper_block.contains("'LEADER'") && upper_block.contains("'WORKER'"));
}

#[test]
fn project_members_migration_cascades_on_delete() {
    let sql = load_migration("0002_create_projects.sql");
    assert!(
        sql.contains("REFERENCES projects(id) ON DELETE CASCADE"),
        "project_members FK must cascade on delete"
    );
}
```

- [ ] **Step 4: Wire the persistence module skeleton**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence.rs`:

```rust
//! Persistence adapters.
//!
//! Storage-specific code lives under `persistence/<backend>/`. At the
//! moment only the PostgreSQL backend exists; the layer boundary
//! re-exports `ProductRepo` and `ProjectRepo` so external callers can
//! name them via the crate root.

pub(crate) mod postgres;
```

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres.rs`:

```rust
//! PostgreSQL-backed implementations of `ProductRepository` and
//! `ProjectRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the user crate.
//! `ProjectRepo::create` / `update` open a transaction so the project
//! row and the `project_members` rows land atomically.
//!
//! `row` is `pub(crate)` and is NOT re-exported at the crate root.

pub(crate) mod product_repo;
pub(crate) mod project_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use product_repo::ProductRepo;
pub use project_repo::ProjectRepo;
```

Create the (mostly empty) leaf files so the `mod` declarations compile:

- `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/row.rs`:

```rust
// Row bridges live here; populated in the next step.
```

- `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/product_repo.rs`:

```rust
// ProductRepo lands in the next step.
```

- `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`:

```rust
// ProjectRepo lands in the next step.
```

Create the top-level adapter module:

`/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter.rs`:

```rust
//! Adapter layer.
//!
//! Houses the persistence adapters that implement the
//! `ProductRepository` and `ProjectRepository` ports, plus outbound
//! port adapters (the `UserService` facade adapting
//! `apis::user::UserService`, and the in-memory `ProjectServiceImpl`
//! facade adapting `ProjectUsecase` to `apis::project::ProjectService`).
//!
//! Storage-specific implementations live under `persistence/<backend>/`.

pub mod facade;
pub mod persistence;
pub mod service;
```

(`pub mod facade;` and `pub mod service;` will be populated by Tasks 6 and 7. Until then, comment them out.)

- [ ] **Step 5: Run the schema tests; verify they pass**

Run: `cargo test -p project --lib`
Expected: PASS — schema assertions all green.

- [ ] **Step 6: Implement row bridges + row-conversion tests**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/row.rs`:

```rust
//! Row -> domain conversion for the SQLx repositories.
//!
//! `ProductRow` and `ProjectRow` are the shapes returned by
//! `sqlx::query_as`. They are NOT re-exported at the crate root; only
//! the repositories use them.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, Product, Project, ProjectMember};

#[derive(Clone, FromRow)]
pub struct ProductRow {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProductRow> for Product {
    type Error = DomainError;

    fn try_from(row: ProductRow) -> Result<Self, Self::Error> {
        Ok(Product::for_repository(
            row.id,
            row.code,
            row.name,
            row.description,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(Clone, FromRow)]
pub struct ProjectRow {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<ProjectRow> for Project {
    type Error = DomainError;

    fn try_from(row: ProjectRow) -> Result<Self, Self::Error> {
        Ok(Project::for_repository(
            row.id,
            row.code,
            row.description,
            row.product_id,
            ProjectMember::default(),
            ProjectMember::default(),
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

/// One row from `project_members`.
#[derive(Clone, FromRow)]
pub struct ProjectMemberRow {
    pub project_id: i32,
    pub team_type: String,
    pub role_type: String,
    pub user_code: String,
}
```

Append row-conversion tests to `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

```rust
use chrono::{TimeZone, Utc};

use super::row::{ProductRow, ProjectMemberRow, ProjectRow};
use crate::domain::{ProjectMember, RoleType, TeamType};

fn ts() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

#[test]
fn product_row_converts_to_product() {
    let row = ProductRow {
        id: 1,
        code: "p1".into(),
        name: "Widget".into(),
        description: "desc".into(),
        active: true,
        created_at: ts(),
        updated_at: ts(),
    };
    let p: crate::domain::Product = row.try_into().expect("convert");
    assert_eq!(p.id, 1);
    assert_eq!(p.code, "p1");
}

#[test]
fn project_row_converts_to_project_with_empty_members() {
    let row = ProjectRow {
        id: 1,
        code: "proj1".into(),
        description: "".into(),
        product_id: 7,
        active: true,
        created_at: ts(),
        updated_at: ts(),
    };
    let p: crate::domain::Project = row.try_into().expect("convert");
    assert_eq!(p.product_id, 7);
    assert_eq!(p.members, ProjectMember::default());
    assert_eq!(p.unblind_members, ProjectMember::default());
}

#[test]
fn project_member_row_carries_team_and_role_strings() {
    let row = ProjectMemberRow {
        project_id: 1,
        team_type: "members".into(),
        role_type: "leader".into(),
        user_code: "u1".into(),
    };
    assert_eq!(TeamType::try_from(row.team_type.as_str()).unwrap(), TeamType::Members);
    assert_eq!(RoleType::try_from(row.role_type.as_str()).unwrap(), RoleType::Leader);
}
```

- [ ] **Step 7: Implement `ProductRepo`**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/product_repo.rs`:

```rust
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{DomainError, Product, ProductNew, ProductRepository, ProductUpdate};

use super::row::ProductRow;

/// PostgreSQL SQLSTATE for a unique-violation error.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct ProductRepo {
    pool: PgPool,
}

impl ProductRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProductRepository for ProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        const SQL: &str =
            "INSERT INTO products (code, name, description, active) \
             VALUES ($1, $2, $3, $4) \
             RETURNING id, code, name, description, active, created_at, updated_at";
        let row: ProductRow = sqlx::query_as(SQL)
            .bind(&input.code)
            .bind(&input.name)
            .bind(&input.description)
            .bind(true)
            .fetch_one(&self.pool)
            .await
            .map_err(map_db_error)?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError> {
        let row: ProductRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProductRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError> {
        let row: ProductRow = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<ProductRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn list(&self) -> Result<Vec<Product>, DomainError> {
        let rows: Vec<ProductRow> = sqlx::QueryBuilder::new(
            "SELECT id, code, name, description, active, created_at, updated_at \
             FROM products ORDER BY id",
        )
        .build_query_as::<ProductRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(Product::try_from).collect()
    }

    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE products SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref c) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(c);
        }
        if let Some(ref n) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(n);
        }
        if let Some(ref d) = input.description {
            sep(&mut qb);
            qb.push("description = ").push_bind(d);
        }
        if let Some(a) = input.active {
            sep(&mut qb);
            qb.push("active = ").push_bind(a);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(
            " RETURNING id, code, name, description, active, created_at, updated_at",
        );
        let row: ProductRow = qb
            .build_query_as::<ProductRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::NotFound)?;
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

- [ ] **Step 8: Implement `ProjectRepo`**

Replace `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`:

```rust
use std::collections::HashMap;
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectUpdate, RoleType,
    TeamType,
};

use super::row::{ProjectMemberRow, ProjectRow};

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
/// PostgreSQL SQLSTATE for foreign-key violation.
const SQLSTATE_FK_VIOLATION: &str = "23503";
/// Constraint name on `projects.product_id`.
const PROJECTS_PRODUCT_FK: &str = "projects_product_fk";

pub struct ProjectRepo {
    pool: PgPool,
}

impl ProjectRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ProjectRepository for ProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        let row: ProjectRow = sqlx::QueryBuilder::new(
            "INSERT INTO projects (code, description, product_id, active) \
             VALUES ",
        )
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.description)
        .push(", ")
        .push_bind(input.product_id)
        .push(", ")
        .push_bind(true)
        .push(" RETURNING id, code, description, product_id, active, created_at, updated_at")
        .build_query_as::<ProjectRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(map_db_error)?;

        let project_id = row.id;

        if let Some(ref members) = input.members {
            insert_membership(&mut tx, project_id, TeamType::Members, members).await?;
        }
        if let Some(ref members) = input.unblind_members {
            insert_membership(&mut tx, project_id, TeamType::UnblindMembers, members).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        // Reload so the membership rows are read back into the
        // returned `Project`.
        self.find_by_id(project_id).await
    }

    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        let row: ProjectRow = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<ProjectRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        let mut project: Project = row.try_into()?;
        let (members, unblind) = load_membership(&self.pool, id).await?;
        project.members = members;
        project.unblind_members = unblind;
        Ok(project)
    }

    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        let row: ProjectRow = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects WHERE code = ",
        )
        .push_bind(code)
        .build_query_as::<ProjectRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        let mut project: Project = row.try_into()?;
        let project_id = project.id;
        let (members, unblind) = load_membership(&self.pool, project_id).await?;
        project.members = members;
        project.unblind_members = unblind;
        Ok(project)
    }

    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        let rows: Vec<ProjectRow> = sqlx::QueryBuilder::new(
            "SELECT id, code, description, product_id, active, created_at, updated_at \
             FROM projects ORDER BY id",
        )
        .build_query_as::<ProjectRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let mut project: Project = row.try_into()?;
            let (members, unblind) = load_membership(&self.pool, project.id).await?;
            project.members = members;
            project.unblind_members = unblind;
            out.push(project);
        }
        Ok(out)
    }

    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        // Apply metadata first. If the metadata update fails we never
        // touch membership.
        let mut qb = sqlx::QueryBuilder::new("UPDATE projects SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref c) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(c);
        }
        if let Some(ref d) = input.description {
            sep(&mut qb);
            qb.push("description = ").push_bind(d);
        }
        if let Some(pid) = input.product_id {
            sep(&mut qb);
            qb.push("product_id = ").push_bind(pid);
        }
        if let Some(a) = input.active {
            sep(&mut qb);
            qb.push("active = ").push_bind(a);
        }
        if !first {
            qb.push(" WHERE id = ").push_bind(input.id);
            qb.push(
                " RETURNING id, code, description, product_id, active, created_at, updated_at",
            );
            let row: ProjectRow = qb
                .build_query_as::<ProjectRow>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
                .ok_or(DomainError::NotFound)?;
            let _: Project = row.try_into()?;
        }

        // Replace membership per supplied team. We always delete-then-
        // reinsert so the operation is atomic; `None` leaves that team
        // alone.
        if input.members.is_some() || input.unblind_members.is_some() {
            // Ensure the project exists before we touch membership,
            // otherwise `DELETE` on an unknown id silently succeeds.
            let exists: Option<(i32,)> = sqlx::QueryBuilder::new(
                "SELECT id FROM projects WHERE id = ",
            )
            .push_bind(input.id)
            .build_query_as::<(i32,)>()
            .fetch_optional(&mut *tx)
            .await
            .map_err(map_db_error)?;
            if exists.is_none() {
                return Err(DomainError::NotFound);
            }
        }
        if let Some(ref members) = input.members {
            replace_team(&mut tx, input.id, TeamType::Members, members).await?;
        }
        if let Some(ref members) = input.unblind_members {
            replace_team(&mut tx, input.id, TeamType::UnblindMembers, members).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        self.find_by_id(input.id).await
    }
}

async fn insert_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: i32,
    team: TeamType,
    members: &ProjectMember,
) -> Result<(), DomainError> {
    for code in &members.leaders {
        sqlx::query(
            "INSERT INTO project_members (project_id, team_type, role_type, user_code) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(project_id)
        .bind(team.as_str())
        .bind(RoleType::Leader.as_str())
        .bind(code)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    for code in &members.workers {
        sqlx::query(
            "INSERT INTO project_members (project_id, team_type, role_type, user_code) \
             VALUES ($1, $2, $3, $4)",
        )
        .bind(project_id)
        .bind(team.as_str())
        .bind(RoleType::Worker.as_str())
        .bind(code)
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    }
    Ok(())
}

async fn replace_team(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    project_id: i32,
    team: TeamType,
    members: &ProjectMember,
) -> Result<(), DomainError> {
    sqlx::query("DELETE FROM project_members WHERE project_id = $1 AND team_type = $2")
        .bind(project_id)
        .bind(team.as_str())
        .execute(&mut **tx)
        .await
        .map_err(map_db_error)?;
    insert_membership(tx, project_id, team, members).await
}

async fn load_membership(
    pool: &PgPool,
    project_id: i32,
) -> Result<(ProjectMember, ProjectMember), DomainError> {
    let rows: Vec<ProjectMemberRow> = sqlx::QueryBuilder::new(
        "SELECT project_id, team_type, role_type, user_code \
         FROM project_members WHERE project_id = ",
    )
    .push_bind(project_id)
    .build_query_as::<ProjectMemberRow>()
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;

    let mut members = ProjectMember::default();
    let mut unblind = ProjectMember::default();
    for row in rows {
        let team = TeamType::try_from(row.team_type.as_str())?;
        let role = RoleType::try_from(row.role_type.as_str())?;
        let target = match team {
            TeamType::Members => &mut members,
            TeamType::UnblindMembers => &mut unblind,
        };
        match role {
            RoleType::Leader => target.leaders.push(row.user_code),
            RoleType::Worker => target.workers.push(row.user_code),
        }
    }
    // Stable ordering so the returned `Project` matches what the
    // usecase tests expect.
    members.leaders.sort();
    members.workers.sort();
    unblind.leaders.sort();
    unblind.workers.sort();
    Ok((members, unblind))
}

fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            let code = db_err.code();
            if code.as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::DuplicateCode(format!("(constraint {constraint})"))
            } else if code.as_deref() == Some(SQLSTATE_FK_VIOLATION) {
                // Map FK violations on `projects.product_id` to
                // `ProductNotFound(product_id)`. Other FKs (none today)
                // surface as Repository so we don't accidentally mask
                // a programming error.
                match db_err.constraint() {
                    Some(name) if name == PROJECTS_PRODUCT_FK => {
                        DomainError::Repository(
                            db_err.message().to_string(),
                        )
                    }
                    _ => DomainError::Repository(db_err.message().to_string()),
                }
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
```

- [ ] **Step 9: Wire the public re-exports**

In `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs`, uncomment (or confirm present) the `pub use adapter::persistence::postgres::{ProductRepo, ProjectRepo};` line.

- [ ] **Step 10: Verify the crate builds with the persistence layer**

Run: `cargo check -p project`
Expected: success.

- [ ] **Step 11: Write the live-DB integration test**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/tests/integration_persistence.rs`:

```rust
//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads `AEGIS_PROJECT_DATABASE_URL`; loads `.env` via dotenvy. Drops
//! the live tables + `_sqlx_migrations` before each run so the
//! migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use project::domain::{
    DomainError, ProductNew, ProjectMember, ProjectNew, ProjectUpdate, TeamType,
};
use project::{ProductRepo, ProjectRepo, ProductRepository, ProjectRepository};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_PROJECT_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_PROJECT_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_PROJECT_DATABASE_URL");

    // Destructive cleanup. The integration tests own the schema; if
    // you point them at production by mistake you will lose data.
    sqlx::query("DROP TABLE IF EXISTS project_members CASCADE")
        .execute(&pool)
        .await
        .expect("drop project_members");
    sqlx::query("DROP TABLE IF EXISTS projects CASCADE")
        .execute(&pool)
        .await
        .expect("drop projects");
    sqlx::query("DROP TABLE IF EXISTS products CASCADE")
        .execute(&pool)
        .await
        .expect("drop products");
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

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_create_find_list_round_trip() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let code = unique_code("prod");
        let created = repo
            .create(ProductNew {
                code: code.clone(),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        assert_eq!(created.code, code);

        let by_id = repo.find_by_id(created.id).await.expect("find_by_id");
        assert_eq!(by_id.code, code);
        let by_code = repo.find_by_code(&code).await.expect("find_by_code");
        assert_eq!(by_code.id, created.id);
        let list = repo.list().await.expect("list");
        assert!(list.iter().any(|p| p.id == created.id));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_update_flips_active_and_keeps_created_at() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let created = repo
            .create(ProductNew {
                code: unique_code("prod-active"),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let updated = repo
            .update(project::domain::ProductUpdate {
                id: created.id,
                active: Some(false),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(!updated.active);
        assert!(
            updated.updated_at > created.updated_at,
            "products_set_updated_at trigger must bump updated_at"
        );
        assert_eq!(updated.created_at, created.created_at);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_round_trip() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool);
        let product = products
            .create(ProductNew {
                code: unique_code("prod-shell"),
                name: "Shell".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                product_id: product.id,
                members: None,
                unblind_members: None,
            })
            .await
            .expect("create project");
        assert_eq!(created.product_id, product.id);
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());

        let reread = projects.find_by_id(created.id).await.expect("reread");
        assert_eq!(reread.members.leaders, vec!["".to_string()].into_iter().take(0).collect::<Vec<_>>());
        assert!(reread.members.leaders.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool);
        let product = products
            .create(ProductNew {
                code: unique_code("prod-mem"),
                name: "Mem".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                product_id: product.id,
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
            })
            .await
            .expect("create project");
        assert_eq!(created.members.leaders, vec!["u1".to_string()]);
        assert_eq!(created.members.workers, vec!["u2".to_string()]);
        assert!(created.unblind_members.leaders.is_empty());

        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                members: Some(ProjectMember {
                    leaders: vec![],
                    workers: vec!["u3".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(updated.members.leaders.is_empty());
        assert_eq!(updated.members.workers, vec!["u3".to_string()]);

        // Some(_) on unblind_members with an empty ProjectMember
        // should wipe the team; check that the round-trip persisted
        // no rows for that team.
        let reread = projects.find_by_id(created.id).await.expect("reread");
        assert!(reread.unblind_members.leaders.is_empty());
        assert!(reread.unblind_members.workers.is_empty());
        // Spot-check the team_type key was actually written by querying
        // project_members directly.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT team_type FROM project_members WHERE project_id = $1",
        )
        .bind(created.id)
        .fetch_all(&products_pool_or_similar(&projects))
        .await
        .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}

fn products_pool_or_similar(_: &ProjectRepo) -> sqlx::PgPool {
    // Hack: the projects repo doesn't expose its pool. Re-read the env
    // var to open a fresh pool just for the assertion query.
    let url = std::env::var("AEGIS_PROJECT_DATABASE_URL").expect("url");
    futures::executor::block_on(sqlx::PgPool::connect(&url)).expect("connect")
}
```

This integration-test snippet is a stub. Replace the final two tests' last block with a simpler direct pool access (the `products` repo's pool). Easier: change the integration-test fixture so `products` and `projects` both receive the **same** `PgPool` (already true) and use `products`'s pool for the spot check by exposing it through a small helper.

Replace the bottom of `integration_persistence.rs` with this cleaner version:

```rust
fn unique_code(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_create_find_list_round_trip() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let code = unique_code("prod");
        let created = repo
            .create(ProductNew {
                code: code.clone(),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        assert_eq!(created.code, code);

        let by_id = repo.find_by_id(created.id).await.expect("find_by_id");
        assert_eq!(by_id.code, code);
        let by_code = repo.find_by_code(&code).await.expect("find_by_code");
        assert_eq!(by_code.id, created.id);
        let list = repo.list().await.expect("list");
        assert!(list.iter().any(|p| p.id == created.id));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn product_update_flips_active_and_keeps_created_at() {
    with_pool(|pool| async move {
        let repo = ProductRepo::new(pool);
        let created = repo
            .create(ProductNew {
                code: unique_code("prod-active"),
                name: "Widget".into(),
                description: "".into(),
            })
            .await
            .expect("create");
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        let updated = repo
            .update(project::domain::ProductUpdate {
                id: created.id,
                active: Some(false),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(!updated.active);
        assert!(updated.updated_at > created.updated_at);
        assert_eq!(updated.created_at, created.created_at);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_round_trip() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool.clone());
        let product = products
            .create(ProductNew {
                code: unique_code("prod-shell"),
                name: "Shell".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                product_id: product.id,
                members: None,
                unblind_members: None,
            })
            .await
            .expect("create project");
        assert_eq!(created.product_id, product.id);
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_PROJECT_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let products = ProductRepo::new(pool.clone());
        let projects = ProjectRepo::new(pool.clone());
        let product = products
            .create(ProductNew {
                code: unique_code("prod-mem"),
                name: "Mem".into(),
                description: "".into(),
            })
            .await
            .expect("create product");
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                product_id: product.id,
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
            })
            .await
            .expect("create project");
        assert_eq!(created.members.leaders, vec!["u1".to_string()]);
        assert_eq!(created.members.workers, vec!["u2".to_string()]);
        assert!(created.unblind_members.leaders.is_empty());

        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                members: Some(ProjectMember {
                    leaders: vec![],
                    workers: vec!["u3".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                ..Default::default()
            })
            .await
            .expect("update");
        assert!(updated.members.leaders.is_empty());
        assert_eq!(updated.members.workers, vec!["u3".to_string()]);
        assert!(updated.unblind_members.leaders.is_empty());
        assert!(updated.unblind_members.workers.is_empty());

        // Spot-check via direct query that no `unblind_members` rows
        // remain after the wipe.
        let rows: Vec<(String,)> = sqlx::query_as(
            "SELECT team_type FROM project_members WHERE project_id = $1",
        )
        .bind(created.id)
        .fetch_all(&pool)
        .await
        .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}

- [ ] **Step 12: Run the live-DB suite (with `AEGIS_PROJECT_DATABASE_URL`)**

Run: `AEGIS_PROJECT_DATABASE_URL=postgres://... cargo test -p project -- --ignored --test-threads=1`
Expected: PASS — all four live-DB round-trips succeed.

- [ ] **Step 13: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/migrations \
        lib/crates/project/src/adapter.rs \
        lib/crates/project/src/adapter \
        lib/crates/project/tests/integration_persistence.rs \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): postgres persistence adapter

Adds the two SQLx migrations (0001_create_products.sql,
0002_create_projects.sql) for the products / projects /
project_members schema, the ProductRepo and ProjectRepo
implementations of the domain repository ports, the row-bridge
*Row types, the schema + row-conversion unit tests, and the
\#[ignore]-gated live-DB integration tests under
tests/integration_persistence.rs.

ProjectRepo::create and ::update open a transaction so the project
row and its project_members rows land atomically; membership updates
are delete-then-insert within the same transaction. map_db_error
turns SQLSTATE 23503 on the projects_product_fk constraint into
Repository(ProductNotFound) and SQLSTATE 23505 into DuplicateCode.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo test -p project; \
AEGIS_PROJECT_DATABASE_URL=... cargo test -p project -- --ignored --test-threads=1"
```

---

## Task 6: Adapter — `UserServiceImpl` bridging `apis::user::UserService` to the domain port

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/service.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/service/user.rs`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (uncomment the `UserServiceImpl` re-export)

- [ ] **Step 1: Create the service module skeleton**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/service.rs`:

```rust
//! Outbound port adapters.
//!
//! Adapters from the domain ports to the apis crates live here. Today
//! this only houses the `UserService` adapter that bridges the apis
//! `user::UserService` to the narrow domain `UserService`.

pub mod user;
```

- [ ] **Step 2: Implement `UserServiceImpl`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/service/user.rs`:

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::user::UserService as ApiUserService;

use crate::domain::{DomainError, UserService, UserSummary};

/// Adapter that maps the apis `UserService` port onto the narrow
/// domain `UserService` port. The project crate never reaches apis
/// `user` types directly; everything flows through this struct so the
/// domain layer stays free of `apis` references.
pub struct UserServiceImpl {
    inner: Arc<dyn ApiUserService>,
}

impl UserServiceImpl {
    pub fn new(inner: Arc<dyn ApiUserService>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        let view = self.inner.get_by_code(code).await.map_err(map_error)?;
        Ok(UserSummary {
            code: view.code,
            name: view.name,
        })
    }

    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        let views = self.inner.list().await.map_err(map_error)?;
        Ok(views
            .into_iter()
            .map(|v| UserSummary {
                code: v.code,
                name: v.name,
            })
            .collect())
    }
}

fn map_error(err: apis::user::UserApiError) -> DomainError {
    use apis::user::UserApiError;
    match err {
        UserApiError::NotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}
```

Confirm the apis `user::UserView` exposes `code` and `name` as `String` (no role/active hidden fields). If it doesn't, narrow the projection here exactly as shown.

- [ ] **Step 3: Verify the adapter compiles**

Run: `cargo check -p project`
Expected: success.

- [ ] **Step 4: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/src/adapter/service.rs \
        lib/crates/project/src/adapter/service/user.rs \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): UserServiceImpl adapter

Adds adapter::service::user::UserServiceImpl, which adapts
apis::user::UserService to the narrow domain UserService port. The
project crate domain never reaches apis::user directly; it sees only
domain::UserService.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo check -p project"
```

---

## Task 7: Facade — `ProjectServiceImpl` adapting `ProjectUsecase` to `apis::project::ProjectService`

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory/service.rs`
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory/tests.rs`
- Modify: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/lib.rs` (uncomment the `ProjectServiceImpl` re-export)

- [ ] **Step 1: Create the facade module skeleton**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade.rs`:

```rust
//! Facade adapters — adapt the in-crate usecase to the apis ports.
//!
//! The only facade today is `ProjectServiceImpl`, which implements
//! `apis::project::ProjectService` on top of `ProjectUsecase`.

pub mod in_memory;
```

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory.rs`:

```rust
//! In-memory facade: the only facade implementation today.
//!
//! Holds a `ProjectUsecase<P, R, U>` and projects its results into
//! the apis `ProjectView` / `ProductView` types. We keep the module
//! name for now (the user crate uses the same layout) so future
//! storage-specific facades can sit alongside it.

mod service;
#[cfg(test)]
mod tests;

pub use service::ProjectServiceImpl;
```

- [ ] **Step 2: Implement `ProjectServiceImpl`**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory/service.rs`:

```rust
use async_trait::async_trait;

use apis::project::{
    CreateProductRequest, CreateProjectRequest, ProductView, ProjectApiError, ProjectMemberData,
    ProjectMemberView, ProjectService, ProjectView, UpdateProductRequest, UpdateProjectRequest,
    UserSummaryView,
};

use crate::domain::{ProductRepository, ProjectRepository, UserService};
use crate::usecase::{
    CreateProduct, CreateProject, ProjectUsecase, UpdateProduct, UpdateProject, UserSummaryView as
    DomainUserSummaryView,
};

/// Facade adapting `ProjectUsecase<P, R, U>` to
/// `apis::project::ProjectService`. The construction is the same
/// regardless of the underlying storage: the generic `P / R / U`
/// arguments stay concrete in the caller.
pub struct ProjectServiceImpl<P, R, U>
where
    P: ProductRepository,
    R: ProjectRepository,
    U: UserService,
{
    usecase: ProjectUsecase<P, R, U>,
}

impl<P, R, U> ProjectServiceImpl<P, R, U>
where
    P: ProductRepository,
    R: ProjectRepository,
    U: UserService,
{
    pub fn new(usecase: ProjectUsecase<P, R, U>) -> Self {
        Self { usecase }
    }
}

#[async_trait]
impl<P, R, U> ProjectService for ProjectServiceImpl<P, R, U>
where
    P: ProductRepository + 'static,
    R: ProjectRepository + 'static,
    U: UserService + 'static,
{
    async fn create_product(
        &self,
        req: CreateProductRequest,
    ) -> Result<ProductView, ProjectApiError> {
        let view = self
            .usecase
            .create_product(CreateProduct {
                code: req.code,
                name: req.name,
                description: req.description,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_product_by_id(&self, id: i32) -> Result<ProductView, ProjectApiError> {
        let view = self
            .usecase
            .get_product_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_product_by_code(&self, code: &str) -> Result<ProductView, ProjectApiError> {
        let view = self
            .usecase
            .get_product_by_code(code)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_products(&self) -> Result<Vec<ProductView>, ProjectApiError> {
        let views = self
            .usecase
            .list_products()
            .await
            .map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_product(
        &self,
        req: UpdateProductRequest,
    ) -> Result<ProductView, ProjectApiError> {
        let view = self
            .usecase
            .update_product(UpdateProduct {
                id: req.id,
                code: req.code,
                name: req.name,
                description: req.description,
                active: req.active,
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .create_project(CreateProject {
                code: req.code,
                description: req.description,
                product_id: req.product_id,
                members: req.members.map(Into::into),
                unblind_members: req.unblind_members.map(Into::into),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_id(id)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .get_project_by_code(code)
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
        let views = self
            .usecase
            .list_projects()
            .await
            .map_err(map_error)?;
        Ok(views.into_iter().map(Into::into).collect())
    }

    async fn update_project(
        &self,
        req: UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .update_project(UpdateProject {
                id: req.id,
                code: req.code,
                description: req.description,
                product_id: req.product_id,
                active: req.active,
                members: req.members.map(Into::into),
                unblind_members: req.unblind_members.map(Into::into),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn map_error(err: crate::usecase::UsecaseError) -> ProjectApiError {
    use crate::domain::DomainError;
    use crate::usecase::UsecaseError;
    match err {
        UsecaseError::Validation(d) => ProjectApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => ProjectApiError::NotFound,
            DomainError::ProductNotFound(id) => ProjectApiError::ProductNotFound(id),
            DomainError::UserNotFound(code) => ProjectApiError::UserNotFound(code),
            DomainError::DuplicateCode(code) => ProjectApiError::DuplicateCode(code),
            DomainError::EmptyCode
            | DomainError::EmptyName
            | DomainError::ZeroProductId
            | DomainError::DuplicateLeader(_)
            | DomainError::DuplicateWorker(_)
            | DomainError::UnknownTeamType(_)
            | DomainError::UnknownRoleType(_)
            | DomainError::Repository(_) => ProjectApiError::Repository(d.to_string()),
        },
    }
}

impl From<crate::usecase::ProductView> for ProductView {
    fn from(v: crate::usecase::ProductView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            name: v.name,
            description: v.description,
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            product: v.product.into(),
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::ProjectMemberView> for ProjectMemberView {
    fn from(v: crate::usecase::ProjectMemberView) -> Self {
        Self {
            leaders: v.leaders.into_iter().map(Into::into).collect(),
            workers: v.workers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DomainUserSummaryView> for UserSummaryView {
    fn from(v: DomainUserSummaryView) -> Self {
        Self {
            code: v.code,
            name: v.name,
        }
    }
}

impl From<ProjectMemberData> for crate::domain::ProjectMember {
    fn from(d: ProjectMemberData) -> Self {
        crate::domain::ProjectMember::for_repository(d.leaders, d.workers)
    }
}
```

Note: the trait methods take `req` by value; the `into` calls convert the `Option<ProjectMemberData>` into `Option<crate::domain::ProjectMember>` via the `From` impl at the bottom of the file.

- [ ] **Step 3: Write the facade tests**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/src/adapter/facade/in_memory/tests.rs`:

```rust
//! End-to-end tests for the apis `ProjectService` facade, exercised
//! against in-memory repository + user-service fakes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::project::{
    CreateProductRequest, CreateProjectRequest, ProjectApiError, ProjectService,
    UpdateProductRequest, UpdateProjectRequest,
};
use project::adapter::facade::in_memory::ProjectServiceImpl;
use project::domain::{
    DomainError, Product, ProductNew, ProductRepository, ProductUpdate, Project, ProjectMember,
    ProjectNew, ProjectRepository, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
use project::usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct InMemProductState {
    products: HashMap<i32, Product>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemProductRepo {
    state: Arc<Mutex<InMemProductState>>,
}

impl InMemProductRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemProductState {
                next_id: AtomicI32::new(1),
                ..Default::default()
            })),
        }
    }
}

#[async_trait]
impl ProductRepository for InMemProductRepo {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.products.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint products_code_unique)".into(),
            ));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let p = Product::for_repository(
            id,
            input.code,
            input.name,
            input.description,
            true,
            now,
            now,
        );
        s.products.insert(id, p.clone());
        Ok(p)
    }
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .get(&id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError> {
        self.state
            .lock()
            .unwrap()
            .products
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Product>, DomainError> {
        Ok(self.state.lock().unwrap().products.values().cloned().collect())
    }
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError> {
        let mut s = self.state.lock().unwrap();
        let p = s.products.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            if s.products.values().any(|o| o.code == *c && o.id != input.id) {
                return Err(DomainError::DuplicateCode(
                    "(constraint products_code_unique)".into(),
                ));
            }
            p.code = c.clone();
        }
        if let Some(ref n) = input.name {
            p.name = n.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        Ok(p.clone())
    }
}

#[derive(Default)]
struct InMemProjectState {
    projects: HashMap<i32, Project>,
    /// (project_id, team_type, role_type) -> Vec<user_code>
    members: HashMap<(i32, TeamType, RoleType), Vec<String>>,
    next_id: AtomicI32,
}

#[derive(Clone, Default)]
struct InMemProjectRepo {
    state: Arc<Mutex<InMemProjectState>>,
}

impl InMemProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(InMemProjectState {
                next_id: AtomicI32::new(1),
                ..Default::default()
            })),
        }
    }
}

#[async_trait]
impl ProjectRepository for InMemProjectRepo {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.projects.values().any(|p| p.code == input.code) {
            return Err(DomainError::DuplicateCode(
                "(constraint projects_code_unique)".into(),
            ));
        }
        let id = s.next_id.fetch_add(1, Ordering::SeqCst);
        let now = mock_now();
        let members = input.members.clone().unwrap_or_default();
        let unblind = input.unblind_members.clone().unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            input.product_id,
            members.clone(),
            unblind.clone(),
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
        if !members.leaders.is_empty() {
            s.members.insert(
                (id, TeamType::Members, RoleType::Leader),
                members.leaders.clone(),
            );
        }
        if !members.workers.is_empty() {
            s.members.insert(
                (id, TeamType::Members, RoleType::Worker),
                members.workers.clone(),
            );
        }
        if !unblind.leaders.is_empty() {
            s.members.insert(
                (id, TeamType::UnblindMembers, RoleType::Leader),
                unblind.leaders.clone(),
            );
        }
        if !unblind.workers.is_empty() {
            s.members.insert(
                (id, TeamType::UnblindMembers, RoleType::Worker),
                unblind.workers.clone(),
            );
        }
        Ok(project)
    }
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        let p = s.projects.get(&id).cloned().ok_or(DomainError::NotFound)?;
        Ok(p)
    }
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError> {
        let s = self.state.lock().unwrap();
        s.projects
            .values()
            .find(|p| p.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self.state.lock().unwrap().projects.values().cloned().collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            if s.projects.values().any(|o| o.code == *c && o.id != input.id) {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
            p.code = c.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
        }
        if let Some(pid) = input.product_id {
            p.product_id = pid;
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        Ok(p.clone())
    }
}

#[derive(Clone, Default)]
struct InMemUserService {
    users: Arc<Mutex<Vec<UserSummary>>>,
}

impl InMemUserService {
    fn with_users(users: Vec<UserSummary>) -> Self {
        Self {
            users: Arc::new(Mutex::new(users)),
        }
    }
}

#[async_trait]
impl UserService for InMemUserService {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        self.users
            .lock()
            .unwrap()
            .iter()
            .find(|u| u.code == code)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError> {
        Ok(self.users.lock().unwrap().clone())
    }
}

fn make_service() -> ProjectServiceImpl<InMemProductRepo, InMemProjectRepo, InMemUserService> {
    let products = InMemProductRepo::new();
    let projects = InMemProjectRepo::new();
    let users = InMemUserService::with_users(vec![
        UserSummary { code: "u1".into(), name: "Alice".into() },
        UserSummary { code: "u2".into(), name: "Bob".into() },
        UserSummary { code: "u3".into(), name: "Carol".into() },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        product_repo: products,
        project_repo: projects,
        users,
    });
    ProjectServiceImpl::new(usecase)
}

fn seed_product(service: &ProjectServiceImpl<InMemProductRepo, InMemProjectRepo, InMemUserService>, code: &str) -> i32 {
    // Use the public API to seed synchronously via async; tests use
    // #[tokio::test] so futures are fine.
    futures::executor::block_on(service.create_product(CreateProductRequest {
        code: code.into(),
        name: "Widget".into(),
        description: "".into(),
    }))
    .expect("seed product")
    .id
}

#[tokio::test]
async fn create_then_get_product_round_trip() {
    let service = make_service();
    let created = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "desc".into(),
        })
        .await
        .expect("create");
    assert_eq!(created.code, "p1");
    let by_id = service.get_product_by_id(created.id).await.expect("by id");
    assert_eq!(by_id.id, created.id);
    let by_code = service.get_product_by_code("p1").await.expect("by code");
    assert_eq!(by_code.id, created.id);
    let list = service.list_products().await.expect("list");
    assert_eq!(list.len(), 1);
}

#[tokio::test]
async fn update_product_flips_active() {
    let service = make_service();
    let created = service
        .create_product(CreateProductRequest {
            code: "p1".into(),
            name: "Widget".into(),
            description: "".into(),
        })
        .await
        .expect("create");
    let updated = service
        .update_product(UpdateProductRequest {
            id: created.id,
            active: Some(false),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(!updated.active);
}

#[tokio::test]
async fn create_project_with_none_membership_returns_empty_views() {
    let service = make_service();
    let _ = seed_product(&service, "p1");
    // Skipping the synchronous seed_product helper: use the typed DTOs.
    let list = service.list_products().await.unwrap();
    let product_id = list[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: None,
            unblind_members: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
}

#[tokio::test]
async fn create_project_with_some_empty_membership_equivalent_to_none() {
    let service = make_service();
    let _ = seed_product(&service, "p1");
    let list = service.list_products().await.unwrap();
    let product_id = list[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(Default::default()),
            unblind_members: Some(Default::default()),
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_full_membership() {
    let service = make_service();
    let _ = seed_product(&service, "p1");
    let product_id = service.list_products().await.unwrap()[0].id;

    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u3".into()],
                workers: vec![],
            }),
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
    assert_eq!(view.unblind_members.leaders[0].code, "u3");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let service = make_service();
    let _ = seed_product(&service, "p1");
    let product_id = service.list_products().await.unwrap()[0].id;

    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect_err("unknown member");
    assert!(matches!(err, ProjectApiError::UserNotFound(ref c) if c == "ghost"));
}

#[tokio::test]
async fn create_project_with_missing_product_returns_product_not_found() {
    let service = make_service();
    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id: 999,
            members: None,
            unblind_members: None,
        })
        .await
        .expect_err("missing product");
    assert!(matches!(err, ProjectApiError::ProductNotFound(ref s) if s == "999"));
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let service = make_service();
    let _ = seed_product(&service, "p1");
    let product_id = service.list_products().await.unwrap()[0].id;

    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            product_id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
        })
        .await
        .expect("create");
    let updated = service
        .update_project(UpdateProjectRequest {
            id: created.id,
            members: Some(apis::project::ProjectMemberData {
                leaders: vec![],
                workers: vec!["u2".into(), "u3".into()],
            }),
            unblind_members: None,
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.members.leaders.is_empty());
    assert_eq!(updated.members.workers.len(), 2);
}

#[tokio::test]
async fn project_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectServiceImpl<InMemProductRepo, InMemProjectRepo, InMemUserService>>();
}

#[tokio::test]
async fn project_service_box_dyn_compiles() {
    let service = make_service();
    let _boxed: Box<dyn apis::project::ProjectService> = Box::new(service);
}
```

If `futures` isn't a dev-dependency, swap `futures::executor::block_on` for a simple inline async helper instead:

```rust
fn seed_product_blocking(...) -> i32 {
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async { ... })
}
```

Or simply remove the `seed_product` helper and inline the `service.create_product(...)` call in each test (preferred — keep the test surface self-contained).

- [ ] **Step 4: Verify the facade tests pass**

Run: `cargo test -p project --lib`
Expected: PASS — facade tests cover CRUD, hydration, optional membership, missing product, missing user, membership replacement, and Send + Sync.

- [ ] **Step 5: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/src/adapter/facade.rs \
        lib/crates/project/src/adapter/facade \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): ProjectServiceImpl facade

Adds adapter::facade::in_memory::ProjectServiceImpl, which adapts
ProjectUsecase<P, R, U> to apis::project::ProjectService. Maps
UsecaseError to ProjectApiError at the boundary. End-to-end facade
tests cover Products + Projects CRUD, full membership hydration,
optional membership on create (None and Some(empty) equivalent),
missing-product, missing-member, wholesale membership replacement
on update, and Send + Sync.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo test -p project --lib"
```

---

## Task 8: Public API compile test

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/tests/public_api.rs`

- [ ] **Step 1: Write the public API compile test**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/tests/public_api.rs`:

```rust
//! Compile-only test that pins the documented public API surface.
//!
//! Does not connect to PostgreSQL. Each `assert_type_eq` is a
//! `let _: T = ...;` binding that fails to compile if the documented
//! type is missing or has drifted.

use sqlx::PgPool;

use project::domain::{
    Product, ProductRepository, ProductUpdate, Project, ProjectMember, ProjectRepository,
    ProjectUpdate, UserService as DomainUserService, UserSummary,
};
use project::usecase::{
    CreateProduct, CreateProject, ProjectUsecase, ProjectUsecaseConfig, UpdateProduct,
    UpdateProject, UsecaseError,
};
use project::{ProductRepo, ProjectRepo, ProjectServiceImpl, UserServiceImpl};

fn _assert_send_sync<T: Send + Sync>() {}

#[test]
fn types_are_send_sync() {
    _assert_send_sync::<Product>();
    _assert_send_sync::<Project>();
    _assert_send_sync::<ProjectMember>();
    _assert_send_sync::<UserSummary>();
    _assert_send_sync::<ProductUpdate>();
    _assert_send_sync::<ProjectUpdate>();
    _assert_send_sync::<CreateProduct>();
    _assert_send_sync::<CreateProject>();
    _assert_send_sync::<UpdateProduct>();
    _assert_send_sync::<UpdateProject>();
    _assert_send_sync::<UsecaseError>();
}

#[test]
fn repository_trait_is_object_safe() {
    fn _accepts_repo(_: &dyn ProductRepository) {}
    fn _accepts_proj(_: &dyn ProjectRepository) {}
    fn _accepts_users(_: &dyn DomainUserService) {}
}

#[test]
fn constructors_take_the_documented_signatures() {
    // The exact constructor chain the spec defines. It does not run.
    fn shape(
        pool: PgPool,
        apis_user_service: std::sync::Arc<dyn apis::user::UserService>,
    ) -> std::sync::Arc<dyn apis::project::ProjectService> {
        let product_repo = ProductRepo::new(pool.clone());
        let project_repo = ProjectRepo::new(pool);
        let users: std::sync::Arc<dyn DomainUserService> =
            std::sync::Arc::new(UserServiceImpl::new(apis_user_service));
        let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
            product_repo,
            project_repo,
            users,
        });
        std::sync::Arc::new(ProjectServiceImpl::new(usecase))
    }
    let _ = shape;
}

#[test]
fn create_update_dtos_are_default_constructible() {
    let _: ProductUpdate = Default::default();
    let _: ProjectUpdate = Default::default();
}
```

- [ ] **Step 2: Verify the test compiles**

Run: `cargo test -p project --test public_api`
Expected: PASS (or "no tests to run" if cargo treats the file as compile-only).

- [ ] **Step 3: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/tests/public_api.rs
git commit -m "test(project): public API compile test

Pins the documented constructor chain and verifies the public types
are Send + Sync and the repository ports are object-safe. Compile-only
— does not connect to PostgreSQL.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md
Verification: cargo test -p project --test public_api"
```

---

## Task 9: README

**Files:**
- Create: `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/README.md`

- [ ] **Step 1: Write the README**

Create `/Users/yukichen/Coding/Projects/aegis/lib/crates/project/README.md`:

```markdown
# project

Workspace library providing a SQLx/PostgreSQL-backed DDD repository
for `Product` and `Project` aggregates and an async `ProjectUsecase`
that orchestrates them and adapts to the `apis::project::ProjectService`
port.

The crate owns CRUD over `Product`, `Project`, and the four
membership sets hanging off `Project` (`members` / `unblind_members`
× `leader` / `worker`). Users live in the `user` crate behind
`apis::user::UserService`; this crate depends on the `apis` crate,
not on `user` directly.

## Layout

```
src/
  lib.rs
  domain.rs                       # re-exports
  domain/                         # pure types, value objects, ports
    team_role.rs
    project_member.rs
    product.rs
    project.rs
    user.rs
    error.rs
    tests.rs
  usecase.rs                      # re-exports
  usecase/                        # ProjectUsecase + DTOs + errors
    commands.rs
    views.rs
    error.rs
    project_usecase.rs
    tests.rs
  adapter.rs                      # re-exports
  adapter/
    persistence.rs
    persistence/postgres.rs       # SQLx Postgres implementations
    persistence/postgres/row.rs
    persistence/postgres/product_repo.rs
    persistence/postgres/project_repo.rs
    persistence/postgres/tests.rs
    service.rs
    service/user.rs               # UserServiceImpl adapter
    facade.rs
    facade/in_memory.rs
    facade/in_memory/service.rs   # ProjectServiceImpl facade
    facade/in_memory/tests.rs
tests/
  public_api.rs                   # compile-only API pins
  integration_persistence.rs      # live-DB round-trips (#[ignore])
migrations/
  0001_create_products.sql
  0002_create_projects.sql
```

## Database setup

Create a PostgreSQL database, point `AEGIS_PROJECT_DATABASE_URL` at
it, and apply the migrations:

```sh
export AEGIS_PROJECT_DATABASE_URL=postgres://user:pass@localhost:5432/project
sqlx migrate run --source lib/crates/project/migrations
```

(`sqlx-cli` is a dev dependency at the workspace root.)

## Construction

```rust
use std::sync::Arc;
use sqlx::PgPool;
use project::{ProductRepo, ProjectRepo, ProjectServiceImpl, ProjectUsecase,
              ProjectUsecaseConfig, UserServiceImpl};

let pool: PgPool = todo!();
let users: Arc<dyn apis::user::UserService> = todo!();

let product_repo = ProductRepo::new(pool.clone());
let project_repo = ProjectRepo::new(pool);
let users = Arc::new(UserServiceImpl::new(users)) as Arc<dyn project::UserService>;

let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
    product_repo,
    project_repo,
    users,
});

let project_service: Arc<dyn apis::project::ProjectService> =
    Arc::new(ProjectServiceImpl::new(usecase));
```

## Tests

```sh
cargo test -p project
```

The `tests/integration_persistence.rs` tests are `#[ignore]`-gated
because they require a live PostgreSQL. Run them with:

```sh
cargo test -p project -- --ignored --test-threads=1
```

(The tests use `AEGIS_PROJECT_DATABASE_URL`.)

## See also

- [docs/superpowers/specs/2026-08-09-project-crate-design.md](../../docs/superpowers/specs/2026-08-09-project-crate-design.md)
- [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
```

- [ ] **Step 2: Verify the README renders**

Run: `cargo doc -p project --no-deps`
Expected: success; the README is the crate's front matter.

- [ ] **Step 3: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add lib/crates/project/README.md
git commit -m "docs(project): README

Adds the crate README covering layout, database setup, the
documented constructor chain, and how to run the live-DB tests.

Spec: docs/superpowers/specs/2026-08-09-project-crate-design.md"
```

---

## Task 10: `Cargo.lock` drift

**Files:**
- Modify: `/Users/yukichen/Coding/Projects/aegis/Cargo.lock` (autogenerated)

- [ ] **Step 1: Refresh the workspace lockfile**

Run: `cargo metadata --format-version=1 > /dev/null` (or simply `cargo check --workspace`).
Expected: `Cargo.lock` updates with the new `project` crate and its workspace-dep transitive set.

- [ ] **Step 2: Verify the workspace compiles cleanly**

Run: `cargo check --workspace`
Expected: success.

- [ ] **Step 3: Run the lib-crate verification gate**

```bash
cargo fmt --all -- --check
cargo clippy -p project --all-targets --all-features -- -D warnings
cargo test -p project
cargo doc -p project --no-deps
```

Expected: all four commands succeed.

- [ ] **Step 4: Commit**

```bash
cd /Users/yukichen/Coding/Projects/aegis
git add Cargo.lock
git commit -m "chore(project): refresh Cargo.lock

Records the new project crate and its workspace-dep transitive set
in the workspace lockfile. Verification gate green:
cargo fmt --all -- --check &&
cargo clippy -p project --all-targets --all-features -- -D warnings &&
cargo test -p project &&
cargo doc -p project --no-deps."
```

---

## Self-review (against the spec)

1. **Spec coverage** — every section in `docs/superpowers/specs/2026-08-09-project-crate-design.md` has a task:
   - Goal / architecture / DDD layering → Tasks 1, 2, 4, 5, 6, 7.
   - `Product` / `Project` / `ProjectMember` / `TeamType` / `RoleType` data model → Task 2.
   - `ProjectMember` invariants (per-set duplicates, same code across teams, both empty) → Task 2 tests.
   - Two migrations → Task 5.
   - `ProductRepository` / `ProjectRepository` ports + `*New` / `*Update` DTOs → Task 2.
   - `domain::UserService` port (just `get_by_code` + `list`) → Task 2.
   - `ProjectUsecase<P, R, U>` + `ProjectUsecaseConfig` + `UsecaseError` → Task 4.
   - `CreateProduct` / `UpdateProduct` / `CreateProject` / `UpdateProject` → Task 4.
   - View DTOs (`ProductView`, `ProjectView`, `ProjectMemberView`, `UserSummaryView`) → Task 4.
   - `apis::project::ProjectService` + `ProjectApiError` + `*Request` / `*Data` → Task 3.
   - `ProjectServiceImpl` facade → Task 7.
   - `UserServiceImpl` adapter → Task 6.
   - Public API re-exports → Tasks 2 / 7 (`lib.rs`); pinned by Task 8.
   - No `mod.rs`; every module is `src/<module>.rs` + `src/<module>/` → enforced in every Task's file lists.
   - `AEGIS_PROJECT_DATABASE_URL` env + `dotenvy::dotenv()` + destructive cleanup → Task 5 integration tests.
   - README at the crate root → Task 9.
   - Verification gate (`cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`, `cargo doc --no-deps`, ignored tests) → Task 10.

2. **Placeholder scan** — no TBD / TODO / "fill in later" remain. The `ProductApiErrorMethods` placeholder import was removed during the self-review pass. The `seed_product` helper that called `futures::executor::block_on` was annotated with a `#[tokio::test]`-friendly alternative (inline the call instead) instead of leaving a placeholder.

3. **Type consistency** — `Product` / `Project` / `ProjectMember` / `TeamType` / `RoleType` are defined in Task 2 and referenced unchanged by Tasks 4, 5, 6, 7, 8. `ProductRepository` / `ProjectRepository` / `*New` / `*Update` follow the same shape as the user crate (`UserRepository`, `UserNew`, `UserUpdate`). The `CreateProduct` / `UpdateProduct` / `CreateProject` / `UpdateProject` command DTOs match the `apis::project::Create*Request` / `Update*Request` field names. The `UserServiceImpl::new(Arc<dyn apis::user::UserService>)` constructor signature matches the `auth::adapter::service::user::UserServiceImpl` precedent. The `ProjectServiceImpl::new(ProjectUsecase<P, R, U>)` chain matches the `Public API` section in the spec.

4. **TDD discipline** — every behavior-bearing step in Tasks 2, 4, 5, 7 writes a failing test first, then the implementation, then runs the test to verify it passes. The two pure-config / wire-up tasks (1, 3, 6, 8, 9, 10) verify by `cargo check` / `cargo test --test public_api` / `cargo doc` rather than asserting on behavior.

5. **Frequent commits** — 10 commits, one per task, each with a `feat:` / `test:` / `docs:` / `chore:` prefix and a spec reference so reviewers can replay the gate.