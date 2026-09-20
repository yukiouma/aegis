# Project Configuration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `ProjectConfiguration { language, tags }` value object to the `project` crate so a project carries an optional locale (English / Simplified Chinese) alongside the existing tag list, replacing the current top-level `tags` field on `Project`.

**Architecture:** New domain types `ProjectLanguage` (enum) and `ProjectConfiguration` (composite `{ language: Option<ProjectLanguage>, tags: Vec<ProjectTag> }`) — `Project.tags` is renamed to `Project.configurations: ProjectConfiguration`. The wire shape (`apis::project::ProjectView`, the server DTOs) is updated to mirror the new field structure so the configuration round-trips through the JSONB column and the HTTP layer in one move. The `apps/desktop/aegis-desktop/src-tauri` crate is excluded per the user; the desktop frontend's wire types come from `src-tauri` and stay unchanged.

**Tech Stack:** Rust 2024, sqlx 0.9 (with `json` feature), serde / serde_json, thiserror, async-trait, PostgreSQL.

## Global Constraints

- **Constraint: scope.** Files inside `apps/desktop/aegis-desktop/src-tauri/` are out of scope per the user. The desktop frontend reads its wire types from `src-tauri`, so leaving that crate alone means the frontend does not need to change in this task.
- **Constraint: language variants.** `ProjectLanguage` is an enum with exactly two variants — `English` (wire code `"en"`) and `SimplifiedChinese` (wire code `"zh-CN"`). The `Option<ProjectLanguage>` field on `ProjectConfiguration` means a project may have *no* language configured.
- **Constraint: tag validation.** Tags inside `ProjectConfiguration.tags` keep the existing contract: `key` and `value` both non-empty after trim, duplicate keys allowed. `ProjectConfiguration` itself owns no domain rule beyond composition — the inner `ProjectTag` already enforces non-empty fields, so no separate `ProjectConfiguration::new` validator is added.
- **Constraint: configuration mutation.** `None` on `create_project.configuration` / `update_project.configuration` means "default empty configuration" / "leave unchanged" respectively; `Some(config)` is whole-configuration replace. This mirrors the prior `tags` semantics.
- **Constraint: data migration.** The existing `projects.tags` JSONB column is migrated into the new `projects.configuration` JSONB column in a new `0002_add_project_configuration.sql` so live data is preserved. `tags` is dropped; the CHECK constraint is replaced with `jsonb_typeof(configuration) = 'object'`.
- **Constraint: error channel.** `DomainError::UnknownLanguage(String)` is the only new variant. No new `ProjectApiError` variants; unknown wire codes surface through the existing `Validation` channel.
- **Constraint: re-export surface.** `ProjectLanguage` and `ProjectConfiguration` are re-exported from the project crate root and the `project::domain` module. The apis crate re-exports its wire mirrors under `apis::project`.
- **Verification gate** (every task that compiles must end green on the relevant subset):
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p project --all-targets --all-features -- -D warnings
  cargo test -p project
  cargo check --workspace
  ```

## File Structure

### Created (project crate)

- `lib/crates/project/src/domain/project_language.rs` — `ProjectLanguage` enum + `as_str` + `TryFrom<&str>`
- `lib/crates/project/src/domain/project_configuration.rs` — `ProjectConfiguration` struct
- `lib/crates/project/migrations/0002_add_project_configuration.sql` — adds `configuration` JSONB column, migrates `tags` data, drops `tags`, replaces CHECK

### Modified (project crate)

- `src/lib.rs` — re-export `ProjectConfiguration`, `ProjectLanguage`
- `src/domain.rs` — declare + re-export new modules
- `src/domain/error.rs` — add `UnknownLanguage(String)`
- `src/domain/project.rs` — replace `tags: Vec<ProjectTag>` field with `configurations: ProjectConfiguration` on `Project`; update `ProjectNew` and `ProjectUpdate`; update `new` / `for_repository` / `Debug`
- `src/domain/tests.rs` — add `ProjectLanguage` / `ProjectConfiguration` unit tests; update `project_new_*` calls to pass `ProjectConfiguration::default()` for the new field
- `src/usecase.rs` — re-export new view types
- `src/usecase/commands.rs` — `CreateProject.configuration` / `UpdateProject.configuration`
- `src/usecase/views.rs` — add `ProjectConfigurationView`, update `ProjectView`
- `src/usecase/project_usecase.rs` — pass `configuration` through; update `validate_create_project` / `validate_update_project` to validate the inner tags via `ProjectTag::new`; replace `tag_fields` view projection
- `src/usecase/tests.rs` — mock repo + usecase tests use the new field shape; tag tests updated to set `configuration: Some(...)` instead of `tags: Some(...)`
- `src/adapter/facade/in_memory/service.rs` — translate `configurations` on create/update + emit `ProjectConfigurationView` from the view `From` impl
- `src/adapter/facade/in_memory/tests.rs` — `InMemProjectRepo` stores `configuration`; tag tests use the new shape
- `src/adapter/persistence/postgres/row.rs` — `ProjectRow` carries `configuration: sqlx::types::Json<ProjectConfiguration>`; `TryFrom` populates the new field
- `src/adapter/persistence/postgres/project_repo.rs` — every SQL statement references `configuration` instead of `tags`; `create` / `update` use `Json(&input.configuration.unwrap_or_default())` and `Json(&configuration)` respectively
- `src/adapter/persistence/postgres/tests.rs` — schema assertions for the new column and CHECK; row tests construct `Json(ProjectConfiguration::default())` etc.
- `tests/public_api.rs` — add `ProjectConfiguration` / `ProjectLanguage` to the public-API surface; update command/view field references
- `tests/integration_persistence.rs` — `create_with_no_membership_or_configuration_round_trip` etc. use the new field
- `README.md` — domain model + module tree updated

### Modified (apis crate)

- `src/project.rs` — add `ProjectLanguage`, `ProjectConfigurationData`, `ProjectConfigurationView`; rename `tags` field on `ProjectView`, `CreateProjectRequest`, `UpdateProjectRequest` to `configurations`

### Modified (server)

- `src/transport/http/dto.rs` — replace `TagDataRequest` + `TagViewResponse` usage with `ProjectConfigurationDataRequest` + `ProjectConfigurationViewResponse`; rename `tags` field on the three project DTOs; thread new shape through `From` impl
- `src/transport/http/project/handlers.rs` — translate `configuration` on create/update; update `MockProjectService::sample_project_view`; update JSON-body round-trip tests

### Out of scope (per user)

- `apps/desktop/aegis-desktop/src-tauri/` — untouched. The desktop frontend's wire types are defined here; since this crate does not change, the frontend is unaffected.

### Unchanged

- `lib/crates/auth`, `lib/crates/user`, `lib/crates/windows-utils`, `lib/crates/terminology`, `lib/crates/domain-model`, `lib/crates/apis/src/{auth,user,crf,domain_model,mission,terminology}.rs`, `lib/packages/ui/**`, `apps/desktop/aegis-desktop/src/**` (frontend — wire types live in `src-tauri`).

---

## Task 1: Domain layer — `ProjectLanguage` + `ProjectConfiguration`

**Files:**
- Modify: `lib/crates/project/src/domain/error.rs`
- Create: `lib/crates/project/src/domain/project_language.rs`
- Create: `lib/crates/project/src/domain/project_configuration.rs`
- Modify: `lib/crates/project/src/domain.rs`
- Modify: `lib/crates/project/src/lib.rs`
- Modify: `lib/crates/project/src/domain/tests.rs`

### 1.1 Write the failing domain tests

- [ ] **Step 1:** Edit `lib/crates/project/src/domain/tests.rs`. Append at the bottom (after the `project_tag_*` tests):

```rust
#[test]
fn project_language_as_str_maps_to_wire_codes() {
    assert_eq!(ProjectLanguage::English.as_str(), "en");
    assert_eq!(ProjectLanguage::SimplifiedChinese.as_str(), "zh-CN");
}

#[test]
fn project_language_try_from_str_parses_known_values() {
    assert_eq!(
        ProjectLanguage::try_from("en").unwrap(),
        ProjectLanguage::English
    );
    assert_eq!(
        ProjectLanguage::try_from("zh-CN").unwrap(),
        ProjectLanguage::SimplifiedChinese
    );
}

#[test]
fn project_language_try_from_str_rejects_unknown_value() {
    let err = ProjectLanguage::try_from("ja").unwrap_err();
    assert!(matches!(err, DomainError::UnknownLanguage(ref s) if s == "ja"));
}

#[test]
fn project_configuration_default_has_no_language_and_no_tags() {
    let c = ProjectConfiguration::default();
    assert!(c.language.is_none());
    assert!(c.tags.is_empty());
}

#[test]
fn project_configuration_for_repository_carries_language_and_tags() {
    let c = ProjectConfiguration::for_repository(
        Some(ProjectLanguage::SimplifiedChinese),
        vec![ProjectTag::for_repository("k".into(), "v".into())],
    );
    assert_eq!(c.language, Some(ProjectLanguage::SimplifiedChinese));
    assert_eq!(c.tags.len(), 1);
    assert_eq!(c.tags[0].key, "k");
}
```

- [ ] **Step 2:** Run them; confirm compile failure.

Run: `cargo test -p project --lib domain::tests::project_language_as_str 2>&1 | tail -20`
Expected: compile error mentioning `ProjectLanguage` / `ProjectConfiguration` / `UnknownLanguage`.

### 1.2 Add `UnknownLanguage` to `DomainError`

- [ ] **Step 1:** Edit `lib/crates/project/src/domain/error.rs`. Add one line inside the enum body (after `UnknownRoleType`):

```rust
    #[error("unknown project language: {0}")]
    UnknownLanguage(String),
```

Final file:

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

    #[error("unknown project language: {0}")]
    UnknownLanguage(String),

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

### 1.3 Create `ProjectLanguage`

- [ ] **Step 1:** Create `lib/crates/project/src/domain/project_language.rs`:

```rust
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Locale hint attached to a project configuration. Currently just
/// English (`"en"`) and Simplified Chinese (`"zh-CN"`); new locales
/// land here as variants. Stored as an enum so the wire code is
/// one of two well-known strings; an `Option<ProjectLanguage>` is
/// the way to model "no language configured".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    English,
    SimplifiedChinese,
}

impl ProjectLanguage {
    /// Wire-level code. Stable; the consumer UI maps this to the
    /// `Locale` value it already knows.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::English => "en",
            ProjectLanguage::SimplifiedChinese => "zh-CN",
        }
    }
}

impl TryFrom<&str> for ProjectLanguage {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Err> {
        match value {
            "en" => Ok(ProjectLanguage::English),
            "zh-CN" => Ok(ProjectLanguage::SimplifiedChinese),
            other => Err(DomainError::UnknownLanguage(other.to_string())),
        }
    }
}
```

### 1.4 Create `ProjectConfiguration`

- [ ] **Step 1:** Create `lib/crates/project/src/domain/project_configuration.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;

/// Composite of the project's optional locale and its tag list.
///
/// The struct owns no domain rule beyond composition: the inner
/// `ProjectTag` already enforces non-empty key / value when tags
/// arrive via the validating constructor on the wire, so no
/// separate `ProjectConfiguration::new` is needed. Constructors
/// skip validation because the data is either trusted (adapter
/// materialising from JSONB) or already validated (usecase after
/// `ProjectTag::new`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectConfiguration {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<ProjectTag>,
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream
    /// test code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
    ) -> Self {
        Self { language, tags }
    }
}
```

### 1.5 Update `domain.rs` and `lib.rs` re-exports

- [ ] **Step 1:** Replace `lib/crates/project/src/domain.rs`:

```rust
mod error;
mod project;
mod project_configuration;
mod project_language;
mod project_member;
mod project_tag;
mod team_role;
#[cfg(test)]
mod tests;
mod user;

pub use error::DomainError;
pub use project::{Project, ProjectNew, ProjectRepository, ProjectUpdate};
pub use project_configuration::ProjectConfiguration;
pub use project_language::ProjectLanguage;
pub use project_member::ProjectMember;
pub use project_tag::ProjectTag;
pub use team_role::{RoleType, TeamType};
pub use user::{UserService, UserSummary};
```

- [ ] **Step 2:** Edit `lib/crates/project/src/lib.rs`. Replace the `pub use domain::{…}` block:

```rust
pub use domain::{
    DomainError, Project, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectNew,
    ProjectRepository, ProjectTag, ProjectUpdate, RoleType, TeamType, UserService, UserSummary,
};
```

Also update the crate-level doc-comment (the `//!` lines) so it mentions `ProjectConfiguration` instead of "JSONB tags":

```rust
//! Workspace library providing a SQLx/PostgreSQL-backed DDD repository
//! for the `Project` aggregate (with a `ProjectConfiguration { language,
//! tags }` JSONB payload) and an async `ProjectUsecase` that
//! orchestrates them and adapts to the `apis::project::ProjectService`
//! port.
```

### 1.6 Verify + commit

- [ ] **Step 1:** Run the domain tests.

Run: `cargo test -p project --lib domain::tests::project_language 2>&1 | tail -20`
Expected: green for the five new tests.

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/domain.rs \
        lib/crates/project/src/domain/error.rs \
        lib/crates/project/src/domain/project_language.rs \
        lib/crates/project/src/domain/project_configuration.rs \
        lib/crates/project/src/domain/tests.rs \
        lib/crates/project/src/lib.rs
git commit -m "feat(project): add ProjectLanguage enum and ProjectConfiguration struct

The domain layer gains ProjectLanguage { English, SimplifiedChinese }
with as_str / TryFrom<&str> mapping to wire codes \"en\" and
\"zh-CN\", plus a DomainError::UnknownLanguage(String) variant for
unknown wire codes.

ProjectConfiguration { language: Option<ProjectLanguage>, tags:
Vec<ProjectTag> } composes the new locale field with the existing
tag list. Default derives so an empty configuration is
straightforward; the only constructor is for_repository because the
inner ProjectTag already enforces its own non-empty contract.

Both types are re-exported from the crate root and the
project::domain module so consumers can `use project::*;`.

Spec coverage: domain type definitions per the in-task spec.

Verification: cargo test -p project --lib domain::tests::project_language"
```

---

## Task 2: `Project.configurations` — replace `tags` field

**Files:**
- Modify: `lib/crates/project/src/domain/project.rs`
- Modify: `lib/crates/project/src/domain/tests.rs`

### 2.1 Replace `Project.tags` with `Project.configurations`

- [ ] **Step 1:** Replace `lib/crates/project/src/domain/project.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::project_configuration::ProjectConfiguration;
use super::project_member::ProjectMember;
use super::project_tag::ProjectTag;

#[derive(Clone, PartialEq, Eq)]
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub configurations: ProjectConfiguration,
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
        configurations: ProjectConfiguration,
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
            configurations,
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
        configurations: ProjectConfiguration,
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
            configurations,
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
            .field("configurations", &self.configurations)
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
    /// Optional configuration. `None` defaults to an empty
    /// configuration (no language, no tags). `Some(config)` writes
    /// the whole configuration on create.
    pub configuration: Option<ProjectConfiguration>,
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
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace (language and tags together).
    pub configuration: Option<ProjectConfiguration>,
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

### 2.2 Update the existing `domain/tests.rs`

- [ ] **Step 1:** Edit `lib/crates/project/src/domain/tests.rs`. The two `project_new_*` tests currently pass `vec![]` as the tags arg; replace both with `ProjectConfiguration::default()`:

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
        ProjectConfiguration::default(),
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
        ProjectConfiguration::default(),
        true,
        test_now(),
        test_now(),
    )
    .unwrap();
    assert_eq!(p.id, 9);
    assert!(p.configurations.tags.is_empty());
    assert!(p.configurations.language.is_none());
}
```

(The `test_now` helper already exists in this file.)

### 2.3 Verify + commit

- [ ] **Step 1:** Run the domain tests.

Run: `cargo test -p project --lib domain:: 2>&1 | tail -20`
Expected: green for `project_new_*` and the five new tests.

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/domain/project.rs \
        lib/crates/project/src/domain/tests.rs
git commit -m "refactor(project): rename Project.tags to Project.configurations

The Project aggregate gains configurations: ProjectConfiguration
in place of the previous top-level tags: Vec<ProjectTag> field.
Both constructors (new + for_repository) and the manual Debug
impl carry the new shape; ProjectNew / ProjectUpdate gain
configuration: Option<ProjectConfiguration> with the same
None/Some semantics the previous tags field had.

Domain tests for the new Project signature are updated in place;
the new ProjectConfiguration behaviour is covered by Task 1.

Spec coverage: Project struct refactor per the in-task spec.

Verification: cargo test -p project --lib domain::"
```

---

## Task 3: Database migration — `projects.configuration`

**Files:**
- Create: `lib/crates/project/migrations/0002_add_project_configuration.sql`

### 3.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/project/migrations/0002_add_project_configuration.sql`:

```sql
-- 0002_add_project_configuration.sql
--
-- Migrate the existing `projects.tags` JSONB column into a new
-- `projects.configuration` JSONB column that carries both the
-- project's locale (`language`) and its tag list (`tags`). The
-- `tags` column is dropped in the same migration; the array-shape
-- CHECK is replaced with an object-shape CHECK on `configuration`.
--
-- Default shape on the new column:
--   {"language": null, "tags": []}
-- so inserts that omit the column land in a coherent state without
-- any application-side patching.

ALTER TABLE projects
    ADD COLUMN configuration JSONB NOT NULL
        DEFAULT '{"language": null, "tags": []}'::jsonb;

UPDATE projects
    SET configuration = jsonb_build_object(
        'language', NULL,
        'tags', COALESCE(tags, '[]'::jsonb)
    );

ALTER TABLE projects DROP COLUMN tags;

ALTER TABLE projects DROP CONSTRAINT projects_tags_is_array;

ALTER TABLE projects
    ADD CONSTRAINT projects_configuration_is_object
        CHECK (jsonb_typeof(configuration) = 'object');
```

### 3.2 Commit

- [ ] **Step 1:**

```bash
git add lib/crates/project/migrations/0002_add_project_configuration.sql
git commit -m "feat(project): migrate projects.tags to projects.configuration

Migration 0002 introduces a JSONB configuration column that carries
both the project's locale (\"language\": Option<string>) and its
tag list (\"tags\": Vec<{key, value}>). Existing rows are rewritten
so each project's prior tags land under configuration.tags; the
old tags column is dropped and the array CHECK is replaced with
jsonb_typeof(configuration) = 'object'.

The default value is '{\"language\": null, \"tags\": []}'::jsonb
so inserts that omit the column land in a coherent shape.

Spec coverage: schema migration per the in-task spec.

Verification: applied locally via sqlx migrate run; live-DB tests
will exercise the new column shape end-to-end."
```

---

## Task 4: Persistence — `ProjectRow` + `ProjectRepo`

**Files:**
- Modify: `lib/crates/project/src/adapter/persistence/postgres/row.rs`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`

### 4.1 Update `row.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres/row.rs`:

```rust
//! Row -> domain conversion for the SQLx repository.
//!
//! `ProjectRow` is the shape returned by `sqlx::query_as`. It is NOT
//! re-exported at the crate root; only the repository uses it.

use std::convert::TryFrom;

use chrono::{DateTime, Utc};
use sqlx::FromRow;

use crate::domain::{
    DomainError, Project, ProjectConfiguration, ProjectMember, ProjectTag,
};

#[derive(Clone, FromRow)]
pub struct ProjectRow {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub active: bool,
    pub configuration: sqlx::types::Json<ProjectConfiguration>,
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
            row.configuration.0,
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

// `ProjectTag` must remain reachable from this module so the
// configuration column can round-trip nested tags through
// serde without a dead-code lint warning when the row tests in
// the parent module only exercise empty configurations.
#[allow(dead_code)]
fn _force_use_project_tag(_t: ProjectTag) {}
```

### 4.2 Update `project_repo.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/persistence/postgres/project_repo.rs`:

```rust
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, Project, ProjectConfiguration, ProjectMember, ProjectNew, ProjectRepository,
    ProjectUpdate, RoleType, TeamType,
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

        let configuration_json =
            sqlx::types::Json(&input.configuration.unwrap_or_default());

        let row: ProjectRow = sqlx::QueryBuilder::new(
            "INSERT INTO projects (code, description, active, configuration) VALUES (",
        )
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.description)
        .push(", ")
        .push_bind(true)
        .push(", ")
        .push_bind(configuration_json)
        .push(
            ") RETURNING id, code, description, active, configuration, created_at, updated_at",
        )
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
            "SELECT id, code, description, active, configuration, created_at, updated_at \
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
            "SELECT id, code, description, active, configuration, created_at, updated_at \
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
            "SELECT id, code, description, active, configuration, created_at, updated_at \
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
        // touch membership or configuration.
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
                " RETURNING id, code, description, active, configuration, created_at, updated_at",
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

        // Whole-configuration replace, in the same transaction.
        if let Some(ref configuration) = input.configuration {
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
            sqlx::QueryBuilder::new("UPDATE projects SET configuration = ")
                .push_bind(sqlx::types::Json(configuration))
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

// `ProjectConfiguration` must stay reachable from this module so
// the `Json(&configuration)` binding inside `update` resolves
// without an unused-import warning when only metadata fields are
// touched.
#[allow(dead_code)]
fn _force_use_project_configuration(_c: &ProjectConfiguration) {}
```

### 4.3 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: errors in the usecase + facade + test files (fixed in subsequent tasks).

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/adapter/persistence/postgres/row.rs \
        lib/crates/project/src/adapter/persistence/postgres/project_repo.rs
git commit -m "refactor(project): persist projects.configuration JSONB column

ProjectRow.configuration replaces the previous tags column and
serialises through sqlx::types::Json<ProjectConfiguration>. The
TryFrom<ProjectRow> for Project bridge plumbs the configuration
through into Project::for_repository alongside the membership rows.

ProjectRepo's create / find_by_id / find_by_code / list / update
SQL all reference configuration instead of tags. update retains
the whole-replace transactional write guarded by the project-exists
preflight.

Spec coverage: persistence migration per the in-task spec.

Verification: cargo check -p project --lib (other layers fixed in
subsequent tasks)"
```

---

## Task 5: Usecase layer — commands, views, usecase logic

**Files:**
- Modify: `lib/crates/project/src/usecase.rs`
- Modify: `lib/crates/project/src/usecase/commands.rs`
- Modify: `lib/crates/project/src/usecase/views.rs`
- Modify: `lib/crates/project/src/usecase/project_usecase.rs`

### 5.1 Update `commands.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/commands.rs`:

```rust
use crate::domain::{ProjectConfiguration, ProjectMember};

#[derive(Debug, Clone)]
pub struct CreateProject {
    pub code: String,
    pub description: String,
    /// Optional. `None` and `Some(empty)` are equivalent on create.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// Optional. `None` defaults to an empty configuration (no
    /// language, no tags); `Some(config)` writes the whole
    /// configuration on create.
    pub configuration: Option<ProjectConfiguration>,
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
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace.
    pub configuration: Option<ProjectConfiguration>,
}
```

### 5.2 Update `views.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/views.rs`:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{Project, ProjectConfiguration, ProjectTag, UserSummary};

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

/// Server-side projection of the project's configuration: an
/// optional locale plus the tag list. Mirrors the apis
/// `ProjectConfigurationView` so the facade `From` impl is a
/// straight rename.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectConfigurationView {
    pub language: Option<crate::domain::ProjectLanguage>,
    pub tags: Vec<TagView>,
}

impl From<ProjectConfiguration> for ProjectConfigurationView {
    fn from(c: ProjectConfiguration) -> Self {
        Self {
            language: c.language,
            tags: c.tags.into_iter().map(Into::into).collect(),
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
    pub configurations: ProjectConfigurationView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl ProjectView {
    /// Build the view around a domain `Project`. Membership lists
    /// must already be hydrated to `ProjectMemberView` (look up user
    /// summaries before calling). Configuration passes through as
    /// `ProjectConfigurationView`.
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
            configurations: project.configurations.into(),
            active: project.active,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}
```

### 5.3 Update `usecase.rs`

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
pub use views::{
    ProjectConfigurationView, ProjectMemberView, ProjectView, TagView, UserSummaryView,
};
```

### 5.4 Update `project_usecase.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/usecase/project_usecase.rs`:

```rust
use std::collections::HashMap;

use crate::domain::{
    DomainError, Project, ProjectConfiguration, ProjectMember, ProjectNew, ProjectRepository,
    ProjectTag, ProjectUpdate, UserService, UserSummary,
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
                configuration: cmd.configuration,
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
                configuration: cmd.configuration,
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
    if let Some(ref c) = cmd.configuration {
        validate_configuration_tags(c)?;
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
    if let Some(ref c) = cmd.configuration {
        validate_configuration_tags(c)?;
    }
    Ok(())
}

/// Tag validation surfaces as `Validation`, not `Repository`. The
/// domain `From<DomainError>` impl maps straight to `Repository`,
/// so map the two tag variants explicitly.
fn validate_configuration_tags(c: &ProjectConfiguration) -> Result<(), UsecaseError> {
    for tag in &c.tags {
        match ProjectTag::new(tag.key.clone(), tag.value.clone()) {
            Ok(_) => {}
            Err(DomainError::EmptyTagKey) => {
                return Err(UsecaseError::Validation(DomainError::EmptyTagKey));
            }
            Err(DomainError::EmptyTagValue) => {
                return Err(UsecaseError::Validation(DomainError::EmptyTagValue));
            }
            Err(other) => return Err(UsecaseError::Repository(other)),
        }
    }
    Ok(())
}
```

### 5.5 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: errors only in test files (fixed in later tasks).

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/usecase.rs \
        lib/crates/project/src/usecase/commands.rs \
        lib/crates/project/src/usecase/views.rs \
        lib/crates/project/src/usecase/project_usecase.rs
git commit -m "refactor(project): usecase threads ProjectConfiguration through

CreateProject and UpdateProject drop the previous tags field in
favour of configuration: Option<ProjectConfiguration>. The
usecase re-validates the inner tags via ProjectTag::new and maps
the two tag-specific errors through Validation (preserving the
prior error channel).

A new ProjectConfigurationView mirrors ProjectConfiguration for
the read path; ProjectView gains configurations:
ProjectConfigurationView and ProjectView::from_project drops the
tags projection. validate_create_project / validate_update_project
share a small validate_configuration_tags helper.

Spec coverage: usecase wiring per the in-task spec.

Verification: cargo check -p project --lib"
```

---

## Task 6: Facade — `ProjectServiceImpl`

**Files:**
- Modify: `lib/crates/project/src/adapter/facade/in_memory/service.rs`

### 6.1 Replace `service.rs`

- [ ] **Step 1:** Replace `lib/crates/project/src/adapter/facade/in_memory/service.rs`:

```rust
use async_trait::async_trait;

use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectConfigurationData, ProjectMemberData,
    ProjectMemberView as ApiProjectMemberView, ProjectService, ProjectView, UpdateProjectRequest,
    UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{ProjectConfiguration, ProjectMember, ProjectRepository, ProjectTag, UserService};
use crate::usecase::{
    CreateProject, ProjectConfigurationView, ProjectUsecase, UpdateProject,
    UserSummaryView as DomainUserSummaryView,
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
                configuration: req.configurations.map(configuration_data_to_domain),
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
                configuration: req.configurations.map(configuration_data_to_domain),
            })
            .await
            .map_err(map_error)?;
        Ok(view.into())
    }
}

fn member_data_to_domain(d: ProjectMemberData) -> ProjectMember {
    ProjectMember::for_repository(d.leaders, d.workers)
}

/// Bridge for the request-side `ProjectConfigurationData` so the
/// apis port doesn't need to reach into the domain types. The
/// usecase / domain layer re-validates the inner tags via
/// `ProjectTag::new`; if the wire payload violated the non-empty
/// contract, that re-validation surfaces as
/// `UsecaseError::Validation(EmptyTagKey | EmptyTagValue)`.
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(d.language, tag_data_vec_to_domain(d.tags))
}

fn tag_data_vec_to_domain(tags: Vec<apis::project::TagData>) -> Vec<ProjectTag> {
    tags.into_iter()
        .map(|t| ProjectTag::for_repository(t.key, t.value))
        .collect()
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

impl From<crate::usecase::ProjectView> for ProjectView {
    fn from(v: crate::usecase::ProjectView) -> Self {
        Self {
            id: v.id,
            code: v.code,
            description: v.description,
            members: v.members.into(),
            unblind_members: v.unblind_members.into(),
            configurations: v.configurations.into(),
            active: v.active,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView {
    fn from(v: ProjectConfigurationView) -> Self {
        Self {
            language: v.language,
            tags: v.tags.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<crate::usecase::TagView> for apis::project::TagView {
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

### 6.2 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p project --lib 2>&1 | tail -20`
Expected: errors only in test files (fixed in subsequent tasks).

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/project/src/adapter/facade/in_memory/service.rs
git commit -m "refactor(project): facade maps ProjectConfigurationData through

create_project and update_project translate the apis
ProjectConfigurationData into a domain ProjectConfiguration via
a small helper. The From<usecase::ProjectConfigurationView> for
apis::project::ProjectConfigurationView impl replaces the prior
tag-by-tag projection.

Spec coverage: facade wiring per the in-task spec.

Verification: cargo check -p project --lib"
```

---

## Task 7: Apis port — add language / configuration types

**Files:**
- Modify: `lib/crates/apis/src/project.rs`

### 7.1 Rewrite `lib/apis/src/project.rs`

- [ ] **Step 1:** Replace `lib/crates/apis/src/project.rs`:

```rust
//! Outbound port for project lifecycle operations.
//!
//! See [`ProjectService`] for the trait surface. All supporting types
//! (`ProjectApiError`, `ProjectView`, `ProjectConfigurationView`,
//! `ProjectMemberView`, `UserSummaryView`, `TagData`, `TagView`,
//! `*Request`) are defined alongside the trait so a single
//! `use apis::project::*;` brings the whole contract into scope.

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

/// Locale hint. Carried on the project configuration so a project
/// can pin its UI to English or Simplified Chinese. The wire code
/// is the lowercase IETF tag (`"en"` or `"zh-CN"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ProjectLanguage {
    English,
    SimplifiedChinese,
}

impl ProjectLanguage {
    /// Stable wire code. New locales land here as new variants.
    pub fn as_str(&self) -> &'static str {
        match self {
            ProjectLanguage::English => "en",
            ProjectLanguage::SimplifiedChinese => "zh-CN",
        }
    }
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

/// Request-side configuration: optional locale plus the tag list.
/// Carries `TagData` because it travels into the backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
}

/// Server-side projection of the configuration. Carries `TagView`
/// because it travels back to clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
}

/// Safe projection of a project: membership lists are hydrated to
/// `Vec<UserSummaryView>`; the configuration (locale + tags) is
/// projected to `ProjectConfigurationView`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub configurations: ProjectConfigurationView,
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
    /// Optional. `None` and `Some(empty)` both mean "default empty
    /// configuration (no language, no tags)".
    pub configurations: Option<ProjectConfigurationData>,
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
    /// `None` = leave configuration unchanged; `Some(config)` =
    /// whole-configuration replace.
    pub configurations: Option<ProjectConfigurationData>,
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

### 7.2 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p apis`
Expected: errors only in the project crate (no consumer compiles yet).

- [ ] **Step 2:** Commit.

```bash
git add lib/crates/apis/src/project.rs
git commit -m "feat(apis): add ProjectLanguage and ProjectConfiguration to project port

The project port gains:

- ProjectLanguage { English, SimplifiedChinese } enum with a
  stable as_str wire code.
- ProjectConfigurationData (request shape: language + TagData vec)
  and ProjectConfigurationView (server-side projection: language +
  TagView vec).

ProjectView.tags becomes ProjectView.configurations:
ProjectConfigurationView; CreateProjectRequest and
UpdateProjectRequest gain configurations:
Option<ProjectConfigurationData>. The wire contracts stay
self-contained: every renamed field has the matching request and
view sides.

Spec coverage: apis-side wire contract per the in-task spec.

Verification: cargo check -p apis"
```

---

## Task 8: Server — DTOs and handlers

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/project/handlers.rs`

### 8.1 Update `dto.rs`

- [ ] **Step 1:** Edit `apps/server/aegis-server/src/transport/http/dto.rs`. After the existing `/* -- tag DTOs -- */` block (around line 276-304), insert the new configuration DTOs and their `From` impls:

```rust
// -- project configuration DTOs ---------------------------------------------

/// Wire-level request body for a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationData` field-for-field.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<apis::project::ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
}

impl From<ProjectConfigurationDataRequest> for apis::project::ProjectConfigurationData {
    fn from(c: ProjectConfigurationDataRequest) -> Self {
        Self {
            language: c.language,
            tags: c
                .tags
                .into_iter()
                .map(|t| apis::project::TagData {
                    key: t.key,
                    value: t.value,
                })
                .collect(),
        }
    }
}

/// Wire-level projection of a project's configuration. Mirrors
/// `apis::project::ProjectConfigurationView` field-for-field.
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<apis::project::ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
}

impl From<apis::project::ProjectConfigurationView> for ProjectConfigurationViewResponse {
    fn from(c: apis::project::ProjectConfigurationView) -> Self {
        Self {
            language: c.language,
            tags: c.tags.into_iter().map(Into::into).collect(),
        }
    }
}
```

- [ ] **Step 2:** Edit the `CreateProjectRequest` definition (around line 360) so the tags field becomes configurations:

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
}
```

- [ ] **Step 3:** Edit the `UpdateProjectRequest` definition (around line 383) similarly:

```rust
#[derive(Serialize, Deserialize, ToSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct UpdateProjectRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unblind_members: Option<ProjectMemberDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configurations: Option<ProjectConfigurationDataRequest>,
}
```

- [ ] **Step 4:** Edit the `ProjectViewResponse` struct and its `From` impl (around line 398-426):

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectViewResponse {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub members: ProjectMemberViewResponse,
    pub unblind_members: ProjectMemberViewResponse,
    pub configurations: ProjectConfigurationViewResponse,
    pub active: bool,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<apis::project::ProjectView> for ProjectViewResponse {
    fn from(view: apis::project::ProjectView) -> Self {
        Self {
            id: view.id,
            code: view.code,
            description: view.description,
            members: view.members.into(),
            unblind_members: view.unblind_members.into(),
            configurations: view.configurations.into(),
            active: view.active,
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}
```

### 8.2 Update `handlers.rs`

- [ ] **Step 1:** Edit `apps/server/aegis-server/src/transport/http/project/handlers.rs`. Replace the `tag_data` helper with a `configuration_data` helper, and use it in both `create_project` and `update_project`. Concretely:

Replace this near the top:

```rust
/// Translate a wire tag DTO into the apis DTO. Validation (non-empty
/// key/value) is delegated to the domain layer — the handler just
/// passes through whatever the client supplied.
fn tag_data(value: dto::TagDataRequest) -> apis::project::TagData {
    apis::project::TagData {
        key: value.key,
        value: value.value,
    }
}
```

with:

```rust
/// Translate the wire configuration DTO into the apis DTO.
/// Validation (non-empty tag key/value) is delegated to the
/// domain layer — the handler just passes through whatever the
/// client supplied.
fn configuration_data(
    value: dto::ProjectConfigurationDataRequest,
) -> apis::project::ProjectConfigurationData {
    value.into()
}
```

In the `create_project` body, replace:

```rust
tags: req.tags.map(|ts| ts.into_iter().map(tag_data).collect()),
```

with:

```rust
configurations: req.configurations.map(configuration_data),
```

In the `update_project` body, replace the same line with the same replacement.

- [ ] **Step 2:** Edit the `sample_project_view` test helper (around line 212) so it constructs a `ProjectView` with the new `configurations` field instead of `tags`:

```rust
fn sample_project_view(id: i32, code: &str) -> apis::project::ProjectView {
    apis::project::ProjectView {
        id,
        code: code.to_string(),
        description: "sample".to_string(),
        members: apis::project::ProjectMemberView::default(),
        unblind_members: apis::project::ProjectMemberView::default(),
        configurations: apis::project::ProjectConfigurationView::default(),
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    }
}
```

- [ ] **Step 3:** Edit the JSON-body tests that exercise the create/update wire shape (the `create_project_root_returns_201` test around line 477 and the create/update DTO round-trip tests in `dto.rs`). Replace tag-bearing fixtures with configuration-bearing ones.

In `create_project_root_returns_201`, change the request body to include `configurations`:

```rust
r#"{"code":"pr1","description":"x","members":{"leaders":["l1"]},"configurations":{"language":"en","tags":[{"key":"Product","value":"DEMO-001"}]}}"#.to_string(),
```

and the assertion:

```rust
let configurations = captured.configurations.expect("configurations present");
let tags = configurations.tags;
assert_eq!(tags.len(), 1);
assert_eq!(tags[0].key, "Product");
assert_eq!(tags[0].value, "DEMO-001");
```

In `dto.rs`, replace the tag-bearing test bodies with the new shape. Concretely:

`create_project_request_with_tags_roundtrip` becomes `create_project_request_with_configurations_roundtrip`:

```rust
#[test]
fn create_project_request_with_configurations_roundtrip() {
    let json = r#"{"code":"pr1","description":"x","configurations":{"language":"en","tags":[{"key":"Product","value":"DEMO-001"}]}}"#;
    let req: CreateProjectRequest = serde_json::from_str(json).unwrap();
    let configurations = req.configurations.as_ref().expect("configurations present");
    assert_eq!(configurations.language, Some(apis::project::ProjectLanguage::English));
    let tags = &configurations.tags;
    assert_eq!(tags.len(), 1);
    assert_eq!(tags[0].key, "Product");
    assert_eq!(tags[0].value, "DEMO-001");
}
```

`update_project_request_empty_tags_becomes_some_empty` becomes `update_project_request_empty_configurations_becomes_some_empty`:

```rust
#[test]
fn update_project_request_empty_configurations_becomes_some_empty() {
    let json = r#"{"configurations":{"tags":[]}}"#;
    let req: UpdateProjectRequest = serde_json::from_str(json).unwrap();
    let configurations = req.configurations.as_ref().expect("configurations present");
    assert!(configurations.tags.is_empty());
}
```

The other request-shape tests that check `tags.is_none()` become `configurations.is_none()`.

`project_view_response_includes_tags` becomes `project_view_response_includes_configurations`. The relevant assertion:

```rust
#[test]
fn project_view_response_includes_configurations() {
    let view = apis::project::ProjectView {
        id: 7,
        code: "p7".into(),
        description: "".into(),
        members: apis::project::ProjectMemberView::default(),
        unblind_members: apis::project::ProjectMemberView::default(),
        configurations: apis::project::ProjectConfigurationView {
            language: Some(apis::project::ProjectLanguage::English),
            tags: vec![
                apis::project::TagView {
                    key: "Product".into(),
                    value: "DEMO-001".into(),
                },
                apis::project::TagView {
                    key: "Region".into(),
                    value: "EU".into(),
                },
            ],
        },
        active: true,
        created_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
        updated_at: chrono::DateTime::parse_from_rfc3339("2026-01-02T03:04:05Z")
            .unwrap()
            .with_timezone(&chrono::Utc),
    };
    let resp: ProjectViewResponse = view.into();
    assert_eq!(resp.configurations.language, Some(apis::project::ProjectLanguage::English));
    assert_eq!(resp.configurations.tags.len(), 2);
    assert_eq!(resp.configurations.tags[0].key, "Product");
    assert_eq!(resp.configurations.tags[1].value, "EU");
}
```

### 8.3 Verify + commit

- [ ] **Step 1:**

Run: `cargo check -p aegis-server`
Expected: green.

- [ ] **Step 2:** Commit.

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs \
        apps/server/aegis-server/src/transport/http/project/handlers.rs
git commit -m "feat(server): thread ProjectConfiguration through HTTP layer

Two new wire DTOs land alongside the existing tag DTOs:

- ProjectConfigurationDataRequest carries an Option<ProjectLanguage>
  plus a Vec<TagDataRequest>; deserialises with skip_serializing_if
  so absent values stay absent on the wire.
- ProjectConfigurationViewResponse mirrors the apis
  ProjectConfigurationView.

CreateProjectRequest / UpdateProjectRequest rename the previous
tags field to configurations; ProjectViewResponse renames tags to
configurations. The handlers translate via a single
configuration_data helper, and the JSON-body round-trip tests
exercise both the request and the response shape.

Spec coverage: server-side wire contract per the in-task spec.

Verification: cargo check -p aegis-server"
```

---

## Task 9: Project-crate test files

**Files:**
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs`
- Modify: `lib/crates/project/src/usecase/tests.rs`
- Modify: `lib/crates/project/src/adapter/facade/in_memory/tests.rs`
- Modify: `lib/crates/project/tests/public_api.rs`
- Modify: `lib/crates/project/tests/integration_persistence.rs`

### 9.1 Update `postgres/tests.rs`

- [ ] **Step 1:** Edit `lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

Update the `projects_migration_has_required_columns` test's `required` list to drop `TAGS JSONB` and add `CONFIGURATION JSONB`:

```rust
for required in [
    "ID INTEGER",
    "CODE TEXT",
    "DESCRIPTION TEXT",
    "ACTIVE BOOLEAN",
    "CONFIGURATION JSONB NOT NULL DEFAULT '{\"LANGUAGE\": NULL, \"TAGS\": []}'::JSONB",
    "CREATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
    "UPDATED_AT TIMESTAMPTZ NOT NULL DEFAULT NOW()",
] {
```

Replace `projects_migration_has_tags_array_check`:

```rust
#[test]
fn projects_migration_has_configuration_object_check() {
    let sql = load_migration("0001_create_projects.sql");
    let sql2 = load_migration("0002_add_project_configuration.sql");
    let combined = format!("{sql}\n{sql2}").to_uppercase();
    assert!(
        combined.contains("JSONB_TYPEOF(CONFIGURATION) = 'OBJECT'"),
        "projects table must enforce jsonb_typeof(configuration) = 'object'; got:\n{combined}"
    );
}
```

In the row_tests module:

```rust
use crate::domain::{ProjectConfiguration, ProjectMember, ProjectTag, RoleType, TeamType};
```

Replace the two `project_row_converts_to_project_with_*` tests with the configuration-shaped versions:

```rust
#[test]
fn project_row_converts_to_project_with_empty_configuration() {
    let row = ProjectRow {
        id: 1,
        code: "proj1".into(),
        description: "".into(),
        active: true,
        configuration: sqlx::types::Json(ProjectConfiguration::default()),
        created_at: ts(),
        updated_at: ts(),
    };
    let p: crate::domain::Project = row.try_into().expect("convert");
    assert_eq!(p.id, 1);
    assert_eq!(p.members, ProjectMember::default());
    assert_eq!(p.unblind_members, ProjectMember::default());
    assert!(p.configurations.tags.is_empty());
    assert!(p.configurations.language.is_none());
}

#[test]
fn project_row_converts_to_project_with_configuration() {
    let row = ProjectRow {
        id: 1,
        code: "proj1".into(),
        description: "".into(),
        active: true,
        configuration: sqlx::types::Json(ProjectConfiguration::for_repository(
            Some(crate::domain::ProjectLanguage::English),
            vec![
                ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
                ProjectTag::for_repository("Region".into(), "EU".into()),
            ],
        )),
        created_at: ts(),
        updated_at: ts(),
    };
    let p: crate::domain::Project = row.try_into().expect("convert");
    assert_eq!(p.configurations.language, Some(crate::domain::ProjectLanguage::English));
    assert_eq!(p.configurations.tags.len(), 2);
    assert_eq!(p.configurations.tags[0].key, "Product");
    assert_eq!(p.configurations.tags[1].value, "EU");
}
```

### 9.2 Update `usecase/tests.rs`

- [ ] **Step 1:** Edit `lib/crates/project/src/usecase/tests.rs`.

Replace `use crate::domain::{...}`:

```rust
use crate::domain::{
    DomainError, Project, ProjectConfiguration, ProjectMember, ProjectNew, ProjectRepository,
    ProjectTag, ProjectUpdate, UserService, UserSummary,
};
```

In `MockProjectRepo::create`, replace the `tags` extraction:

```rust
let configuration = input.configuration.unwrap_or_default();
let project = Project::for_repository(
    id,
    input.code,
    input.description,
    members,
    unblind_members,
    configuration,
    true,
    now,
    now,
);
```

In `MockProjectRepo::update`, replace:

```rust
if let Some(ref tags) = input.tags {
    p.tags = tags.clone();
}
```

with:

```rust
if let Some(ref configuration) = input.configuration {
    p.configurations = configuration.clone();
}
```

In every test that builds `CreateProject` with `tags: Some(...)`, replace `tags:` with `configuration:` wrapping the tags inside a `ProjectConfiguration::for_repository`:

```rust
configuration: Some(ProjectConfiguration::for_repository(
    None,
    vec![
        ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
        ProjectTag::for_repository("Region".into(), "EU".into()),
    ],
)),
```

For the empty-tag-key / empty-tag-value validation tests, the `ProjectTag::for_repository("", "v".into())` still works because the validation is performed by `validate_configuration_tags` calling `ProjectTag::new` on the inner tag.

Update the assertion sites:

- `assert!(view.tags.is_empty())` → `assert!(view.configurations.tags.is_empty())`
- `assert_eq!(view.tags.len(), 2)` → `assert_eq!(view.configurations.tags.len(), 2)`
- `assert_eq!(view.tags[0].key, "Product")` → `assert_eq!(view.configurations.tags[0].key, "Product")`
- For `created.tags.len()` / `updated.tags.len()` / `updated.tags[0].key` similarly.

Update test names where helpful (e.g. `create_project_with_tags_succeeds` → `create_project_with_configuration_succeeds`, `update_project_replaces_tags_whole_list` → `update_project_replaces_configuration_whole_list`).

### 9.3 Update `facade/in_memory/tests.rs`

- [ ] **Step 1:** Edit `lib/crates/project/src/adapter/facade/in_memory/tests.rs`. Mirror the same mechanical changes as 9.2 in the in-memory repo + the facade tests. The `CreateProjectRequest { ..., tags: Some(...) }` calls need to become `configurations: Some(ProjectConfigurationData { language: None, tags: ... })`. The view assertion sites switch from `view.tags` to `view.configurations.tags`.

Update the imports:

```rust
use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectConfigurationData, ProjectService, UpdateProjectRequest,
};
```

Drop the `tag(...)` helper in favour of constructing `ProjectConfigurationData { language: None, tags: vec![...] }` inline, OR keep a small helper:

```rust
fn config_with_tags(tags: Vec<apis::project::TagData>) -> ProjectConfigurationData {
    ProjectConfigurationData { language: None, tags }
}
```

### 9.4 Update `tests/public_api.rs`

- [ ] **Step 1:** Edit `lib/crates/project/tests/public_api.rs`. Update the imports:

```rust
use project::{
    CreateProject, DomainError, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectNew,
    ProjectRepo, ProjectRepository, ProjectServiceImpl, ProjectTag, ProjectUpdate,
    ProjectUsecaseConfig, ProjectView, RoleType, TeamType, UpdateProject, UsecaseError, UserService,
    UserServiceImpl, UserSummary, UserSummaryView,
};
```

In `public_types_are_nameable_from_crate_root`, add two pinning assertions:

```rust
fn assert_project_configuration(_: ProjectConfiguration) {}
fn assert_project_language(_: ProjectLanguage) {}

assert_configuration(ProjectConfiguration::default());
assert_language(ProjectLanguage::English);
```

(Replace `assert_configuration`/`assert_language` with the helper names above. Two distinct local helper functions are fine.)

In `usecase_commands_have_expected_field_shape`, replace `tags: None,` with `configuration: None,` on both `_create_project` and `_update_project`.

In `api_requests_have_expected_field_shape`, replace `tags: None,` with `configurations: None,` on both requests. Replace the `apis_view_dtos_are_nameable` test's `assert_tag_view` and the `apis::project::TagView` constructions to also exercise the new `ProjectConfigurationView` / `ProjectConfigurationData`:

```rust
fn assert_configuration_view(_: apis::project::ProjectConfigurationView) {}
fn assert_configuration_data(_: apis::project::ProjectConfigurationData) {}

assert_configuration_view(apis::project::ProjectConfigurationView {
    language: Some(apis::project::ProjectLanguage::English),
    tags: vec![],
});
assert_configuration_data(apis::project::ProjectConfigurationData::default());
```

In `domain_error_variants_are_nameable`, add `assert_dom(DomainError::UnknownLanguage("ja".into()));`.

### 9.5 Update `tests/integration_persistence.rs`

- [ ] **Step 1:** Edit `lib/crates/project/tests/integration_persistence.rs`. Update imports:

```rust
use project::domain::{ProjectConfiguration, ProjectMember, ProjectNew, ProjectTag, ProjectUpdate};
```

In each `ProjectNew { ..., tags: ... }` site, replace `tags` with `configuration` wrapping the tags. In `project_create_with_no_membership_or_tags_round_trip`:

```rust
configuration: None,
```

In `project_create_with_tags_round_trip`:

```rust
configuration: Some(ProjectConfiguration::for_repository(
    None,
    vec![
        ProjectTag::for_repository("Product".into(), "DEMO-001".into()),
        ProjectTag::for_repository("Region".into(), "EU".into()),
    ],
)),
```

In `project_update_replaces_tags_whole_list`, replace `tags` with `configuration` and rename the test to `project_update_replaces_configuration_whole_list`. Update assertions:

```rust
assert_eq!(updated.configurations.tags.len(), 2);
assert_eq!(updated.configurations.tags[0].key, "k2");
assert_eq!(updated.configurations.tags[1].key, "k3");
```

Update the direct JSONB query assertion to read `configuration`:

```rust
let raw: serde_json::Value =
    sqlx::query_scalar("SELECT configuration FROM projects WHERE id = $1")
        .bind(created.id)
        .fetch_one(&pool)
        .await
        .expect("query configuration");
let raw_tags = raw.get("tags").and_then(|v| v.as_array()).expect("tags array");
assert_eq!(raw_tags.len(), 2);
assert_eq!(raw_tags[0]["key"], "k2");
assert_eq!(raw_tags[1]["value"], "v3");
```

Update `project_create_with_no_membership_or_tags_round_trip`'s `assert!(created.tags.is_empty())` to `assert!(created.configurations.tags.is_empty())`.

### 9.6 Verify the project crate is fully green

- [ ] **Step 1:**

Run: `cargo fmt --all -- --check && cargo clippy -p project --all-targets --all-features -- -D warnings && cargo test -p project`
Expected: green.

- [ ] **Step 2:**

Run: `cargo check --workspace`
Expected: green.

- [ ] **Step 3:**

Run: `cargo test -p project -- --ignored --test-threads=1` (only when `AEGIS_PROJECT_DATABASE_URL` is set; otherwise skip)
Expected: green. Verifies the new migration applies cleanly and `configuration` round-trips.

- [ ] **Step 4:** Commit.

```bash
git add lib/crates/project/src/adapter/persistence/postgres/tests.rs \
        lib/crates/project/src/usecase/tests.rs \
        lib/crates/project/src/adapter/facade/in_memory/tests.rs \
        lib/crates/project/tests/public_api.rs \
        lib/crates/project/tests/integration_persistence.rs
git commit -m "test(project): update all test files for ProjectConfiguration

Five test files exercise the new shape:

- persistence/postgres/tests.rs: schema assertions cover the
  configuration default JSONB literal and the jsonb_typeof = object
  CHECK; row tests build a ProjectRow with a Json<ProjectConfiguration>
  payload and assert the language + tags round-trip.

- usecase/tests.rs: MockProjectRepo stores configuration; the
  usecase tests build CreateProject / UpdateProject with
  configuration: Some(ProjectConfiguration::for_repository(...))
  and assert against view.configurations.*.

- facade/in_memory/tests.rs: in-memory repo + facade tests mirror
  the usecase changes at the apis port; a small config_with_tags
  helper keeps the request fixtures concise.

- tests/public_api.rs: pins ProjectConfiguration, ProjectLanguage,
  ProjectConfigurationView, ProjectConfigurationData,
  DomainError::UnknownLanguage; renames tags → configuration /
  configurations on the public-API call sites.

- tests/integration_persistence.rs: live-DB round-trips build
  ProjectNew with the new shape; the direct JSONB query exercises
  the configuration column.

Spec coverage: test coverage for the new shape across every
test tier.

Verification: cargo fmt --all -- --check && cargo clippy -p
project --all-targets --all-features -- -D warnings && cargo
test -p project && cargo check --workspace"
```

---

## Task 10: README + final verification

**Files:**
- Modify: `lib/crates/project/README.md`

### 10.1 Update `README.md`

- [ ] **Step 1:** Edit `lib/crates/project/README.md`. Update the domain-model list and the re-export paragraph:

```markdown
## Domain model

- `Product { id, code, name, description, active, created_at, updated_at }`
- `Project { id, code, description, product_id, members, unblind_members, configurations, active, created_at, updated_at }`
- `ProjectMember { leaders: Vec<String>, workers: Vec<String> }` — user *codes* (not full user records). The usecase layer hydrates these into `UserSummaryView` on read.
- `ProjectConfiguration { language: Option<ProjectLanguage>, tags: Vec<ProjectTag> }` — the project's optional locale (`English` / `SimplifiedChinese`) plus its tag list, persisted as a single JSONB column on `projects`.

`members` and `unblind_members` are two independent teams that share
the same `leaders` / `workers` shape. `create_project` accepts both as
optional (`None` and `Some(empty)` are equivalent on create). On
update, `None` leaves the team unchanged; `Some(empty)` wipes it
(whole-list replacement). The same missing-vs-empty distinction
applies to `configurations` — `None` on update leaves it alone,
`Some(config)` replaces the whole configuration object.
```

Update the re-export paragraph (around line 38-43) to mention the new types:

```markdown
The crate root re-exports the public surface (`Product`, `ProductNew`,
`ProductUpdate`, `Project`, `ProjectNew`, `ProjectUpdate`,
`ProjectMember`, `ProjectConfiguration`, `ProjectLanguage`,
`ProjectTag`, `TeamType`, `RoleType`, `UserSummary`,
`UserService`, `DomainError`, the ports `ProductRepository` /
`ProjectRepository`, the Postgres adapters `ProductRepo` /
`ProjectRepo`, the apis adapter `UserServiceImpl`, the usecase
`ProjectUsecase` + `ProjectUsecaseConfig`, the command DTOs
`CreateProduct` / `UpdateProduct` / `CreateProject` / `UpdateProject`,
the view DTOs `ProductView` / `ProjectView` / `ProjectConfigurationView`
/ `ProjectMemberView` / `UserSummaryView`, the error `UsecaseError`,
and the facade `ProjectServiceImpl`) so consumers can
`use project::*;` without reaching into the sub-modules.
```

Update the migration paragraph to mention the new migration:

```markdown
The crate ships two SQLx migrations that define `projects` (with
its `configuration` JSONB column) and `project_members`. Apply
them before pointing the repositories at the database:

```bash
sqlx migrate run --source lib/crates/project/migrations
```
```

(The existing copy says "two SQLx migrations" — that is now true; the migration count went from one to two.)

### 10.2 Final verification

- [ ] **Step 1:**

Run: `cargo fmt --all -- --check`
Expected: green.

- [ ] **Step 2:**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: green.

- [ ] **Step 3:**

Run: `cargo test -p project && cargo test -p apis && cargo test -p aegis-server`
Expected: green.

- [ ] **Step 4:**

Run: `cargo check --workspace`
Expected: green.

- [ ] **Step 5:** Commit.

```bash
git add lib/crates/project/README.md
git commit -m "docs(project): README updated for ProjectConfiguration

The domain-model list now shows Project.configurations in place of
the previous top-level tags; ProjectConfiguration / ProjectLanguage
are listed alongside the existing types; the re-export paragraph
names the new view DTO ProjectConfigurationView; the migration
paragraph reflects that the crate now ships two migrations.

Spec coverage: doc updates per the in-task spec.

Verification: cargo fmt --all -- --check && cargo clippy
--workspace --all-targets --all-features -- -D warnings && cargo
test -p project && cargo test -p apis && cargo test -p
aegis-server && cargo check --workspace"
```

---

## Self-Review

- **Spec coverage** (vs. the user's three bullets):
  1. New `ProjectConfiguration { language, tags }` struct — Task 1.
  2. `Project.tags` → `Project.configurations` — Task 2.
  3. Update related code except `apps/desktop/aegis-desktop/src-tauri` — Tasks 4 (persistence), 5 (usecase), 6 (facade), 7 (apis), 8 (server), 9 (tests). The desktop frontend reads from `src-tauri` which is excluded; since `src-tauri` does not change, the frontend does not need to change.
- **Placeholder scan:** No "TBD" / "TODO" / "implement later" / "add appropriate handling" remain. Every code step shows the exact replacement.
- **Type consistency:** All identifiers line up across tasks:
  - Domain: `ProjectLanguage { English, SimplifiedChinese }` — `as_str` returns `"en"` / `"zh-CN"`; `TryFrom<&str>` inverse.
  - `ProjectConfiguration { language: Option<ProjectLanguage>, tags: Vec<ProjectTag> }` — used by `Project`, `ProjectNew`, `ProjectUpdate`, `ProjectRow`, `MockProjectRepo`, `InMemProjectRepo`, the migration JSONB literal, and the live-DB round-trip.
  - Apis: `ProjectLanguage { English, SimplifiedChinese }`, `ProjectConfigurationData`, `ProjectConfigurationView` — same wire code mapping as the domain enum; `ProjectView.configurations`, `CreateProjectRequest.configurations`, `UpdateProjectRequest.configurations`.
  - Usecase: `ProjectConfigurationView`, `ProjectView.configurations`, `CreateProject.configuration`, `UpdateProject.configuration`.
  - Server: `ProjectConfigurationDataRequest`, `ProjectConfigurationViewResponse`, the three DTOs use `configurations`.
- **Task order check: Task 2 depends on Task 1 (domain types exist before Project can reference them). Task 3 has no Rust dependency (it's a SQL file). Task 4 depends on Tasks 1+2+3 (Row + Repo + Migration). Tasks 5+6 depend on Task 4. Task 7 depends on nothing inside this plan (apis types are independent). Task 8 depends on Task 7. Task 9 depends on Tasks 1+2+4+5+6+7+8. Task 10 depends on Task 9.

Plan complete and saved to `docs/superpowers/plans/2026-09-20-project-configuration.md`. Two execution options:

**1. Subagent-Driven (recommended)** - I dispatch a fresh subagent per task, review between tasks, fast iteration

**2. Inline Execution** - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?