# Project Configuration `sdtmig` Field Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an optional `sdtmig: Option<ModelVersion>` field to `ProjectConfiguration` so a project can pin the SDTM Implementation Guide (SDTM-IG) version it targets. The values come from `domain_model::SdtmVersion` (`id` ↔ `version_id`, `name` ↔ `version_name`) but the project crate does not need to verify existence — whatever the caller sends is saved verbatim.

**Architecture:** Domain value object `ModelVersion` lives next to `ProjectTag` / `ProjectLanguage` in the project crate (`domain/model_version.rs`). It serialises through the existing JSONB `configuration` column by tagging the wire object with a `sdtmig` key. The apis request/view DTOs gain a matching field, the usecase view mirrors it, the facade bridges the request and view, and the server HTTP DTOs gain a camelCase field. A new migration backfills the JSONB column with `"sdtmig": null` for existing rows so the JSON object shape stays uniform.

**Tech Stack:** Rust 2024, SQLx/Postgres JSONB, axum, serde, utoipa.

## Global Constraints

- edition = "2024", resolver = "3"; every shared dep via `{ workspace = true }`.
- Wire DTOs are duplicated by hand across `lib/crates/apis/src/project.rs`, `apps/server/aegis-server/src/transport/http/dto.rs`. TS DTOs in `apps/desktop/aegis-desktop` are **out of scope** for this plan.
- The optional field uses `Option<ModelVersion>` end-to-end; absent vs `Some(null)` (the inner object being `None` on the JSON side) is equivalent.
- The apis crate must NOT import from the `project` crate. Project types cross the boundary through the `From` impls in `ProjectServiceImpl`.
- The migration must preserve the existing default object shape (`{"language": null, "tags": []}`) and add `sdtmig: null` to that default so future inserts that omit the column land in a coherent state.
- Domain layer never validates cross-crate references — no check that `version_id` exists in `sdtm_versions`. The caller is trusted (consistent with how `ProjectTag::new` only validates non-empty, not cross-record uniqueness).

---

## File Structure

### Files to create
- `lib/crates/project/src/domain/model_version.rs` — `ModelVersion` value object.
- `lib/crates/project/migrations/0003_add_project_configuration_sdtmig.sql` — JSONB default backfill.

### Files to modify
- `lib/crates/project/src/domain/project_configuration.rs` — add `sdtmig: Option<ModelVersion>` field, widen `for_repository` constructor.
- `lib/crates/project/src/domain.rs` — re-export `ModelVersion`.
- `lib/crates/project/src/domain/tests.rs` — pin `ProjectConfiguration` default shape; new tests for `ModelVersion`.
- `lib/crates/project/src/usecase/views.rs` — add `sdtmig` to `ProjectConfigurationView` + `From` impl.
- `lib/crates/project/src/adapter/facade/in_memory/service.rs` — bridge `apis::project::ModelVersionData` ↔ domain.
- `lib/crates/project/src/adapter/persistence/postgres/tests.rs` — assert migration content + row round-trip.
- `lib/crates/apis/src/project.rs` — add `ModelVersionData` + `ModelVersionView` types; thread `sdtmig` through request / view.
- `apps/server/aegis-server/src/transport/http/dto.rs` — add wire `ModelVersionRequest` / `ModelVersionResponse`, wire through `ProjectConfigurationDataRequest` / `ProjectConfigurationViewResponse` / `CreateProjectRequest` / `UpdateProjectRequest` / `ProjectViewResponse`.
- `apps/server/aegis-server/src/transport/http/openapi.rs` — register the new schema components.

### Tests to update (shape only — same name, new fields)
- `lib/crates/project/tests/public_api.rs` — exercise the new field on `CreateProject` / `UpdateProject` / `CreateProjectRequest` / `UpdateProjectRequest`.
- `lib/crates/project/src/usecase/tests.rs` — at least one new test for `create_project_with_sdtmig_succeeds`; mock repo must accept the new field through `for_repository`.
- `lib/crates/project/src/adapter/facade/in_memory/tests.rs` — same for the facade; add a `model_version` helper.

---

## Task 1: Add the `ModelVersion` domain value object

**Files:**
- Create: `lib/crates/project/src/domain/model_version.rs`
- Modify: `lib/crates/project/src/domain.rs`
- Test: `lib/crates/project/src/domain/tests.rs`

**Interfaces:**
- Consumes: nothing.
- Produces:
  - `pub struct ModelVersion { pub version_id: i64, pub version_name: String }`
  - `ModelVersion::new(version_id: i64, version_name: String) -> Result<Self, DomainError>` — validating constructor; rejects empty / whitespace `version_name`.
  - `ModelVersion::for_repository(version_id: i64, version_name: String) -> Self` — non-validating, used by the adapter and downstream tests.

- [ ] **Step 1: Write the failing tests**

Add to `lib/crates/project/src/domain/tests.rs` (at the bottom, before any closing block — these belong with the existing `project_*` tests):

```rust
#[test]
fn model_version_new_rejects_empty_name() {
    let err = ModelVersion::new(1, "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmigName));
}

#[test]
fn model_version_new_rejects_whitespace_name() {
    let err = ModelVersion::new(1, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmigName));
}

#[test]
fn model_version_new_accepts_valid_input() {
    let v = ModelVersion::new(7, "2024-03-29".into()).unwrap();
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn model_version_for_repository_carries_fields() {
    let v = ModelVersion::for_repository(7, "2024-03-29".into());
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p project --lib domain::tests::model_version`
Expected: compile error — `ModelVersion` and `DomainError::EmptySdtmigName` not yet defined.

- [ ] **Step 3: Add `DomainError::EmptySdtmigName`**

Edit `lib/crates/project/src/domain/error.rs`; insert after the existing `EmptyTagValue` variant:

```rust
    #[error("sdtmig version name must not be empty")]
    EmptySdtmigName,
```

- [ ] **Step 4: Create `ModelVersion` value object**

Create `lib/crates/project/src/domain/model_version.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Pointer to the SDTM Implementation Guide (SDTM-IG) version a
/// project targets. `version_id` is the surrogate key from
/// `domain_model::SdtmVersion::id`; `version_name` is the
/// human-readable workbook suffix (e.g. `"2024-03-29"`). The two
/// fields travel together so callers can render the version name
/// without re-querying the domain-model crate.
///
/// The domain layer does NOT verify that `(version_id,
/// version_name)` actually exists in the `sdtm_versions` table — the
/// project crate trusts the caller (matching how `ProjectTag::new`
/// only enforces non-empty strings, not cross-record references).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersion {
    pub version_id: i64,
    pub version_name: String,
}

impl ModelVersion {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs).
    ///
    /// Rejects empty / whitespace `version_name`. `version_id` is
    /// accepted as-is because no cross-table check is performed.
    pub fn new(version_id: i64, version_name: String) -> Result<Self, DomainError> {
        if version_name.trim().is_empty() {
            return Err(DomainError::EmptySdtmigName);
        }
        Ok(Self {
            version_id,
            version_name,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column, and for downstream
    /// test / integration code that needs to construct a version
    /// pointer from a trusted source.
    #[allow(dead_code)]
    pub fn for_repository(version_id: i64, version_name: String) -> Self {
        Self {
            version_id,
            version_name,
        }
    }
}
```

- [ ] **Step 5: Re-export `ModelVersion`**

Edit `lib/crates/project/src/domain.rs`. Insert `mod model_version;` after `mod project_configuration;` (keep alphabetical — after `project_member`, before `project_tag`):

```rust
mod error;
mod model_version;
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
pub use model_version::ModelVersion;
pub use project::{Project, ProjectNew, ProjectRepository, ProjectUpdate};
pub use project_configuration::ProjectConfiguration;
pub use project_language::ProjectLanguage;
pub use project_member::ProjectMember;
pub use project_tag::ProjectTag;
pub use team_role::{RoleType, TeamType};
pub use user::{UserService, UserSummary};
```

- [ ] **Step 6: Run the tests to verify they pass**

Run: `cargo test -p project --lib domain::tests::model_version`
Expected: PASS (4 new tests green).

- [ ] **Step 7: Commit**

```bash
git add lib/crates/project/src/domain/model_version.rs \
        lib/crates/project/src/domain/error.rs \
        lib/crates/project/src/domain.rs \
        lib/crates/project/src/domain/tests.rs
git commit -m "feat(project): add ModelVersion value object"
```

---

## Task 2: Add `sdtmig` field to `ProjectConfiguration`

**Files:**
- Modify: `lib/crates/project/src/domain/project_configuration.rs`

**Interfaces:**
- Consumes: `ModelVersion` from Task 1.
- Produces: `ProjectConfiguration { language, tags, sdtmig }` and a widened `for_repository` constructor.

- [ ] **Step 1: Write the failing tests**

Append to `lib/crates/project/src/domain/tests.rs`:

```rust
#[test]
fn project_configuration_default_has_no_sdtmig() {
    let c = ProjectConfiguration::default();
    assert!(c.sdtmig.is_none());
}

#[test]
fn project_configuration_for_repository_carries_sdtmig() {
    let c = ProjectConfiguration::for_repository(
        None,
        vec![],
        Some(ModelVersion::for_repository(3, "2024-03-29".into())),
    );
    assert_eq!(
        c.sdtmig,
        Some(ModelVersion::for_repository(3, "2024-03-29".into()))
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p project --lib domain::tests::project_configuration_default_has_no_sdtmig`
Expected: compile error — `ProjectConfiguration` has no `sdtmig` field.

- [ ] **Step 3: Update `ProjectConfiguration`**

Edit `lib/crates/project/src/domain/project_configuration.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::model_version::ModelVersion;
use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;

/// Composite of the project's optional locale, its tag list, and
/// the SDTM-IG version it targets.
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
    /// Optional pointer to the SDTM-IG version this project targets.
    /// `None` means "no SDTM-IG version configured". The domain
    /// layer does not verify the referenced version exists in the
    /// domain-model crate's `sdtm_versions` table; whatever the
    /// caller sends is saved verbatim.
    pub sdtmig: Option<ModelVersion>,
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream
    /// test code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
        sdtmig: Option<ModelVersion>,
    ) -> Self {
        Self {
            language,
            tags,
            sdtmig,
        }
    }
}
```

- [ ] **Step 4: Cascade the constructor widening through the crate**

The `Project::for_repository` adapter constructor and the existing tests / mocks all call `ProjectConfiguration::for_repository` with two arguments. Update them to pass `None` as the third argument.

`lib/crates/project/src/domain/project.rs` — `Project::for_repository` already accepts the whole `ProjectConfiguration` by value, no change required.

Update all call sites of `ProjectConfiguration::for_repository` to add a third argument of `None`:
- `lib/crates/project/src/usecase/tests.rs` — three call sites in `create_project_with_configuration_succeeds`, `create_project_with_duplicate_tag_keys_succeeds`, `create_project_with_empty_tag_key_returns_validation_error`, `create_project_with_empty_tag_value_returns_validation_error`, `update_project_replaces_configuration_whole_list`, `update_project_leaves_configuration_unchanged_when_none`.
- `lib/crates/project/src/adapter/facade/in_memory/service.rs` — `configuration_data_to_domain` calls `ProjectConfiguration::for_repository(language, tags)`.
- `lib/crates/project/src/adapter/facade/in_memory/tests.rs` — `configuration_with_tags` builds a request-side DTO (no direct call); the facade tests use the request DTO so they do not need direct updates.
- `lib/crates/project/src/adapter/persistence/postgres/tests.rs` — two call sites in `project_row_converts_to_project_with_configuration`.
- `lib/crates/project/tests/public_api.rs` — `assert_project_configuration(ProjectConfiguration::default())` needs no change; the `_force_domain_inputs` already exists.
- `lib/crates/project/tests/integration_persistence.rs` — three call sites in `project_create_with_configuration_round_trip` and `project_update_replaces_configuration_whole_list`.

Each call site becomes:

```rust
ProjectConfiguration::for_repository(language, tags, None)
```

For `configuration_data_to_domain` in the facade, the new shape is:

```rust
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(
        d.language.map(api_language_to_domain),
        tag_data_vec_to_domain(d.tags),
        d.sdtmig.map(model_version_data_to_domain),
    )
}

fn model_version_data_to_domain(d: apis::project::ModelVersionData) -> ModelVersion {
    ModelVersion::for_repository(d.version_id, d.version_name)
}
```

(These adapter-side calls are written ahead of the apis side change in Task 5; the file will not compile until then. That is expected — run `cargo check -p project` after Task 5 completes.)

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p project --lib domain::tests::project_configuration`
Expected: PASS (both new tests + all existing green).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/project/src/domain/project_configuration.rs \
        lib/crates/project/src/usecase/tests.rs \
        lib/crates/project/src/adapter/facade/in_memory/service.rs \
        lib/crates/project/src/adapter/persistence/postgres/tests.rs \
        lib/crates/project/tests/integration_persistence.rs
git commit -m "feat(project): carry sdtmig on ProjectConfiguration"
```

---

## Task 3: Migration — backfill `sdtmig: null` on existing rows

**Files:**
- Create: `lib/crates/project/migrations/0003_add_project_configuration_sdtmig.sql`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs`

**Interfaces:**
- Consumes: existing `projects.configuration` JSONB column.
- Produces: every row's `configuration` has the key `sdtmig` with value `null`; the column default is updated to match.

- [ ] **Step 1: Write the failing migration-content test**

Append to `lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

```rust
#[test]
fn sdtmig_migration_adds_sdtmig_key_to_configuration() {
    let sql = load_migration("0003_add_project_configuration_sdtmig.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("'SDTMIG'"),
        "the 0003 migration must add the sdtmig key to the configuration JSONB; got:\n{sql}"
    );
    assert!(
        upper.contains("CONFIGURATION = CONFIGURATION ||"),
        "the 0003 migration must merge the sdtmig key into existing rows; got:\n{sql}"
    );
}

#[test]
fn sdtmig_migration_updates_default_to_include_sdtmig_null() {
    let sql = load_migration("0003_add_project_configuration_sdtmig.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DEFAULT"),
        "the 0003 migration must update the column default; got:\n{sql}"
    );
    assert!(
        upper.contains("'SDTMIG'"),
        "the column default must include sdtmig; got:\n{sql}"
    );
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p project --lib adapter::persistence::postgres::tests::sdtmig_migration`
Expected: panic — `load_migration("0003_…")` cannot open the file.

- [ ] **Step 3: Write the migration**

Create `lib/crates/project/migrations/0003_add_project_configuration_sdtmig.sql`:

```sql
-- 0003_add_project_configuration_sdtmig.sql
--
-- Adds the `sdtmig` key to the existing `projects.configuration`
-- JSONB column so the configuration object has a uniform shape:
--   {"language": null, "tags": [], "sdtmig": null}
--
-- Steps:
--   1. Backfill every existing row with `sdtmig: null` so the
--      object shape stays consistent for callers that read the
--      column before any application-side patch lands.
--   2. Update the column default to include `sdtmig: null` so new
--      inserts that omit the column land in a coherent state.
--
-- Mirrors the structure of `0002_add_project_configuration.sql`.
UPDATE projects
SET configuration = configuration || '{"sdtmig": null}'::jsonb;
ALTER TABLE projects
ALTER COLUMN configuration SET DEFAULT '{"language": null, "tags": [], "sdtmig": null}'::jsonb;
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p project --lib adapter::persistence::postgres::tests::sdtmig_migration`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/project/migrations/0003_add_project_configuration_sdtmig.sql \
        lib/crates/project/src/adapter/persistence/postgres/tests.rs
git commit -m "feat(project): migration 0003 adds sdtmig to configuration JSONB"
```

---

## Task 4: Usecase view mirrors `sdtmig`

**Files:**
- Modify: `lib/crates/project/src/usecase/views.rs`

**Interfaces:**
- Consumes: `ModelVersion` from Task 1, `ProjectConfiguration` from Task 2.
- Produces: `ProjectConfigurationView { language, tags, sdtmig }` and a `From<ProjectConfiguration>` impl that includes the new field.

- [ ] **Step 1: Write the failing test**

Append to `lib/crates/project/src/usecase/tests.rs` (at the bottom of the file, before `project_usecase_is_send_sync`):

```rust
#[tokio::test]
async fn create_project_with_sdtmig_succeeds() {
    let (_projects, _users, usecase) = make_usecase();
    let view = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![],
                Some(ModelVersion::for_repository(7, "2024-03-29".into())),
            )),
        })
        .await
        .expect("create");
    let sdtmig = view.configurations.sdtmig.expect("sdtmig present");
    assert_eq!(sdtmig.version_id, 7);
    assert_eq!(sdtmig.version_name, "2024-03-29");
}

#[tokio::test]
async fn create_project_with_empty_sdtmig_name_returns_validation_error() {
    let (_projects, _users, usecase) = make_usecase();
    let err = usecase
        .create_project(CreateProject {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configuration: Some(ProjectConfiguration::for_repository(
                None,
                vec![],
                Some(ModelVersion::for_repository(7, "   ".into())),
            )),
        })
        .await
        .expect_err("empty sdtmig name rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptySdtmigName)
    ));
}
```

The mock `MockProjectRepo` already calls `ProjectConfiguration::unwrap_or_default()` so the JSONB round-trip in the in-memory fake continues to work without changes (the default now has `sdtmig: None`).

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p project --lib usecase::tests::create_project_with_sdtmig_succeeds`
Expected: compile error — `ProjectConfigurationView` has no `sdtmig` field; `ModelVersion` is not yet in scope in the test.

Add `use crate::domain::ModelVersion;` to the `use crate::domain::{...}` line at the top of `lib/crates/project/src/usecase/tests.rs` to bring the new type into scope.

- [ ] **Step 3: Update `ProjectConfigurationView`**

Edit `lib/crates/project/src/usecase/views.rs`. Update the import line and the struct / `From` impl:

```rust
use crate::domain::{ModelVersion, Project, ProjectConfiguration, ProjectTag, UserSummary};

/// Server-side projection of a single `ModelVersion` pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelVersionView {
    pub version_id: i64,
    pub version_name: String,
}

impl From<ModelVersion> for ModelVersionView {
    fn from(v: ModelVersion) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Server-side projection of the project's configuration: an
/// optional locale, the tag list, and the optional SDTM-IG version
/// pointer. Mirrors the apis `ProjectConfigurationView` so the
/// facade `From` impl is a straight rename.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectConfigurationView {
    pub language: Option<crate::domain::ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
}

impl From<ProjectConfiguration> for ProjectConfigurationView {
    fn from(c: ProjectConfiguration) -> Self {
        Self {
            language: c.language,
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
        }
    }
}
```

- [ ] **Step 4: Wire usecase validation**

Edit `lib/crates/project/src/usecase/project_usecase.rs`. Update `validate_configuration_tags` to also validate the sdtmig name when present. Rename to `validate_configuration` for accuracy and update both call sites in `validate_create_project` / `validate_update_project`:

```rust
fn validate_configuration(c: &ProjectConfiguration) -> Result<(), UsecaseError> {
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
    if let Some(ref m) = c.sdtmig {
        match ModelVersion::new(m.version_id, m.version_name.clone()) {
            Ok(_) => {}
            Err(DomainError::EmptySdtmigName) => {
                return Err(UsecaseError::Validation(DomainError::EmptySdtmigName));
            }
            Err(other) => return Err(UsecaseError::Repository(other)),
        }
    }
    Ok(())
}
```

Update the import in `project_usecase.rs`:

```rust
use crate::domain::{
    DomainError, ModelVersion, Project, ProjectConfiguration, ProjectMember, ProjectNew,
    ProjectRepository, ProjectTag, ProjectUpdate, UserService, UserSummary,
};
```

Replace both call sites `validate_configuration_tags(c)?;` with `validate_configuration(c)?;`.

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p project --lib`
Expected: PASS (all unit tests, including the two new sdtmig tests).

- [ ] **Step 6: Commit**

```bash
git add lib/crates/project/src/usecase/views.rs \
        lib/crates/project/src/usecase/project_usecase.rs \
        lib/crates/project/src/usecase/tests.rs \
        lib/crates/project/src/usecase.rs
git commit -m "feat(project): usecase view and validation include sdtmig"
```

---

## Task 5: Apis request / view DTOs gain `sdtmig`

**Files:**
- Modify: `lib/crates/apis/src/project.rs`

**Interfaces:**
- Consumes: nothing new (domain model).
- Produces:
  - `pub struct ModelVersionData { pub version_id: i64, pub version_name: String }`
  - `pub struct ModelVersionView { pub version_id: i64, pub version_name: String }`
  - `ProjectConfigurationData { language, tags, sdtmig }`
  - `ProjectConfigurationView { language, tags, sdtmig }`
  - `CreateProjectRequest` / `UpdateProjectRequest` carry the new field via the configuration struct.

- [ ] **Step 1: Add the apis types and thread them through**

Edit `lib/crates/apis/src/project.rs`. Insert the new types and update the configuration + request structs:

```rust
/// Wire-shaped pointer to a `SdtmVersion` (SDTM Implementation
/// Guide version). The pair `(version_id, version_name)` is what
/// the client picked from `DomainModelService::list_versions`; the
/// project crate does not verify the row still exists at write
/// time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersionData {
    pub version_id: i64,
    pub version_name: String,
}

/// Server-side projection of [`ModelVersionData`]. Kept as a
/// distinct type so the request DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelVersionView {
    pub version_id: i64,
    pub version_name: String,
}
```

Update `ProjectConfigurationData`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
    pub sdtmig: Option<ModelVersionData>,
}
```

Update `ProjectConfigurationView`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
}
```

`CreateProjectRequest` and `UpdateProjectRequest` already carry the configuration as an `Option<ProjectConfigurationData>` — no change needed.

- [ ] **Step 2: Compile-check the apis crate**

Run: `cargo check -p apis`
Expected: PASS (downstream crates compile only after their adapters catch up).

- [ ] **Step 3: Commit**

```bash
git add lib/crates/apis/src/project.rs
git commit -m "feat(apis): project configuration carries sdtmig"
```

---

## Task 6: Facade bridges apis ↔ domain for `sdtmig`

**Files:**
- Modify: `lib/crates/project/src/adapter/facade/in_memory/service.rs`

**Interfaces:**
- Consumes: `ModelVersion` / `ModelVersionData` / `ModelVersionView` from earlier tasks.
- Produces: working `From` impls between apis configuration request / view and domain configuration.

- [ ] **Step 1: Add new imports and helper**

Edit `lib/crates/project/src/adapter/facade/in_memory/service.rs`. Update the apis import line to include `ModelVersionData` / `ModelVersionView`, the domain import to include `ModelVersion`, and the usecase import to include `ModelVersionView`. The view `From` impl needs a new arm:

```rust
use apis::project::{
    CreateProjectRequest, ModelVersionData, ModelVersionView as ApiModelVersionView,
    ProjectApiError, ProjectConfigurationData, ProjectLanguage as ApiProjectLanguage,
    ProjectMemberData, ProjectMemberView as ApiProjectMemberView, ProjectService, ProjectView,
    TagData, UpdateProjectRequest, UserSummaryView as ApiUserSummaryView,
};

use crate::domain::{
    ModelVersion, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectRepository,
    ProjectTag, UserService,
};
use crate::usecase::{
    CreateProject, ModelVersionView, ProjectConfigurationView, ProjectUsecase, UpdateProject,
    UserSummaryView as DomainUserSummaryView,
};
```

- [ ] **Step 2: Bridge the request-side helper**

Replace the `configuration_data_to_domain` body:

```rust
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(
        d.language.map(api_language_to_domain),
        tag_data_vec_to_domain(d.tags),
        d.sdtmig.map(model_version_data_to_domain),
    )
}

fn model_version_data_to_domain(d: ModelVersionData) -> ModelVersion {
    ModelVersion::for_repository(d.version_id, d.version_name)
}
```

- [ ] **Step 3: Bridge the view-side helper**

Update the existing `impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView`:

```rust
impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView {
    fn from(v: ProjectConfigurationView) -> Self {
        Self {
            language: v.language.map(domain_language_to_api),
            tags: v.tags.into_iter().map(Into::into).collect(),
            sdtmig: v.sdtmig.map(model_version_view_to_api),
        }
    }
}

fn model_version_view_to_api(v: ModelVersionView) -> ApiModelVersionView {
    ApiModelVersionView {
        version_id: v.version_id,
        version_name: v.version_name,
    }
}
```

- [ ] **Step 4: Run the facade tests**

Run: `cargo test -p project --lib adapter::facade::in_memory::tests`
Expected: PASS.

- [ ] **Step 5: Add a facade round-trip test**

Append to `lib/crates/project/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn create_project_with_sdtmig_round_trips_through_ap_view() {
    let service = make_service();
    let view = service
        .create_project(CreateProjectRequest {
            code: "proj1".into(),
            description: "".into(),
            members: None,
            unblind_members: None,
            configurations: Some(apis::project::ProjectConfigurationData {
                language: None,
                tags: vec![],
                sdtmig: Some(apis::project::ModelVersionData {
                    version_id: 7,
                    version_name: "2024-03-29".into(),
                }),
            }),
        })
        .await
        .expect("create");
    let sdtmig = view.configurations.sdtmig.expect("sdtmig present");
    assert_eq!(sdtmig.version_id, 7);
    assert_eq!(sdtmig.version_name, "2024-03-29");
}
```

- [ ] **Step 6: Commit**

```bash
git add lib/crates/project/src/adapter/facade/in_memory/service.rs \
        lib/crates/project/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(project): facade bridges sdtmig between apis and domain"
```

---

## Task 7: Server HTTP wire DTOs gain `sdtmig`

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

**Interfaces:**
- Consumes: apis `ModelVersionData` / `ModelVersionView` from Task 5.
- Produces: wire-level `ModelVersionRequest` / `ModelVersionResponse` and matching fields on the configuration / request / view DTOs.

- [ ] **Step 1: Add the wire DTOs and thread them**

In `apps/server/aegis-server/src/transport/http/dto.rs`, add a new DTO pair right above the project configuration section (around line 306):

```rust
/// Wire-level request body for the SDTM-IG version pointer a
/// project targets. Mirrors `apis::project::ModelVersionData`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

impl From<ModelVersionRequest> for apis::project::ModelVersionData {
    fn from(v: ModelVersionRequest) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Wire-level projection of the SDTM-IG version pointer. Mirrors
/// `apis::project::ModelVersionView`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ModelVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

impl From<apis::project::ModelVersionView> for ModelVersionResponse {
    fn from(v: apis::project::ModelVersionView) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}
```

Update `ProjectConfigurationDataRequest`:

```rust
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionRequest>,
}

impl From<ProjectConfigurationDataRequest> for apis::project::ProjectConfigurationData {
    fn from(c: ProjectConfigurationDataRequest) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c
                .tags
                .into_iter()
                .map(|t| apis::project::TagData {
                    key: t.key,
                    value: t.value,
                })
                .collect(),
            sdtmig: c.sdtmig.map(Into::into),
        }
    }
}
```

Update `ProjectConfigurationViewResponse`:

```rust
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionResponse>,
}

impl From<apis::project::ProjectConfigurationView> for ProjectConfigurationViewResponse {
    fn from(c: apis::project::ProjectConfigurationView) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
        }
    }
}
```

`CreateProjectRequest` and `UpdateProjectRequest` already carry the configuration as `Option<ProjectConfigurationDataRequest>`, so they pick up the new field through the inner struct — no direct edits needed.

- [ ] **Step 2: Register the new schemas in OpenAPI**

In `apps/server/aegis-server/src/transport/http/openapi.rs`, find the section that registers `CreateProjectRequest` / `UpdateProjectRequest` / `ProjectViewResponse` / `ProjectListResponse`. Add `ModelVersionRequest` and `ModelVersionResponse` alongside the existing project components:

```rust
        dto::ModelVersionRequest,
        dto::ModelVersionResponse,
```

…and in the `components(schemas(...))` block, add:

```rust
            "ModelVersionRequest",
            "ModelVersionResponse",
```

Mirroring how `TagDataRequest` / `TagViewResponse` are registered today.

- [ ] **Step 3: Compile-check the server**

Run: `cargo check -p aegis-server`
Expected: PASS.

- [ ] **Step 4: Run the server's project handler tests**

Run: `cargo test -p aegis-server --lib transport::http::project::handlers`
Expected: PASS (existing tests stay green because the new fields are optional).

- [ ] **Step 5: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs \
        apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(server): project configuration wire DTOs carry sdtmig"
```

---

## Task 8: Public-API compile test updates

**Files:**
- Modify: `lib/crates/project/tests/public_api.rs`

**Interfaces:**
- Consumes: apis / domain additions from prior tasks.
- Produces: green compile-only test that references the new fields on `CreateProject` / `UpdateProject` / `CreateProjectRequest` / `UpdateProjectRequest`.

- [ ] **Step 1: Add coverage for the new field on each command / request**

In `lib/crates/project/tests/public_api.rs`, update the two shape-pinning tests:

```rust
#[test]
fn usecase_commands_have_expected_field_shape() {
    let _create_project = CreateProject {
        code: "proj1".into(),
        description: "".into(),
        members: None,
        unblind_members: None,
        configuration: Some(ProjectConfiguration::for_repository(
            None,
            vec![],
            Some(project::ModelVersion::for_repository(7, "2024-03-29".into())),
        )),
    };

    let _update_project = UpdateProject {
        id: 1,
        code: None,
        description: None,
        active: None,
        members: None,
        unblind_members: None,
        configuration: Some(ProjectConfiguration::for_repository(
            None,
            vec![],
            Some(project::ModelVersion::for_repository(7, "2024-03-29".into())),
        )),
    };
}
```

Extend the apis request test to confirm the wire shape compiles with the new field too:

```rust
#[test]
fn api_requests_have_expected_field_shape() {
    let _create_project = CreateProjectRequest {
        code: "proj1".into(),
        description: "".into(),
        members: None,
        unblind_members: None,
        configurations: Some(ProjectConfigurationData {
            language: None,
            tags: vec![],
            sdtmig: Some(apis::project::ModelVersionData {
                version_id: 7,
                version_name: "2024-03-29".into(),
            }),
        }),
    };

    let _update_project = UpdateProjectRequest {
        id: 1,
        code: None,
        description: None,
        active: None,
        members: None,
        unblind_members: None,
        configurations: Some(ProjectConfigurationData {
            language: None,
            tags: vec![],
            sdtmig: Some(apis::project::ModelVersionData {
                version_id: 7,
                version_name: "2024-03-29".into(),
            }),
        }),
    };
}
```

The `apis_view_dtos_are_nameable` test should also pin the apis view DTOs:

```rust
assert_configuration_view(ProjectConfigurationView {
    language: Some(apis::project::ProjectLanguage::English),
    tags: vec![],
    sdtmig: None,
});
```

Add `ModelVersion` to the imports at the top of the file:

```rust
use project::{
    CreateProject, DomainError, ModelVersion, ProjectConfiguration, ProjectLanguage, ProjectMember,
    ProjectNew, ProjectRepo, ProjectRepository, ProjectServiceImpl, ProjectTag, ProjectUpdate,
    ProjectUsecaseConfig, ProjectView, RoleType, TeamType, UpdateProject, UsecaseError, UserService,
    UserServiceImpl, UserSummary, UserSummaryView,
};
```

(The type re-export lives at the crate root once `domain.rs` re-exports it — Task 1.)

- [ ] **Step 2: Run the public-API tests**

Run: `cargo test -p project --test public_api`
Expected: PASS.

- [ ] **Step 3: Commit**

```bash
git add lib/crates/project/tests/public_api.rs
git commit -m "test(project): public_api pins sdtmig field shape"
```

---

## Task 9: Verification gate

- [ ] **Step 1: Format**

```bash
cargo fmt --all -- --check
```
Expected: clean.

- [ ] **Step 2: Clippy**

```bash
cargo clippy -p project --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
```
Expected: clean.

- [ ] **Step 3: Tests**

```bash
cargo test -p project
cargo test -p aegis-server
```
Expected: green.

- [ ] **Step 4: Docs**

```bash
cargo doc -p project --no-deps
cargo doc -p apis --no-deps
```
Expected: builds.

- [ ] **Step 5: Live-DB (only when `AEGIS_DATABASE_URL` is set)**

```bash
cargo test -p project -- --ignored --test-threads=1
```
Expected: green (drops `projects` / `project_members` / `_sqlx_migrations`, runs migration 0003 alongside 0001 + 0002, then exercises the existing integration tests). If you add a new round-trip test for sdtmig against the live DB, follow the same `with_pool` pattern as the existing tests in `tests/integration_persistence.rs`.

- [ ] **Step 6: Final commit (if any fmt/clippy cleanups were needed)**

```bash
git add -A
git commit -m "chore(project): clippy / fmt cleanups after sdtmig feature"
```

---

## Self-Review

**1. Spec coverage**
- "add a new optional field named `sdtmig` with type `ModelVersion`" → Tasks 2, 4, 5, 7 thread `Option<ModelVersion>` end-to-end.
- "`ModelVersion { version_id: i64, version_name: String }`" → Task 1 defines the value object with exactly those fields; Tasks 5 and 7 mirror it on both wire DTOs.
- "do NOT check that the record exists, just save what user sends" → Task 1's `ModelVersion::new` only validates the non-empty name (a syntactic check, not a cross-record lookup). The usecase `validate_configuration` (Task 4) only calls `ModelVersion::new`, never reaches into the domain-model crate.
- "Update related code files affected by this modification, EXCEPT the desktop app" → Tasks 2-8 cover the project crate, the apis crate, and the server. The plan deliberately does NOT touch `apps/desktop/aegis-desktop`.
- "version_id / version_name will come from `domain_model::SdtmVersion`" → documented in the `ModelVersion` doc-comment as informational only; no compile-time dep is added.

**2. Placeholder scan**
- No "TBD", "TODO", or "fill in details" — every step ships concrete code.
- Every "validate X" step shows the actual `assert_eq!` / `matches!` body.
- Migration content is verbatim in Task 3.
- All field-shape tests pin the exact field list (Tasks 4, 6, 8).

**3. Type consistency**
- `ModelVersion::for_repository(version_id: i64, version_name: String) -> Self` (Task 1) → used by every widening call site in Task 2.
- `ProjectConfiguration::for_repository(language, tags, sdtmig)` (Task 2) → consistent across domain tests, facade tests, row tests, integration tests.
- `ModelVersionData { version_id, version_name }` (Task 5) ↔ `ModelVersion { version_id, version_name }` (Task 1) — same field order and types.
- `ModelVersionView` (Task 5) ↔ `ModelVersionView` (Task 4) ↔ `ModelVersionResponse` (Task 7) — same field order and types.
- `sdtmig: Option<…>` everywhere; never raw `ModelVersion`.
