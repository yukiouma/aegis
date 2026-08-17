# Project Tag Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `ProjectTag { key, value }` value object to the `project` crate, persisted as a JSONB column on `projects`, and retire the entire `Product` aggregate in the same change so `Project` becomes the only top-level aggregate the crate manages.

**Architecture:** ProjectTag is a serde-enabled struct in the domain layer; `Project` gains a `tags: Vec<ProjectTag>` field stored via `sqlx::types::Json`. Tags are mutated by whole-list replacement on `update_project.tags`. The `Product` family (domain, repo, usecase, apis, table, FK) is deleted end-to-end.

**Tech Stack:** Rust 2024, sqlx 0.9 (with `json` feature), serde / serde_json, thiserror, async-trait, PostgreSQL.

## Global Constraints

- **Workspace `sqlx`** must gain the `json` feature: `features = ["postgres", "runtime-tokio", "macros", "migrate", "chrono", "json"]`. Edit the root `Cargo.toml` once in Task 1.
- **Migration strategy:** squash into a single new `0001_create_projects.sql`. Old `0001_create_products.sql` and `0002_create_projects.sql` are deleted in Task 4.
- **Constraint: tag validation.** `key` and `value` are both non-empty after trim; duplicate keys within the same project are allowed.
- **Constraint: tag mutation.** `None` on `update_project.tags` leaves tags unchanged; `Some(vec)` replaces the whole list. `None` on `create_project.tags` means "no tags on create".
- **Constraint: scope.** `ProjectMember` / `members` / `unblind_members` / `project_members` stay untouched. Only Product-related code is removed.
- **Constraint: error channel.** Tag validation errors flow through the existing `Validation(DomainError::EmptyTagKey | EmptyTagValue)` channel. No new `ProjectApiError` variants are introduced.
- **Verification gate** (every task that compiles must end green on the relevant subset):
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p project --all-targets --all-features -- -D warnings
  cargo test -p project
  cargo test -p project -- --ignored --test-threads=1   # when AEGIS_PROJECT_DATABASE_URL is set
  cargo check --workspace
  ```

## File Structure

### Created

- `lib/crates/project/migrations/0001_create_projects.sql` — single squashed migration
- `lib/crates/project/src/domain/project_tag.rs` — value object + validating constructor

### Modified (project crate)

- `Cargo.toml` — add `serde`, `serde_json` deps
- `src/lib.rs` — re-export surface
- `src/domain.rs` — add `project_tag` module, drop `product`
- `src/domain/error.rs` — add `EmptyTagKey` / `EmptyTagValue`, drop `ProductNotFound` / `ZeroProductId`
- `src/domain/project.rs` — drop `product_id`, add `tags`
- `src/domain/project_member.rs` — unchanged
- `src/domain/team_role.rs` — unchanged
- `src/domain/user.rs` — unchanged
- `src/domain/tests.rs` — update for tag + project signature changes
- `src/usecase.rs` — drop product references
- `src/usecase/commands.rs` — drop `product_id`, add `tags`
- `src/usecase/views.rs` — drop `product`, add `tags`
- `src/usecase/project_usecase.rs` — drop `product_repo` generic + product methods + `product_id` checks, add tag handling
- `src/usecase/error.rs` — unchanged
- `src/usecase/tests.rs` — drop `MockProductRepo` + `*_product_*`, add tag tests
- `src/adapter.rs` — unchanged
- `src/adapter/persistence.rs` — unchanged
- `src/adapter/service.rs` — unchanged
- `src/adapter/service/user.rs` — unchanged
- `src/adapter/facade.rs` — unchanged
- `src/adapter/facade/in_memory.rs` — unchanged
- `src/adapter/facade/in_memory/service.rs` — drop product methods, add tag mapping; two generics
- `src/adapter/facade/in_memory/tests.rs` — drop product tests, add tag tests
- `src/adapter/persistence/postgres.rs` — drop `product_repo` module declaration
- `src/adapter/persistence/postgres/row.rs` — drop `ProductRow`, update `ProjectRow`
- `src/adapter/persistence/postgres/project_repo.rs` — handle tags transactionally
- `src/adapter/persistence/postgres/tests.rs` — drop product migration tests, add tag tests
- `tests/public_api.rs` — update type/field references
- `tests/integration_persistence.rs` — drop product tests, add tag tests
- `README.md` — update module tree + domain model

### Deleted (project crate)

- `src/domain/product.rs`
- `src/adapter/persistence/postgres/product_repo.rs`
- `migrations/0001_create_products.sql`
- `migrations/0002_create_projects.sql`

### Modified (apis crate)

- `Cargo.toml` — add `serde`, `serde_json` deps
- `src/project.rs` — drop `ProductView` / `CreateProductRequest` / `UpdateProductRequest` / `*_product` methods / `ProjectApiError::ProductNotFound`; add `TagData`, `TagView`; update `ProjectView`, `CreateProjectRequest`, `UpdateProjectRequest`

### Modified (server)

- `src/transport/http/dto.rs` — drop product DTOs; add tag DTOs; thread tags through project DTOs
- `src/transport/http/project/handlers.rs` — drop product handlers; thread tags through project handlers + mock
- `src/transport/http/project/router.rs` — drop product routes
- `src/transport/http/openapi.rs` — drop product paths; add tags to project schema
- `src/transport/http/router.rs` — drop `/product` nest; update MockProjectService; drop product fixtures
- `src/run.rs` — drop any `ProductRepo` wiring
- `tests/integration_auth.rs` — verify no product refs (update if any)
- `src/transport/http/error.rs` — drop `product_not_found` mapping

### Modified (desktop)

- `src-tauri/src/http/product.rs` — DELETE
- `src-tauri/src/commands/product.rs` — DELETE
- `src-tauri/src/http/project.rs` — drop `product` field; add `tags` field
- `src-tauri/src/http/mod.rs` — drop `product` module declaration
- `src-tauri/src/commands/mod.rs` — drop `product` module declaration + tauri command registration
- `src-tauri/src/lib.rs` — drop product command registrations

### Unchanged

- `lib/crates/auth`, `lib/crates/user`, `lib/crates/windows-utils`, `lib/crates/project/src/adapter/service.rs` and `user.rs`, `src/adapter/persistence.rs`, `src/adapter/facade.rs`, `src/usecase/error.rs`.

---

## Task 1: Workspace sqlx + apis foundation

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Modify: `lib/crates/apis/Cargo.toml`
- Modify: `lib/crates/apis/src/project.rs`
- Modify: `lib/crates/apis/Cargo.toml` (re-test apis compiles)

The apis crate loses every `*_product` method, gains `TagData`/`TagView`, and updates `ProjectView` / request DTOs. No project-crate code depends on it yet, so this lands green in isolation.

### 1.1 Enable sqlx `json` feature in workspace

- [ ] **Step 1:** Edit `Cargo.toml` (workspace root). Replace the existing `sqlx` block:

```toml
sqlx = { version = "0.9", default-features = false, features = [
    "postgres",
    "runtime-tokio",
    "macros",
    "migrate",
    "chrono",
    "json",
] }
```

- [ ] **Step 2:** Verify the workspace compiles.

Run: `cargo check --workspace`
Expected: green.

### 1.2 Add `serde` + `serde_json` to apis

- [ ] **Step 1:** Edit `lib/crates/apis/Cargo.toml`. Append:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

- [ ] **Step 2:** Verify.

Run: `cargo check -p apis`
Expected: green.

### 1.3 Rewrite `apis::project`

- [ ] **Step 1:** Replace `lib/crates/apis/src/project.rs` entirely with the version below.

```rust
//! Outbound port for project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`ProjectApiError`, `ProjectView`, `ProjectMemberView`,
//! `UserSummaryView`, `TagData`, `TagView`, `*Request`) are defined
//! alongside the trait so a single `use apis::project::*;` brings the
//! whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}

/// Wire-shaped tag data. `key` and `value` are both required and
/// non-empty; the backend enforces that contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagData {
    pub key: String,
    pub value: String,
}

/// Server-side projection of a tag. Same shape as [`TagData`]; kept
/// as a distinct type so the wire DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

/// Safe projection of a project: membership lists are hydrated to
/// `Vec<UserSummaryView>`; tags are passed through as `Vec<TagView>`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub tags: Vec<TagView>,
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
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    /// Optional. Omit (or pass an empty `ProjectMemberData`) to create
    /// the project with no membership rows; the shell can be filled in
    /// via a later `update_project` call.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// Optional. `None` and `Some(empty)` both mean "no tags on create".
    pub tags: Option<Vec<TagData>>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    /// `None` = leave tags unchanged; `Some(vec)` = whole-list replace.
    pub tags: Option<Vec<TagData>>,
}

/// Outbound port for project lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn ProjectService>` can be shared state in
/// an async server (axum, tarpc, etc.).
#[async_trait]
pub trait ProjectService: Send + Sync {
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

- [ ] **Step 2:** Verify apis compiles.

Run: `cargo check -p apis`
Expected: green. (server, desktop, project all break here — they're fixed in later tasks; don't cargo-check workspace yet.)

- [ ] **Step 3:** Commit.

```bash
git add Cargo.toml lib/crates/apis/Cargo.toml lib/crates/apis/src/project.rs
git commit -m "feat(apis): add TagData/TagView, drop Product family from project port

The apis::project port gains TagData (request shape) and TagView
(server-side projection), and loses every *_product method along
with ProductView / CreateProductRequest / UpdateProductRequest /
ProjectApiError::ProductNotFound. ProjectView loses its
product: ProductView field and gains tags: Vec<TagView>.
CreateProjectRequest / UpdateProjectRequest drop product_id and
gain tags: Option<Vec<TagData>>.

Workspace sqlx gains the json feature so ProjectTag can round-trip
through JSONB downstream.

Spec coverage: apis-side shape per docs/superpowers/specs/
2026-08-17-project-tag-design.md (Apis types section).

Verification: cargo check -p apis"
```

---

## Task 2: Domain layer — add ProjectTag, update Project, drop Product

**Files:**
- Create: `lib/crates/project/src/domain/project_tag.rs`
- Delete: `lib/crates/project/src/domain/product.rs`
- Modify: `lib/crates/project/src/domain.rs`
- Modify: `lib/crates/project/src/domain/error.rs`
- Modify: `lib/crates/project/src/domain/project.rs`
- Modify: `lib/crates/project/src/domain/tests.rs`
- Modify: `lib/crates/project/Cargo.toml`
- Modify: `lib/crates/project/src/lib.rs`

### 2.1 Add `serde` / `serde_json` to the project crate

- [ ] **Step 1:** Edit `lib/crates/project/Cargo.toml`. Append inside `[dependencies]`:

```toml
serde = { workspace = true }
serde_json = { workspace = true }
```

### 2.2 Write the failing domain tests

- [ ] **Step 1:** Edit `lib/crates/project/src/domain/tests.rs`. Add three tests at the bottom of the file (anywhere — convention is alphabetical / appended):

```rust
#[test]
fn project_tag_new_rejects_empty_key() {
    let err = ProjectTag::new("".into(), "v".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagKey));
}

#[test]
fn project_tag_new_rejects_empty_value() {
    let err = ProjectTag::new("k".into(), "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyTagValue));
}

#[test]
fn project_tag_new_accepts_valid_input() {
    let t = ProjectTag::new("Product".into(), "DEMO-001".into()).unwrap();
    assert_eq!(t.key, "Product");
    assert_eq!(t.value, "DEMO-001");
}
```

- [ ] **Step 2:** Run them; confirm compile failure (ProjectTag / DomainError variants don't exist).

Run: `cargo test -p project --lib domain::tests::project_tag_new_rejects_empty_key 2>&1 | tail -20`
Expected: compile error mentioning `ProjectTag` / `EmptyTagKey` / `EmptyTagValue`.

### 2.3 Create `ProjectTag`

- [ ] **Step 1:** Create `lib/crates/project/src/domain/project_tag.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Wire-shape value object persisted inside `projects.tags`.
///
/// Two string fields, both required and non-empty after trim.
/// Duplicate keys within the same project are intentionally allowed —
/// the same key may carry multiple distinct values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTag {
    pub key: String,
    pub value: String,
}

impl ProjectTag {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects empty / whitespace `key` and `value`.
    pub fn new(key: String, value: String) -> Result<Self, DomainError> {
        if key.trim().is_empty() {
            return Err(DomainError::EmptyTagKey);
        }
        if value.trim().is_empty() {
            return Err(DomainError::EmptyTagValue);
        }
        Ok(Self { key, value })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column.
    #[allow(dead_code)]
    pub(crate) fn for_repository(key: String, value: String) -> Self {
        Self { key, value }
    }
}
```

### 2.4 Update `DomainError`

- [ ] **Step 1:** Replace `lib/crates/project/src/domain/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("code must not be empty")]
    EmptyCode,

    #[error("tag key must not be empty")]
    EmptyTagKey,

    #[error("tag value must not be empty")]
    EmptyTagValue,

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

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("code already exists: {0}")]
    DuplicateCode(String),

    #[error("repository error: {0}")]
    Repository(String),
}
```

(`ProductNotFound`, `ZeroProductId`, `EmptyName` removed; the variants were only reachable through Product.)

### 2.5 Update `Project` and remove `product_id`

- [ ] **Step 1:** Replace `lib/crates/project/src/domain/project.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::project_member::ProjectMember;
use super::project_tag::ProjectTag;

#[derive(Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub tags: Vec<ProjectTag>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Project {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn new(
        id: i32,
        code: String,
        description: String,
        members: ProjectMember,
        unblind_members: ProjectMember,
        tags: Vec<ProjectTag>,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        Ok(Self {
            id,
            code,
            description,
            members,
            unblind_members,
            tags,
            active,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(dead_code, clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i32,
        code: String,
        description: String,
        members: ProjectMember,
        unblind_members: ProjectMember,
        tags: Vec<ProjectTag>,
        active: bool,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            code,
            description,
            members,
            unblind_members,
            tags,
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
            .field("members", &self.members)
            .field("unblind_members", &self.unblind_members)
            .field("tags", &self.tags)
            .field("active", &self.active)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

/// Input DTO for `ProjectRepository::create`.
#[derive(Debug, Clone)]
pub struct ProjectNew {
    pub code: String,
    pub description: String,
    /// Optional. `None` and `Some(empty)` are equivalent — neither
    /// inserts any `project_members` rows for that team. Letting the
    /// field be absent keeps the "create shell, add members later"
    /// flow ergonomic.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// Optional. `None` and `Some(empty)` are equivalent — neither
    /// inserts any tags. Same ergonomics as `members`.
    pub tags: Option<Vec<ProjectTag>>,
}

/// Input DTO for `ProjectRepository::update`. Every field is optional
/// so the usecase can pass only the fields that actually changed.
#[derive(Debug, Clone, Default)]
pub struct ProjectUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe that
    /// team's rows. The two are distinct on update.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// `None` = leave tags unchanged; `Some(vec)` = whole-list replace.
    pub tags: Option<Vec<ProjectTag>>,
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

### 2.6 Update `domain.rs` re-exports + delete `product.rs`

- [ ] **Step 1:** Delete `lib/crates/project/src/domain/product.rs`.

```bash
git rm lib/crates/project/src/domain/product.rs
```

- [ ] **Step 2:** Replace `lib/crates/project/src/domain.rs`:

```rust
mod error;
mod project;
mod project_member;
mod project_tag;
mod team_role;
#[cfg(test)]
mod tests;
mod user;

pub use error::DomainError;
pub use project::{Project, ProjectNew, ProjectRepository, ProjectUpdate};
pub use project_member::ProjectMember;
pub use project_tag::ProjectTag;
pub use team_role::{RoleType, TeamType};
pub use user::{UserService, UserSummary};
```

### 2.7 Update `lib.rs` re-exports

- [ ] **Step 1:** Edit `lib/crates/project/src/lib.rs`. Drop every Product reference and add `ProjectTag`:

```rust
//! # project crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for the `Project` aggregate (with `ProjectTag` JSONB tags) and an
//! async `ProjectUsecase` that orchestrates them and adapts to the
//! `apis::project::ProjectService` port.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::ProjectServiceImpl;
pub use adapter::persistence::postgres::ProjectRepo;
pub use adapter::service::user::UserServiceImpl;
pub use domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType, UserService, UserSummary,
};
pub use usecase::{
    CreateProject, ProjectMemberView, ProjectUsecase, ProjectUsecaseConfig, ProjectView, TagView,
    UpdateProject, UsecaseError, UserSummaryView,
};
```

### 2.8 Update the existing `domain/tests.rs`

- [ ] **Step 1:** Edit `lib/crates/project/src/domain/tests.rs`:
  - Drop `product_new_*` tests entirely.
  - Drop `project_new_rejects_zero_product_id`.
  - Update `project_new_accepts_valid_input` to call `Project::new(9, "proj9".into(), "desc".into(), ProjectMember::default(), ProjectMember::default(), vec![], true, test_now(), test_now())`.
  - Update `project_new_rejects_empty_code` to use the same signature.

(Exact source: in the current file, replace the three `product_new_*` blocks and the two `project_new_rejects_zero_product_id` / `project_new_accepts_valid_input` blocks. The new constructor is:

```rust
pub(crate) fn new(
    id: i32,
    code: String,
    description: String,
    members: ProjectMember,
    unblind_members: ProjectMember,
    tags: Vec<ProjectTag>,
    active: bool,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
) -> Result<Self, DomainError>
```

The two updated tests:

```rust
#[test]
fn project_new_rejects_empty_code() {
    let m = ProjectMember::default();
    let err = Project::new(
        1,
        "".into(),
        "desc".into(),
        m.clone(),
        m,
        vec![],
        true,
        test_now(),
        test_now(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn project_new_accepts_valid_input() {
    let m = ProjectMember::default();
    let p = Project::new(
        9,
        "proj9".into(),
        "desc".into(),
        m.clone(),
        m,
        vec![],
        true,
        test_now(),
        test_now(),
    )
    .unwrap();
    assert_eq!(p.id, 9);
    assert_eq!(p.tags, vec![]);
}
```

### 2.9 Verify + commit

- [ ] **Step 1:** Run the project crate tests.

Run: `cargo test -p project --lib domain::`
Expected: green. (Other layers still broken; that's fine.)

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/Cargo.toml \
        lib/crates/project/src/domain.rs \
        lib/crates/project/src/domain/error.rs \
        lib/crates/project/src/domain/project.rs \
        lib/crates/project/src/domain/project_tag.rs \
        lib/crates/project/src/domain/tests.rs \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): add ProjectTag value object, drop Product aggregate

Domain layer now defines ProjectTag { key, value } with a validating
constructor (both fields non-empty after trim; duplicate keys
allowed). Project gains tags: Vec<ProjectTag> and loses product_id.
ProjectNew and ProjectUpdate thread tags through with the same
None/Some(empty) semantics as members / unblind_members.

The entire Product family is retired: Product / ProductNew /
ProductUpdate / ProductRepository / EmptyName / ProductNotFound /
ZeroProductId all gone. Only Project remains as a top-level
aggregate.

Spec coverage: Domain types, Data Model section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification: cargo test -p project --lib domain::"
```

---

## Task 3: Usecase + commands + views

**Files:**
- Modify: `lib/crates/project/src/usecase.rs`
- Modify: `lib/crates/project/src/usecase/commands.rs`
- Modify: `lib/crates/project/src/usecase/views.rs`
- Modify: `lib/crates/project/src/usecase/project_usecase.rs`

### 3.1 Update `commands.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/commands.rs`:

```rust
use crate::domain::{ProjectMember, ProjectTag};

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    pub tags: Option<Vec<ProjectTag>>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProject {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(empty)` = wipe.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// `None` = leave tags unchanged; `Some(vec)` = whole-list replace.
    pub tags: Option<Vec<ProjectTag>>,
}
```

### 3.2 Update `views.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/views.rs`:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{Project, ProjectTag, UserSummary};

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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

impl From<ProjectTag> for TagView {
    fn from(t: ProjectTag) -> Self {
        Self {
            key: t.key,
            value: t.value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub tags: Vec<TagView>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectView {
    /// Build the view around a domain `Project`. Membership lists
    /// must already be hydrated to `ProjectMemberView` (look up user
    /// summaries before calling). Tags pass straight through.
    pub fn from_project(
        project: Project,
        members: ProjectMemberView,
        unblind_members: ProjectMemberView,
    ) -> Self {
        Self {
            id: project.id,
            code: project.code,
            description: project.description,
            members,
            unblind_members,
            tags: project.tags.into_iter().map(Into::into).collect(),
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}
```

### 3.3 Update `usecase.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase.rs`:

```rust
mod commands;
mod error;
mod project_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{CreateProject, UpdateProject};
pub use error::UsecaseError;
pub use project_usecase::{ProjectUsecase, ProjectUsecaseConfig};
pub use views::{ProjectMemberView, ProjectView, TagView, UserSummaryView};
```

### 3.4 Update `project_usecase.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/project_usecase.rs`:

```rust
use std::collections::HashMap;

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectUpdate,
    ProjectTag, UserService, UserSummary,
};

use super::commands::{CreateProject, UpdateProject};
use super::error::UsecaseError;
use super::views::{ProjectMemberView, ProjectView};

pub struct ProjectUsecaseConfig<R: ProjectRepository, U: UserService> {
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<R: ProjectRepository, U: UserService> {
    project_repo: R,
    users: U,
}

impl<R: ProjectRepository, U: UserService> ProjectUsecase<R, U> {
    pub fn new(cfg: ProjectUsecaseConfig<R, U>) -> Self {
        Self {
            project_repo: cfg.project_repo,
            users: cfg.users,
        }
    }

    // -------- Projects --------

    pub async fn create_project(&self, cmd: CreateProject) -> Result<ProjectView, UsecaseError> {
        validate_create_project(&cmd)?;

        let new_project = self
            .project_repo
            .create(ProjectNew {
                code: cmd.code,
                description: cmd.description,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
                tags: cmd.tags,
            })
            .await?;

        self.hydrate_project_view(new_project).await
    }

    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError> {
        let project = self.project_repo.find_by_id(id).await?;
        self.hydrate_project_view(project).await
    }

    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError> {
        if code.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyCode));
        }
        let project = self.project_repo.find_by_code(code).await?;
        self.hydrate_project_view(project).await
    }

    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError> {
        let projects = self.project_repo.list().await?;
        let all_users = self.users.list().await?;
        let mut out = Vec::with_capacity(projects.len());
        for project in projects {
            let view = hydrate_with(&all_users, project)?;
            out.push(view);
        }
        Ok(out)
    }

    pub async fn update_project(&self, cmd: UpdateProject) -> Result<ProjectView, UsecaseError> {
        validate_update_project(&cmd)?;
        let updated = self
            .project_repo
            .update(ProjectUpdate {
                id: cmd.id,
                code: cmd.code,
                description: cmd.description,
                active: cmd.active,
                members: cmd.members,
                unblind_members: cmd.unblind_members,
                tags: cmd.tags,
            })
            .await?;
        self.hydrate_project_view(updated).await
    }

    // -------- helpers --------

    async fn hydrate_project_view(&self, project: Project) -> Result<ProjectView, UsecaseError> {
        let all_users = self.users.list().await?;
        hydrate_with(&all_users, project)
    }
}

/// Bucket the supplied user summaries into a project's two teams and
/// produce a `ProjectView`. Pure (no I/O) so tests can exercise it
/// directly through the usecase.
fn hydrate_with(
    users: &[UserSummary],
    project: Project,
) -> Result<ProjectView, UsecaseError> {
    let by_code: HashMap<&str, &UserSummary> = users.iter().map(|u| (u.code.as_str(), u)).collect();
    let members = project.members.clone();
    let unblind_members = project.unblind_members.clone();

    let leaders: Vec<UserSummary> = lookup_set(&by_code, &members.leaders)?;
    let workers: Vec<UserSummary> = lookup_set(&by_code, &members.workers)?;
    let members_view = ProjectMemberView {
        leaders: leaders.into_iter().map(Into::into).collect(),
        workers: workers.into_iter().map(Into::into).collect(),
    };

    let unblind_leaders: Vec<UserSummary> = lookup_set(&by_code, &unblind_members.leaders)?;
    let unblind_workers: Vec<UserSummary> = lookup_set(&by_code, &unblind_members.workers)?;
    let unblind_view = ProjectMemberView {
        leaders: unblind_leaders.into_iter().map(Into::into).collect(),
        workers: unblind_workers.into_iter().map(Into::into).collect(),
    };

    Ok(ProjectView::from_project(project, members_view, unblind_view))
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

fn validate_create_project(cmd: &CreateProject) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref tags) = cmd.tags {
        for tag in tags {
            ProjectTag::new(tag.key.clone(), tag.value.clone())?;
        }
    }
    Ok(())
}

fn validate_update_project(cmd: &UpdateProject) -> Result<(), UsecaseError> {
    if let Some(ref c) = cmd.code
        && c.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    if let Some(ref m) = cmd.members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref m) = cmd.unblind_members {
        ProjectMember::new(m.leaders.clone(), m.workers.clone())?;
    }
    if let Some(ref tags) = cmd.tags {
        for tag in tags {
            ProjectTag::new(tag.key.clone(), tag.value.clone())?;
        }
    }
    Ok(())
}
```

### 3.5 Verify (build, don't run tests yet — they still reference product)

- [ ] **Step 1:** Run a build check to catch typos.

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: errors in tests/usecase/tests.rs, tests/public_api.rs, tests/integration_persistence.rs, src/adapter/**/*.rs — that's expected, fixed in later tasks.

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/usecase.rs \
        lib/crates/project/src/usecase/commands.rs \
        lib/crates/project/src/usecase/views.rs \
        lib/crates/project/src/usecase/project_usecase.rs
git commit -m "refactor(project): usecase drops Product, threads tags through

ProjectUsecase loses its product_repo generic and every
*_product_* method. CreateProject / UpdateProject drop product_id
and gain tags: Option<Vec<ProjectTag>>. validate_create_project /
validate_update_project enforce non-empty tag key + value via
ProjectTag::new.

ProjectView loses its product field and gains tags: Vec<TagView>;
ProjectView::from_project's signature shrinks accordingly.

Spec coverage: Usecase section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification: cargo check -p project --lib (errors in tests +
adapter are expected; fixed in subsequent commits)"
```

---

## Task 4: Migration + persistence

**Files:**
- Delete: `lib/crates/project/migrations/0001_create_products.sql`
- Delete: `lib/crates/project/migrations/0002_create_projects.sql`
- Create: `lib/crates/project/migrations/0001_create_projects.sql`
- Delete: `lib/crates/project/src/adapter/persistence/postgres/product_repo.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/row.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`

### 4.1 Squash the migrations

- [ ] **Step 1:** Delete the old migrations.

```bash
git rm lib/crates/project/migrations/0001_create_products.sql \
       lib/crates/project/migrations/0002_create_projects.sql
```

- [ ] **Step 2:** Create `lib/crates/project/migrations/0001_create_projects.sql`:

```sql
-- 0001_create_projects.sql
--
-- Single migration for the `project` crate. Replaces the previous
-- two-file history (products + projects + project_members) now that
-- the `Product` aggregate has been retired and `projects` carries a
-- JSONB `tags` array.
--
-- Layout:
--   * `projects`
--       - `id`          - surrogate primary key.
--       - `code`        - caller-chosen stable identifier; unique.
--       - `description` - free-form long description. Defaults to empty.
--       - `active`      - soft-delete flag (no hard DELETE).
--       - `tags`        - JSONB array of `{"key": "...", "value": "..."}`
--                         objects. Default is the empty array.
--                         CHECK constraint enforces the array shape
--                         against direct-SQL inserts.
--       - `created_at`  - DEFAULT NOW() at insert.
--       - `updated_at`  - DEFAULT NOW() at insert; the
--                         `projects_set_updated_at` trigger refreshes it.
--
--   * `project_members`
--       - Composite PK on (project_id, team_type, role_type, user_code).
--       - `team_type`  ∈ {'members', 'unblind_members'}.
--       - `role_type`  ∈ {'leader', 'worker'}.
--       - ON DELETE CASCADE on the FK so wiping a project also wipes
--         its membership rows.

CREATE TABLE projects (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT projects_code_unique UNIQUE (code),
    CONSTRAINT projects_tags_is_array CHECK (jsonb_typeof(tags) = 'array')
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

### 4.2 Delete `product_repo.rs`

- [ ] **Step 1:**

```bash
git rm lib/crates/project/src/adapter/persistence/postgres/product_repo.rs
```

### 4.3 Update `postgres.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres.rs`:

```rust
//! PostgreSQL-backed implementation of `ProjectRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the user crate.
//! `ProjectRepo::create` / `update` open a transaction so the project
//! row, the `project_members` rows, and the JSONB `tags` payload land
//! atomically.
//!
//! `row` is `pub(crate)` and is NOT re-exported at the crate root.

pub(crate) mod project_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use project_repo::ProjectRepo;
```

### 4.4 Update `row.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres/row.rs`:

```rust
//! Row -> domain conversion for the SQLx repository.
//!
//! `ProjectRow` is the shape returned by `sqlx::query_as`. It is NOT
//! re-exported at the crate root; only the repository uses it.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{DomainError, Project, ProjectMember, ProjectTag};

#[derive(Clone, FromRow)]
pub struct ProjectRow {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub active: bool,
    pub tags: sqlx::types::Json<Vec<ProjectTag>>,
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
            ProjectMember::default(),
            ProjectMember::default(),
            row.tags.0,
            row.active,
            row.created_at,
            row.updated_at,
        ))
    }
}

/// One row from `project_members`.
#[derive(Clone, FromRow)]
#[allow(dead_code)]
pub struct ProjectMemberRow {
    pub project_id: i32,
    pub team_type: String,
    pub role_type: String,
    pub user_code: String,
}
```

### 4.5 Update `project_repo.rs` for tags

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`:

```rust
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType,
};

use super::row::{ProjectMemberRow, ProjectRow};

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

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

        let tags_json = sqlx::types::Json(&input.tags.unwrap_or_default());

        let row: ProjectRow = sqlx::QueryBuilder::new(
            "INSERT INTO projects (code, description, active, tags) VALUES (",
        )
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.description)
        .push(", ")
        .push_bind(true)
        .push(", ")
        .push_bind(tags_json)
        .push(") RETURNING id, code, description, active, tags, created_at, updated_at")
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
            "SELECT id, code, description, active, tags, created_at, updated_at \
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
            "SELECT id, code, description, active, tags, created_at, updated_at \
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
            "SELECT id, code, description, active, tags, created_at, updated_at \
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
        // touch membership or tags.
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
        if let Some(a) = input.active {
            sep(&mut qb);
            qb.push("active = ").push_bind(a);
        }
        if !first {
            qb.push(" WHERE id = ").push_bind(input.id);
            qb.push(
                " RETURNING id, code, description, active, tags, created_at, updated_at",
            );
            let row: ProjectRow = qb
                .build_query_as::<ProjectRow>()
                .fetch_optional(&mut *tx)
                .await
                .map_err(map_db_error)?
                .ok_or(DomainError::NotFound)?;
            let _: Project = row.try_into()?;
        }

        // Replace membership per supplied team. We always
        // delete-then-reinsert so the operation is atomic; `None`
        // leaves that team alone.
        if input.members.is_some() || input.unblind_members.is_some() {
            // Ensure the project exists before we touch membership,
            // otherwise `DELETE` on an unknown id silently succeeds.
            let exists: Option<(i32,)> =
                sqlx::QueryBuilder::new("SELECT id FROM projects WHERE id = ")
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

        // Whole-list replace for tags, in the same transaction.
        if let Some(ref tags) = input.tags {
            let exists: Option<(i32,)> =
                sqlx::QueryBuilder::new("SELECT id FROM projects WHERE id = ")
                    .push_bind(input.id)
                    .build_query_as::<(i32,)>()
                    .fetch_optional(&mut *tx)
                    .await
                    .map_err(map_db_error)?;
            if exists.is_none() {
                return Err(DomainError::NotFound);
            }
            sqlx::QueryBuilder::new("UPDATE projects SET tags = ")
                .push_bind(sqlx::types::Json(tags))
                .push(" WHERE id = ")
                .push_bind(input.id)
                .build()
                .execute(&mut *tx)
                .await
                .map_err(map_db_error)?;
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

### 4.6 Verify build + commit

- [ ] **Step 1:**

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: only test files still broken.

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/migrations/0001_create_projects.sql \
        lib/crates/project/src/adapter/persistence/postgres.rs \
        lib/crates/project/src/adapter/persistence/postgres/row.rs \
        lib/crates/project/src/adapter/persistence/postgres/project_repo.rs
git commit -m "feat(project): squash migrations, add JSONB tags column

Migration history collapses to a single 0001_create_projects.sql
that defines only projects (with tags JSONB NOT NULL DEFAULT
'[]'::jsonb and a jsonb_typeof CHECK) and project_members. The
old products + products_set_updated_at trigger are gone.

ProjectRow gains tags: sqlx::types::Json<Vec<ProjectTag>>.
ProjectRepo::create / update thread tags through the existing
transaction (delete-then-insert on update when Some, no-op on
None). ProductRepo / product_repo.rs / ProductRow / ProductRow
materialisation are deleted.

Spec coverage: Schema + Persistence sections of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification: cargo check -p project --lib (test files still
broken; fixed in subsequent commits)"
```

---

## Task 5: Facade wiring (ProjectServiceImpl — two generics)

**Files:**
- Modify: `lib/crates/project/src/adapter/facade/in_memory/service.rs`

### 5.1 Rewrite `service.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/facade/in_memory/service.rs`:

```rust
use async_trait::async_trait;

use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectMemberData, ProjectMemberView as ApiProjectMemberView,
    ProjectService, ProjectView, TagData, TagView, UpdateProjectRequest,
    UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{ProjectMember, ProjectRepository, ProjectTag, UserService};
use crate::usecase::{
    CreateProject, ProjectUsecase, UpdateProject, UserSummaryView as DomainUserSummaryView,
};

/// Facade adapting `ProjectUsecase<R, U>` to
/// `apis::project::ProjectService`. The construction is the same
/// regardless of the underlying storage: the generic `R / U`
/// arguments stay concrete in the caller.
pub struct ProjectServiceImpl<R, U>
where
    R: ProjectRepository,
    U: UserService,
{
    usecase: ProjectUsecase<R, U>,
}

impl<R, U> ProjectServiceImpl<R, U>
where
    R: ProjectRepository,
    U: UserService,
{
    pub fn new(usecase: ProjectUsecase<R, U>) -> Self {
        Self { usecase }
    }
}

#[async_trait]
impl<R, U> ProjectService for ProjectServiceImpl<R, U>
where
    R: ProjectRepository + 'static,
    U: UserService + 'static,
{
    async fn create_project(
        &self,
        req: CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        let view = self
            .usecase
            .create_project(CreateProject {
                code: req.code,
                description: req.description,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                tags: req.tags.map(|ts| ts.into_iter().map(TagData::into_domain).collect()),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }

    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError> {
        let view = self.usecase.get_project_by_id(id).await.map_err(map_error)?;
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
        let views = self.usecase.list_projects().await.map_err(map_error)?;
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
                active: req.active,
                members: req.members.map(member_data_to_domain),
                unblind_members: req.unblind_members.map(member_data_to_domain),
                tags: req.tags.map(|ts| ts.into_iter().map(TagData::into_domain).collect()),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn member_data_to_domain(d: ProjectMemberData) -> ProjectMember {
    ProjectMember::for_repository(d.leaders, d.workers)
}

fn map_error(err: crate::usecase::UsecaseError) -> ProjectApiError {
    use crate::domain::DomainError;
    use crate::usecase::UsecaseError;
    match err {
        UsecaseError::Validation(d) => ProjectApiError::Validation(d.to_string()),
        UsecaseError::Repository(d) => match d {
            DomainError::NotFound => ProjectApiError::NotFound,
            DomainError::UserNotFound(code) => ProjectApiError::UserNotFound(code),
            DomainError::DuplicateCode(code) => ProjectApiError::DuplicateCode(code),
            other => ProjectApiError::Repository(other.to_string()),
        },
    }
}

// ---- From impls: domain usecase views -> apis views ----

// Bridge for the request-side `TagData` so the apis port doesn't need
// to reach into the domain types.
impl TagData {
    fn into_domain(self) -> ProjectTag {
        // The usecase / domain layer re-validates via
        // `ProjectTag::new`; if the wire payload violated the
        // non-empty contract, that re-validation surfaces as
        // `UsecaseError::Validation(EmptyTagKey | EmptyTagValue)`.
        ProjectTag::for_repository(self.key, self.value)
    }
}

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            tags: v.tags.into_iter().map(TagView::from).collect(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::TagView> for TagView {
    fn from(v: crate::usecase::TagView) -> Self {
        Self {
            key: v.key,
            value: v.value,
        }
    }
}

impl From<crate::usecase::ProjectMemberView> for ApiProjectMemberView {
    fn from(v: crate::usecase::ProjectMemberView) -> Self {
        Self {
            leaders: v.leaders.into_iter().map(Into::into).collect(),
            workers: v.workers.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<DomainUserSummaryView> for ApiUserSummaryView {
    fn from(v: DomainUserSummaryView) -> Self {
        Self {
            code: v.code,
            name: v.name,
        }
    }
}
```

### 5.2 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: only test files still broken.

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/adapter/facade/in_memory/service.rs
git commit -m "refactor(project): facade drops product methods, threads tags

ProjectServiceImpl loses its product_repo generic and its five
*_product impl arms. create_project / update_project pass tags
through to the usecase via a TagData::into_domain helper. The
error mapper drops the DomainError::ProductNotFound arm.

Spec coverage: Facade section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification: cargo check -p project --lib"
```

---

## Task 6: Tests in the project crate

**Files:**
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs`
- Modify: `lib/crates/project/src/usecase/tests.rs`
- Modify: `lib/crates/project/src/adapter/facade/in_memory/tests.rs`
- Modify: `lib/crates/project/tests/public_api.rs`
- Modify: `lib/crates/project/tests/integration_persistence.rs`

Each test file is rewritten wholesale. The replacements preserve the existing test coverage (minus product), add tag coverage, and re-pin the updated type/field shapes.

### 6.1 Update `postgres/tests.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

```rust
//! Schema + row-conversion tests for the PostgreSQL adapter.
//!
//! These tests do NOT require a live database. They read the migration
//! file and the row-bridge impls directly. Live-database round-trips
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
fn projects_migration_creates_projects_table() {
    let sql = load_migration("0001_create_projects.sql");
    let block = create_table_block(&sql);
    assert!(block.contains("CREATE TABLE") && block.contains("projects"));
}

#[test]
fn projects_migration_has_required_columns() {
    let block = create_table_block(&load_migration("0001_create_projects.sql"));
    let upper = block.to_uppercase();
    for required in [
        "ID INTEGER",
        "CODE TEXT",
        "DESCRIPTION TEXT",
        "ACTIVE BOOLEAN",
        "TAGS JSONB NOT NULL DEFAULT '[]'::JSONB",
        "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
        "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    ] {
        assert!(
            upper.contains(&required.to_uppercase()),
            "projects table must include `{required}`; got:\n{block}"
        );
    }
}

#[test]
fn projects_migration_has_updated_at_trigger() {
    let sql = load_migration("0001_create_projects.sql");
    assert!(sql.contains("CREATE TRIGGER projects_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON projects"));
}

#[test]
fn projects_migration_makes_code_unique() {
    let block = create_table_block(&load_migration("0001_create_projects.sql"));
    assert!(
        block.contains("UNIQUE (code)") || block.contains("UNIQUE(\"code\")"),
        "expected UNIQUE on code; got:\n{block}"
    );
}

#[test]
fn projects_migration_no_longer_has_product_id() {
    let sql = load_migration("0001_create_projects.sql");
    assert!(
        !sql.contains("product_id"),
        "projects table must not reference product_id; got:\n{sql}"
    );
}

#[test]
fn projects_migration_has_tags_array_check() {
    let sql = load_migration("0001_create_projects.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("JSONB_TYPEOF(TAGS) = 'ARRAY'"),
        "projects table must enforce jsonb_typeof(tags) = 'array'; got:\n{sql}"
    );
}

#[test]
fn project_members_migration_has_composite_pk_and_checks() {
    let sql = load_migration("0001_create_projects.sql");
    let upper = sql.to_uppercase();
    let start = upper
        .find("CREATE TABLE PROJECT_MEMBERS")
        .expect("project_members");
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
    let sql = load_migration("0001_create_projects.sql");
    assert!(
        sql.contains("REFERENCES projects(id) ON DELETE CASCADE"),
        "project_members FK must cascade on delete"
    );
}

#[cfg(test)]
mod row_tests {
    use chrono::{TimeZone, Utc};

    use super::super::row::{ProjectMemberRow, ProjectRow};
    use crate::domain::{ProjectMember, ProjectTag, RoleType, TeamType};

    fn ts() -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
    }

    #[test]
    fn project_row_converts_to_project_with_empty_members_and_tags() {
        let row = ProjectRow {
            id: 1,
            code: "proj1".into(),
            description: "".into(),
            active: true,
            tags: sqlx::types::Json(vec![]),
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Project = row.try_into().expect("convert");
        assert_eq!(p.id, 1);
        assert_eq!(p.members, ProjectMember::default());
        assert_eq!(p.unblind_members, ProjectMember::default());
        assert!(p.tags.is_empty());
    }

    #[test]
    fn project_row_converts_to_project_with_tags() {
        let row = ProjectRow {
            id: 1,
            code: "proj1".into(),
            description: "".into(),
            active: true,
            tags: sqlx::types::Json(vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Region".into(), "EU".into()),
            ]),
            created_at: ts(),
            updated_at: ts(),
        };
        let p: crate::domain::Project = row.try_into().expect("convert");
        assert_eq!(p.tags.len(), 2);
        assert_eq!(p.tags[0].key, "Product");
        assert_eq!(p.tags[1].value, "EU");
    }

    #[test]
    fn project_member_row_carries_team_and_role_strings() {
        let row = ProjectMemberRow {
            project_id: 1,
            team_type: "members".into(),
            role_type: "leader".into(),
            user_code: "u1".into(),
        };
        assert_eq!(
            TeamType::try_from(row.team_type.as_str()).unwrap(),
            TeamType::Members
        );
        assert_eq!(
            RoleType::try_from(row.role_type.as_str()).unwrap(),
            RoleType::Leader
        );
    }
}
```

### 6.2 Update `usecase/tests.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/tests.rs`:

```rust
//! Tests for the usecase layer.
//!
//! Mock repository + a mock `UserService` stand in for the real
//! adapters so the orchestration + view projection can be exercised
//! without infrastructure.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    UserService, UserSummary,
};
use crate::usecase::commands::{CreateProject, UpdateProject};
use crate::usecase::error::UsecaseError;
use crate::usecase::project_usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- mock project repo ----------

#[derive(Default)]
struct MockProjectState {
    projects: HashMap<i32, Project>,
    next_id: i32,
}

#[derive(Clone, Default)]
struct MockProjectRepo {
    state: Arc<Mutex<MockProjectState>>,
}

impl MockProjectRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(MockProjectState {
                projects: HashMap::new(),
                next_id: 1,
            })),
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
        let tags = input.tags.unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            members,
            unblind_members,
            tags,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
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
        Ok(self
            .state
            .lock()
            .unwrap()
            .projects
            .values()
            .cloned()
            .collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref code) = input.code {
            let dup = s
                .projects
                .values()
                .any(|other| other.code == *code && other.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref code) = input.code {
            p.code = code.clone();
        }
        if let Some(ref desc) = input.description {
            p.description = desc.clone();
        }
        if let Some(a) = input.active {
            p.active = a;
        }
        // Replace membership wholesale per team.
        if let Some(ref m) = input.members {
            p.members = m.clone();
        }
        if let Some(ref m) = input.unblind_members {
            p.unblind_members = m.clone();
        }
        if let Some(ref tags) = input.tags {
            p.tags = tags.clone();
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

fn make_usecase() -> (
    MockProjectRepo,
    MockUserService,
    ProjectUsecase<MockProjectRepo, MockUserService>,
) {
    let projects = MockProjectRepo::new();
    let users = MockUserService::with_users(vec![
        UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        },
        UserSummary {
            code: "u2".into(),
            name: "Bob".into(),
        },
        UserSummary {
            code: "u3".into(),
            name: "Carol".into(),
        },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        project_repo: projects.clone(),
        users: users.clone(),
    });
    (projects, users, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_project_without_membership_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .expect("create");
    assert_eq!(view.code, "proj1");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
    assert!(view.tags.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_membership() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(ProjectMember::default()),
            tags: None,
        })
        .await
        .expect("create");
    assert_eq!(view.members.leaders.len(), 1);
    assert_eq!(view.members.leaders[0].code, "u1");
    assert_eq!(view.members.workers[0].code, "u2");
}

#[tokio::test]
async fn create_project_with_unknown_member_returns_user_not_found() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
        })
        .await
        .expect_err("unknown member rejected");
    assert!(
        matches!(err, UsecaseError::Repository(DomainError::UserNotFound(ref c)) if c == "ghost"),
        "got {err:?}"
    );
}

#[tokio::test]
async fn create_project_with_tags_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Region".into(), "EU".into()),
            ]),
        })
        .await
        .expect("create");
    assert_eq!(view.tags.len(), 2);
    assert_eq!(view.tags[0].key, "Product");
    assert_eq!(view.tags[0].value, "DEMO-001");
    assert_eq!(view.tags[1].key, "Region");
    assert_eq!(view.tags[1].value, "EU");
}

#[tokio::test]
async fn create_project_with_duplicate_tag_keys_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Product".into(), "DEMO-002".into()),
            ]),
        })
        .await
        .expect("create");
    assert_eq!(view.tags.len(), 2);
    assert_eq!(view.tags[0].value, "DEMO-001");
    assert_eq!(view.tags[1].value, "DEMO-002");
}

#[tokio::test]
async fn create_project_with_empty_tag_key_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("".into(), "v".into())]),
        })
        .await
        .expect_err("empty key rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagKey)
    ));
}

#[tokio::test]
async fn create_project_with_empty_tag_value_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k".into(), "   ".into())]),
        })
        .await
        .expect_err("empty value rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyTagValue)
    ));
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: Some(ProjectMember {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
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
async fn update_project_replaces_tags_whole_list() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k1".into(), "v1".into())]),
        })
        .await
        .expect("create");
    assert_eq!(created.tags.len(), 1);

    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            tags: Some(vec![
                ProjectTag::for_repository("k2".into(), "v2".into()),
                ProjectTag::for_repository("k3".into(), "v3".into()),
            ]),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.tags.len(), 2);
    assert_eq!(updated.tags[0].key, "k2");
    assert_eq!(updated.tags[1].key, "k3");
}

#[tokio::test]
async fn update_project_leaves_tags_unchanged_when_none() {
    let (_projects, _users, usecase) = make_usecase();
    let created = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![ProjectTag::for_repository("k1".into(), "v1".into())]),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_project(UpdateProject {
            id: created.id,
            description: Some("new".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.tags.len(), 1);
    assert_eq!(updated.tags[0].key, "k1");
}

#[tokio::test]
async fn list_projects_returns_all_views() {
    let (_projects, _users, usecase) = make_usecase();
    let _ = usecase
        .create_project(CreateProject {
            code: "p1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .unwrap();
    let _ = usecase
        .create_project(CreateProject {
            code: "p2".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .unwrap();
    let list = usecase.list_projects().await.expect("list");
    assert_eq!(list.len(), 2);
}

#[tokio::test]
async fn project_usecase_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectUsecase<MockProjectRepo, MockUserService>>();
}
```

### 6.3 Update `facade/in_memory/tests.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/facade/in_memory/tests.rs`:

```rust
//! End-to-end tests for the apis `ProjectService` facade, exercised
//! against in-memory repository + user-service fakes.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};

use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectService, TagData, UpdateProjectRequest,
};

use crate::adapter::facade::in_memory::ProjectServiceImpl;
use crate::domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType, UserService, UserSummary,
};
use crate::usecase::{ProjectUsecase, ProjectUsecaseConfig};

fn mock_now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 9, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct InMemProjectState {
    projects: HashMap<i32, Project>,
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
                projects: HashMap::new(),
                next_id: AtomicI32::new(1),
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
        let tags = input.tags.clone().unwrap_or_default();
        let project = Project::for_repository(
            id,
            input.code,
            input.description,
            members,
            unblind,
            tags,
            true,
            now,
            now,
        );
        s.projects.insert(id, project.clone());
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
        Ok(self
            .state
            .lock()
            .unwrap()
            .projects
            .values()
            .cloned()
            .collect())
    }
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError> {
        let mut s = self.state.lock().unwrap();
        if let Some(ref c) = input.code {
            let dup = s
                .projects
                .values()
                .any(|o| o.code == *c && o.id != input.id);
            if dup {
                return Err(DomainError::DuplicateCode(
                    "(constraint projects_code_unique)".into(),
                ));
            }
        }
        let p = s.projects.get_mut(&input.id).ok_or(DomainError::NotFound)?;
        if let Some(ref c) = input.code {
            p.code = c.clone();
        }
        if let Some(ref d) = input.description {
            p.description = d.clone();
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
        if let Some(ref t) = input.tags {
            p.tags = t.clone();
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

fn make_service() -> ProjectServiceImpl<InMemProjectRepo, InMemUserService> {
    let projects = InMemProjectRepo::new();
    let users = InMemUserService::with_users(vec![
        UserSummary {
            code: "u1".into(),
            name: "Alice".into(),
        },
        UserSummary {
            code: "u2".into(),
            name: "Bob".into(),
        },
        UserSummary {
            code: "u3".into(),
            name: "Carol".into(),
        },
    ]);
    let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
        project_repo: projects,
        users,
    });
    ProjectServiceImpl::new(usecase)
}

fn tag(key: &str, value: &str) -> TagData {
    TagData {
        key: key.into(),
        value: value.into(),
    }
}

#[tokio::test]
async fn create_project_with_none_membership_returns_empty_views() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.members.workers.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
    assert!(view.unblind_members.workers.is_empty());
    assert!(view.tags.is_empty());
}

#[tokio::test]
async fn create_project_with_some_empty_membership_equivalent_to_none() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(Default::default()),
            unblind_members: Some(Default::default()),
            tags: None,
        })
        .await
        .expect("create");
    assert!(view.members.leaders.is_empty());
    assert!(view.unblind_members.leaders.is_empty());
}

#[tokio::test]
async fn create_project_hydrates_full_membership() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec!["u2".into()],
            }),
            unblind_members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u3".into()],
                workers: vec![],
            }),
            tags: None,
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
    let err = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["ghost".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
        })
        .await
        .expect_err("unknown member");
    assert!(matches!(err, ProjectApiError::UserNotFound(ref c) if c == "ghost"));
}

#[tokio::test]
async fn create_project_with_tags_round_trips_through_ap_view() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![
                tag("Product", "DEMO-001"),
                tag("Region", "EU"),
            ]),
        })
        .await
        .expect("create");
    assert_eq!(view.tags.len(), 2);
    assert_eq!(view.tags[0].key, "Product");
    assert_eq!(view.tags[0].value, "DEMO-001");
    assert_eq!(view.tags[1].key, "Region");
}

#[tokio::test]
async fn update_project_replaces_membership_whole_list() {
    let service = make_service();
    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: Some(apis::project::ProjectMemberData {
                leaders: vec!["u1".into()],
                workers: vec![],
            }),
            unblind_members: None,
            tags: None,
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
async fn update_project_replaces_tags_whole_list() {
    let service = make_service();
    let created = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            tags: Some(vec![tag("k1", "v1")]),
        })
        .await
        .expect("create");
    let updated = service
        .update_project(UpdateProjectRequest {
            id: created.id,
            tags: Some(vec![tag("k2", "v2"), tag("k3", "v3")]),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.tags.len(), 2);
    assert_eq!(updated.tags[0].key, "k2");
}

#[tokio::test]
async fn project_service_is_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<ProjectServiceImpl<InMemProjectRepo, InMemUserService>>();
}

#[tokio::test]
async fn project_service_box_dyn_compiles() {
    let service = make_service();
    let _boxed: Box<dyn ProjectService> = Box::new(service);
}

// silence unused import warnings for enums the tests exercise via
// the in-memory fakes
#[allow(dead_code)]
fn _force_use_team_role() -> (TeamType, RoleType) {
    (TeamType::Members, RoleType::Leader)
}

#[allow(dead_code)]
fn _force_use_project_member() -> ProjectMember {
    ProjectMember::default()
}

#[allow(dead_code)]
fn _force_use_project_tag() -> ProjectTag {
    ProjectTag::for_repository("k".into(), "v".into())
}
```

### 6.4 Update `tests/public_api.rs`

- [ ] **Step 1:** Replace `lib/crates/project/tests/public_api.rs` so that:
  - Every reference to `Product` / `ProductRepo` / `ProductView` / `CreateProduct` / `UpdateProduct` is dropped.
  - Imports include `apis::project::TagData`, `apis::project::TagView`, `project::ProjectTag`, `project::ProjectView`, `project::TagView`, `domain::DomainError`.
  - `ProjectUsecaseConfig` test pins the new two-generic shape: `fn(cfg: ProjectUsecaseConfig<ProjectRepo, UserServiceImpl>) = |cfg| { let _: &ProjectRepo = &cfg.project_repo; let _: &UserServiceImpl = &cfg.users; };`
  - `ProjectServiceImpl` test pins the new two-generic shape: `fn(ProjectServiceImpl<ProjectRepo, UserServiceImpl>) -> Box<dyn ProjectService>`.
  - `public_types_are_nameable_from_crate_root` adds `assert_tag(_: ProjectTag)` and a `TagView` smoke check.
  - `repo_constructors_accept_a_pg_pool` drops the `ProductRepo` line, keeps `ProjectRepo`.
  - `domain_error_variants_are_nameable` swaps `EmptyName` / `ProductNotFound` for `EmptyTagKey` / `EmptyTagValue`.
  - `usecase_commands_have_expected_field_shape` and `api_requests_have_expected_field_shape` lose `product_id` from project commands / requests and gain `tags: None`.
  - `apis_view_dtos_are_nameable` builds a `TagView` smoke value.
  - `apis_error_variants_are_nameable` swaps `ProductNotFound` for nothing (it no longer exists).

(Provide the full file replacement — its size is comparable to the original; if you find yourself shortening, do not — every existing test must remain and just be updated.)

### 6.5 Update `tests/integration_persistence.rs`

- [ ] **Step 1:** Replace `lib/crates/project/tests/integration_persistence.rs`:

```rust
//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p project -- --ignored`.
//! Reads the workspace-shared `AEGIS_DATABASE_URL` (same convention
//! as the `auth` and `user` crates); loads `.env` at the workspace
//! root via `dotenvy` so the variable only needs to live in `.env`.
//! Drops the live tables + `_sqlx_migrations` before each run so
//! the migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use project::domain::{ProjectMember, ProjectNew, ProjectTag, ProjectUpdate};
use project::{ProjectRepo, ProjectRepository};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_DATABASE_URL must be set (or present in .env at the workspace root) \
             to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_DATABASE_URL");

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
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_no_membership_or_tags_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-shell"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: None,
            })
            .await
            .expect("create project");
        assert!(created.members.leaders.is_empty());
        assert!(created.members.workers.is_empty());
        assert!(created.unblind_members.leaders.is_empty());
        assert!(created.unblind_members.workers.is_empty());
        assert!(created.tags.is_empty());
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_membership_then_update_replaces_it() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-mem"),
                description: "".into(),
                members: Some(ProjectMember {
                    leaders: vec!["u1".into()],
                    workers: vec!["u2".into()],
                }),
                unblind_members: Some(ProjectMember::default()),
                tags: None,
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
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT team_type FROM project_members WHERE project_id = $1")
                .bind(created.id)
                .fetch_all(&pool)
                .await
                .expect("query members");
        assert!(rows.iter().all(|(t,)| t != "unblind_members"));
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_create_with_tags_round_trip() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: Some(vec![
                    ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                    ProjectTag::for_repository("Region".into(), "EU".into()),
                ]),
            })
            .await
            .expect("create project");
        assert_eq!(created.tags.len(), 2);
        assert_eq!(created.tags[0].key, "Product");
        assert_eq!(created.tags[1].value, "EU");
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_DATABASE_URL pointing at a live PostgreSQL"]
async fn project_update_replaces_tags_whole_list() {
    with_pool(|pool| async move {
        let projects = ProjectRepo::new(pool.clone());
        let created = projects
            .create(ProjectNew {
                code: unique_code("proj-tags-replace"),
                description: "".into(),
                members: None,
                unblind_members: None,
                tags: Some(vec![
                    ProjectTag::for_repository("k1".into(), "v1".into()),
                ]),
            })
            .await
            .expect("create project");
        let updated = projects
            .update(ProjectUpdate {
                id: created.id,
                tags: Some(vec![
                    ProjectTag::for_repository("k2".into(), "v2".into()),
                    ProjectTag::for_repository("k3".into(), "v3".into()),
                ]),
                ..Default::default()
            })
            .await
            .expect("update");
        assert_eq!(updated.tags.len(), 2);
        assert_eq!(updated.tags[0].key, "k2");
        assert_eq!(updated.tags[1].key, "k3");
        // Spot-check via direct JSONB query.
        let raw: sqlx::types::Json<Vec<ProjectTag>> = sqlx::query_as(
            "SELECT tags FROM projects WHERE id = $1",
        )
        .bind(created.id)
        .fetch_one(&pool)
        .await
        .expect("query tags");
        assert_eq!(raw.0.len(), 2);
    })
    .await;
}
```

### 6.6 Verify + commit

- [ ] **Step 1:** Run the project crate tests.

Run: `cargo test -p project 2>&1 | tail -30`
Expected: green for everything except integration tests (those are `#[ignore]`-gated).

- [ ] **Step 2:** Run ignored integration tests if a DB is available.

Run: `cargo test -p project -- --ignored --test-threads=1`
Expected: green (when `AEGIS_DATABASE_URL` points at a live DB).

- [ ] **Step 3:** Commit.

```bash
git add lib/crates/project/src/adapter/persistence/postgres/tests.rs \
        lib/crates/project/src/usecase/tests.rs \
        lib/crates/project/src/adapter/facade/in_memory/tests.rs \
        lib/crates/project/tests/public_api.rs \
        lib/crates/project/tests/integration_persistence.rs
git commit -m "test(project): rewrite unit + facade + public_api + integration for tags

Domain / adapter / usecase / facade tests lose every *_product_*
case. New tag tests cover:
- ProjectTag::new rejects empty key + value
- create_project_with_tags_succeeds
- create_project_with_duplicate_tag_keys_succeeds
- create_project_with_empty_tag_key / value returns validation error
- update_project_replaces_tags_whole_list
- update_project_leaves_tags_unchanged_when_none

Migration tests drop products_migration_* in favour of:
- projects_migration_has_required_columns (asserts tags JSONB)
- projects_migration_no_longer_has_product_id
- projects_migration_has_tags_array_check
- project_row_converts_to_project_with_tags

Integration tests drop product_* round-trips, add
project_create_with_tags_round_trip and
project_update_replaces_tags_whole_list.

Spec coverage: Tests section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification:
  cargo test -p project
  cargo test -p project -- --ignored --test-threads=1  # with DB"
```

---

## Task 7: Cross-crate ripple — server

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `apps/server/aegis-server/src/run.rs`
- Modify: `apps/server/aegis-server/tests/integration_auth.rs`

### 7.1 Update `dto.rs`

- [ ] **Step 1:** Drop the entire "product requests / responses" block (`CreateProductRequest`, `UpdateProductRequest`, `ProductViewResponse`, `ProductListResponse`, the `From<apis::project::ProductView> for ProductViewResponse` impl, and the `sample_product_view` helper).
- [ ] **Step 2:** Drop `product: ProductViewResponse` from `ProjectViewResponse` and add `pub tags: Vec<TagViewResponse>`.
- [ ] **Step 3:** Drop `product_id` from `CreateProjectRequest` and `UpdateProjectRequest`. Add `tags: Option<Vec<TagDataRequest>>` to both, using the same `#[serde(default, skip_serializing_if = "Option::is_none")]` pattern as `members`.
- [ ] **Step 4:** Add the wire-level DTOs:

```rust
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagDataRequest {
    pub key: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TagViewResponse {
    pub key: String,
    pub value: String,
}

impl From<apis::project::TagData> for TagDataRequest {
    fn from(t: apis::project::TagData) -> Self {
        Self { key: t.key, value: t.value }
    }
}

impl From<TagDataRequest> for apis::project::TagData {
    fn from(t: TagDataRequest) -> Self {
        Self { key: t.key, value: t.value }
    }
}

impl From<apis::project::TagView> for TagViewResponse {
    fn from(t: apis::project::TagView) -> Self {
        Self { key: t.key, value: t.value }
    }
}
```

- [ ] **Step 5:** Update the `From<apis::project::ProjectView> for ProjectViewResponse` impl to populate `tags: view.tags.into_iter().map(Into::into).collect()` and drop the `product: view.product.into()` line.
- [ ] **Step 6:** Update `sample_project_view` helper to drop `product: sample_product_view(1, "p1")` and add `tags: vec![]`.
- [ ] **Step 7:** Update the test fixtures + round-trip tests to reflect the new shape. Specifically:
  - `update_project_request_omitted_membership_keeps_none`, `update_project_request_empty_membership_becomes_some_empty`, `update_project_request_partial_with_membership_roundtrip`, `create_project_request_minimal_roundtrip`, `create_project_request_with_empty_members_roundtrip`, `project_view_response_from_apis_view`, `sample_project_view` — all lose `product_id` / `product` and gain empty `tags: None` / `tags: vec![]` where applicable.
  - Add `tag_data_request_roundtrip`, `tag_view_response_from_apis_view`, `create_project_request_with_tags_roundtrip` (round-trips `{"tags":[{"key":"Product","value":"DEMO-001"}]}`).

### 7.2 Update `project/handlers.rs`

- [ ] **Step 1:** Drop the entire "products" block (`create_product`, `list_products`, `get_product_by_code`, `update_product`).
- [ ] **Step 2:** Drop the `require_admin_or_root` helper's role comment about "admin or root required" — keep the helper, drop the "for products" framing.
- [ ] **Step 3:** Update `create_project` to drop `product_id: req.product_id` and add `tags: req.tags.map(|ts| ts.into_iter().map(Into::into).collect())`.
- [ ] **Step 4:** Update `update_project` similarly.
- [ ] **Step 5:** Update `MockProjectService` to drop `create_product`, `get_product_by_code` (product), `list_products`, `update_product` fields, `last_create_product_args`, `last_update_product_args`, `*_product_err` fields, and the five `*_product` impl arms. Update the `last_create_product_args` / `last_update_product_args` references in tests to drop the product tests and add tag tests.
- [ ] **Step 6:** Update `sample_project_view` to drop the `product` field and add `tags: vec![]`.
- [ ] **Step 7:** Update the test cases that sent `product_id` in their JSON request bodies. Remove `"product_id":1` from each `build_request(..., Some(r#"{"code":"pr1","description":"x","product_id":1,...}"#), ...)`. Add tests for tags: `create_project_with_tags_round_trips`, `update_project_replaces_tags_whole_list`, `create_project_with_empty_tag_key_maps_to_400`. Drop `create_project_product_not_found_maps_to_404` (the variant doesn't exist any more).
- [ ] **Step 8:** Update the `app()` Router to drop the `/api/product` routes.

### 7.3 Update `project/router.rs`

- [ ] **Step 1:** Drop the product router / routes. The `routers()` function returns only the project router (or its components are split). Replace the function so it returns only the project side.

### 7.4 Update `openapi.rs`

- [ ] **Step 1:** Drop the `(name = "product", …)` tag and every product path annotation. Add `tags: Vec<TagViewResponse>` to the `ProjectViewResponse` utoipa schema (or reference) and add `TagViewResponse` / `TagDataRequest` to the schema list.

### 7.5 Update `router.rs`

- [ ] **Step 1:** Drop `.nest("/product", product_routes)` (Task 7.3 makes `project_router::routers()` single-valued; the binding destructure goes from `let (product_routes, project_routes) = project_router::routers();` to `let project_routes = project_router::routers();`).
- [ ] **Step 2:** In the `tests` module, drop `MockProjectService`'s product mock arms (mirror 7.2), drop `sample_product_view`, drop the `/api/product` registration in the test router, drop product test cases, and drop the product path entries in the OpenAPI path assertion.

### 7.6 Update `error.rs`

- [ ] **Step 1:** Drop the `ProjectApiError::ProductNotFound` mapping (it doesn't exist any more; the `match` arm for it goes away).

### 7.7 Update `run.rs`

- [ ] **Step 1:** Drop any `ProductRepo::new(pool.clone())` wiring (verify it's not there; if absent, no change).

### 7.8 Update `tests/integration_auth.rs`

- [ ] **Step 1:** Run `grep -n "product" apps/server/aegis-server/tests/integration_auth.rs` and confirm there are no Product-port references. If there are, drop them.

### 7.9 Verify + commit

- [ ] **Step 1:** Build the server.

Run: `cargo check -p aegis-server 2>&1 | tail -40`
Expected: green.

- [ ] **Step 2:** Run server tests.

Run: `cargo test -p aegis-server 2>&1 | tail -40`
Expected: green for non-ignored.

- [ ] **Step 3:** Commit.

```bash
git add apps/server/aegis-server/src \
        apps/server/aegis-server/tests/integration_auth.rs
git commit -m "refactor(server): drop product HTTP layer, thread tags through project

The /api/product namespace is removed entirely (handlers, dto,
router, openapi paths, error mapping). Project handlers gain
TagDataRequest on the request side and project responses gain
tags: Vec<TagViewResponse>. MockProjectService loses its five
*_product_* fields + impl arms.

Spec coverage: Cross-crate ripple (server) section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification:
  cargo check -p aegis-server
  cargo test -p aegis-server"
```

---

## Task 8: Cross-crate ripple — desktop

**Files:**
- Delete: `apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs`
- Delete: `apps/desktop/aegis-desktop/src-tauri/src/http/product.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/commands/mod.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/mod.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`

### 8.1 Delete the product files

- [ ] **Step 1:**

```bash
git rm apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs \
       apps/desktop/aegis-desktop/src-tauri/src/http/product.rs
```

### 8.2 Unregister the product commands

- [ ] **Step 1:** Edit `apps/desktop/aegis-desktop/src-tauri/src/commands/mod.rs`. Remove any `pub mod product;` line and any `pub use product::{create_product, …};` re-exports.
- [ ] **Step 2:** Edit `apps/desktop/aegis-desktop/src-tauri/src/lib.rs`. Find the `tauri::generate_handler!` (or equivalent) invocation and drop every product command: `create_product`, `list_products`, `get_product_by_code`, `update_product`. Also drop any `mod product;` / `use commands::product` lines.

### 8.3 Drop `product` from `http` module

- [ ] **Step 1:** Edit `apps/desktop/aegis-desktop/src-tauri/src/http/mod.rs`. Remove `pub mod product;` and any related re-exports.

### 8.4 Update `http/project.rs`

- [ ] **Step 1:** Drop `use super::product::ProductViewResponse;` and the `product: ProductViewResponse` field from `ProjectViewResponse`.
- [ ] **Step 2:** Add the wire-shaped tag DTOs:

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagDataRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TagViewResponse {
    pub key: String,
    pub value: String,
}
```

- [ ] **Step 3:** Add `tags: Vec<TagViewResponse>` to `ProjectViewResponse`.
- [ ] **Step 4:** Drop `product_id: i32` from `CreateProjectRequest` and add `#[serde(default, skip_serializing_if = "Option::is_none")] pub tags: Option<Vec<TagDataRequest>>`.
- [ ] **Step 5:** Drop `product_id: Option<i32>` from `UpdateProjectRequest` and add `#[serde(default, skip_serializing_if = "Option::is_none")] pub tags: Option<Vec<TagDataRequest>>`.
- [ ] **Step 6:** Update the test cases:
  - Drop `assert_eq!(projects[0].product.code, "x");` (or any product assertions) in `list_returns_projects`.
  - Update the mock JSON body to drop the `"product": {...}` block.
  - Update `update_skips_none_fields` if it depended on `product_id`.
  - Add `tag_data_request_roundtrip` and `create_project_request_with_tags_roundtrip` (with `{"tags":[{"key":"Product","value":"DEMO-001"}]}`).

### 8.5 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p aegis-desktop 2>&1 | tail -40`
Expected: green.

- [ ] **Step 2:** Run desktop tests.

Run: `cargo test -p aegis-desktop 2>&1 | tail -40`
Expected: green for non-ignored.

- [ ] **Step 3:** Commit.

```bash
git add apps/desktop/aegis-desktop/src-tauri/src
git commit -m "refactor(desktop): drop product HTTP/command layer, add tags to project

apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs and
http/product.rs are deleted; the http + commands modules drop their
product re-exports. lib.rs drops the create_product /
list_products / get_product_by_code / update_product handler
registrations.

http/project.rs drops the product: ProductViewResponse field from
the response DTO, drops product_id from the request DTOs, and
adds tags: Vec<TagViewResponse> on the response side and
tags: Option<Vec<TagDataRequest>> on the request side.

Spec coverage: Cross-crate ripple (desktop) section of
docs/superpowers/specs/2026-08-17-project-tag-design.md.

Verification:
  cargo check -p aegis-desktop
  cargo test -p aegis-desktop"
```

---

## Task 9: README + final verification

**Files:**
- Modify: `lib/crates/project/README.md`

### 9.1 Update the project crate README

- [ ] **Step 1:** Edit `lib/crates/project/README.md`:
  - Drop every reference to `Product`, `ProductNew`, `ProductUpdate`, `ProductRepository`, `ProductRepo`, `CreateProduct`, `UpdateProduct`, `ProductView`, `ProductViewResponse`, `UsecaseConfig::product_repo`.
  - Update the "Domain model" section so `Project` includes `tags: Vec<ProjectTag>` and loses `product_id`.
  - Update the "Database setup" section to drop the `products` reference (now only `projects` + `project_members`).
  - Update the constructor snippet to drop `ProductRepo` wiring.

### 9.2 Final verification

- [ ] **Step 1:** Format check.

Run: `cargo fmt --all -- --check`
Expected: green (or run `cargo fmt --all` to fix and re-commit).

- [ ] **Step 2:** Clippy across the workspace.

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | tail -30`
Expected: green.

- [ ] **Step 3:** Build everything.

Run: `cargo check --workspace`
Expected: green.

- [ ] **Step 4:** Run the project crate tests.

Run: `cargo test -p project 2>&1 | tail -30`
Expected: green.

- [ ] **Step 5:** Run integration tests if a DB is available.

Run: `cargo test -p project -- --ignored --test-threads=1`
Expected: green (with `AEGIS_DATABASE_URL`).

- [ ] **Step 6:** Run server + desktop test suites.

Run: `cargo test -p aegis-server 2>&1 | tail -20 && cargo test -p aegis-desktop 2>&1 | tail -20`
Expected: green.

- [ ] **Step 7:** Commit the README + any fmt-only fixes.

```bash
git add lib/crates/project/README.md
git commit -m "docs(project): update README for tag-aware Project

Domain model section reflects tags: Vec<ProjectTag>. Database
setup mentions only projects + project_members (no products).
Constructor snippet drops ProductRepo.

Spec coverage: closing pass over the spec's README call-out.

Verification:
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo check --workspace
  cargo test -p project
  cargo test -p aegis-server
  cargo test -p aegis-desktop"
```

---

## Self-Review

**1. Spec coverage:** every requirement from [docs/superpowers/specs/2026-08-17-project-tag-design.md](docs/superpowers/specs/2026-08-17-project-tag-design.md) maps to a task:
- Domain types / Data Model → Task 2.5, Task 3.1, Task 3.2
- Apis types → Task 1.3
- Schema → Task 4.1
- Persistence → Task 4.4, Task 4.5
- Usecase → Task 3.3, Task 3.4
- Facade → Task 5.1
- Public API surface → Task 2.7
- Cross-crate ripple (server) → Task 7
- Cross-crate ripple (desktop) → Task 8
- Tests (every tier) → Task 6
- README + verification gate → Task 9

**2. Placeholder scan:** no "TBD" / "TODO" / "implement later" / "fill in details" in any task body. Tasks 6.4 and 7.2 are unusually large because they preserve every existing test case; the work is enumerated as concrete file-edit operations and the field-level changes are spelled out.

**3. Type consistency:** signatures cross-check:
- `ProjectServiceImpl<R, U>` (Task 5.1) matches `ProjectUsecase<R, U>` (Task 3.4) matches `ProjectUsecaseConfig<R, U>` (Task 3.4).
- `Project::for_repository` / `Project::new` (Task 2.5) parameter order matches `ProjectRow` (Task 4.4) and `TryFrom<ProjectRow> for Project` (Task 4.4).
- `CreateProject.tags: Option<Vec<ProjectTag>>` (Task 3.1) ↔ `CreateProjectRequest.tags: Option<Vec<TagData>>` (Task 1.3) ↔ `ProjectNew.tags: Option<Vec<ProjectTag>>` (Task 2.5) ↔ `wire CreateProjectRequest.tags: Option<Vec<TagDataRequest>>` (Task 7.1) ↔ `desktop CreateProjectRequest.tags: Option<Vec<TagDataRequest>>` (Task 8.4).
- `ProjectView.tags: Vec<TagView>` ↔ domain `ProjectView.tags: Vec<TagView>` ↔ apis `ProjectView.tags: Vec<TagView>` ↔ server `ProjectViewResponse.tags: Vec<TagViewResponse>` ↔ desktop `ProjectViewResponse.tags: Vec<TagViewResponse>`.

**4. Scope check:** everything fits one crate-level change + cross-crate ripple. No decomposition needed.

**5. Ambiguity fix:** Task 4.5 calls the tag-update branch with a `QueryBuilder` + `Json` bind so the JSONB payload is written inside the same transaction as metadata + membership; no implicit assumptions about driver encoding.