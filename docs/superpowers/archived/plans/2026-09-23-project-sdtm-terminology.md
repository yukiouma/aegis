# Project Configuration — Optional SDTM Terminology Version Pointer

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a fourth optional control to a project's stored `configurations` JSONB column — `sdtm_terminology` — that points at a row in `terminology::terminology_versions`. Distinct from the existing `sdtmig` pointer.

**Architecture:** Thread a new `TerminologyVersionData { version_id, version_name }` pointer through the project domain → apis port → server wire DTOs → desktop HTTP mirror → TS types → ConfigurationGeneralSection picker. JSONB column gains a `sdtm_terminology` key (nullable). Picker filters the terminology-versions list to `kind === "sdtm"`.

**Tech Stack:** Rust (axum + sqlx + serde + thiserror), React + MUI + TanStack Query, vitest.

---

## File Structure

### New
- `lib/crates/project/src/domain/terminology_version.rs` — domain pointer struct + validators.
- `lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql` — backfill + default migration.

### Modified (Rust)
- `lib/crates/project/src/domain.rs` — re-export.
- `lib/crates/project/src/domain/error.rs` — `EmptySdtmTerminologyName` variant.
- `lib/crates/project/src/domain/project_configuration.rs` — new field + `for_repository`.
- `lib/crates/project/src/domain/tests.rs` — new tests for the pointer + configuration carrying it.
- `lib/crates/project/src/usecase/project_usecase.rs` — validation arm.
- `lib/crates/project/src/usecase/views.rs` — domain-side `ProjectConfigurationView` + `From` impl.
- `lib/crates/project/src/usecase/tests.rs` — round-trip + validation tests.
- `lib/crates/project/src/adapter/facade/in_memory/service.rs` — data↔domain↔apis `From` impls.
- `lib/crates/project/src/adapter/facade/in_memory/tests.rs` — facade round-trip test.
- `lib/crates/project/src/adapter/persistence/postgres/tests.rs` — migration shape tests.
- `lib/crates/apis/src/project.rs` — port DTOs.
- `apps/server/aegis-server/src/transport/http/dto.rs` — wire DTOs + `From` impls.
- `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs` — Rust HTTP mirror + round-trip unit test.

### Modified (TS)
- `apps/desktop/aegis-desktop/src/shared/api/types.ts` — `ProjectConfigurationTerminology` + new field on `ProjectConfiguration`.
- `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx` — new picker, state, dirty flag, save body, reseed.
- `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx` — focused cases.
- `lib/packages/ui/src/i18n/locales/en.ts` — two keys.
- `lib/packages/ui/src/i18n/locales/zhCN.ts` — two keys.

---

## Task 1: Domain pointer type + DomainError variant

**Files:**
- Create: `lib/crates/project/src/domain/terminology_version.rs`
- Modify: `lib/crates/project/src/domain.rs`
- Modify: `lib/crates/project/src/domain/error.rs`
- Test: `lib/crates/project/src/domain/tests.rs`

- [ ] **Step 1: Write the failing domain tests**

Append to `lib/crates/project/src/domain/tests.rs`:

```rust
#[test]
fn terminology_version_data_new_rejects_empty_name() {
    let err = TerminologyVersionData::new(1, "".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmTerminologyName));
}

#[test]
fn terminology_version_data_new_rejects_whitespace_name() {
    let err = TerminologyVersionData::new(1, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptySdtmTerminologyName));
}

#[test]
fn terminology_version_data_new_accepts_valid_input() {
    let v = TerminologyVersionData::new(7, "2024-03-29".into()).unwrap();
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn terminology_version_data_for_repository_carries_fields() {
    let v = TerminologyVersionData::for_repository(7, "2024-03-29".into());
    assert_eq!(v.version_id, 7);
    assert_eq!(v.version_name, "2024-03-29");
}

#[test]
fn project_configuration_default_has_no_sdtm_terminology() {
    let c = ProjectConfiguration::default();
    assert!(c.sdtm_terminology.is_none());
}

#[test]
fn project_configuration_for_repository_carries_sdtm_terminology() {
    let c = ProjectConfiguration::for_repository(
        None,
        vec![],
        None,
        Some(TerminologyVersionData::for_repository(7, "2024-03-29".into())),
    );
    assert_eq!(
        c.sdtm_terminology,
        Some(TerminologyVersionData::for_repository(7, "2024-03-29".into()))
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p project --lib domain::tests::terminology_version_data_new_rejects_empty_name`
Expected: compile error — `TerminologyVersionData`, `EmptySdtmTerminologyName`, and `ProjectConfiguration::for_repository` 4-arg form do not exist yet.

- [ ] **Step 3: Add the variant to DomainError**

Edit `lib/crates/project/src/domain/error.rs` — add the line below the existing `EmptySdtmigName`:

```rust
    #[error("sdtm terminology version name must not be empty")]
    EmptySdtmTerminologyName,
```

- [ ] **Step 4: Create the pointer module**

Create `lib/crates/project/src/domain/terminology_version.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// Pointer to a `TerminologyVersion` a project targets. Mirrors
/// `ModelVersion` shape-wise but carries a distinct Rust type so the
/// wire field name (`sdtm_terminology`) can diverge from `sdtmig`
/// without alias gymnastics, and so the two pointer types can evolve
/// validation independently.
///
/// `version_id` is the surrogate key from
/// `terminology::TerminologyVersion::id`; `version_name` is the
/// human-readable workbook suffix (e.g. "2024-03-29"). The domain
/// layer does NOT verify the row still exists at write time — the
/// trust contract matches `ModelVersion`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionData {
    pub version_id: i64,
    pub version_name: String,
}

impl TerminologyVersionData {
    /// Validating constructor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs). Rejects
    /// empty / whitespace `version_name`.
    pub fn new(version_id: i64, version_name: String) -> Result<Self, DomainError> {
        if version_name.trim().is_empty() {
            return Err(DomainError::EmptySdtmTerminologyName);
        }
        Ok(Self {
            version_id,
            version_name,
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from the JSONB column, and for downstream
    /// test / integration code that needs to construct a pointer
    /// from a trusted source.
    #[allow(dead_code)]
    pub fn for_repository(version_id: i64, version_name: String) -> Self {
        Self {
            version_id,
            version_name,
        }
    }
}
```

- [ ] **Step 5: Wire the module into domain.rs**

Edit `lib/crates/project/src/domain.rs`:

```rust
mod error;
mod model_version;
mod project;
mod project_configuration;
mod project_language;
mod project_member;
mod project_tag;
mod team_role;
mod terminology_version;
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
pub use terminology_version::TerminologyVersionData;
pub use user::{UserService, UserSummary};
```

- [ ] **Step 6: Add the field to ProjectConfiguration**

Edit `lib/crates/project/src/domain/project_configuration.rs`:

```rust
use serde::{Deserialize, Serialize};

use super::model_version::ModelVersion;
use super::project_language::ProjectLanguage;
use super::project_tag::ProjectTag;
use super::terminology_version::TerminologyVersionData;

/// Composite of the project's optional locale, its tag list, the
/// SDTM-IG version it targets, and the SDTM terminology release it
/// pins. The struct owns no domain rule beyond composition: the inner
/// `ProjectTag` already enforces non-empty key / value when tags
/// arrive via the validating constructor on the wire, so no separate
/// `ProjectConfiguration::new` is needed. Constructors skip
/// validation because the data is either trusted (adapter
/// materialising from JSONB) or already validated (usecase after
/// `ProjectTag::new` / `ModelVersion::new` /
/// `TerminologyVersionData::new`).
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
    /// Optional pointer to the SDTM controlled-terminology release
    /// this project pins. Same trust contract as `sdtmig` — the
    /// domain does not verify the referenced version exists in
    /// `terminology::terminology_versions` at write time.
    pub sdtm_terminology: Option<TerminologyVersionData>,
}

impl ProjectConfiguration {
    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising from the JSONB column, and for downstream test
    /// code that needs to construct from a trusted source.
    pub fn for_repository(
        language: Option<ProjectLanguage>,
        tags: Vec<ProjectTag>,
        sdtmig: Option<ModelVersion>,
        sdtm_terminology: Option<TerminologyVersionData>,
    ) -> Self {
        Self {
            language,
            tags,
            sdtmig,
            sdtm_terminology,
        }
    }
}
```

- [ ] **Step 7: Update existing `for_repository` callers**

`ProjectConfiguration::for_repository` now takes 4 args. Existing
callers in `lib/crates/project/src/usecase/project_usecase.rs`
(passes through), `lib/crates/project/src/usecase/tests.rs`
(`for_repository(None, vec![], Some(...))` for sdtmig),
`lib/crates/project/src/domain/tests.rs`
(`for_repository(Some(...), vec![...], None)`),
`lib/crates/project/src/adapter/facade/in_memory/tests.rs`
(none — only constructs `apis::project::ProjectConfigurationData`),
and `lib/crates/project/src/adapter/persistence/postgres/tests.rs`
(`for_repository(None, vec![], None)`) need a 4th `None` argument.

Edit each call site by appending `, None` before the closing `)`:

- `lib/crates/project/src/usecase/tests.rs:380` →
  `ProjectConfiguration::for_repository(None, vec![], Some(ModelVersion::for_repository(7, "2024-03-29".into())), None)`
- `lib/crates/project/src/usecase/tests.rs:399` →
  same pattern with `Some(ModelVersion::for_repository(7, "   ".into())), None)`
- `lib/crates/project/src/domain/tests.rs:171-179` → `for_repository(Some(ProjectLanguage::SimplifiedChinese), vec![ProjectTag::for_repository("k".into(), "v".into())], None, None)`
- `lib/crates/project/src/domain/tests.rs:189-198` → `for_repository(None, vec![], Some(ModelVersion::for_repository(3, "2024-03-29".into())), None)`
- `lib/crates/project/src/adapter/persistence/postgres/tests.rs` —
  search for `ProjectConfiguration::for_repository(` and append `, None`
  to each call. Verify with:

Run: `grep -rn "ProjectConfiguration::for_repository" lib/crates/project/`

- [ ] **Step 8: Run tests to verify they pass**

Run: `cargo test -p project --lib domain::tests`
Expected: all 6 new tests pass; existing `project_configuration_*` tests pass.

- [ ] **Step 9: Commit**

```bash
git add lib/crates/project/src/domain/terminology_version.rs \
        lib/crates/project/src/domain.rs \
        lib/crates/project/src/domain/error.rs \
        lib/crates/project/src/domain/project_configuration.rs \
        lib/crates/project/src/domain/tests.rs \
        lib/crates/project/src/usecase/tests.rs \
        lib/crates/project/src/adapter/persistence/postgres/tests.rs
git commit -m "feat(project): add sdtm_terminology pointer type

Adds a TerminologyVersionData struct alongside the existing
ModelVersion pointer so the project configuration can pin a SDTM
controlled-terminology release distinct from the SDTM-IG version it
targets. The new field is optional; ProjectConfiguration::for_repository
now takes a fourth argument.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 2: Validation arm in the usecase

**Files:**
- Modify: `lib/crates/project/src/usecase/project_usecase.rs`
- Test: `lib/crates/project/src/usecase/tests.rs`

- [ ] **Step 1: Write the failing usecase tests**

Append to `lib/crates/project/src/usecase/tests.rs`:

```rust
#[tokio::test]
async fn create_project_with_sdtm_terminology_succeeds() {
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
                None,
                Some(TerminologyVersionData::for_repository(7, "2024-03-29".into())),
            )),
        })
        .await
        .expect("create");
    let term = view
        .configurations
        .sdtm_terminology
        .expect("sdtm_terminology present");
    assert_eq!(term.version_id, 7);
    assert_eq!(term.version_name, "2024-03-29");
}

#[tokio::test]
async fn create_project_with_empty_sdtm_terminology_name_returns_validation_error() {
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
                None,
                Some(TerminologyVersionData::for_repository(7, "   ".into())),
            )),
        })
        .await
        .expect_err("empty sdtm_terminology name rejected");
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptySdtmTerminologyName)
    ));
}
```

Update the import at the top of the file to add `TerminologyVersionData`:

```rust
use crate::domain::{
    DomainError, ModelVersion, Project, ProjectConfiguration, ProjectMember, ProjectNew,
    ProjectRepository, ProjectTag, ProjectUpdate, TerminologyVersionData, UserService, UserSummary,
};
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p project --lib usecase::tests::create_project_with_sdtm_terminology_succeeds`
Expected: compile error — `ProjectConfigurationView` (domain-side) has no `sdtm_terminology` field, and `validate_configuration` does not check the new field.

- [ ] **Step 3: Add the validation arm**

Edit `lib/crates/project/src/usecase/project_usecase.rs`:

```rust
use crate::domain::{
    DomainError, ModelVersion, Project, ProjectConfiguration, ProjectMember, ProjectNew,
    ProjectRepository, ProjectTag, ProjectUpdate, TerminologyVersionData, UserService, UserSummary,
};
```

(`TerminologyVersionData` joins the existing imports.)

In `validate_configuration`, after the existing `if let Some(ref m) = c.sdtmig` block, append:

```rust
    if let Some(ref t) = c.sdtm_terminology {
        match TerminologyVersionData::new(t.version_id, t.version_name.clone()) {
            Ok(_) => {}
            Err(DomainError::EmptySdtmTerminologyName) => {
                return Err(UsecaseError::Validation(DomainError::EmptySdtmTerminologyName));
            }
            Err(other) => return Err(UsecaseError::Repository(other)),
        }
    }
```

- [ ] **Step 4: Extend the domain-side ProjectConfigurationView**

Edit `lib/crates/project/src/usecase/views.rs`:

```rust
use crate::domain::{
    ModelVersion, Project, ProjectConfiguration, ProjectTag, TerminologyVersionData, UserSummary,
};
```

(Add `TerminologyVersionData` to the imports.)

Add a sibling `TerminologyVersionView`:

```rust
/// Server-side projection of a single `TerminologyVersionData`
/// pointer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyVersionView {
    pub version_id: i64,
    pub version_name: String,
}

impl From<TerminologyVersionData> for TerminologyVersionView {
    fn from(v: TerminologyVersionData) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}
```

Extend `ProjectConfigurationView` + `From` impl:

```rust
/// Server-side projection of the project's configuration: an
/// optional locale, the tag list, the optional SDTM-IG version
/// pointer, and the optional SDTM terminology pointer. Mirrors the
/// apis `ProjectConfigurationView` so the facade `From` impl is a
/// straight rename.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectConfigurationView {
    pub language: Option<crate::domain::ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
    pub sdtm_terminology: Option<TerminologyVersionView>,
}

impl From<ProjectConfiguration> for ProjectConfigurationView {
    fn from(c: ProjectConfiguration) -> Self {
        Self {
            language: c.language,
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
            sdtm_terminology: c.sdtm_terminology.map(Into::into),
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test -p project --lib usecase::tests::create_project_with_sdtm_terminology_succeeds usecase::tests::create_project_with_empty_sdtm_terminology_name_returns_validation_error`
Expected: PASS for both.

- [ ] **Step 6: Run the full project test suite**

Run: `cargo test -p project --lib`
Expected: PASS (no regressions).

- [ ] **Step 7: Commit**

```bash
git add lib/crates/project/src/usecase/project_usecase.rs \
        lib/crates/project/src/usecase/views.rs \
        lib/crates/project/src/usecase/tests.rs
git commit -m "feat(project): validate and project sdtm_terminology pointer

validate_configuration rejects empty/whitespace version_name via
DomainError::EmptySdtmTerminologyName. The domain-side
ProjectConfigurationView grows a sdtm_terminology field, threaded
through the From<ProjectConfiguration> impl.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 3: Migration backfill + shape test

**Files:**
- Create: `lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql`
- Modify: `lib/crates/project/src/adapter/persistence/postgres/tests.rs`

- [ ] **Step 1: Write the failing migration shape tests**

Append to `lib/crates/project/src/adapter/persistence/postgres/tests.rs`:

```rust
#[test]
fn sdtm_terminology_migration_adds_sdtm_terminology_key_to_configuration() {
    let sql = load_migration("0004_add_project_configuration_sdtm_terminology.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("SDTM_TERMINOLOGY"),
        "the 0004 migration must add the sdtm_terminology key to the configuration JSONB; got:\n{sql}"
    );
    assert!(
        upper.contains("CONFIGURATION = CONFIGURATION ||"),
        "the 0004 migration must merge the sdtm_terminology key into existing rows; got:\n{sql}"
    );
}

#[test]
fn sdtm_terminology_migration_updates_default_to_include_sdtm_terminology_null() {
    let sql = load_migration("0004_add_project_configuration_sdtm_terminology.sql");
    let upper = sql.to_uppercase();
    assert!(
        upper.contains("DEFAULT"),
        "the 0004 migration must update the column default; got:\n{sql}"
    );
    assert!(
        upper.contains("SDTM_TERMINOLOGY"),
        "the column default must include sdtm_terminology; got:\n{sql}"
    );
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test -p project --lib adapter::persistence::postgres::tests::sdtm_terminology_migration_adds_sdtm_terminology_key_to_configuration`
Expected: FAIL — `migration file 0004_add_project_configuration_sdtm_terminology.sql must exist`.

- [ ] **Step 3: Create the migration**

Create `lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql`:

```sql
-- 0004_add_project_configuration_sdtm_terminology.sql
--
-- Adds the `sdtm_terminology` key to the existing
-- `projects.configuration` JSONB column so the configuration
-- object has a uniform shape:
--   {"language": null, "tags": [], "sdtmig": null, "sdtm_terminology": null}
--
-- Steps:
--   1. Backfill every existing row with `sdtm_terminology: null` so
--      the object shape stays consistent for callers that read the
--      column before any application-side patch lands.
--   2. Update the column default to include
--      `sdtm_terminology: null` so new inserts that omit the column
--      land in a coherent state.
--
-- Mirrors the structure of `0003_add_project_configuration_sdtmig.sql`.
UPDATE projects
SET configuration = configuration || '{"sdtm_terminology": null}'::jsonb;
ALTER TABLE projects
ALTER COLUMN configuration SET DEFAULT '{"language": null, "tags": [], "sdtmig": null, "sdtm_terminology": null}'::jsonb;
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p project --lib adapter::persistence::postgres::tests::sdtm_terminology`
Expected: PASS for both new tests.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/project/migrations/0004_add_project_configuration_sdtm_terminology.sql \
        lib/crates/project/src/adapter/persistence/postgres/tests.rs
git commit -m "feat(project): add 0004 migration for sdtm_terminology JSONB key

Backfills existing rows with sdtm_terminology: null and updates the
column default to include the new key.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 4: Apis port DTOs

**Files:**
- Modify: `lib/crates/apis/src/project.rs`

- [ ] **Step 1: Add the new structs and fields**

Edit `lib/crates/apis/src/project.rs` — after the existing
`ModelVersionView` definition, append:

```rust
/// Wire-shaped pointer to a `TerminologyVersion` (controlled
/// terminology release) a project pins. Mirrors the shape of
/// [`ModelVersionData`] but lives at a distinct type so the wire
/// field name (`sdtm_terminology`) is free to evolve independently.
/// `version_id` is the surrogate key from
/// `terminology::TerminologyVersion::id`; `version_name` is the
/// workbook suffix (e.g. `"2024-03-29"`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionData {
    pub version_id: i64,
    pub version_name: String,
}

/// Server-side projection of [`TerminologyVersionData`]. Kept as a
/// distinct type so the request DTO can diverge later without
/// breaking the projection contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TerminologyVersionView {
    pub version_id: i64,
    pub version_name: String,
}
```

Replace `ProjectConfigurationData`:

```rust
/// Request-side configuration: optional locale plus the tag list
/// plus the optional SDTM-IG version pointer plus the optional
/// SDTM terminology pointer. Carries `TagData` because it travels
/// into the backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationData {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagData>,
    pub sdtmig: Option<ModelVersionData>,
    pub sdtm_terminology: Option<TerminologyVersionData>,
}
```

Replace `ProjectConfigurationView`:

```rust
/// Server-side projection of the configuration. Carries `TagView`
/// because it travels back to clients.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectConfigurationView {
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagView>,
    pub sdtmig: Option<ModelVersionView>,
    pub sdtm_terminology: Option<TerminologyVersionView>,
}
```

- [ ] **Step 2: Build the apis crate**

Run: `cargo check -p apis`
Expected: FAIL — downstream consumers (`lib/crates/project`'s facade) construct `ProjectConfigurationData` without the new field.

- [ ] **Step 3: Update the facade's data↔domain↔apis plumbing**

Edit `lib/crates/project/src/adapter/facade/in_memory/service.rs`:

Update imports:

```rust
use crate::domain::{
    ModelVersion, ProjectConfiguration, ProjectLanguage, ProjectMember, ProjectRepository, ProjectTag,
    TerminologyVersionData, UserService,
};
use crate::usecase::{
    CreateProject, ModelVersionView, ProjectConfigurationView, ProjectUsecase,
    TerminologyVersionView, UpdateProject, UserSummaryView as DomainUserSummaryView,
};
```

(Add `TerminologyVersionData` to the `crate::domain::` group and
`TerminologyVersionView` to the `crate::usecase::` group.)

Replace `configuration_data_to_domain`:

```rust
fn configuration_data_to_domain(d: ProjectConfigurationData) -> ProjectConfiguration {
    ProjectConfiguration::for_repository(
        d.language.map(api_language_to_domain),
        tag_data_vec_to_domain(d.tags),
        d.sdtmig.map(model_version_data_to_domain),
        d.sdtm_terminology.map(terminology_version_data_to_domain),
    )
}

fn terminology_version_data_to_domain(d: apis::project::TerminologyVersionData) -> TerminologyVersionData {
    TerminologyVersionData::for_repository(d.version_id, d.version_name)
}
```

Replace the `From<ProjectConfigurationView> for apis::project::ProjectConfigurationView` block:

```rust
impl From<ProjectConfigurationView> for apis::project::ProjectConfigurationView {
    fn from(v: ProjectConfigurationView) -> Self {
        Self {
            language: v.language.map(domain_language_to_api),
            tags: v.tags.into_iter().map(Into::into).collect(),
            sdtmig: v.sdtmig.map(model_version_view_to_api),
            sdtm_terminology: v.sdtm_terminology.map(terminology_version_view_to_api),
        }
    }
}

fn terminology_version_view_to_api(v: TerminologyVersionView) -> apis::project::TerminologyVersionView {
    apis::project::TerminologyVersionView {
        version_id: v.version_id,
        version_name: v.version_name,
    }
}
```

- [ ] **Step 4: Update the facade test helpers**

Edit `lib/crates/project/src/adapter/facade/in_memory/tests.rs`:

Add `TerminologyVersionData` to the apis `use`:

```rust
use apis::project::{
    CreateProjectRequest, ProjectApiError, ProjectConfigurationData, ProjectService, TagData,
    TerminologyVersionData, UpdateProjectRequest,
};
```

Update `configuration_with_tags`:

```rust
fn configuration_with_tags(tags: Vec<TagData>) -> ProjectConfigurationData {
    ProjectConfigurationData {
        language: None,
        tags,
        sdtmig: None,
        sdtm_terminology: None,
    }
}
```

- [ ] **Step 5: Append a facade round-trip test**

Append to `lib/crates/project/src/adapter/facade/in_memory/tests.rs`:

```rust
#[tokio::test]
async fn create_project_with_sdtm_terminology_round_trips_through_ap_view() {
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
                sdtmig: None,
                sdtm_terminology: Some(TerminologyVersionData {
                    version_id: 7,
                    version_name: "2024-03-29".into(),
                }),
            }),
        })
        .await
        .expect("create");
    let term = view
        .configurations
        .sdtm_terminology
        .expect("sdtm_terminology present");
    assert_eq!(term.version_id, 7);
    assert_eq!(term.version_name, "2024-03-29");
}
```

- [ ] **Step 6: Run the project + apis check**

Run: `cargo check -p apis -p project`
Expected: SUCCESS.

- [ ] **Step 7: Run the project test suite**

Run: `cargo test -p project --lib`
Expected: PASS — new facade test plus all existing tests.

- [ ] **Step 8: Commit**

```bash
git add lib/crates/apis/src/project.rs \
        lib/crates/project/src/adapter/facade/in_memory/service.rs \
        lib/crates/project/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(apis): thread sdtm_terminology through ProjectService port

Adds TerminologyVersionData / TerminologyVersionView to the apis
project port and a fourth sdtm_terminology field on both
ProjectConfigurationData and ProjectConfigurationView. The project
crate's facade is updated to translate between the apis types and
the domain/usecase views.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 5: Server wire DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`

- [ ] **Step 1: Add the wire request/response pair**

After the existing `ModelVersionRequest` / `ModelVersionResponse`
impls in `dto.rs`, append:

```rust
/// Wire-level request body for the SDTM controlled-terminology
/// version pointer a project pins. Mirrors
/// `apis::project::TerminologyVersionData`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

impl From<TerminologyVersionRequest> for apis::project::TerminologyVersionData {
    fn from(v: TerminologyVersionRequest) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}

/// Wire-level projection of the SDTM controlled-terminology version
/// pointer. Mirrors `apis::project::TerminologyVersionView`.
#[derive(Serialize, Deserialize, ToSchema, Clone)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}

impl From<apis::project::TerminologyVersionView> for TerminologyVersionResponse {
    fn from(v: apis::project::TerminologyVersionView) -> Self {
        Self {
            version_id: v.version_id,
            version_name: v.version_name,
        }
    }
}
```

- [ ] **Step 2: Extend the request configuration shape**

Replace `ProjectConfigurationDataRequest`:

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionRequest>,
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
            sdtm_terminology: c.sdtm_terminology.map(Into::into),
        }
    }
}
```

- [ ] **Step 3: Extend the response configuration shape**

Replace `ProjectConfigurationViewResponse`:

```rust
#[derive(Serialize, Deserialize, ToSchema, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionResponse>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionResponse>,
}

impl From<apis::project::ProjectConfigurationView> for ProjectConfigurationViewResponse {
    fn from(c: apis::project::ProjectConfigurationView) -> Self {
        Self {
            language: c.language.map(Into::into),
            tags: c.tags.into_iter().map(Into::into).collect(),
            sdtmig: c.sdtmig.map(Into::into),
            sdtm_terminology: c.sdtm_terminology.map(Into::into),
        }
    }
}
```

- [ ] **Step 4: Verify the server compiles**

Run: `cargo check -p aegis-server`
Expected: SUCCESS.

- [ ] **Step 5: Run the project-handler tests**

Run: `cargo test -p aegis-server --lib transport::http::project::handlers::tests`
Expected: PASS — the existing
`create_project_root_returns_201` etc. all carry
`{ language, tags, sdtmig }` in `configurations`, and the `From`
impl now adds `sdtm_terminology: None` automatically.

- [ ] **Step 6: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs
git commit -m "feat(server): wire DTOs for sdtm_terminology pointer

Adds TerminologyVersionRequest/Response to the server wire layer
and threads a fourth sdtm_terminology field through both the request
and response ProjectConfiguration shapes. skip_serializing_if on
None keeps absent selections from hitting the wire.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 6: Desktop HTTP mirror + round-trip unit test

**Files:**
- Modify: `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`

- [ ] **Step 1: Add the mirror structs and fields**

Edit `apps/desktop/aegis-desktop/src-tauri/src/http/project.rs`:

After the existing `ModelVersionResponse` definition, append:

```rust
/// Wire-level request body for the SDTM controlled-terminology
/// version pointer a project pins. Mirrors the server's
/// `apps/server/aegis-server/src/transport/http/dto.rs::TerminologyVersionRequest`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionRequest {
    pub version_id: i64,
    pub version_name: String,
}

/// Wire-level projection of the SDTM controlled-terminology version
/// pointer. Mirrors the server's
/// `apps/server/aegis-server/src/transport/http/dto.rs::TerminologyVersionResponse`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminologyVersionResponse {
    pub version_id: i64,
    pub version_name: String,
}
```

Replace `ProjectConfigurationDataRequest`:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationDataRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<TagDataRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtmig: Option<ModelVersionRequest>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sdtm_terminology: Option<TerminologyVersionRequest>,
}
```

Replace `ProjectConfigurationViewResponse`:

```rust
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectConfigurationViewResponse {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<ProjectLanguage>,
    pub tags: Vec<TagViewResponse>,
    pub sdtmig: Option<ModelVersionResponse>,
    pub sdtm_terminology: Option<TerminologyVersionResponse>,
}
```

- [ ] **Step 2: Append the round-trip test**

Append a new test inside the existing `mod tests` block:

```rust
    #[test]
    fn terminology_version_request_round_trips() {
        let body = TerminologyVersionRequest {
            version_id: 11,
            version_name: "2024-03-29".into(),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(j, r#"{"versionId":11,"versionName":"2024-03-29"}"#);
        let parsed: TerminologyVersionRequest = serde_json::from_str(&j).unwrap();
        assert_eq!(parsed, body);
    }

    #[test]
    fn project_configuration_data_request_serialises_sdtm_terminology() {
        let body = ProjectConfigurationDataRequest {
            language: None,
            tags: vec![],
            sdtmig: None,
            sdtm_terminology: Some(TerminologyVersionRequest {
                version_id: 11,
                version_name: "2024-03-29".into(),
            }),
        };
        let j = serde_json::to_string(&body).unwrap();
        assert_eq!(
            j,
            r#"{"sdtmTerminology":{"versionId":11,"versionName":"2024-03-29"}}"#
        );
    }
```

- [ ] **Step 3: Run the desktop-tauri test suite**

Run: `cargo test -p aegis-desktop --lib http::project::tests`
Expected: PASS — new tests plus all existing
`project_configuration_data_request_omits_empty` /
`model_version_request_round_trips` etc.

- [ ] **Step 4: Commit**

```bash
git add apps/desktop/aegis-desktop/src-tauri/src/http/project.rs
git commit -m "feat(desktop-tauri): mirror sdtm_terminology wire DTO

Adds TerminologyVersionRequest/Response to the desktop HTTP layer
and threads a fourth sdtm_terminology field through both the request
and response ProjectConfiguration shapes. Round-trip tests pin the
wire JSON shape.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 7: TS types + i18n keys

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/shared/api/types.ts`
- Modify: `lib/packages/ui/src/i18n/locales/en.ts`
- Modify: `lib/packages/ui/src/i18n/locales/zhCN.ts`

- [ ] **Step 1: Extend the TS interface**

In `apps/desktop/aegis-desktop/src/shared/api/types.ts`, after the
existing `ProjectConfigurationSdtmig` block, append:

```ts
/** Wire-shaped pointer to a project's SDTM controlled-terminology
 *  version selection. `versionId` / `versionName` are camelCase to
 *  mirror the server's `#[serde(rename_all = "camelCase")]` on
 *  `apps/server/aegis-server/src/transport/http/dto.rs::TerminologyVersionRequest`.
 *  Distinct from `ProjectConfigurationSdtmig` — the SDTM-IG version
 *  targets the implementation guide; this targets the controlled
 *  terminology release. */
export interface ProjectConfigurationTerminology {
  versionId: number;
  versionName: string;
}
```

Update `ProjectConfiguration`:

```ts
export interface ProjectConfiguration {
  language: ProjectLanguage | null;
  tags: Tag[];
  sdtmig?: ProjectConfigurationSdtmig | null;
  sdtmTerminology?: ProjectConfigurationTerminology | null;
}
```

- [ ] **Step 2: Add the i18n keys (English)**

In `lib/packages/ui/src/i18n/locales/en.ts`, after the existing
`'project.configuration.general.sdtmig.none'` line, append:

```ts
  'project.configuration.general.sdtmTerminology': 'SDTM Terminology',
  'project.configuration.general.sdtmTerminology.none': '(unspecified)',
```

- [ ] **Step 3: Add the i18n keys (Simplified Chinese)**

In `lib/packages/ui/src/i18n/locales/zhCN.ts`, after the existing
`'project.configuration.general.sdtmig.none'` line, append:

```ts
  'project.configuration.general.sdtmTerminology': 'SDTM 术语',
  'project.configuration.general.sdtmTerminology.none': '（未指定）',
```

- [ ] **Step 4: Typecheck the affected workspaces**

Run:
```bash
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop typecheck
```
Expected: SUCCESS.

- [ ] **Step 5: Commit**

```bash
git add apps/desktop/aegis-desktop/src/shared/api/types.ts \
        lib/packages/ui/src/i18n/locales/en.ts \
        lib/packages/ui/src/i18n/locales/zhCN.ts
git commit -m "feat(desktop): TS types + i18n for sdtm_terminology pointer

Adds ProjectConfigurationTerminology to the desktop's TS API
mirror and the two i18n keys (en + zhCN) for the picker label and
its '(unspecified)' option.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 8: ConfigurationGeneralSection picker

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx`

- [ ] **Step 1: Update imports and add a hook load**

Edit the import block at the top of `ConfigurationGeneralSection.tsx`:

```tsx
import { useListTerminologyVersions } from "../../terminology/data/list";
```

Add inside the function component, after the existing
`const versions = useListSdtmVersions();`:

```tsx
  const terminologyVersions = useListTerminologyVersions();
  // The terminology service mixes SDTM and ADaM releases; the picker
  // surfaces only SDTM because that's what a project pins.
  const sdtmTerminologyReleases = (terminologyVersions.data ?? []).filter(
    (v) => v.kind === "sdtm",
  );
```

- [ ] **Step 2: Extend local state**

After the existing `sdtmig` state declaration, append:

```tsx
  const [sdtmTerminology, setSdtmTerminology] =
    useState<ProjectConfigurationTerminology | null>(
      initial.sdtmTerminology ?? null,
    );
  const sdtmTerminologyTouchedRef = useRef(false);
```

Add `ProjectConfigurationTerminology` to the type-only import group:

```tsx
import {
  type ApiError,
  type ProjectConfiguration,
  type ProjectConfigurationSdtmig,
  type ProjectConfigurationTerminology,
  type ProjectLanguage,
  type Tag,
} from "../../../shared/api";
```

- [ ] **Step 3: Extend the reseed effect**

Inside the existing `useEffect` that watches `initial`, add the
sdtm_terminology branch and reset its touched flag:

```tsx
    setSdtmTerminology(initial.sdtmTerminology ?? null);
    sdtmTerminologyTouchedRef.current = false;
```

- [ ] **Step 4: Extend dirty + onSave**

Replace the `dirty` line:

```tsx
  const dirty =
    languageTouchedRef.current ||
    tagsTouchedRef.current ||
    sdtmTouchedRef.current ||
    sdtmTerminologyTouchedRef.current;
```

Replace the `onSave` function:

```tsx
  async function onSave() {
    const configurations: ProjectConfiguration = {
      language,
      tags,
      sdtmig,
      sdtmTerminology,
    };
    await update.mutateAsync({
      code: projectCode,
      body: { configurations },
    });
  }
```

- [ ] **Step 5: Insert the new FormControl**

After the closing `</FormControl>` of the SDTMIG picker (the block
ending at the current `</Select>` + `</FormControl>`), insert:

```tsx
      <FormControl size="small" disabled={readonly}>
        <InputLabel id="config-sdtm-terminology-label">
          {t("project.configuration.general.sdtmTerminology")}
        </InputLabel>
        <Select<string>
          labelId="config-sdtm-terminology-label"
          label={t("project.configuration.general.sdtmTerminology")}
          value={sdtmTerminology ? String(sdtmTerminology.versionId) : ""}
          onChange={(e) => {
            const v = e.target.value;
            if (v === "") {
              setSdtmTerminology(null);
            } else {
              const id = Number(v);
              const found = sdtmTerminologyReleases.find((x) => x.id === id);
              setSdtmTerminology(
                found
                  ? { versionId: found.id, versionName: found.name }
                  : { versionId: id, versionName: "" },
              );
            }
            sdtmTerminologyTouchedRef.current = true;
          }}
          inputProps={{ "data-testid": "config-sdtm-terminology-input" }}
        >
          <MenuItem value="">
            {t("project.configuration.general.sdtmTerminology.none")}
          </MenuItem>
          {sdtmTerminology &&
            !sdtmTerminologyReleases.some(
              (v) => v.id === sdtmTerminology.versionId,
            ) && (
              <MenuItem value={String(sdtmTerminology.versionId)}>
                {sdtmTerminology.versionName}
              </MenuItem>
            )}
          {sdtmTerminologyReleases.map((v) => (
            <MenuItem key={v.id} value={String(v.id)}>
              {v.name}
            </MenuItem>
          ))}
        </Select>
      </FormControl>
```

(Place it directly **above** the `<Box>` wrapping the TagEditor so
the visual order is `Language → SDTMIG → SDTM Terminology → Tags`.)

- [ ] **Step 6: Typecheck**

Run: `pnpm --filter aegis-desktop typecheck`
Expected: SUCCESS.

- [ ] **Step 7: Commit**

```bash
git add apps/desktop/aegis-desktop/src/features/project-workspace/components/ConfigurationGeneralSection.tsx
git commit -m "feat(desktop): SDTM Terminology picker on ConfigurationGeneralSection

Adds a sibling <FormControl> mirroring the existing SDTMIG
Version picker: filters useListTerminologyVersions() down to SDTM
releases, persists { versionId, versionName } in the
ProjectConfiguration payload, and respects the same touched/dirty
save semantics.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 9: Vitest focused cases

**Files:**
- Modify: `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`

- [ ] **Step 1: Add the terminology-versions mock fixture**

After the existing `sdtmVersionsResponse` const, append:

```tsx
const terminologyVersionsResponse = {
  versions: [
    {
      id: 10,
      kind: "sdtm" as const,
      name: "2024-03-29",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    {
      id: 11,
      kind: "sdtm" as const,
      name: "2023-09-29",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
    // Adam release — must NOT appear as a MenuItem in the picker.
    {
      id: 12,
      kind: "adam" as const,
      name: "ADAM-2024",
      createdAt: "2026-01-01T00:00:00Z",
      updatedAt: "2026-01-01T00:00:00Z",
    },
  ],
};
```

- [ ] **Step 2: Extend the leader-view beforeEach mocks**

Find the existing `describe("ProjectConfigurationPage — leader view", () => { beforeEach(...) })` block and extend it to register `list_terminology_versions`:

```tsx
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () => projectFixture,
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
    });
```

- [ ] **Step 3: Extend the non-leader-view beforeEach mocks**

Find the existing `describe("ProjectConfigurationPage — non-leader view", () => { beforeEach(...) })` block and add `list_terminology_versions` to its mock set as well.

- [ ] **Step 4: Extend the existing "Save fires update_project" assertion**

In the test at `it("clicking Save fires update_project with configurations { language, tags, sdtmig }", …)`,
update the inner `expect.objectContaining({ configurations: expect.objectContaining({ ... }) })`
to also include `sdtmTerminology: null`:

```tsx
            configurations: expect.objectContaining({
              language: "en",
              tags: expect.arrayContaining([
                { key: "Product", value: "DEMO-002" },
              ]),
              sdtmig: null,
              sdtmTerminology: null,
            }),
```

- [ ] **Step 5: Add focused cases inside the leader describe block**

Append to `apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx`,
inside the leader `describe`:

```tsx
  it("renders the SDTM Terminology Select with the seeded selection when initial.sdtmTerminology is set", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () =>
        makeProject({
          code: "alpha",
          members: {
            leaders: [{ code: "alice", name: "Alice" }],
            workers: [{ code: "bob", name: "Bob" }],
          },
          unblindMembers: { leaders: [], workers: [] },
          configurations: {
            language: "en",
            tags: [{ key: "Product", value: "DEMO-001" }],
            sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
            sdtmTerminology: {
              versionId: 10,
              versionName: "2024-03-29",
            },
          },
        }),
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
    });
    await renderPage();
    expect(await screen.findByText("2024-03-29")).toBeInTheDocument();
  });

  it("SDTM Terminology picker filters out Adam releases", async () => {
    await renderPage();
    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    expect(
      await screen.findByRole("option", { name: "2024-03-29" }),
    ).toBeInTheDocument();
    expect(
      screen.queryByRole("option", { name: "ADAM-2024" }),
    ).not.toBeInTheDocument();
  });

  it("picking a SDTM Terminology release enables Save", async () => {
    await renderPage();
    const save = await screen.findByTestId("config-general-save");
    expect(save).toBeDisabled();

    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    await userEvent.click(
      await screen.findByRole("option", { name: "2024-03-29" }),
    );

    await waitFor(() => expect(save).not.toBeDisabled());
  });

  it("picking '(unspecified)' on SDTM Terminology → Save → body.configurations.sdtmTerminology === null", async () => {
    mockCommands({
      is_logged_in: () => true,
      current_user: () => leader,
      get_project_by_code: () =>
        makeProject({
          code: "alpha",
          members: {
            leaders: [{ code: "alice", name: "Alice" }],
            workers: [{ code: "bob", name: "Bob" }],
          },
          unblindMembers: { leaders: [], workers: [] },
          configurations: {
            language: "en",
            tags: [{ key: "Product", value: "DEMO-001" }],
            sdtmig: { versionId: 2, versionName: "SDTMIG v3.2" },
            sdtmTerminology: {
              versionId: 10,
              versionName: "2024-03-29",
            },
          },
        }),
      list_sdtm_versions: () => sdtmVersionsResponse,
      list_terminology_versions: () => terminologyVersionsResponse,
      update_project: () => projectFixture,
    });
    await renderPage();

    await userEvent.click(await screen.findByLabelText(/sdtm terminology/i));
    await userEvent.click(
      await screen.findByRole("option", { name: /\(unspecified\)/i }),
    );

    const save = await screen.findByTestId("config-general-save");
    await waitFor(() => expect(save).not.toBeDisabled());
    await userEvent.click(save);

    await waitFor(() =>
      expect(invoke).toHaveBeenCalledWith(
        "update_project",
        expect.objectContaining({
          code: "alpha",
          body: expect.objectContaining({
            configurations: expect.objectContaining({
              sdtmTerminology: null,
            }),
          }),
        }),
      ),
    );
  });
```

- [ ] **Step 6: Add a non-leader disabled-check inside the non-leader describe block**

Append to the non-leader describe:

```tsx
  it("SDTM Terminology Select is disabled for non-leaders", async () => {
    await renderPage();
    const select = await screen.findByLabelText(/sdtm terminology/i);
    await waitFor(() => {
      const fc = select.closest(".MuiFormControl-root");
      const root = select.closest(".MuiInputBase-root");
      expect(
        fc!.classList.contains("Mui-disabled") ||
          root!.classList.contains("Mui-disabled"),
      ).toBe(true);
    });
  });
```

- [ ] **Step 7: Run the test suite**

Run: `pnpm --filter aegis-desktop test -- src/test/features/project-workspace/project-configuration-page.test.tsx`
Expected: PASS — all new cases plus the existing suite.

- [ ] **Step 8: Commit**

```bash
git add apps/desktop/aegis-desktop/src/test/features/project-workspace/project-configuration-page.test.tsx
git commit -m "test(desktop): SDTM Terminology picker cases

Adds four leader cases (seeded selection, Adam filter-out, picking
enables Save, '(unspecified)' clears) plus a non-leader disabled
case. The existing 'Save fires update_project' assertion is extended
to also assert sdtmTerminology: null in the payload.

Co-Authored-By: Claude Code <noreply@anthropic.com>"
```

---

## Task 10: Final cross-crate verification

- [ ] **Step 1: Run the workspace check + targeted tests**

```bash
cargo check --workspace
cargo test -p project --lib
cargo test -p aegis-server --lib transport::http::project::handlers::tests
cargo test -p aegis-desktop --lib http::project::tests
pnpm --filter @aegis/ui typecheck
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
```

Expected: every command succeeds.

- [ ] **Step 2: Run the full Rust test suite**

Run: `cargo test --workspace`
Expected: PASS — no regressions.

- [ ] **Step 3: Run clippy on the touched crates**

```bash
cargo clippy -p project --all-targets --all-features -- -D warnings
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
```

Expected: no warnings.

- [ ] **Step 4: Run rustfmt**

Run: `cargo fmt --all -- --check`
Expected: clean.

(No `git commit` for this task — verification only.)

---

## Self-Review Checklist

- [x] **Spec coverage:** every requirement in
  `docs/superpowers/specs/2026-09-23-project-sdtm-terminology-design.md`
  maps to a task: domain pointer (Task 1), validation (Task 2),
  migration (Task 3), apis (Task 4), server wire DTOs (Task 5),
  desktop HTTP mirror (Task 6), TS types + i18n (Task 7), UI picker
  (Task 8), vitest (Task 9), verification gate (Task 10).
- [x] **Placeholder scan:** no TBD / TODO / "implement later" / "add
  appropriate error handling" placeholders. Every code block shows
  the actual code.
- [x] **Type consistency:** `TerminologyVersionData` (project
  domain) ↔ `apis::project::TerminologyVersionData` ↔
  `dto::TerminologyVersionRequest` ↔ Rust HTTP mirror
  `TerminologyVersionRequest` ↔ TS `ProjectConfigurationTerminology`.
  All carry `version_id` + `version_name`. The `From` chain is
  consistent across Tasks 4–6. Field name `sdtm_terminology` (Rust
  snake) ↔ `sdtmTerminology` (wire camelCase, via
  `rename_all = "camelCase"`) is consistent in all wire DTOs.
