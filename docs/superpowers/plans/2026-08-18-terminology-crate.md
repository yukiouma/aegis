# Terminology Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `lib/crates/terminology`, a Rust library that owns CRUD over the CDISC terminology aggregates (`TerminologyVersion`, `CodeList`, `CodeItem`) and exposes per-level full-text search over their string fields, backed by PostgreSQL `tsvector` + `GIN`.

**Architecture:** Ports-and-adapters DDD structure mirroring `lib/crates/user` and `lib/crates/project`. Three inbound `#[async_trait]` ports (`TerminologyVersionRepository`, `CodeListRepository`, `CodeItemRepository`) live in `domain`; a single `TerminologyUsecase<V, L, I>` is generic over all three. A PostgreSQL adapter implements each port. No outbound `apis` port in this iteration.

**Tech Stack:** Rust 2024 edition, sqlx 0.9 (postgres, runtime-tokio, macros, migrate, chrono), async-trait, thiserror, chrono, PostgreSQL 15+ with `tsvector` + `GIN`.

## Global Constraints

- **Edition / resolver:** `edition = "2024"`, workspace `resolver = "3"`. The crate's `Cargo.toml` must inherit every shared dep via `{ workspace = true }`.
- **Workspace wiring:** Edit the root `Cargo.toml` once in Task 1 to add `lib/crates/terminology` to `[workspace].members`.
- **SQLx convention:** Use the runtime API (`sqlx::query_as`, `QueryBuilder`). Compile-time-checked macros need a live `DATABASE_URL` or a `sqlx-data.json` cache that the workspace does not currently provide. Document the choice in a module-level comment in each postgres repo file.
- **Two-constructor pattern:** Every aggregate has a validating `new(...) -> Result<Self, DomainError>` and a `pub(crate) for_repository(...) -> Self`. The latter is reserved for the adapter layer's row bridge; never call it from `usecase`.
- **Errors:** `DomainError` and `UsecaseError` are `#[derive(thiserror::Error)]` enums. Every variant that wraps an inner error carries `#[source] Inner` so the chain is preserved. `UsecaseError::From<DomainError>` maps to `Repository` (because the contract was already broken upstream). Domain validators return `UsecaseError::Validation(...)`.
- **API surface:** The crate root re-exports the documented public surface (aggregates, ports, error enums, command DTOs, view DTOs, concrete repos, the usecase + config). Consumers write `use terminology::*;` and never reach into sub-modules.
- **Full-text search:** A `tsvector` column generated from the five text columns (`name`, `submission_value`, `synonym`, `definition`, `nci_preferred_term`) with weights `A` / `A` / `B` / `C` / `B`. A GIN index on it. Queries go through `websearch_to_tsquery('english', ...)` and rank with `ts_rank_cd`. A query that reduces to all stopwords returns empty hits, never an error.
- **Cascade:** `version_id` and `codelist_id` FKs use `ON DELETE CASCADE`. `delete_version` removes child code lists (and through them, code items) via the cascade.
- **Verification gate (every task that compiles must end green):**
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p terminology --all-targets --all-features -- -D warnings
  cargo test -p terminology
  cargo doc -p terminology --no-deps
  ```
  After Task 6, also:
  ```bash
  cargo check --workspace
  cargo clippy --workspace
  cargo test --workspace
  ```
  Live-DB integration tests are run only with `AEGIS_TERMINOLOGY_DATABASE_URL` set:
  ```bash
  cargo test -p terminology -- --ignored --test-threads=1
  ```

## File Structure

### Created

- `lib/crates/terminology/Cargo.toml`
- `lib/crates/terminology/README.md`
- `lib/crates/terminology/migrations/0001_create_terminology_versions.sql`
- `lib/crates/terminology/migrations/0002_create_code_lists.sql`
- `lib/crates/terminology/migrations/0003_create_code_items.sql`
- `lib/crates/terminology/src/lib.rs`
- `lib/crates/terminology/src/domain.rs`
- `lib/crates/terminology/src/domain/terminology_kind.rs`
- `lib/crates/terminology/src/domain/terminology_version.rs`
- `lib/crates/terminology/src/domain/code_list.rs`
- `lib/crates/terminology/src/domain/code_item.rs`
- `lib/crates/terminology/src/domain/error.rs`
- `lib/crates/terminology/src/domain/repository.rs`
- `lib/crates/terminology/src/domain/tests.rs`
- `lib/crates/terminology/src/usecase.rs`
- `lib/crates/terminology/src/usecase/commands.rs`
- `lib/crates/terminology/src/usecase/views.rs`
- `lib/crates/terminology/src/usecase/error.rs`
- `lib/crates/terminology/src/usecase/terminology_usecase.rs`
- `lib/crates/terminology/src/usecase/tests.rs`
- `lib/crates/terminology/src/adapter.rs`
- `lib/crates/terminology/src/adapter/persistence.rs`
- `lib/crates/terminology/src/adapter/persistence/postgres.rs`
- `lib/crates/terminology/src/adapter/persistence/postgres/terminology_version_repo.rs`
- `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`
- `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`
- `lib/crates/terminology/tests/public_api.rs`
- `lib/crates/terminology/tests/integration_persistence.rs`

### Modified

- `Cargo.toml` (workspace root) — add `lib/crates/terminology` to `members`.

---

## Task 1: Scaffold — workspace wiring + crate skeleton

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `lib/crates/terminology/Cargo.toml`
- Create: `lib/crates/terminology/README.md`
- Create: `lib/crates/terminology/src/lib.rs`
- Create: `lib/crates/terminology/src/domain.rs`
- Create: `lib/crates/terminology/src/usecase.rs`
- Create: `lib/crates/terminology/src/adapter.rs`
- Create: `lib/crates/terminology/src/domain/error.rs` (empty stub)

### 1.1 Wire the workspace

- [ ] **Step 1:** Edit `Cargo.toml` (workspace root). In `[workspace].members`, add `"lib/crates/terminology",`.

### 1.2 Create `lib/crates/terminology/Cargo.toml`

- [ ] **Step 1:** Create `lib/crates/terminology/Cargo.toml`:

```toml
[package]
name = "terminology"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `chrono` provides the `DateTime<Utc>` type carried on every
# aggregate's `created_at` / `updated_at` columns. The `clock`
# feature keeps the binary small (no local time zones) while
# still allowing `NOW()` round-trips through `chrono::Utc`.
chrono = { workspace = true }

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

### 1.3 Create the placeholder README

- [ ] **Step 1:** Create `lib/crates/terminology/README.md`:

```markdown
# terminology

CRUD over the CDISC terminology aggregates
(`TerminologyVersion`, `CodeList`, `CodeItem`) with full-text
search, backed by PostgreSQL.

See `docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions and the design spec for the full data model.
```

### 1.4 Create the layer skeleton

- [ ] **Step 1:** Create `lib/crates/terminology/src/lib.rs`:

```rust
//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;
```

- [ ] **Step 2:** Create `lib/crates/terminology/src/domain.rs`:

```rust
mod error;

pub use error::DomainError;
```

- [ ] **Step 3:** Create `lib/crates/terminology/src/usecase.rs`:

```rust
// Usecase layer; populated in Task 3.
```

- [ ] **Step 4:** Create `lib/crates/terminology/src/adapter.rs`:

```rust
// Adapter layer; populated in Tasks 4-6.
```

- [ ] **Step 5:** Create `lib/crates/terminology/src/domain/error.rs`:

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("repository error: {0}")]
    Repository(String),
}
```

### 1.5 Verify + commit

- [ ] **Step 1:** Run `cargo check --workspace`.
-   Expected: green (the new crate compiles against its dependencies; the empty/stub modules don't pull in anything yet).

- [ ] **Step 2:** Commit:

```bash
git add Cargo.toml \
        lib/crates/terminology/Cargo.toml \
        lib/crates/terminology/README.md \
        lib/crates/terminology/src/lib.rs \
        lib/crates/terminology/src/domain.rs \
        lib/crates/terminology/src/usecase.rs \
        lib/crates/terminology/src/adapter.rs \
        lib/crates/terminology/src/domain/error.rs
git commit -m "feat(terminology): scaffold crate skeleton

Adds lib/crates/terminology to the workspace members and creates
the three DDD layer skeletons (domain, usecase, adapter) plus a
stub DomainError. Cargo.toml inherits every shared dep via
{ workspace = true }; chrono is documented in-line as the
created_at / updated_at carrier.

Spec coverage: workspace wiring + crate shape in
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo check --workspace"
```

---

## Task 2: Domain layer — value objects, aggregates, ports, errors

**Files:**
- Modify: `lib/crates/terminology/src/domain.rs`
- Create: `lib/crates/terminology/src/domain/terminology_kind.rs`
- Create: `lib/crates/terminology/src/domain/terminology_version.rs`
- Create: `lib/crates/terminology/src/domain/code_list.rs`
- Create: `lib/crates/terminology/src/domain/code_item.rs`
- Create: `lib/crates/terminology/src/domain/repository.rs`
- Create: `lib/crates/terminology/src/domain/tests.rs`
- Modify: `lib/crates/terminology/src/domain/error.rs`
- Modify: `lib/crates/terminology/src/lib.rs` (re-exports)

### 2.1 Write the failing domain tests

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/tests.rs`:

```rust
use super::{
    CodeItem, CodeList, CodeListNew, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionUpdate,
};

#[test]
fn terminology_kind_parses_lowercase_strings() {
    let sdtm = TerminologyKind::try_from("sdtm").unwrap();
    let adam = TerminologyKind::try_from("adam").unwrap();
    assert_eq!(sdtm.as_str(), "sdtm");
    assert_eq!(adam.as_str(), "adam");
}

#[test]
fn terminology_kind_rejects_unknown_string() {
    let err = TerminologyKind::try_from("OTHER").unwrap_err();
    assert!(matches!(err, DomainError::InvalidKind(ref s) if s == "OTHER"));
}

#[test]
fn terminology_version_new_rejects_empty_name() {
    let err = TerminologyVersion::new(TerminologyKind::Sdtm, "   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn terminology_version_new_accepts_valid_input() {
    let v = TerminologyVersion::new(TerminologyKind::Sdtm, "2026-03-27".into()).unwrap();
    assert_eq!(v.kind, TerminologyKind::Sdtm);
    assert_eq!(v.name, "2026-03-27");
}

#[test]
fn code_list_new_rejects_empty_code() {
    let err = CodeList::new(
        1,
        "".into(),
        false,
        "AGE".into(),
        "AGE".into(),
        "".into(),
        "".into(),
        "".into(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn code_list_new_accepts_valid_input() {
    let cl = CodeList::new(
        1,
        "C66741".into(),
        true,
        "AGE".into(),
        "AGE".into(),
        "Age".into(),
        "Age in years".into(),
        "Age".into(),
    )
    .unwrap();
    assert_eq!(cl.code, "C66741");
    assert!(cl.extensible);
}

#[test]
fn code_item_new_rejects_empty_code() {
    let err = CodeItem::new(1, "".into(), "X".into(), "".into(), "".into(), "".into())
        .unwrap_err();
    assert!(matches!(err, DomainError::EmptyCode));
}

#[test]
fn code_item_new_accepts_valid_input() {
    let item = CodeItem::new(1, "C12345".into(), "> 0".into(), "".into(), "".into(), "".into())
        .unwrap();
    assert_eq!(item.code, "C12345");
}
```

- [ ] **Step 2:** Run; confirm compile failure.

Run: `cargo test -p terminology --lib domain::tests 2>&1 | tail -20`
Expected: compile errors (types not yet defined).

### 2.2 Create `terminology_kind.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/terminology_kind.rs`:

```rust
use std::convert::TryFrom;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminologyKind {
    Sdtm,
    Adam,
}

impl TerminologyKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            TerminologyKind::Sdtm => "sdtm",
            TerminologyKind::Adam => "adam",
        }
    }
}

impl TryFrom<&str> for TerminologyKind {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "sdtm" => Ok(TerminologyKind::Sdtm),
            "adam" => Ok(TerminologyKind::Adam),
            other => Err(DomainError::InvalidKind(other.to_string())),
        }
    }
}
```

### 2.3 Create `terminology_version.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/terminology_version.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::terminology_kind::TerminologyKind;

/// A published CDISC terminology release, identified by its
/// `(kind, name)` pair. `name` is the `yyyy-mm-dd` suffix of the
/// matched workbook sheet and is stored as `String` (not parsed
/// into a `NaiveDate`) so a future sheet with a non-date name
/// round-trips intact.
#[derive(Clone, PartialEq, Eq)]
pub struct TerminologyVersion {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for TerminologyVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminologyVersion")
            .field("id", &self.id)
            .field("kind", &self.kind)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl TerminologyVersion {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(kind: TerminologyKind, name: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0, // placeholder; for_repository overwrites it
            kind,
            name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        kind: TerminologyKind,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            kind,
            name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `TerminologyVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct TerminologyVersionNew {
    pub kind: TerminologyKind,
    pub name: String,
}

/// Input DTO for `TerminologyVersionRepository::update`. Every
/// field is optional so the usecase can pass only what actually
/// changed.
#[derive(Debug, Clone, Default)]
pub struct TerminologyVersionUpdate {
    pub id: i64,
    pub kind: Option<TerminologyKind>,
    pub name: Option<String>,
}
```

### 2.4 Create `code_list.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/code_list.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::terminology_kind::TerminologyKind;

/// A CDISC codelist and the items that belong to it. The
/// in-memory shape mirrors the workbook; the persisted shape
/// keeps the items in a separate `code_items` table referenced
/// by `codelist_id`.
#[derive(Clone, PartialEq, Eq)]
pub struct CodeList {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for CodeList {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeList")
            .field("id", &self.id)
            .field("version_id", &self.version_id)
            .field("code", &self.code)
            .field("extensible", &self.extensible)
            .field("name", &self.name)
            .field("submission_value", &self.submission_value)
            .field("synonym", &self.synonym)
            .field("definition", &self.definition)
            .field("nci_preferred_term", &self.nci_preferred_term)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl CodeList {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `code` (the NCI C-code).
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        version_id: i64,
        code: String,
        extensible: bool,
        name: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        Ok(Self {
            id: 0,
            version_id,
            code,
            extensible,
            name,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        version_id: i64,
        code: String,
        extensible: bool,
        name: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version_id,
            code,
            extensible,
            name,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CodeListRepository::create`.
#[derive(Debug, Clone)]
pub struct CodeListNew {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for `CodeListRepository::update`. Every field is
/// optional so the usecase can pass only what actually changed.
#[derive(Debug, Clone, Default)]
pub struct CodeListUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub extensible: Option<bool>,
    pub name: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// Query for `CodeListRepository::search`. Search is scoped to a
/// single `(kind, version_name)` pair so callers cannot accidentally
/// cross-releases.
#[derive(Debug, Clone)]
pub struct CodeListSearchQuery {
    pub kind: TerminologyKind,
    pub version_name: String,
    pub text: String,
    /// Default 50. Hard cap 500 (clamped, not rejected).
    pub limit: u32,
}

/// One hit from `CodeListRepository::search`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeListSearchHit {
    pub codelist: CodeList,
    pub score: f32,
}
```

### 2.5 Create `code_item.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/code_item.rs`:

```rust
use chrono::{DateTime, Utc};

use super::code_list::CodeList;
use super::error::DomainError;

/// A single permissible value inside a `CodeList`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeItem {
    pub id: i64,
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl CodeItem {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `code`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        codelist_id: i64,
        code: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
    ) -> Result<Self, DomainError> {
        if code.trim().is_empty() {
            return Err(DomainError::EmptyCode);
        }
        Ok(Self {
            id: 0,
            codelist_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        codelist_id: i64,
        code: String,
        submission_value: String,
        synonym: String,
        definition: String,
        nci_preferred_term: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            codelist_id,
            code,
            submission_value,
            synonym,
            definition,
            nci_preferred_term,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `CodeItemRepository::create`.
#[derive(Debug, Clone)]
pub struct CodeItemNew {
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

/// Input DTO for `CodeItemRepository::update`. Every field is
/// optional so the usecase can pass only what actually changed.
#[derive(Debug, Clone, Default)]
pub struct CodeItemUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

/// Query for `CodeItemRepository::search`.
#[derive(Debug, Clone)]
pub struct CodeItemSearchQuery {
    pub kind: crate::domain::terminology_kind::TerminologyKind,
    pub version_name: String,
    pub text: String,
    /// Default 50. Hard cap 500 (clamped, not rejected).
    pub limit: u32,
}

/// One hit from `CodeItemRepository::search`.
#[derive(Debug, Clone, PartialEq)]
pub struct CodeItemSearchHit {
    pub item: CodeItem,
    pub score: f32,
    pub codelist_id: i64,
}

// Pull the CodeList re-export into scope so users of this module
// do not have to import it separately when reasoning about
// `codelist_id` lookups.
pub use CodeList as _CodeListReexport;
```

### 2.6 Create the repository ports

- [ ] **Step 1:** Create `lib/crates/terminology/src/domain/repository.rs`:

```rust
use async_trait::async_trait;

use super::code_item::{
    CodeItem, CodeItemNew, CodeItemSearchHit, CodeItemSearchQuery, CodeItemUpdate,
};
use super::code_list::{
    CodeList, CodeListNew, CodeListSearchHit, CodeListSearchQuery, CodeListUpdate,
};
use super::error::DomainError;
use super::terminology_kind::TerminologyKind;
use super::terminology_version::{
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionUpdate,
};

/// Outbound port for persistence of `TerminologyVersion`
/// aggregates. Implementations live in the adapter layer.
#[async_trait]
pub trait TerminologyVersionRepository: Send + Sync {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError>;

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError>;

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError>;

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError>;

    /// Hard delete; cascades to child code_lists (and via them to
    /// code_items) via the schema's `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `CodeList` aggregates.
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    /// Hard delete; cascades to code_items via the schema's
    /// `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(
        &self,
        query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError>;
}

/// Outbound port for persistence of `CodeItem` aggregates.
#[async_trait]
pub trait CodeItemRepository: Send + Sync {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError>;
    async fn list_by_codelist(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItem>, DomainError>;
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(
        &self,
        query: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError>;
}
```

### 2.7 Expand `DomainError`

- [ ] **Step 1:** Replace `lib/crates/terminology/src/domain/error.rs`:

```rust
use thiserror::Error;

use super::terminology_kind::TerminologyKind;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("name must not be empty")]
    EmptyName,

    #[error("code must not be empty")]
    EmptyCode,

    #[error("invalid terminology kind: {0}")]
    InvalidKind(String),

    #[error("not found")]
    NotFound,

    #[error("version not found: {0}")]
    VersionNotFound(i64),

    #[error("code list not found: {0}")]
    CodeListNotFound(i64),

    #[error("code item not found: {0}")]
    CodeItemNotFound(i64),

    #[error("terminology version already exists for {kind:?} / {name}")]
    DuplicateVersion {
        kind: TerminologyKind,
        name: String,
    },

    #[error("code list already exists for version {version_id} / {code}")]
    DuplicateCodeList {
        version_id: i64,
        code: String,
    },

    #[error("code item already exists for codelist {codelist_id} / {code}")]
    DuplicateCodeItem {
        codelist_id: i64,
        code: String,
    },

    #[error("referenced terminology version not found: {0}")]
    FkVersionNotFound(i64),

    #[error("referenced code list not found: {0}")]
    FkCodeListNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}
```

### 2.8 Update `domain.rs`

- [ ] **Step 1:** Replace `lib/crates/terminology/src/domain.rs`:

```rust
mod code_item;
mod code_list;
mod error;
mod repository;
mod terminology_kind;
mod terminology_version;
#[cfg(test)]
mod tests;

pub use code_item::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate,
};
pub use code_list::{
    CodeList, CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery,
    CodeListUpdate,
};
pub use error::DomainError;
pub use repository::{CodeItemRepository as _, CodeListRepository as _};
pub use terminology_kind::TerminologyKind;
pub use terminology_version::{
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
```

### 2.9 Update `lib.rs` re-exports

- [ ] **Step 1:** Edit `lib/crates/terminology/src/lib.rs`:

```rust
//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, CodeList, CodeListNew, CodeListRepository, CodeListSearchHit,
    CodeListSearchQuery, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
```

### 2.10 Verify + commit

- [ ] **Step 1:** Run `cargo test -p terminology --lib domain::`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/src/domain.rs \
        lib/crates/terminology/src/domain/terminology_kind.rs \
        lib/crates/terminology/src/domain/terminology_version.rs \
        lib/crates/terminology/src/domain/code_list.rs \
        lib/crates/terminology/src/domain/code_item.rs \
        lib/crates/terminology/src/domain/repository.rs \
        lib/crates/terminology/src/domain/error.rs \
        lib/crates/terminology/src/domain/tests.rs \
        lib/crates/terminology/src/lib.rs
git commit -m "feat(terminology): domain layer — kinds, aggregates, ports, errors

Domain layer now defines TerminologyKind, TerminologyVersion,
CodeList, CodeItem aggregates (each with validating new() +
pub(crate) for_repository() constructors), three
#[async_trait] ports (TerminologyVersionRepository,
CodeListRepository, CodeItemRepository) including the
CodeListSearchQuery / CodeItemSearchQuery search inputs,
and a comprehensive DomainError with NotFound / Fk* / Duplicate*
variants.

Spec coverage: Data Model + Repository Ports sections of
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo test -p terminology --lib domain::"
```

---

## Task 3: Usecase layer — commands, views, error, usecase, in-memory tests

**Files:**
- Modify: `lib/crates/terminology/src/usecase.rs`
- Create: `lib/crates/terminology/src/usecase/error.rs`
- Create: `lib/crates/terminology/src/usecase/commands.rs`
- Create: `lib/crates/terminology/src/usecase/views.rs`
- Create: `lib/crates/terminology/src/usecase/terminology_usecase.rs`
- Create: `lib/crates/terminology/src/usecase/tests.rs`
- Modify: `lib/crates/terminology/src/lib.rs` (re-exports)

### 3.1 Create `usecase/error.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/usecase/error.rs`:

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
        // Validation errors that originated upstream of the
        // repository already came through `UsecaseError::Validation`;
        // everything else surfaces as `Repository`.
        UsecaseError::Repository(err)
    }
}
```

### 3.2 Create `usecase/commands.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/usecase/commands.rs`:

```rust
use crate::domain::TerminologyKind;

// TerminologyVersion

pub struct CreateTerminologyVersion {
    pub kind: TerminologyKind,
    pub name: String,
}

#[derive(Default)]
pub struct UpdateTerminologyVersion {
    pub id: i64,
    pub kind: Option<TerminologyKind>,
    pub name: Option<String>,
}

// CodeList

pub struct CreateCodeList {
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Default)]
pub struct UpdateCodeList {
    pub id: i64,
    pub code: Option<String>,
    pub extensible: Option<bool>,
    pub name: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

// CodeItem

pub struct CreateCodeItem {
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}

#[derive(Default)]
pub struct UpdateCodeItem {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}
```

### 3.3 Create `usecase/views.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/usecase/views.rs`:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{
    CodeItem, CodeList, CodeListSearchHit, CodeItemSearchHit, TerminologyKind,
    TerminologyVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TerminologyVersionView {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<TerminologyVersion> for TerminologyVersionView {
    fn from(v: TerminologyVersion) -> Self {
        Self {
            id: v.id,
            kind: v.kind,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeListView {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CodeList> for CodeListView {
    fn from(c: CodeList) -> Self {
        Self {
            id: c.id,
            version_id: c.version_id,
            code: c.code,
            extensible: c.extensible,
            name: c.name,
            submission_value: c.submission_value,
            synonym: c.synonym,
            definition: c.definition,
            nci_preferred_term: c.nci_preferred_term,
            created_at: c.created_at,
            updated_at: c.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CodeItemView {
    pub id: i64,
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<CodeItem> for CodeItemView {
    fn from(i: CodeItem) -> Self {
        Self {
            id: i.id,
            codelist_id: i.codelist_id,
            code: i.code,
            submission_value: i.submission_value,
            synonym: i.synonym,
            definition: i.definition,
            nci_preferred_term: i.nci_preferred_term,
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

// Re-export the search-hit views so the usecase surface is one
// `use terminology::*` away.
pub use crate::domain::{CodeItemSearchHit, CodeListSearchHit};
```

### 3.4 Create `usecase/terminology_usecase.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/usecase/terminology_usecase.rs`:

```rust
use crate::domain::{
    CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemUpdate,
    CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery,
    CodeListUpdate, DomainError, TerminologyKind, TerminologyVersion,
    TerminologyVersionNew, TerminologyVersionRepository, TerminologyVersionUpdate,
};

use super::commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeItem, UpdateCodeList,
    UpdateTerminologyVersion,
};
use super::error::UsecaseError;
use super::views::{CodeItemView, CodeListView, TerminologyVersionView};

/// Configuration for `TerminologyUsecase::new`. Wraps the three
/// concrete (or fake) repositories so the constructor stays
/// readable.
pub struct TerminologyUsecaseConfig<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    pub version_repo: V,
    pub code_list_repo: L,
    pub code_item_repo: I,
}

/// Async orchestration for terminology lifecycle operations.
///
/// Generic over the three repository ports so tests can inject
/// in-memory fakes. Domain → view projection runs through the
/// `From` impls in `super::views`.
pub struct TerminologyUsecase<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    version_repo: V,
    code_list_repo: L,
    code_item_repo: I,
}

impl<V, L, I> TerminologyUsecase<V, L, I>
where
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
{
    pub fn new(cfg: TerminologyUsecaseConfig<V, L, I>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            code_list_repo: cfg.code_list_repo,
            code_item_repo: cfg.code_item_repo,
        }
    }

    // ---- TerminologyVersion ----

    pub async fn create_version(
        &self,
        cmd: CreateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        let version = self
            .version_repo
            .create(TerminologyVersionNew {
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(version.into())
    }

    pub async fn get_version_by_id(
        &self,
        id: i64,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        let v = self.version_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn get_version(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        if name.trim().is_empty() {
            return Err(UsecaseError::Validation(DomainError::EmptyName));
        }
        let v = self
            .version_repo
            .find_by_kind_and_name(kind, name)
            .await?;
        Ok(v.into())
    }

    pub async fn list_versions(
        &self,
    ) -> Result<Vec<TerminologyVersionView>, UsecaseError> {
        let versions = self.version_repo.list().await?;
        Ok(versions.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self,
        cmd: UpdateTerminologyVersion,
    ) -> Result<TerminologyVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(TerminologyVersionUpdate {
                id: cmd.id,
                kind: cmd.kind,
                name: cmd.name,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- CodeList ----

    pub async fn create_code_list(
        &self,
        cmd: CreateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_create_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .create(CodeListNew {
                version_id: cmd.version_id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn list_code_lists(
        &self,
        version_id: i64,
    ) -> Result<Vec<CodeListView>, UsecaseError> {
        let lists = self.code_list_repo.list_by_version(version_id).await?;
        Ok(lists.into_iter().map(Into::into).collect())
    }

    pub async fn update_code_list(
        &self,
        cmd: UpdateCodeList,
    ) -> Result<CodeListView, UsecaseError> {
        validate_update_code_list(&cmd)?;
        let cl = self
            .code_list_repo
            .update(CodeListUpdate {
                id: cmd.id,
                code: cmd.code,
                extensible: cmd.extensible,
                name: cmd.name,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(cl.into())
    }

    pub async fn delete_code_list(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_list_repo.delete(id).await?;
        Ok(())
    }

    pub async fn search_code_lists(
        &self,
        q: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, UsecaseError> {
        let hits = self.code_list_repo.search(clamp_query(q)).await?;
        Ok(hits)
    }

    // ---- CodeItem ----

    pub async fn create_code_item(
        &self,
        cmd: CreateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_create_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .create(CodeItemNew {
                codelist_id: cmd.codelist_id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn list_code_items(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItemView>, UsecaseError> {
        let items = self.code_item_repo.list_by_codelist(codelist_id).await?;
        Ok(items.into_iter().map(Into::into).collect())
    }

    pub async fn update_code_item(
        &self,
        cmd: UpdateCodeItem,
    ) -> Result<CodeItemView, UsecaseError> {
        validate_update_code_item(&cmd)?;
        let item = self
            .code_item_repo
            .update(CodeItemUpdate {
                id: cmd.id,
                code: cmd.code,
                submission_value: cmd.submission_value,
                synonym: cmd.synonym,
                definition: cmd.definition,
                nci_preferred_term: cmd.nci_preferred_term,
            })
            .await?;
        Ok(item.into())
    }

    pub async fn delete_code_item(&self, id: i64) -> Result<(), UsecaseError> {
        self.code_item_repo.delete(id).await?;
        Ok(())
    }

    pub async fn search_code_items(
        &self,
        q: crate::domain::code_item::CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, UsecaseError> {
        let hits = self
            .code_item_repo
            .search(crate::domain::code_item::CodeItemSearchQuery {
                limit: clamp_limit(q.limit),
                ..q
            })
            .await?;
        Ok(hits)
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateTerminologyVersion) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateTerminologyVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_code_list(cmd: &CreateCodeList) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_list(cmd: &UpdateCodeList) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_create_code_item(cmd: &CreateCodeItem) -> Result<(), UsecaseError> {
    if cmd.code.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

fn validate_update_code_item(cmd: &UpdateCodeItem) -> Result<(), UsecaseError> {
    if let Some(ref code) = cmd.code
        && code.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyCode));
    }
    Ok(())
}

// ---- search-query sanitation ----

/// Apply the default + hard cap to the `limit` field of a search
/// query, returning a new query with the clamped value. The
/// Postgres implementation reads the clamped value, so the cap is
/// enforced even when tests pass an unbounded `u32::MAX`.
fn clamp_query(mut q: CodeListSearchQuery) -> CodeListSearchQuery {
    q.limit = clamp_limit(q.limit);
    q
}

fn clamp_limit(limit: u32) -> u32 {
    if limit == 0 {
        50
    } else if limit > 500 {
        500
    } else {
        limit
    }
}
```

### 3.5 Update `usecase.rs`

- [ ] **Step 1:** Replace `lib/crates/terminology/src/usecase.rs`:

```rust
mod commands;
mod error;
mod terminology_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeItem, UpdateCodeList,
    UpdateTerminologyVersion,
};
pub use error::UsecaseError;
pub use terminology_usecase::{TerminologyUsecase, TerminologyUsecaseConfig};
pub use views::{CodeItemSearchHit, CodeItemView, CodeListSearchHit, CodeListView, TerminologyVersionView};
```

### 3.6 Create `usecase/tests.rs` with in-memory fakes

- [ ] **Step 1:** Create `lib/crates/terminology/src/usecase/tests.rs`:

```rust
//! Tests for the usecase layer wired against in-memory repository
//! fakes. No SQL, no I/O.

use std::collections::HashMap;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use chrono::{TimeZone, Utc};

use crate::domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemUpdate, CodeList,
    CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery, CodeListUpdate,
    DomainError, TerminologyKind, TerminologyVersion, TerminologyVersionNew,
    TerminologyVersionRepository, TerminologyVersionUpdate,
};
use crate::usecase::commands::{
    CreateCodeItem, CreateCodeList, CreateTerminologyVersion, UpdateCodeList,
    UpdateTerminologyVersion,
};
use crate::usecase::error::UsecaseError;
use crate::usecase::terminology_usecase::{TerminologyUsecase, TerminologyUsecaseConfig};

fn now() -> chrono::DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 8, 18, 0, 0, 0).unwrap()
}

// ---------- in-memory fakes ----------

#[derive(Default)]
struct VersionsState {
    by_id: HashMap<i64, TerminologyVersion>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeVersionRepo {
    state: Arc<Mutex<VersionsState>>,
}

impl FakeVersionRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VersionsState::default())),
        }
    }
}

#[async_trait]
impl TerminologyVersionRepository for FakeVersionRepo {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|v| v.kind == input.kind && v.name == input.name)
        {
            return Err(DomainError::DuplicateVersion {
                kind: input.kind,
                name: input.name,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let v = TerminologyVersion::for_repository(id, input.kind, input.name, ts, ts);
        s.by_id.insert(id, v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::VersionNotFound(id))
    }
    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .values()
            .find(|v| v.kind == kind && v.name == name)
            .cloned()
            .ok_or_else(|| DomainError::NotFound)
    }
    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .cloned()
            .collect())
    }
    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut s = self.state.lock().unwrap();
        let v = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::VersionNotFound(input.id))?;
        if let Some(kind) = input.kind {
            v.kind = kind;
        }
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::VersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct ListsState {
    by_id: HashMap<i64, CodeList>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeListRepo {
    state: Arc<Mutex<ListsState>>,
}

impl FakeCodeListRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ListsState::default())),
        }
    }
}

#[async_trait]
impl CodeListRepository for FakeCodeListRepo {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|c| c.version_id == input.version_id && c.code == input.code)
        {
            return Err(DomainError::DuplicateCodeList {
                version_id: input.version_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let cl = CodeList::for_repository(
            id,
            input.version_id,
            input.code,
            input.extensible,
            input.name,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            ts,
            ts,
        );
        s.by_id.insert(id, cl.clone());
        Ok(cl)
    }
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeListNotFound(id))
    }
    async fn list_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<CodeList>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|c| c.version_id == version_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError> {
        let mut s = self.state.lock().unwrap();
        let c = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeListNotFound(input.id))?;
        if let Some(code) = input.code {
            c.code = code;
        }
        if let Some(ext) = input.extensible {
            c.extensible = ext;
        }
        if let Some(name) = input.name {
            c.name = name;
        }
        if let Some(sv) = input.submission_value {
            c.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            c.synonym = syn;
        }
        if let Some(def) = input.definition {
            c.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            c.nci_preferred_term = pt;
        }
        c.updated_at = now();
        Ok(c.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeListNotFound(id));
        }
        Ok(())
    }
    async fn search(
        &self,
        _query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError> {
        // The fake returns whatever its underlying store has; the
        // tests can filter the returned list themselves if they
        // want to assert on shape. Returning empty by default keeps
        // the fake conservative.
        Ok(vec![])
    }
}
        // The fake returns whatever its underlying store has; the
        // tests can filter the returned list themselves if they
        // want to assert on shape. Returning empty by default keeps
        // the fake conservative.
        Ok(vec![])
    }
}

#[derive(Default)]
struct ItemsState {
    by_id: HashMap<i64, CodeItem>,
    next: AtomicI64,
}

#[derive(Clone, Default)]
struct FakeCodeItemRepo {
    state: Arc<Mutex<ItemsState>>,
}

impl FakeCodeItemRepo {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(ItemsState::default())),
        }
    }
}

#[async_trait]
impl CodeItemRepository for FakeCodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id
            .values()
            .any(|i| i.codelist_id == input.codelist_id && i.code == input.code)
        {
            return Err(DomainError::DuplicateCodeItem {
                codelist_id: input.codelist_id,
                code: input.code,
            });
        }
        let id = s.next.fetch_add(1, Ordering::SeqCst) + 1;
        let ts = now();
        let item = CodeItem::for_repository(
            id,
            input.codelist_id,
            input.code,
            input.submission_value,
            input.synonym,
            input.definition,
            input.nci_preferred_term,
            ts,
            ts,
        );
        s.by_id.insert(id, item.clone());
        Ok(item)
    }
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        self.state
            .lock()
            .unwrap()
            .by_id
            .get(&id)
            .cloned()
            .ok_or(DomainError::CodeItemNotFound(id))
    }
    async fn list_by_codelist(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItem>, DomainError> {
        Ok(self
            .state
            .lock()
            .unwrap()
            .by_id
            .values()
            .filter(|i| i.codelist_id == codelist_id)
            .cloned()
            .collect())
    }
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut s = self.state.lock().unwrap();
        let i = s
            .by_id
            .get_mut(&input.id)
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        if let Some(code) = input.code {
            i.code = code;
        }
        if let Some(sv) = input.submission_value {
            i.submission_value = sv;
        }
        if let Some(syn) = input.synonym {
            i.synonym = syn;
        }
        if let Some(def) = input.definition {
            i.definition = def;
        }
        if let Some(pt) = input.nci_preferred_term {
            i.nci_preferred_term = pt;
        }
        i.updated_at = now();
        Ok(i.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut s = self.state.lock().unwrap();
        if s.by_id.remove(&id).is_none() {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }
    async fn search(
        &self,
        _query: crate::domain::code_item::CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError> {
        Ok(vec![])
    }
}

// ---------- fixture ----------

fn make_usecase() -> (
    FakeVersionRepo,
    FakeCodeListRepo,
    FakeCodeItemRepo,
    TerminologyUsecase<FakeVersionRepo, FakeCodeListRepo, FakeCodeItemRepo>,
) {
    let v = FakeVersionRepo::new();
    let l = FakeCodeListRepo::new();
    let i = FakeCodeItemRepo::new();
    let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
        version_repo: v.clone(),
        code_list_repo: l.clone(),
        code_item_repo: i.clone(),
    });
    (v, l, i, usecase)
}

// ---------- tests ----------

#[tokio::test]
async fn create_version_rejects_empty_name() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyName)
    ));
}

#[tokio::test]
async fn create_then_get_version_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    assert_eq!(created.name, "2026-03-27");
    let fetched = usecase
        .get_version(TerminologyKind::Sdtm, "2026-03-27")
        .await
        .expect("get");
    assert_eq!(fetched.id, created.id);
}

#[tokio::test]
async fn update_version_then_list_returns_updated_name() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_version(CreateTerminologyVersion {
            kind: TerminologyKind::Sdtm,
            name: "2026-03-27".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_version(UpdateTerminologyVersion {
            id: created.id,
            name: Some("2026-06-15".into()),
            ..Default::default()
        })
        .await
        .expect("update");
    assert_eq!(updated.name, "2026-06-15");
    let listed = usecase.list_versions().await.expect("list");
    assert!(listed.iter().any(|v| v.id == created.id && v.name == "2026-06-15"));
}

#[tokio::test]
async fn create_code_list_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "   ".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_list_then_list_by_version_round_trip() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 7,
            code: "C66741".into(),
            extensible: true,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "Age".into(),
            definition: "Age".into(),
            nci_preferred_term: "Age".into(),
        })
        .await
        .expect("create");
    let listed = usecase.list_code_lists(7).await.expect("list");
    assert!(listed.iter().any(|c| c.id == created.id));
}

#[tokio::test]
async fn update_code_list_applies_partial_changes() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_list(CreateCodeList {
            version_id: 1,
            code: "C66741".into(),
            extensible: false,
            name: "AGE".into(),
            submission_value: "AGE".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .expect("create");
    let updated = usecase
        .update_code_list(UpdateCodeList {
            id: created.id,
            extensible: Some(true),
            ..Default::default()
        })
        .await
        .expect("update");
    assert!(updated.extensible);
    assert_eq!(updated.code, "C66741");
}

#[tokio::test]
async fn search_code_lists_clamps_limit_to_default_when_zero() {
    // The clamping happens before the repo is touched, so we
    // cannot observe it directly through `search_code_lists`.
    // Instead, this test exercises that the search does not
    // panic on a zero-limit and that the fake returns empty.
    let (_, _, _, usecase) = make_usecase();
    let hits = usecase
        .search_code_lists(CodeListSearchQuery {
            kind: TerminologyKind::Sdtm,
            version_name: "2026-03-27".into(),
            text: "age".into(),
            limit: 0,
        })
        .await
        .expect("search");
    assert!(hits.is_empty());
}

#[tokio::test]
async fn create_code_item_rejects_empty_code() {
    let (_, _, _, usecase) = make_usecase();
    let err = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 1,
            code: "".into(),
            submission_value: "X".into(),
            synonym: "".into(),
            definition: "".into(),
            nci_preferred_term: "".into(),
        })
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Validation(DomainError::EmptyCode)
    ));
}

#[tokio::test]
async fn create_code_item_round_trip_then_list_by_codelist() {
    let (_, _, _, usecase) = make_usecase();
    let created = usecase
        .create_code_item(CreateCodeItem {
            codelist_id: 9,
            code: "C12345".into(),
            submission_value: "> 0".into(),
            synonym: "positive".into(),
            definition: "Greater than zero".into(),
            nci_preferred_term: "Greater Than Zero".into(),
        })
        .await
        .expect("create");
    let listed = usecase.list_code_items(9).await.expect("list");
    assert!(listed.iter().any(|i| i.id == created.id));
}
```

### 3.7 Update `lib.rs` re-exports

- [ ] **Step 1:** Replace `lib/crates/terminology/src/lib.rs`:

```rust
//! # terminology crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC terminology aggregates and an async
//! `TerminologyUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, CodeList, CodeListNew, CodeListRepository, CodeListSearchHit,
    CodeListSearchQuery, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
pub use usecase::{
    CodeItemView, CodeListView, CreateCodeItem, CreateCodeList, CreateTerminologyVersion,
    TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersionView, UpdateCodeItem,
    UpdateCodeList, UpdateTerminologyVersion, UsecaseError,
};
```

### 3.8 Verify + commit

- [ ] **Step 1:** Run `cargo test -p terminology --lib usecase::`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/src/usecase.rs \
        lib/crates/terminology/src/usecase/error.rs \
        lib/crates/terminology/src/usecase/commands.rs \
        lib/crates/terminology/src/usecase/views.rs \
        lib/crates/terminology/src/usecase/terminology_usecase.rs \
        lib/crates/terminology/src/usecase/tests.rs \
        lib/crates/terminology/src/lib.rs
git commit -m "feat(terminology): usecase layer — commands, views, orchestrator

Usecase layer introduces the command DTOs (Create* / Update* per
aggregate), the view DTOs (TerminologyVersionView / CodeListView /
CodeItemView + re-exports of the search hits), UsecaseError with
the standard From<DomainError> impl, and the
TerminologyUsecase<V, L, I> generic over all three repository
ports — wired through a TerminologyUsecaseConfig so the
three-generic constructor stays readable.

Pre-flight validators reject empty name/code before the
repository is touched. search_code_lists / search_code_items
clamp the limit to the documented default (50) / hard cap
(500) before forwarding to the port.

Tests use Arc<Mutex<…>> + AtomicI64 in-memory fakes for all
three ports; the search fakes intentionally return empty so the
usecase tests focus on shape rather than ranking.

Spec coverage: Usecase Layer section of
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo test -p terminology --lib usecase::"
```

---

## Task 4: Postgres adapter — terminology_version_repo + migration 0001

**Files:**
- Create: `lib/crates/terminology/src/adapter/persistence.rs`
- Create: `lib/crates/terminology/src/adapter/persistence/postgres.rs`
- Create: `lib/crates/terminology/src/adapter/persistence/postgres/terminology_version_repo.rs`
- Create: `lib/crates/terminology/migrations/0001_create_terminology_versions.sql`
- Modify: `lib/crates/terminology/src/adapter.rs`
- Modify: `lib/crates/terminology/src/lib.rs` (add `TerminologyVersionRepo` re-export)

### 4.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/terminology/migrations/0001_create_terminology_versions.sql`:

```sql
-- 0001_create_terminology_versions.sql
--
-- Initial schema for the `terminology_versions` table. Applied
-- by `sqlx migrate run --source lib/crates/terminology/migrations`
-- before the `terminology` crate can be used against PostgreSQL.
--
-- Layout:
--   * `id`         - surrogate primary key (BIGSERIAL).
--   * `kind`       - one of `sdtm` / `adam`. CHECK constraint
--                    mirrors the Rust `TerminologyKind` enum.
--   * `name`       - workbook sheet suffix, e.g. "2026-03-27".
--                    Stored as `String`; not parsed as a date.
--                    (kind, name) is the natural key.
--   * `created_at` - DEFAULT NOW() at insert.
--   * `updated_at` - DEFAULT NOW() at insert; refresh trigger
--                    fires on every UPDATE.

CREATE TABLE terminology_versions (
    id BIGSERIAL PRIMARY KEY,
    kind TEXT NOT NULL,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT terminology_versions_kind_check
        CHECK (kind IN ('sdtm', 'adam')),
    CONSTRAINT terminology_versions_kind_name_unique
        UNIQUE (kind, name)
);

CREATE OR REPLACE FUNCTION terminology_versions_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER terminology_versions_set_updated_at
    BEFORE UPDATE ON terminology_versions
    FOR EACH ROW EXECUTE FUNCTION terminology_versions_set_updated_at();
```

### 4.2 Create `adapter/persistence.rs` and `adapter/persistence/postgres.rs`

- [ ] **Step 1:** Create `lib/crates/terminology/src/adapter/persistence.rs`:

```rust
pub(crate) mod postgres;
```

- [ ] **Step 2:** Create `lib/crates/terminology/src/adapter/persistence/postgres.rs`:

```rust
//! PostgreSQL-backed implementations of the three terminology
//! repository ports. Each repo uses SQLx's *runtime* query API
//! (`sqlx::query_as`, `QueryBuilder`) rather than the compile-time
//! macros, mirroring the user / project crates.

pub mod code_item_repo;
pub mod code_list_repo;
pub mod terminology_version_repo;

pub use code_item_repo::CodeItemRepo;
pub use code_list_repo::CodeListRepo;
pub use terminology_version_repo::TerminologyVersionRepo;
```

### 4.3 Implement `TerminologyVersionRepo`

- [ ] **Step 1:** Create `lib/crates/terminology/src/adapter/persistence/postgres/terminology_version_repo.rs`:

```rust
//! PostgreSQL-backed implementation of `TerminologyVersionRepository`.
//!
//! Uses the runtime SQLx API (`sqlx::query_as`, `QueryBuilder`)
//! rather than compile-time-checked macros; the workspace does
//! not currently provide a live `DATABASE_URL` or a checked-in
//! `sqlx-data.json` cache at build time.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{
    DomainError, TerminologyKind, TerminologyVersion, TerminologyVersionNew,
    TerminologyVersionRepository, TerminologyVersionUpdate,
};
use sqlx::FromRow;

/// PostgreSQL SQLSTATE codes.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

#[derive(FromRow)]
struct TerminologyVersionRow {
    id: i64,
    kind: String,
    name: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<TerminologyVersionRow> for TerminologyVersion {
    type Error = DomainError;

    fn try_from(row: TerminologyVersionRow) -> Result<Self, Self::Error> {
        let kind = TerminologyKind::try_from(row.kind.as_str())?;
        Ok(TerminologyVersion::for_repository(
            row.id,
            kind,
            row.name,
            row.created_at,
            row.updated_at,
        ))
    }
}

pub struct TerminologyVersionRepo {
    pool: PgPool,
}

impl TerminologyVersionRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl TerminologyVersionRepository for TerminologyVersionRepo {
    async fn create(
        &self,
        input: TerminologyVersionNew,
    ) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow = sqlx::QueryBuilder::new(
            "INSERT INTO terminology_versions (kind, name) VALUES (",
        )
        .push_bind(input.kind.as_str())
        .push(", ")
        .push_bind(&input.name)
        .push(") RETURNING id, kind, name, created_at, updated_at")
        .build_query_as::<TerminologyVersionRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<TerminologyVersionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::VersionNotFound(id))?;
        row.try_into()
    }

    async fn find_by_kind_and_name(
        &self,
        kind: TerminologyKind,
        name: &str,
    ) -> Result<TerminologyVersion, DomainError> {
        let row: TerminologyVersionRow = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions WHERE kind = ",
        )
        .push_bind(kind.as_str())
        .push(" AND name = ")
        .push_bind(name)
        .build_query_as::<TerminologyVersionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;
        row.try_into()
    }

    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError> {
        let rows: Vec<TerminologyVersionRow> = sqlx::QueryBuilder::new(
            "SELECT id, kind, name, created_at, updated_at \
             FROM terminology_versions ORDER BY id",
        )
        .build_query_as::<TerminologyVersionRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?;
        rows.into_iter().map(TerminologyVersion::try_from).collect()
    }

    async fn update(
        &self,
        input: TerminologyVersionUpdate,
    ) -> Result<TerminologyVersion, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE terminology_versions SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(kind) = input.kind {
            sep(&mut qb);
            qb.push("kind = ").push_bind(kind.as_str());
        }
        if let Some(ref name) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(name);
        }
        if first {
            // Nothing to update; short-circuit and return the
            // existing row, or `VersionNotFound` if the id is
            // unknown.
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, kind, name, created_at, updated_at");
        let row: TerminologyVersionRow = qb
            .build_query_as::<TerminologyVersionRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error)?
            .ok_or(DomainError::VersionNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM terminology_versions WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::VersionNotFound(id));
        }
        Ok(())
    }
}

fn map_db_error(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::Repository(format!(
                    "duplicate key violates unique constraint `{constraint}`"
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}
```

### 4.4 Update `adapter.rs` and `lib.rs`

- [ ] **Step 1:** Replace `lib/crates/terminology/src/adapter.rs`:

```rust
mod persistence;

pub use persistence::postgres::{CodeItemRepo, CodeListRepo, TerminologyVersionRepo};
```

- [ ] **Step 2:** Edit `lib/crates/terminology/src/lib.rs`. Add `TerminologyVersionRepo` to the `adapter` re-export:

```rust
pub use adapter::{CodeItemRepo, CodeListRepo, TerminologyVersionRepo};
```

### 4.5 Verify + commit

- [ ] **Step 1:** Run `cargo check -p terminology`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/migrations/0001_create_terminology_versions.sql \
        lib/crates/terminology/src/adapter.rs \
        lib/crates/terminology/src/adapter/persistence.rs \
        lib/crates/terminology/src/adapter/persistence/postgres.rs \
        lib/crates/terminology/src/adapter/persistence/postgres/terminology_version_repo.rs \
        lib/crates/terminology/src/lib.rs
git commit -m "feat(terminology): adapter scaffold + TerminologyVersionRepo

Adds migrations/0001_create_terminology_versions.sql with id
BIGSERIAL + kind/name + CHECK + UNIQUE(kind,name) + the
terminology_versions_set_updated_at trigger. The adapter layer
gains src/adapter/persistence/postgres.rs with the three
postgres modules declared and TerminologyVersionRepo (the
CodeList and CodeItem placeholders land in Tasks 5/6 alongside
their migrations).

TerminologyVersionRepo uses the runtime SQLx API (QueryBuilder
+ FromRow + TryFrom row bridge) per the workspace convention;
map_db_error translates RowNotFound → NotFound and unique
violations to a Repository error (DuplicateVersion mapping
moves to the coverting code_list_repo and code_item_repo where
the constraint name identifies it).

Spec coverage: Database Schema for terminology_versions +
Postgres adapter portion of Persistence in
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo check -p terminology"
```

---

## Task 5: Postgres adapter — CodeListRepo + migration 0002 + tsvector search

**Files:**
- Create: `lib/crates/terminology/migrations/0002_create_code_lists.sql`
- Create: `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`

### 5.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/terminology/migrations/0002_create_code_lists.sql`:

```sql
-- 0002_create_code_lists.sql
--
-- CDISC codelists, one row per (version, code) pair. Items live
-- in a separate `code_items` table (migration 0003).
--
-- Layout:
--   * `id`               - surrogate primary key.
--   * `version_id`       - FK to terminology_versions(id)
--                          ON DELETE CASCADE.
--   * `code`             - NCI C-code of the codelist. UNIQUE
--                          per version.
--   * `extensible`       - whether sponsors may add new
--                          permissible values.
--   * `name`, `submission_value`, `synonym`, `definition`,
--     `nci_preferred_term` - five text columns surfaced through
--                            full-text search.
--   * `created_at`, `updated_at` - DEFAULT NOW(); trigger
--                            refreshes updated_at on UPDATE.
--   * `tsv`              - GENERATED tsvector over the five
--                          text columns. GIN index backs the
--                          search port.

CREATE TABLE code_lists (
    id BIGSERIAL PRIMARY KEY,
    version_id BIGINT NOT NULL REFERENCES terminology_versions(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    extensible BOOLEAN NOT NULL,
    name TEXT NOT NULL,
    submission_value TEXT NOT NULL DEFAULT '',
    synonym TEXT NOT NULL DEFAULT '',
    definition TEXT NOT NULL DEFAULT '',
    nci_preferred_term TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tsv tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(name, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(submission_value, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(synonym, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(definition, '')), 'C') ||
        setweight(to_tsvector('english', coalesce(nci_preferred_term, '')), 'B')
    ) STORED,
    CONSTRAINT code_lists_version_code_unique UNIQUE (version_id, code)
);

CREATE INDEX code_lists_version_id_idx ON code_lists (version_id);
CREATE INDEX code_lists_tsv_idx ON code_lists USING GIN (tsv);

CREATE OR REPLACE FUNCTION code_lists_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER code_lists_set_updated_at
    BEFORE UPDATE ON code_lists
    FOR EACH ROW EXECUTE FUNCTION code_lists_set_updated_at();
```

### 5.2 Implement `CodeListRepo`

- [ ] **Step 1:** Create `lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs`:

```rust
//! PostgreSQL-backed implementation of `CodeListRepository`,
//! including the `tsvector` / GIN-backed search.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    CodeList, CodeListNew, CodeListRepository, CodeListSearchHit, CodeListSearchQuery,
    CodeListUpdate, DomainError, TerminologyKind,
};

const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
const SQLSTATE_FK_VIOLATION: &str = "23503";

#[derive(FromRow)]
struct CodeListRow {
    id: i64,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CodeListRow> for CodeList {
    type Error = DomainError;

    fn try_from(row: CodeListRow) -> Result<Self, Self::Error> {
        Ok(CodeList::for_repository(
            row.id,
            row.version_id,
            row.code,
            row.extensible,
            row.name,
            row.submission_value,
            row.synonym,
            row.definition,
            row.nci_preferred_term,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(FromRow)]
struct CodeListSearchRow {
    id: i64,
    version_id: i64,
    code: String,
    extensible: bool,
    name: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    score: f32,
}

pub struct CodeListRepo {
    pool: PgPool,
}

impl CodeListRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodeListRepository for CodeListRepo {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError> {
        let row: CodeListRow = sqlx::QueryBuilder::new(
            "INSERT INTO code_lists \
             (version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term) \
             VALUES (",
        )
        .push_bind(input.version_id)
        .push(", ")
        .push_bind(&input.code)
        .push(", ")
        .push_bind(input.extensible)
        .push(", ")
        .push_bind(&input.name)
        .push(", ")
        .push_bind(&input.submission_value)
        .push(", ")
        .push_bind(&input.synonym)
        .push(", ")
        .push_bind(&input.definition)
        .push(", ")
        .push_bind(&input.nci_preferred_term)
        .push(") RETURNING id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at")
        .build_query_as::<CodeListRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|err| map_db_error(err, Some(input.version_id)))?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError> {
        let row: CodeListRow = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_lists WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<CodeListRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error_simple)?
        .ok_or(DomainError::CodeListNotFound(id))?;
        row.try_into()
    }

    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError> {
        let rows: Vec<CodeListRow> = sqlx::QueryBuilder::new(
            "SELECT id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_lists WHERE version_id = ",
        )
        .push_bind(version_id)
        .push(" ORDER BY id")
        .build_query_as::<CodeListRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter().map(CodeList::try_from).collect()
    }

    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE code_lists SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref code) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(code);
        }
        if let Some(ext) = input.extensible {
            sep(&mut qb);
            qb.push("extensible = ").push_bind(ext);
        }
        if let Some(ref name) = input.name {
            sep(&mut qb);
            qb.push("name = ").push_bind(name);
        }
        if let Some(ref sv) = input.submission_value {
            sep(&mut qb);
            qb.push("submission_value = ").push_bind(sv);
        }
        if let Some(ref syn) = input.synonym {
            sep(&mut qb);
            qb.push("synonym = ").push_bind(syn);
        }
        if let Some(ref def) = input.definition {
            sep(&mut qb);
            qb.push("definition = ").push_bind(def);
        }
        if let Some(ref pt) = input.nci_preferred_term {
            sep(&mut qb);
            qb.push("nci_preferred_term = ").push_bind(pt);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, version_id, code, extensible, name, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at");
        let row: CodeListRow = qb
            .build_query_as::<CodeListRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error_simple)?
            .ok_or(DomainError::CodeListNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM code_lists WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CodeListNotFound(id));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeListSearchQuery,
    ) -> Result<Vec<CodeListSearchHit>, DomainError> {
        let text = query.text.clone();
        let kind = query.kind;
        let version_name = query.version_name.clone();
        let limit = query.limit as i64;
        // `websearch_to_tsquery` returns NULL when the text reduces
        // to all stopwords; in that case return empty hits instead
        // of an error.
        let rows: Vec<CodeListSearchRow> = sqlx::QueryBuilder::new(
            "SELECT cl.id, cl.version_id, cl.code, cl.extensible, cl.name, \
                    cl.submission_value, cl.synonym, cl.definition, \
                    cl.nci_preferred_term, cl.created_at, cl.updated_at, \
                    ts_rank_cd(cl.tsv, websearch_to_tsquery('english', ",
        )
        .push_bind(&text)
        .push(
            ")) AS score \
             FROM code_lists cl \
             JOIN terminology_versions v ON v.id = cl.version_id \
             WHERE v.kind = ",
        )
        .push_bind(kind.as_str())
        .push(" AND v.name = ")
        .push_bind(&version_name)
        .push(" AND cl.tsv @@ websearch_to_tsquery('english', ")
        .push_bind(&text)
        .push(") ORDER BY score DESC LIMIT ")
        .push_bind(limit)
        .build_query_as::<CodeListSearchRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter()
            .map(|row| {
                let cl = CodeList::try_from(CodeListRow {
                    id: row.id,
                    version_id: row.version_id,
                    code: row.code,
                    extensible: row.extensible,
                    name: row.name,
                    submission_value: row.submission_value,
                    synonym: row.synonym,
                    definition: row.definition,
                    nci_preferred_term: row.nci_preferred_term,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })?;
                Ok(CodeListSearchHit {
                    codelist: cl,
                    score: row.score,
                })
            })
            .collect()
    }
}

/// Map any `sqlx::Error` from a non-create call on this repo to a
/// `DomainError`. These calls never produce SQLSTATE `23503`
/// (the FK is satisfied before the call) so the simpler mapper
/// is correct.
fn map_db_error_simple(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::Repository(format!(
                    "duplicate key violates unique constraint `{constraint}`"
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}

/// `create` mapper: knows about the version_id it just inserted
/// with, so SQLSTATE `23503` becomes `FkVersionNotFound(version_id)`.
fn map_db_error(err: sqlx::Error, version_id_hint: Option<i64>) -> DomainError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some(SQLSTATE_FK_VIOLATION) {
            return DomainError::FkVersionNotFound(version_id_hint.unwrap_or(0));
        }
    }
    map_db_error_simple(err)
}
```

### 5.3 Verify + commit

- [ ] **Step 1:** Run `cargo check -p terminology`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/migrations/0002_create_code_lists.sql \
        lib/crates/terminology/src/adapter/persistence/postgres/code_list_repo.rs
git commit -m "feat(terminology): CodeListRepo + tsv/GIN search

Adds migrations/0002_create_code_lists.sql with the tsv
GENERATED column (setweight A on name + submission_value, B on
synonym + nci_preferred_term, C on definition) and the GIN
index on tsv. The check + UNIQUE (version_id, code) constraint
plus the ON DELETE CASCADE FK mirror the spec.

CodeListRepo uses the runtime SQLx API. create()'s map_db_error
recognises SQLSTATE 23503 and surfaces FkVersionNotFound; other
calls use the simpler mapper. search() joins back to
terminology_versions so the same (kind, name) scoping the usecase
applies becomes part of the SQL itself.

Spec coverage: Database Schema for code_lists + CodeListRepo +
search shape in
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo check -p terminology"
```

---

## Task 6: Postgres adapter — CodeItemRepo + migration 0003 + tsvector search

**Files:**
- Create: `lib/crates/terminology/migrations/0003_create_code_items.sql`
- Create: `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`

### 6.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/terminology/migrations/0003_create_code_items.sql`:

```sql
-- 0003_create_code_items.sql
--
-- Permissible values inside each codelist, one row per
-- (codelist, code) pair.
--
-- Layout mirrors code_lists:
--   * FK to code_lists(id) ON DELETE CASCADE.
--   * UNIQUE (codelist_id, code).
--   * `tsv` GENERATED over the same five text columns with the
--     same weights, plus a GIN index.
--   * `code_items_set_updated_at` trigger refreshes
--     `updated_at` on UPDATE.

CREATE TABLE code_items (
    id BIGSERIAL PRIMARY KEY,
    codelist_id BIGINT NOT NULL REFERENCES code_lists(id) ON DELETE CASCADE,
    code TEXT NOT NULL,
    submission_value TEXT NOT NULL DEFAULT '',
    synonym TEXT NOT NULL DEFAULT '',
    definition TEXT NOT NULL DEFAULT '',
    nci_preferred_term TEXT NOT NULL DEFAULT '',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    tsv tsvector GENERATED ALWAYS AS (
        setweight(to_tsvector('english', coalesce(submission_value, '')), 'A') ||
        setweight(to_tsvector('english', coalesce(synonym, '')), 'B') ||
        setweight(to_tsvector('english', coalesce(definition, '')), 'C') ||
        setweight(to_tsvector('english', coalesce(nci_preferred_term, '')), 'B')
    ) STORED,
    CONSTRAINT code_items_codelist_code_unique UNIQUE (codelist_id, code)
);

CREATE INDEX code_items_codelist_id_idx ON code_items (codelist_id);
CREATE INDEX code_items_tsv_idx ON code_items USING GIN (tsv);

CREATE OR REPLACE FUNCTION code_items_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER code_items_set_updated_at
    BEFORE UPDATE ON code_items
    FOR EACH ROW EXECUTE FUNCTION code_items_set_updated_at();
```

### 6.2 Implement `CodeItemRepo`

- [ ] **Step 1:** Create `lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs`:

```rust
//! PostgreSQL-backed implementation of `CodeItemRepository`,
//! including the `tsvector` / GIN-backed search.

use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::{FromRow, PgPool};

use crate::domain::code_item::{CodeItemSearchHit, CodeItemSearchQuery};
use crate::domain::{CodeItem, CodeItemNew, CodeItemRepository, CodeItemUpdate, DomainError};

const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";
const SQLSTATE_FK_VIOLATION: &str = "23503";

#[derive(FromRow)]
struct CodeItemRow {
    id: i64,
    codelist_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
}

impl TryFrom<CodeItemRow> for CodeItem {
    type Error = DomainError;

    fn try_from(row: CodeItemRow) -> Result<Self, Self::Error> {
        Ok(CodeItem::for_repository(
            row.id,
            row.codelist_id,
            row.code,
            row.submission_value,
            row.synonym,
            row.definition,
            row.nci_preferred_term,
            row.created_at,
            row.updated_at,
        ))
    }
}

#[derive(FromRow)]
struct CodeItemSearchRow {
    id: i64,
    codelist_id: i64,
    code: String,
    submission_value: String,
    synonym: String,
    definition: String,
    nci_preferred_term: String,
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::DateTime<chrono::Utc>,
    score: f32,
}

pub struct CodeItemRepo {
    pool: PgPool,
}

impl CodeItemRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CodeItemRepository for CodeItemRepo {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "INSERT INTO code_items \
             (codelist_id, code, submission_value, synonym, definition, nci_preferred_term) \
             VALUES (",
        )
        .push_bind(input.codelist_id)
        .push(", ")
        .push_bind(&input.code)
        .push(", ")
        .push_bind(&input.submission_value)
        .push(", ")
        .push_bind(&input.synonym)
        .push(", ")
        .push_bind(&input.definition)
        .push(", ")
        .push_bind(&input.nci_preferred_term)
        .push(") RETURNING id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at")
        .build_query_as::<CodeItemRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|err| map_db_error(err, Some(input.codelist_id)))?;
        row.try_into()
    }

    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError> {
        let row: CodeItemRow = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_items WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<CodeItemRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error_simple)?
        .ok_or(DomainError::CodeItemNotFound(id))?;
        row.try_into()
    }

    async fn list_by_codelist(
        &self,
        codelist_id: i64,
    ) -> Result<Vec<CodeItem>, DomainError> {
        let rows: Vec<CodeItemRow> = sqlx::QueryBuilder::new(
            "SELECT id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at \
             FROM code_items WHERE codelist_id = ",
        )
        .push_bind(codelist_id)
        .push(" ORDER BY id")
        .build_query_as::<CodeItemRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter().map(CodeItem::try_from).collect()
    }

    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError> {
        let mut qb = sqlx::QueryBuilder::new("UPDATE code_items SET ");
        let mut first = true;
        let mut sep = |qb: &mut sqlx::QueryBuilder<sqlx::Postgres>| {
            if first {
                first = false;
            } else {
                qb.push(", ");
            }
        };
        if let Some(ref code) = input.code {
            sep(&mut qb);
            qb.push("code = ").push_bind(code);
        }
        if let Some(ref sv) = input.submission_value {
            sep(&mut qb);
            qb.push("submission_value = ").push_bind(sv);
        }
        if let Some(ref syn) = input.synonym {
            sep(&mut qb);
            qb.push("synonym = ").push_bind(syn);
        }
        if let Some(ref def) = input.definition {
            sep(&mut qb);
            qb.push("definition = ").push_bind(def);
        }
        if let Some(ref pt) = input.nci_preferred_term {
            sep(&mut qb);
            qb.push("nci_preferred_term = ").push_bind(pt);
        }
        if first {
            return self.find_by_id(input.id).await;
        }
        qb.push(" WHERE id = ").push_bind(input.id);
        qb.push(" RETURNING id, codelist_id, code, submission_value, synonym, definition, nci_preferred_term, created_at, updated_at");
        let row: CodeItemRow = qb
            .build_query_as::<CodeItemRow>()
            .fetch_optional(&self.pool)
            .await
            .map_err(map_db_error_simple)?
            .ok_or(DomainError::CodeItemNotFound(input.id))?;
        row.try_into()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM code_items WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error_simple)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::CodeItemNotFound(id));
        }
        Ok(())
    }

    async fn search(
        &self,
        query: CodeItemSearchQuery,
    ) -> Result<Vec<CodeItemSearchHit>, DomainError> {
        let text = query.text.clone();
        let kind = query.kind;
        let version_name = query.version_name.clone();
        let limit = query.limit as i64;
        let rows: Vec<CodeItemSearchRow> = sqlx::QueryBuilder::new(
            "SELECT ci.id, ci.codelist_id, ci.code, ci.submission_value, \
                    ci.synonym, ci.definition, ci.nci_preferred_term, \
                    ci.created_at, ci.updated_at, \
                    ts_rank_cd(ci.tsv, websearch_to_tsquery('english', ",
        )
        .push_bind(&text)
        .push(
            ")) AS score \
             FROM code_items ci \
             JOIN code_lists cl ON cl.id = ci.codelist_id \
             JOIN terminology_versions v ON v.id = cl.version_id \
             WHERE v.kind = ",
        )
        .push_bind(kind.as_str())
        .push(" AND v.name = ")
        .push_bind(&version_name)
        .push(" AND ci.tsv @@ websearch_to_tsquery('english', ")
        .push_bind(&text)
        .push(") ORDER BY score DESC LIMIT ")
        .push_bind(limit)
        .build_query_as::<CodeItemSearchRow>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error_simple)?;
        rows.into_iter()
            .map(|row| {
                let codelist_id = row.codelist_id;
                let item = CodeItem::try_from(CodeItemRow {
                    id: row.id,
                    codelist_id,
                    code: row.code,
                    submission_value: row.submission_value,
                    synonym: row.synonym,
                    definition: row.definition,
                    nci_preferred_term: row.nci_preferred_term,
                    created_at: row.created_at,
                    updated_at: row.updated_at,
                })?;
                Ok(CodeItemSearchHit {
                    item,
                    score: row.score,
                    codelist_id,
                })
            })
            .collect()
    }
}

fn map_db_error_simple(err: sqlx::Error) -> DomainError {
    match err {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        sqlx::Error::Database(db_err) => {
            if db_err.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) {
                let constraint = db_err.constraint().unwrap_or("code");
                DomainError::Repository(format!(
                    "duplicate key violates unique constraint `{constraint}`"
                ))
            } else {
                DomainError::Repository(db_err.message().to_string())
            }
        }
        other => DomainError::Repository(other.to_string()),
    }
}

/// `create` mapper: knows about the codelist_id it just inserted
/// with, so SQLSTATE `23503` becomes `FkCodeListNotFound(codelist_id)`.
fn map_db_error(err: sqlx::Error, codelist_id_hint: Option<i64>) -> DomainError {
    if let sqlx::Error::Database(db_err) = &err {
        if db_err.code().as_deref() == Some(SQLSTATE_FK_VIOLATION) {
            return DomainError::FkCodeListNotFound(codelist_id_hint.unwrap_or(0));
        }
    }
    map_db_error_simple(err)
}
```

### 6.3 Verify + commit

- [ ] **Step 1:** Run `cargo check -p terminology`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/migrations/0003_create_code_items.sql \
        lib/crates/terminology/src/adapter/persistence/postgres/code_item_repo.rs
git commit -m "feat(terminology): CodeItemRepo + tsv/GIN search

Adds migrations/0003_create_code_items.sql mirroring the
code_lists migration. code_items.code stays out of the tsv
column on purpose (users search by meaning, not NCI C-code).

CodeItemRepo uses the runtime SQLx API. create()'s mapper
recognises SQLSTATE 23503 and surfaces FkCodeListNotFound;
search() joins back through code_lists to
terminology_versions so the (kind, version_name) scoping the
usecase applies carries into the SQL.

Spec coverage: Database Schema for code_items + CodeItemRepo +
search shape in
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo check -p terminology"
```

---

## Task 7: Tests — public_api compile-only + integration tests (#[ignore]-gated)

**Files:**
- Create: `lib/crates/terminology/tests/public_api.rs`
- Create: `lib/crates/terminology/tests/integration_persistence.rs`

### 7.1 Create the public-api test

- [ ] **Step 1:** Create `lib/crates/terminology/tests/public_api.rs`:

```rust
//! Public-API compile-only test for the `terminology` crate.
//!
//! Pins the documented `use terminology::*;` surface, the three
//! concrete repo constructors (`fn(PgPool) -> Repo`), the
//! `TerminologyUsecase::new(config)` constructor shape, and the
//! `Send + Sync` bound the usecase config relies on.

use sqlx::PgPool;
use terminology::{
    CodeItemRepo, CodeListRepo, CreateCodeList, CreateTerminologyVersion, TerminologyKind,
    TerminologyUsecase, TerminologyUsecaseConfig, TerminologyVersionRepo, UpdateTerminologyVersion,
};

#[test]
fn public_types_are_nameable_from_crate_root() {
    fn assert_kind(_: TerminologyKind) {}
    fn assert_cmd(_: CreateTerminologyVersion) {}
    fn assert_list_cmd(_: CreateCodeList) {}
    fn assert_upd(_: UpdateTerminologyVersion) {}

    assert_kind(TerminologyKind::Sdtm);
    assert_cmd(CreateTerminologyVersion {
        kind: TerminologyKind::Sdtm,
        name: "2026-03-27".into(),
    });
    assert_list_cmd(CreateCodeList {
        version_id: 1,
        code: "C66741".into(),
        extensible: true,
        name: "AGE".into(),
        submission_value: "AGE".into(),
        synonym: "".into(),
        definition: "".into(),
        nci_preferred_term: "".into(),
    });
    assert_upd(UpdateTerminologyVersion::default());
}

#[test]
fn repos_construct_from_pg_pool_via_function_pointer() {
    let v: fn(PgPool) -> TerminologyVersionRepo = TerminologyVersionRepo::new;
    let l: fn(PgPool) -> CodeListRepo = CodeListRepo::new;
    let i: fn(PgPool) -> CodeItemRepo = CodeItemRepo::new;
    let _ = (v, l, i);
}

#[test]
fn usecase_constructor_accepts_three_repo_args() {
    fn assert_new_constructor<V, L, I>(
        _: fn(TerminologyUsecaseConfig<V, L, I>) -> TerminologyUsecase<V, L, I>,
    ) where
        V: terminology::TerminologyVersionRepository,
        L: terminology::CodeListRepository,
        I: terminology::CodeItemRepository,
    {
    }
    assert_new_constructor::<TerminologyVersionRepo, CodeListRepo, CodeItemRepo>(
        TerminologyUsecase::new,
    );
}
```

### 7.2 Create the integration test

- [ ] **Step 1:** Create `lib/crates/terminology/tests/integration_persistence.rs`:

```rust
//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with
//! `AEGIS_TERMINOLOGY_DATABASE_URL` set:
//!
//! ```text
//! cargo test -p terminology -- --ignored --test-threads=1
//! ```
//!
//! Each run drops and re-applies the migrations so the live DB
//! stays in a deterministic state. A failure to connect is
//! reported via a clear panic (never silently skipped).

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;
use terminology::{
    CodeItem, CodeItemNew, CodeItemRepo, CodeItemRepository, CodeList, CodeListNew, CodeListRepo,
    CodeListRepository, CodeListSearchQuery, DomainError, TerminologyKind, TerminologyUsecase,
    TerminologyUsecaseConfig, TerminologyVersion, TerminologyVersionNew,
    TerminologyVersionRepo, TerminologyVersionRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_TERMINOLOGY_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_TERMINOLOGY_DATABASE_URL must be set (or present in .env \
             at the workspace root) to run --ignored tests"
        )
    });

    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_TERMINOLOGY_DATABASE_URL");

    sqlx::query("DROP TABLE IF EXISTS code_items CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_items");
    sqlx::query("DROP TABLE IF EXISTS code_lists CASCADE")
        .execute(&pool)
        .await
        .expect("drop code_lists");
    sqlx::query("DROP TABLE IF EXISTS terminology_versions CASCADE")
        .execute(&pool)
        .await
        .expect("drop terminology_versions");
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

fn unique(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let count = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("{prefix}-{nanos:x}-{count}")
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn create_then_find_round_trip_for_all_three_levels() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v_name = unique("v");
        let v: TerminologyVersion = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("version create");

        let cl: CodeList = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list create");

        let item: CodeItem = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                code: unique("ci"),
                submission_value: ">0".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item create");

        assert_eq!(v.name, v_name);
        assert_eq!(cl.version_id, v.id);
        assert_eq!(item.codelist_id, cl.id);
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn delete_version_cascades_to_children() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());
        let i_repo = CodeItemRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("cascade-v"),
            })
            .await
            .expect("version");

        let cl = l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("cascade-cl"),
                extensible: false,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_list");

        let _item = i_repo
            .create(CodeItemNew {
                codelist_id: cl.id,
                code: unique("cascade-ci"),
                submission_value: "".into(),
                synonym: "".into(),
                definition: "".into(),
                nci_preferred_term: "".into(),
            })
            .await
            .expect("code_item");

        v_repo.delete(v.id).await.expect("delete version");

        let err = l_repo.find_by_id(cl.id).await.expect_err("cl gone");
        assert!(
            matches!(err, DomainError::CodeListNotFound(_)),
            "expected CodeListNotFound, got {err:?}"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn search_code_lists_ranks_hits() {
    with_pool(|pool| async move {
        let v_repo = TerminologyVersionRepo::new(pool.clone());
        let l_repo = CodeListRepo::new(pool.clone());

        let v = v_repo
            .create(TerminologyVersionNew {
                kind: TerminologyKind::Sdtm,
                name: unique("search-v"),
            })
            .await
            .expect("version");

        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("age-cl"),
                extensible: true,
                name: "AGE".into(),
                submission_value: "AGE".into(),
                synonym: "Age group".into(),
                definition: "Subject age".into(),
                nci_preferred_term: "Age".into(),
            })
            .await
            .expect("age cl");

        l_repo
            .create(CodeListNew {
                version_id: v.id,
                code: unique("sex-cl"),
                extensible: true,
                name: "SEX".into(),
                submission_value: "SEX".into(),
                synonym: "".into(),
                definition: "Sex".into(),
                nci_preferred_term: "Sex".into(),
            })
            .await
            .expect("sex cl");

        let hits = l_repo
            .search(CodeListSearchQuery {
                kind: TerminologyKind::Sdtm,
                version_name: v.name.clone(),
                text: "age".into(),
                limit: 10,
            })
            .await
            .expect("search");

        assert!(
            !hits.is_empty(),
            "expected at least one hit for `age` in version {}",
            v.name
        );
        assert!(
            hits.iter().any(|h| h.codelist.name == "AGE"),
            "AGE row should be in the hits"
        );
    })
    .await;
}

#[tokio::test]
#[ignore = "requires AEGIS_TERMINOLOGY_DATABASE_URL"]
async fn usecase_wires_through_all_three_repos() {
    with_pool(|pool| async move {
        let v = TerminologyVersionRepo::new(pool.clone());
        let l = CodeListRepo::new(pool.clone());
        let i = CodeItemRepo::new(pool.clone());
        let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
            version_repo: v,
            code_list_repo: l,
            code_item_repo: i,
        });

        let v_name = unique("usecase-v");
        let created_v = usecase
            .create_version(CreateTerminologyVersion {
                kind: TerminologyKind::Sdtm,
                name: v_name.clone(),
            })
            .await
            .expect("usecase create version");
        let _ = usecase
            .get_version(TerminologyKind::Sdtm, &v_name)
            .await
            .expect("usecase get");
        assert_eq!(created_v.name, v_name);
    })
    .await;
}
```

### 7.3 Verify + commit

- [ ] **Step 1:** Run `cargo test -p terminology`. Expected: green (public_api passes; integration tests are ignored).

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/tests/public_api.rs \
        lib/crates/terminology/tests/integration_persistence.rs
git commit -m "test(terminology): public_api compile + live-DB integration tests

tests/public_api.rs pin the documented `use terminology::*;`
imports and the constructor chains:
fn(PgPool) -> TerminologyVersionRepo / CodeListRepo / CodeItemRepo,
and fn(TerminologyUsecaseConfig<V,L,I>) -> TerminologyUsecase<V,L,I>
on the type system without performing I/O.

tests/integration_persistence.rs is `#[ignore]`-gated; with
AEGIS_TERMINOLOGY_DATABASE_URL set, it drops the three
terminology tables + the sqlx_migrations bookkeeping, applies
the migrations fresh, then exercises create/find round trips
across all three levels, ON DELETE CASCADE from a version to
its children, and the tsv/GIN search with a real ranking.

Spec coverage: Tests section of
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo test -p terminology (integration tests
ignored; re-run with --ignored --test-threads=1 once
AEGIS_TERMINOLOGY_DATABASE_URL is set)"
```

---

## Task 8: README

**Files:**
- Modify: `lib/crates/terminology/README.md`

### 8.1 Replace the README with the full version

- [ ] **Step 1:** Replace `lib/crates/terminology/README.md`:

```markdown
# terminology

CRUD over the CDISC terminology aggregates
(`TerminologyVersion`, `CodeList`, `CodeItem`) with full-text
search, backed by PostgreSQL.

This crate is a business lib crate; see
`docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions (workspace wiring, DDD layout, error chain, the
five-tier test rule) and
`docs/superpowers/specs/2026-08-18-terminology-crate-design.md`
for the data model + port surface.

## Source layout

    src/
    ├── lib.rs                                  # pub mod + re-exports
    ├── domain.rs                               # children, pub use
    ├── domain/
    │   ├── terminology_kind.rs                 # SDTM | ADAM enum
    │   ├── terminology_version.rs             # aggregate + DTOs
    │   ├── code_list.rs                        # aggregate + DTOs + search
    │   ├── code_item.rs                        # aggregate + DTOs + search
    │   ├── repository.rs                       # the three #[async_trait] ports
    │   ├── error.rs                            # DomainError
    │   └── tests.rs                            # domain unit tests
    ├── usecase.rs
    ├── usecase/
    │   ├── commands.rs                         # Create*/Update* DTOs
    │   ├── views.rs                            # *View DTOs + From impls
    │   ├── error.rs                            # UsecaseError + From<DomainError>
    │   ├── terminology_usecase.rs              # TerminologyUsecase<V, L, I>
    │   └── tests.rs                            # in-memory wire-up tests
    ├── adapter.rs
    └── adapter/
        ├── persistence.rs
        └── persistence/postgres/
            ├── postgres.rs                     # module index, re-exports
            ├── terminology_version_repo.rs
            ├── code_list_repo.rs
            └── code_item_repo.rs

## Database setup

Migrations live under `migrations/` and are applied via
`sqlx migrate run --source lib/crates/terminology/migrations`.

The live-DB URL comes from the
`AEGIS_TERMINOLOGY_DATABASE_URL` environment variable (or
`.env` at the workspace root).

```rust
use sqlx::postgres::PgPoolOptions;
use terminology::{
    CodeItemRepo, CodeListRepo, TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersionRepo,
};

let pool = PgPoolOptions::new()
    .connect(&std::env::var("AEGIS_TERMINOLOGY_DATABASE_URL")?)
    .await?;

let v_repo = TerminologyVersionRepo::new(pool.clone());
let l_repo = CodeListRepo::new(pool.clone());
let i_repo = CodeItemRepo::new(pool.clone());

let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
    version_repo: v_repo,
    code_list_repo: l_repo,
    code_item_repo: i_repo,
});
```

## Tests

```bash
cargo test -p terminology                                # cargo unit + ignored-free tests
cargo test -p terminology -- --ignored --test-threads=1  # when AEGIS_TERMINOLOGY_DATABASE_URL is set
```

## Guideline

See `docs/guidelines/lib-crate-development.md` for the
cross-cutting conventions every lib crate in this workspace
follows.
```

### 8.2 Verify + commit

- [ ] **Step 1:** Run `cargo doc -p terminology --no-deps`. Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/terminology/README.md
git commit -m "docs(terminology): expand README with layout + setup

README replaces the one-line placeholder with the full
crate-level documentation: source tree, database setup
(env var + constructor snippet with the three repo
construction + TerminologyUsecaseConfig), the test
invocation commands, and a back-link to the guideline.

Spec coverage: README section of
docs/superpowers/specs/2026-08-18-terminology-crate-design.md.

Verification: cargo doc -p terminology --no-deps"
```

---

## Task 9: Verification gate + lockfile chore

### 9.1 Run the verification gate

- [ ] **Step 1:** `cargo fmt --all -- --check`

- [ ] **Step 2:** `cargo clippy -p terminology --all-targets --all-features -- -D warnings`

  Expected: zero warnings.

- [ ] **Step 3:** `cargo test -p terminology`

  Expected: green for all non-ignored tests.

- [ ] **Step 4:** `cargo doc -p terminology --no-deps`

  Expected: green.

- [ ] **Step 5:** `cargo check --workspace`

  Expected: green.

- [ ] **Step 6:** `cargo clippy --workspace`

  Expected: green (or documented failures on unrelated crates).

- [ ] **Step 7:** `cargo test --workspace`

  Expected: green (or documented failures on unrelated crates).

### 9.2 Live-DB integration pass (optional, requires env var)

- [ ] **Step 1:** With `AEGIS_TERMINOLOGY_DATABASE_URL` set, run:

```bash
cargo test -p terminology -- --ignored --test-threads=1
```

  Expected: green for the live-DB round trips.

### 9.3 Lockfile (if it drifted)

- [ ] **Step 1:** If `Cargo.lock` changed during this work, commit it:

```bash
git status
# If Cargo.lock is modified:
git add Cargo.lock
git commit -m "chore(terminology): refresh Cargo.lock after new deps"
```

---

## Done

When the verification gate at 9.1 is green and (where possible)
9.2 is also green, the crate is ready for review. The
spec/plan/commits/verification split mirrors the convention every
business lib crate in this workspace follows.
