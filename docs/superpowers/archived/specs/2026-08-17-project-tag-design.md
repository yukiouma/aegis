# Project Tag Design

## Goal

Extend the `project` crate with a `ProjectTag { key, value }` value object so each `Project` owns an arbitrary list of tags persisted as a PostgreSQL `JSONB` column. In the same change, retire the `Product` aggregate (and every related type, port, request, view, and table) so `Project` becomes the only top-level aggregate the crate manages.

A tag is shaped exactly like the user's example: `{"key": "Product", "value": "DEMO-001"}`. A project's tag list is stored as a JSONB array of those objects: `[{"key": "Product", "value": "DEMO-001"}, {"key": "Region", "value": "EU"}]`.

## Scope

In scope:

- New `ProjectTag` value object in the domain layer.
- New `tags: Vec<ProjectTag>` field on the `Project` aggregate.
- New `tags JSONB NOT NULL DEFAULT '[]'::jsonb` column on `projects`.
- New tag-shaped request / view DTOs on the apis port.
- Removal of the entire `Product` family (domain, repo, usecase, apis, table, FK, `product_id`).

Out of scope:

- Removing `ProjectMember` / `members` / `unblind_members` / `project_members`. Those are project-scoped, not product-scoped.
- New endpoints for tag CRUD beyond `create_project` / `update_project`. Tags are mutated via whole-list replacement on `update_project.tags`, mirroring how `members` already work.

## Constraints (confirmed during brainstorming)

- Only the new `tags` field is stored as JSONB. The rest of `projects` keeps typed columns (`code`, `description`, `active`, `created_at`, `updated_at`).
- Duplicate tag keys are allowed within the same project. The same key may appear multiple times with different values.
- Both `key` and `value` must be non-empty after trim. Empty-after-trim is rejected with `DomainError::EmptyTagKey` / `EmptyTagValue`.
- Tags are mutated by whole-list replacement: `update_project.tags = None` leaves tags unchanged; `Some(vec)` replaces the whole list.
- Migration history is squashed into a single new `0001_create_projects.sql` (old `0001_create_products.sql` and `0002_create_projects.sql` are deleted).

## Architecture

The crate keeps the existing ports-and-adapters DDD structure from the [project-crate design spec](2026-08-09-project-crate-design.md), minus everything that was Product-specific.

```
domain   ← Project, ProjectTag, ProjectMember, TeamType, RoleType,
           UserService (port), UserSummary, DomainError.
           No Product, no ProductRepository.
usecase  ← ProjectUsecase<R, U> (generic over R: ProjectRepository, U: UserService).
           Command DTOs: CreateProject, UpdateProject. View DTOs: ProjectView,
           ProjectMemberView, UserSummaryView. UsecaseError.
           No CreateProduct / UpdateProduct / ProductView.
adapter
  service/user        ← UserServiceImpl adapting apis::user::UserService to the
                        domain UserService port (unchanged).
  persistence/postgres ← ProjectRepo only. No ProductRepo. Tags serialised via
                        sqlx::types::Json<Vec<ProjectTag>>.
  facade/in_memory     ← ProjectServiceImpl adapting usecase → apis::project::ProjectService.
```

The `apis::project::ProjectService` trait loses every `*_product` method; `ProjectView` loses `product: ProductView` and gains `tags: Vec<TagView>`; `CreateProjectRequest` / `UpdateProjectRequest` lose `product_id` and gain `tags: Option<Vec<TagData>>`.

## Data Model

### Domain types

```rust
// src/domain/project_tag.rs (new)
use serde::{Deserialize, Serialize};

use super::error::DomainError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectTag {
    pub key: String,
    pub value: String,
}

impl ProjectTag {
    /// Validating constructor. Both fields must be non-empty after trim.
    /// Duplicate keys within the same project are NOT rejected here;
    /// callers (and downstream consumers) decide whether they care.
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
    pub(crate) fn for_repository(key: String, value: String) -> Self {
        Self { key, value }
    }
}
```

```rust
// src/domain/project.rs (updated)
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    // product_id removed.
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub tags: Vec<ProjectTag>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProjectNew {
    pub code: String,
    pub description: String,
    // product_id removed.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// Optional. None and Some(empty) both mean "no tags on create".
    pub tags: Option<Vec<ProjectTag>>,
}

pub struct ProjectUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    // product_id removed.
    pub active: Option<bool>,
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
    /// None = leave tags unchanged; Some(vec) = whole-list replace.
    pub tags: Option<Vec<ProjectTag>>,
}
```

### `DomainError` additions

```rust
#[derive(Debug, Error)]
pub enum DomainError {
    // … existing variants …
    #[error("tag key must not be empty")]
    EmptyTagKey,
    #[error("tag value must not be empty")]
    EmptyTagValue,
    // ProductNotFound removed.
    // ZeroProductId removed.
}
```

### Apis types (`lib/crates/apis/src/project.rs`)

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TagData {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    // product: ProductView removed.
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub tags: Vec<TagView>,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateProjectRequest {
    pub code: String,
    pub description: String,
    // product_id removed.
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    pub tags: Option<Vec<TagData>>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateProjectRequest {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    // product_id removed.
    pub active: Option<bool>,
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
    pub tags: Option<Vec<TagData>>,
}
```

The `ProjectService` trait drops every `*_product` method. `ProjectApiError` drops `ProductNotFound`.

## Schema

The two old migration files are deleted and replaced by a single new `lib/crates/project/migrations/0001_create_projects.sql`:

```sql
-- 0001_create_projects.sql
--
-- Single migration for the `project` crate. Replaces the previous
-- two-file history (products + projects + project_members) now that
-- the `Product` aggregate has been retired.

CREATE TABLE projects (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    tags JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT projects_code_unique UNIQUE (code),
    CONSTRAINT projects_tags_is_array CHECK (jsonb_typeof(tags) === 'array')
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

The `CHECK (jsonb_typeof(tags) = 'array')` constraint is belt-and-braces against a stray non-array insert at the SQL boundary; it does not replace application-level validation.

## Persistence

`src/adapter/persistence/postgres/row.rs` drops `ProductRow`. `ProjectRow` gains a `tags` column.

```rust
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
```

`ProjectRepo::create` and `update` continue to wrap membership writes in a transaction. `create` / `update` extend that transaction to write `tags` via `sqlx::types::Json(&tags)` so the project row, membership rows, and tag list land atomically. `update` writes tags via delete-then-insert when `tags` is `Some`, matching the membership semantics.

`ProductRepo` and `product_repo.rs` are deleted. The `product_repo` module declaration in `src/adapter/persistence/postgres.rs` is removed. The `pub use product_repo::ProductRepo;` line at the bottom of that file is removed; only `pub use project_repo::ProjectRepo;` remains.

## Usecase

`ProjectUsecase<P, R, U>` becomes `ProjectUsecase<R, U>`. `ProjectUsecaseConfig` drops `product_repo`. The five `*_product` methods are deleted. `create_project` no longer pre-flights a product; `validate_create_project` loses the `ZeroProductId` check and gains `for tag in tags: ProjectTag::new(tag.key.clone(), tag.value.clone())?`.

`hydrate_project_view` loses the product parameter and the `users.list()` call stays; `ProjectView::from_project` is renamed/reworked to drop the product argument and accept `tags`.

```rust
pub struct ProjectUsecaseConfig<R: ProjectRepository, U: UserService> {
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<R, U> { /* … */ }

impl<R: ProjectRepository, U: UserService> ProjectUsecase<R, U> {
    pub async fn create_project(&self, cmd: CreateProject) -> Result<ProjectView, UsecaseError> { … }
    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError> { … }
    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError> { … }
    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError> { … }
    pub async fn update_project(&self, cmd: UpdateProject) -> Result<ProjectView, UsecaseError> { … }
}
```

`UsecaseError` loses the `ProductNotFound` mapping.

## Facade

`ProjectServiceImpl<P, R, U>` becomes `ProjectServiceImpl<R, U>`. `service.rs` drops every `*_product` impl arm and the `ProductView` → `ApiProductView` `From`. New mapping `From<TagData> for ProjectTag` and `From<ProjectTag> for TagView` round-trip the request/response DTOs.

`map_error` loses the `DomainError::ProductNotFound` arm; everything else is unchanged.

## Public API surface at the crate root

`src/lib.rs`:

```rust
pub use domain::{
    DomainError, Project, ProjectMember, ProjectNew, ProjectRepository, ProjectTag, ProjectUpdate,
    RoleType, TeamType, UserService, UserSummary,
};
// Product, ProductNew, ProductUpdate, ProductRepository removed.

pub use usecase::{
    CreateProject, ProjectMemberView, ProjectUsecase, ProjectUsecaseConfig, ProjectView,
    TagView, UpdateProject, UsecaseError, UserSummaryView,
};
// CreateProduct, UpdateProduct, ProductView removed.

pub use adapter::facade::in_memory::ProjectServiceImpl;
pub use adapter::persistence::postgres::ProjectRepo;
// ProductRepo removed.
```

## Cross-crate ripple

`apis::project::ProjectService` loses methods and gains fields, so consumers of that port need updating in the same change:

- `apps/server/aegis-server/src/transport/http/project/handlers.rs` — remove `product_*` HTTP handlers; project handlers thread `tags` through request/response DTOs.
- `apps/server/aegis-server/src/transport/http/dto.rs` — drop `product` DTOs; add tag DTOs.
- `apps/server/aegis-server/src/transport/http/openapi.rs` — drop product paths; add `tags` field to project schema.
- `apps/server/aegis-server/src/transport/http/router.rs` — unregister product routes.
- `apps/server/aegis-server/src/run.rs` — remove the `ProductRepo` wiring (it isn't wired today, but verify).
- `apps/server/aegis-server/tests/integration_auth.rs` — drop any product fixtures (verify).
- `apps/desktop/aegis-desktop/src-tauri/src/commands/product.rs` — delete.
- `apps/desktop/aegis-desktop/src-tauri/src/http/product.rs` — delete.
- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` — add tag fields.

The desktop project-window work in flight (the `_project` route segment, `openProjectWorkspace`, etc.) does not conflict because it works against `Project`, not `Product`.

## Tests

Per [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md):

1. **Domain unit tests** (`src/domain/tests.rs`): add `project_tag_new_rejects_empty_key`, `project_tag_new_rejects_empty_value`, `project_tag_new_accepts_valid`. Remove the `*product_new_*` and `project_new_rejects_zero_product_id` cases; update `project_new_accepts_valid_input` for the new signature.
2. **Adapter unit tests** (`src/adapter/persistence/postgres/tests.rs`): drop `products_migration_*` tests; add `projects_migration_has_tags_jsonb_column` (asserts `TAGS JSONB NOT NULL DEFAULT`), `projects_migration_no_longer_has_product_id` (asserts absence of `product_id`), `projects_migration_has_tags_array_check` (asserts the `jsonb_typeof` CHECK). Keep `project_members_*`. Update `row_tests::project_row_converts_to_project_with_empty_members` to assert `tags` round-trips.
3. **Usecase unit tests** (`src/usecase/tests.rs`): remove `MockProductRepo` and every `*_product_*` test. Add `create_project_with_tags_succeeds`, `create_project_with_empty_tag_key_returns_validation_error`, `update_project_replaces_tags_whole_list`, `create_project_with_duplicate_tag_keys_succeeds`.
4. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`): mirror the usecase test additions; drop `*_product_*` cases. Add a `project_service_is_send_sync` regression for the now two-generic-param `ProjectServiceImpl`.
5. **`tests/public_api.rs`**: drop all `*_product*` references (constructors, fields, view DTOs). Add `ProjectTag`, `DomainError::EmptyTagKey`, `DomainError::EmptyTagValue`, `CreateProject.tags`, `UpdateProject.tags`, `ProjectView.tags` shape pins. Update the `ProjectUsecaseConfig` and `ProjectServiceImpl` type signatures to drop the now-removed `ProductRepo` generic argument.
6. **`tests/integration_persistence.rs`**: drop the `product_*` round-trips. Add `project_create_with_tags_round_trip` (insert three tags, reload, assert equality), `project_update_replaces_tags_whole_list` (insert two, replace with three, assert no overlap).

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p project --all-targets --all-features -- -D warnings
cargo test -p project
cargo doc -p project --no-deps
cargo test -p project -- --ignored --test-threads=1   # when AEGIS_PROJECT_DATABASE_URL is set

cargo check --workspace
cargo clippy --workspace
cargo test --workspace
```

The cross-crate changes (server, desktop, apis) ship in the same commit(s) so the workspace compiles end-to-end.

## Open questions

None at design time. Implementation-time unknowns (e.g. the exact `serde` feature set already enabled in the workspace `Cargo.toml`) are surfaced during the plan step.