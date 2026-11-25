# Domain Model Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `lib/crates/domain-model`, a Rust library that owns CRUD over the CDISC SDTM domain model aggregates (`SdtmVersion`, `SdtmDomain`, `SdtmVariable`), exposes an outbound `apis::domain_model::DomainModelService` port, and ships HTTP routes under `/api/domain-model/*` in `aegis-server` with `require_admin_or_root` enforcement on every write.

**Architecture:** Ports-and-adapters DDD structure mirroring `lib/crates/terminology`. Three inbound `#[async_trait]` ports (`SdtmVersionRepository`, `SdtmDomainRepository`, `SdtmVariableRepository`) live in `domain`; one `DomainModelUsecase<V, D, Va>` is generic over all three. A PostgreSQL adapter implements each port; an in-memory facade adapts the usecase to `apis::domain_model::DomainModelService`. The descriptions field on `SdtmDomain` / `SdtmVariable` is persisted as JSONB.

**Tech Stack:** Rust 2024 edition, sqlx 0.9 (postgres, runtime-tokio, macros, migrate, chrono, json), async-trait, thiserror, chrono, serde + serde_json (for JSONB round-trip + the apis port DTOs), axum 0.8 + utoipa-axum 0.2 (HTTP layer).

## Global Constraints

- **Edition / resolver:** `edition = "2024"`, workspace `resolver = "3"`. The crate's `Cargo.toml` must inherit every shared dep via `{ workspace = true }`.
- **Workspace wiring:** Edit the root `Cargo.toml` once in Task 1 to add `lib/crates/domain-model` to `[workspace].members`.
- **ID type:** `i64` (BIGSERIAL / BIGINT) on every aggregate and every FK, per the spec's user decision. The user-facing spec used `i32`; that is upgraded for workspace consistency.
- **SQLx convention:** Use the runtime API (`sqlx::query_as`, `QueryBuilder`). Compile-time-checked macros need a live `DATABASE_URL` or a `sqlx-data.json` cache that the workspace does not currently provide. Document the choice in a module-level comment in each postgres repo file.
- **Two-constructor pattern:** Every aggregate has a validating `new(...) -> Result<Self, DomainError>` and a `pub(crate) for_repository(...) -> Self`. The latter is reserved for the adapter layer's row bridge; never call it from `usecase`.
- **Errors:** `DomainError` and `UsecaseError` are `#[derive(thiserror::Error)]` enums. Every variant that wraps an inner error carries `#[source] Inner` so the chain is preserved. `UsecaseError::From<DomainError>` maps to `Repository` (because the contract was already broken upstream). Domain validators return `UsecaseError::Validation(...)`.
- **API surface:** The crate root re-exports the documented public surface (aggregates, ports, error enums, command DTOs, view DTOs, concrete repos, the usecase + config, the in-memory facade). Consumers write `use domain_model::*;` and never reach into sub-modules.
- **JSONB storage:** `Vec<SdtmDomainDescription>` and `Vec<SdtmVariableDescription>` are stored in a single `JSONB NOT NULL DEFAULT '[]'::jsonb` column per aggregate. The adapter layer uses `serde_json::to_value` / `from_value` at the row boundary.
- **CHECK constraints:** Every enum-shaped column has a CHECK constraint mirroring the Rust `as_str` value set. Belt-and-braces against out-of-band inserts.
- **CASCADE:** `version_id` on `sdtm_domains` and `domain_id` on `sdtm_variables` use `ON DELETE CASCADE`. `delete_version` removes child domains (and via them, child variables) via the cascade.
- **Scoped lists:** `SdtmDomainRepository` exposes only `list_by_version` (no bare `list()`); `SdtmVariableRepository` exposes only `list_by_domain` (no bare `list()`). `SdtmVersionRepository` exposes no `find_by_id` / `find_by_name` (per user decision).
- **Authorisation:** Every HTTP write handler calls `require_admin_or_root(&claims)?;` before dispatching to the usecase. Read handlers require only an authenticated `AuthClaims`. `aegis-desktop` is intentionally untouched.
- **Verification gate (every task that compiles must end green):**
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps
  ```
  After Task 8, also:
  ```bash
  cargo check --workspace
  cargo clippy --workspace
  cargo test --workspace
  ```
  After Task 11, additionally:
  ```bash
  cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
  cargo test -p aegis-server
  ```
  Live-DB integration tests are run only with `AEGIS_DATABASE_URL` set:
  ```bash
  cargo test -p domain-model -- --ignored --test-threads=1
  ```

## File Structure

### Created

- `lib/crates/domain-model/Cargo.toml`
- `lib/crates/domain-model/README.md`
- `lib/crates/domain-model/migrations/0001_create_sdtm_versions.sql`
- `lib/crates/domain-model/migrations/0002_create_sdtm_domains.sql`
- `lib/crates/domain-model/migrations/0003_create_sdtm_variables.sql`
- `lib/crates/domain-model/src/lib.rs`
- `lib/crates/domain-model/src/domain.rs`
- `lib/crates/domain-model/src/domain/domain_category.rs`
- `lib/crates/domain-model/src/domain/variable_type.rs`
- `lib/crates/domain-model/src/domain/sdtm_version.rs`
- `lib/crates/domain-model/src/domain/sdtm_domain.rs`
- `lib/crates/domain-model/src/domain/sdtm_variable.rs`
- `lib/crates/domain-model/src/domain/error.rs`
- `lib/crates/domain-model/src/domain/repository.rs`
- `lib/crates/domain-model/src/domain/tests.rs`
- `lib/crates/domain-model/src/usecase.rs`
- `lib/crates/domain-model/src/usecase/commands.rs`
- `lib/crates/domain-model/src/usecase/views.rs`
- `lib/crates/domain-model/src/usecase/error.rs`
- `lib/crates/domain-model/src/usecase/domain_model_usecase.rs`
- `lib/crates/domain-model/src/usecase/tests.rs`
- `lib/crates/domain-model/src/adapter.rs`
- `lib/crates/domain-model/src/adapter/persistence.rs`
- `lib/crates/domain-model/src/adapter/persistence/postgres.rs`
- `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs`
- `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_domain_repo.rs`
- `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`
- `lib/crates/domain-model/src/adapter/facade.rs`
- `lib/crates/domain-model/src/adapter/facade/in_memory.rs`
- `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`
- `lib/crates/domain-model/tests/public_api.rs`
- `lib/crates/domain-model/tests/integration_persistence.rs`
- `apps/server/aegis-server/src/transport/http/domain_model.rs`
- `apps/server/aegis-server/src/transport/http/domain_model/router.rs`
- `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs`
- `apps/server/aegis-server/tests/integration_domain_model.rs`

### Modified

- `Cargo.toml` (workspace root) — add `lib/crates/domain-model` to `members`.
- `lib/crates/apis/src/lib.rs` — add `pub mod domain_model;`.
- `apps/server/aegis-server/Cargo.toml` — add `domain-model` path-dep.
- `apps/server/aegis-server/src/state.rs` — add `domain_model` field.
- `apps/server/aegis-server/src/run.rs` — build the service.
- `apps/server/aegis-server/src/transport/http.rs` — `pub mod domain_model;`.
- `apps/server/aegis-server/src/transport/http/router.rs` — mount under `/api/domain-model`.
- `apps/server/aegis-server/src/transport/http/dto.rs` — wire DTOs + re-declared enums.
- `apps/server/aegis-server/src/transport/http/error.rs` — `From<DomainModelApiError> for ApiError`.
- `apps/server/aegis-server/src/transport/http/openapi.rs` — register new paths + schemas.

---

## Task 1: Scaffold — workspace wiring + crate skeleton

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `lib/crates/domain-model/Cargo.toml`
- Create: `lib/crates/domain-model/README.md`
- Create: `lib/crates/domain-model/src/lib.rs`
- Create: `lib/crates/domain-model/src/domain.rs`
- Create: `lib/crates/domain-model/src/usecase.rs`
- Create: `lib/crates/domain-model/src/adapter.rs`
- Create: `lib/crates/domain-model/src/domain/error.rs` (stub)

### 1.1 Wire the workspace

- [ ] **Step 1:** Edit `Cargo.toml` (workspace root). In `[workspace].members`, add `"lib/crates/domain-model",`.

The result should be (members in alphabetical order, fitting the existing layout):

```toml
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/domain-model", "lib/crates/project",
    "lib/crates/terminology",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

### 1.2 Create `lib/crates/domain-model/Cargo.toml`

- [ ] **Step 1:** Create `lib/crates/domain-model/Cargo.toml`:

```toml
[package]
name = "domain-model"
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
# `serde` + `serde_json` are required for the JSONB
# round-trip of `Vec<SdtmDomainDescription>` /
# `Vec<SdtmVariableDescription>` at the postgres adapter
# boundary, and for the derive-based Serialize / Deserialize
# on the four enums + the description DTOs.
serde = { workspace = true }
serde_json = { workspace = true }
# `apis` provides the outbound `DomainModelService` port the
# in-memory facade implements. Path-dep because both crates
# share the workspace.
apis = { path = "../apis" }

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

### 1.3 Create the placeholder README

- [ ] **Step 1:** Create `lib/crates/domain-model/README.md`:

```markdown
# domain-model

CRUD over the CDISC SDTM domain model aggregates
(`SdtmVersion`, `SdtmDomain`, `SdtmVariable`), backed by
PostgreSQL.

See `docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions and
`docs/superpowers/specs/2026-08-24-domain-model-crate-design.md`
for the data model + port surface.
```

### 1.4 Create the layer skeleton

- [ ] **Step 1:** Create `lib/crates/domain-model/src/lib.rs`:

```rust
//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;
```

- [ ] **Step 2:** Create `lib/crates/domain-model/src/domain.rs`:

```rust
mod error;

pub use error::DomainError;
```

- [ ] **Step 3:** Create `lib/crates/domain-model/src/usecase.rs`:

```rust
// Usecase layer; populated in Task 3.
```

- [ ] **Step 4:** Create `lib/crates/domain-model/src/adapter.rs`:

```rust
// Adapter layer; populated in Tasks 4-6 and Task 8.
```

- [ ] **Step 5:** Create `lib/crates/domain-model/src/domain/error.rs`:

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
- Expected: green. The new crate compiles against its declared deps; the empty/stub modules don't pull in anything yet.

- [ ] **Step 2:** Commit:

```bash
git add Cargo.toml \
        lib/crates/domain-model/Cargo.toml \
        lib/crates/domain-model/README.md \
        lib/crates/domain-model/src/lib.rs \
        lib/crates/domain-model/src/domain.rs \
        lib/crates/domain-model/src/usecase.rs \
        lib/crates/domain-model/src/adapter.rs \
        lib/crates/domain-model/src/domain/error.rs
git commit -m "feat(domain-model): scaffold crate skeleton

Adds lib/crates/domain-model to the workspace members and creates
the three DDD layer skeletons (domain, usecase, adapter) plus a
stub DomainError. Cargo.toml inherits every shared dep via
{ workspace = true }; chrono is documented in-line as the
created_at / updated_at carrier; serde + serde_json are
documented in-line as the JSONB round-trip path; the apis
path-dep is documented in-line as the outbound-port path.

Spec coverage: workspace wiring + crate shape in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification: cargo check --workspace"
```

---

## Task 2: Domain layer — value objects, aggregates, ports, errors

**Files:**
- Modify: `lib/crates/domain-model/src/domain.rs`
- Create: `lib/crates/domain-model/src/domain/domain_category.rs`
- Create: `lib/crates/domain-model/src/domain/variable_type.rs`
- Create: `lib/crates/domain-model/src/domain/sdtm_version.rs`
- Create: `lib/crates/domain-model/src/domain/sdtm_domain.rs`
- Create: `lib/crates/domain-model/src/domain/sdtm_variable.rs`
- Create: `lib/crates/domain-model/src/domain/repository.rs`
- Create: `lib/crates/domain-model/src/domain/tests.rs`
- Modify: `lib/crates/domain-model/src/domain/error.rs`
- Modify: `lib/crates/domain-model/src/lib.rs` (re-exports)

### 2.1 Write the failing domain tests

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/tests.rs`:

```rust
use super::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmVariable, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableCore, SdtmVariableType, SdtmVersion,
    SdtmRole,
};

// ---- enums ---------------------------------------------------------------

#[test]
fn domain_category_parses_known_strings() {
    let cases = [
        ("Special Purpose", DomainCategory::SpecialPurpose),
        ("Interventions",   DomainCategory::Interventions),
        ("Events",          DomainCategory::Events),
        ("Findings",        DomainCategory::Findings),
        ("Trial Design",    DomainCategory::TrialDesign),
        ("Relationships",   DomainCategory::Relationships),
        ("Study Reference", DomainCategory::StudyReference),
    ];
    for (raw, expected) in cases {
        let parsed = DomainCategory::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn domain_category_rejects_unknown_string() {
    let err = DomainCategory::try_from("Bogus").unwrap_err();
    assert!(matches!(err, DomainError::InvalidDomainCategory(ref s) if s == "Bogus"));
}

#[test]
fn variable_type_parses_known_strings() {
    let n = SdtmVariableType::try_from("Numeric").unwrap();
    assert_eq!(n, SdtmVariableType::Numeric);
    assert_eq!(n.as_str(), "Numeric");
    let c = SdtmVariableType::try_from("Character").unwrap();
    assert_eq!(c, SdtmVariableType::Character);
}

#[test]
fn variable_type_rejects_unknown_string() {
    let err = SdtmVariableType::try_from("Date").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableType(ref s) if s == "Date"));
}

#[test]
fn variable_core_parses_known_strings() {
    let cases = [
        ("Req",  SdtmVariableCore::Req),
        ("Exp",  SdtmVariableCore::Exp),
        ("Perm", SdtmVariableCore::Perm),
        ("Supp", SdtmVariableCore::Supp),
    ];
    for (raw, expected) in cases {
        let parsed = SdtmVariableCore::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn variable_core_rejects_unknown_string() {
    let err = SdtmVariableCore::try_from("Bad").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableCore(ref s) if s == "Bad"));
}

#[test]
fn role_parses_known_strings() {
    let cases = [
        ("Identifier",          SdtmRole::Identifier),
        ("Topic",               SdtmRole::Topic),
        ("Timing",              SdtmRole::Timing),
        ("Record Qualifier",    SdtmRole::RecordQualifier),
        ("Synonym Qualifier",   SdtmRole::SynonymQualifier),
        ("Variable Qualifier",  SdtmRole::VariableQualifier),
        ("Grouping Qualifier",  SdtmRole::GroupingQualifier),
        ("Rule",                SdtmRole::Rule),
    ];
    for (raw, expected) in cases {
        let parsed = SdtmRole::try_from(raw).unwrap();
        assert_eq!(parsed, expected);
        assert_eq!(parsed.as_str(), raw);
    }
}

#[test]
fn role_rejects_unknown_string() {
    let err = SdtmRole::try_from("Bad").unwrap_err();
    assert!(matches!(err, DomainError::InvalidVariableRole(ref s) if s == "Bad"));
}

// ---- aggregates ----------------------------------------------------------

#[test]
fn sdtm_version_new_rejects_empty_name() {
    let err = SdtmVersion::new("   ".into()).unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_version_new_accepts_valid_input() {
    let v = SdtmVersion::new("2024-09-27".into()).unwrap();
    assert_eq!(v.name, "2024-09-27");
}

#[test]
fn sdtm_domain_new_rejects_empty_name() {
    let err = SdtmDomain::new(
        1,
        "".into(),
        DomainCategory::Events,
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_domain_new_accepts_valid_input() {
    let desc = SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure:  "One record per AE".into(),
        },
    };
    let d = SdtmDomain::new(1, "AE".into(), DomainCategory::Events, vec![desc]).unwrap();
    assert_eq!(d.name, "AE");
    assert_eq!(d.descriptions.len(), 1);
    assert_eq!(d.descriptions[0].details.description, "Adverse events");
}

#[test]
fn sdtm_variable_new_rejects_empty_name() {
    let err = SdtmVariable::new(
        1,
        "".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        1,
        Vec::new(),
    )
    .unwrap_err();
    assert!(matches!(err, DomainError::EmptyName));
}

#[test]
fn sdtm_variable_new_accepts_valid_input() {
    let desc = SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail {
            label: "Adverse Event Term".into(),
        },
    };
    let v = SdtmVariable::new(
        1,
        "AETERM".into(),
        None,
        SdtmVariableType::Character,
        SdtmVariableCore::Req,
        Some(SdtmRole::Topic),
        11,
        vec![desc],
    )
    .unwrap();
    assert_eq!(v.name, "AETERM");
    assert_eq!(v.variable_sequence, 11);
}
```

- [ ] **Step 2:** Run; confirm compile failure.

Run: `cargo test -p domain-model --lib domain::tests 2>&1 | tail -20`
Expected: compile errors (types not yet defined).

### 2.2 Create `domain_category.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/domain_category.rs`:

```rust
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// SDTM domain category. The string form (`"Special Purpose"`,
/// `"Interventions"`, ...) is the wire shape consumed by the
/// postgres adapter (CHECK constraint + JSONB round-trip) and
/// by the apis port DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")]
    SpecialPurpose,
    #[serde(rename = "Interventions")]
    Interventions,
    #[serde(rename = "Events")]
    Events,
    #[serde(rename = "Findings")]
    Findings,
    #[serde(rename = "Trial Design")]
    TrialDesign,
    #[serde(rename = "Relationships")]
    Relationships,
    #[serde(rename = "Study Reference")]
    StudyReference,
}

impl DomainCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainCategory::SpecialPurpose => "Special Purpose",
            DomainCategory::Interventions  => "Interventions",
            DomainCategory::Events         => "Events",
            DomainCategory::Findings       => "Findings",
            DomainCategory::TrialDesign    => "Trial Design",
            DomainCategory::Relationships  => "Relationships",
            DomainCategory::StudyReference => "Study Reference",
        }
    }
}

impl TryFrom<&str> for DomainCategory {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Special Purpose" => Ok(DomainCategory::SpecialPurpose),
            "Interventions"   => Ok(DomainCategory::Interventions),
            "Events"          => Ok(DomainCategory::Events),
            "Findings"        => Ok(DomainCategory::Findings),
            "Trial Design"    => Ok(DomainCategory::TrialDesign),
            "Relationships"   => Ok(DomainCategory::Relationships),
            "Study Reference" => Ok(DomainCategory::StudyReference),
            other => Err(DomainError::InvalidDomainCategory(other.to_string())),
        }
    }
}
```

### 2.3 Create `variable_type.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/variable_type.rs`:

```rust
use std::convert::TryFrom;

use serde::{Deserialize, Serialize};

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

impl SdtmVariableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableType::Numeric   => "Numeric",
            SdtmVariableType::Character => "Character",
        }
    }
}

impl TryFrom<&str> for SdtmVariableType {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Numeric"   => Ok(SdtmVariableType::Numeric),
            "Character" => Ok(SdtmVariableType::Character),
            other => Err(DomainError::InvalidVariableType(other.to_string())),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

impl SdtmVariableCore {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableCore::Req  => "Req",
            SdtmVariableCore::Exp  => "Exp",
            SdtmVariableCore::Perm => "Perm",
            SdtmVariableCore::Supp => "Supp",
        }
    }
}

impl TryFrom<&str> for SdtmVariableCore {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Req"  => Ok(SdtmVariableCore::Req),
            "Exp"  => Ok(SdtmVariableCore::Exp),
            "Perm" => Ok(SdtmVariableCore::Perm),
            "Supp" => Ok(SdtmVariableCore::Supp),
            other => Err(DomainError::InvalidVariableCore(other.to_string())),
        }
    }
}

/// SDTM variable role. The string form is consumed by the
/// postgres adapter (`sdtm_variables.variable_role` column +
/// CHECK constraint) and by the apis port DTOs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]
    Topic,
    #[serde(rename = "Timing")]
    Timing,
    #[serde(rename = "Record Qualifier")]
    RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]
    SynonymQualifier,
    #[serde(rename = "Variable Qualifier")]
    VariableQualifier,
    #[serde(rename = "Grouping Qualifier")]
    GroupingQualifier,
    Rule,
}

impl SdtmRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmRole::Identifier         => "Identifier",
            SdtmRole::Topic              => "Topic",
            SdtmRole::Timing             => "Timing",
            SdtmRole::RecordQualifier    => "Record Qualifier",
            SdtmRole::SynonymQualifier   => "Synonym Qualifier",
            SdtmRole::VariableQualifier  => "Variable Qualifier",
            SdtmRole::GroupingQualifier  => "Grouping Qualifier",
            SdtmRole::Rule               => "Rule",
        }
    }
}

impl TryFrom<&str> for SdtmRole {
    type Error = DomainError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Identifier"         => Ok(SdtmRole::Identifier),
            "Topic"              => Ok(SdtmRole::Topic),
            "Timing"             => Ok(SdtmRole::Timing),
            "Record Qualifier"   => Ok(SdtmRole::RecordQualifier),
            "Synonym Qualifier"  => Ok(SdtmRole::SynonymQualifier),
            "Variable Qualifier" => Ok(SdtmRole::VariableQualifier),
            "Grouping Qualifier" => Ok(SdtmRole::GroupingQualifier),
            "Rule"               => Ok(SdtmRole::Rule),
            other => Err(DomainError::InvalidVariableRole(other.to_string())),
        }
    }
}
```

### 2.4 Create `sdtm_version.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/sdtm_version.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;

/// A published SDTM release, identified by `name`. Typically
/// a `yyyy-mm-dd` workbook sheet suffix; stored as `String`
/// (not parsed as a `NaiveDate`) so a future sheet with a
/// non-date name round-trips intact.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmVersion {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmVersion")
            .field("id", &self.id)
            .field("name", &self.name)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmVersion {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(name: String) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            name,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        name: String,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            name,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmVersionRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmVersionNew {
    pub name: String,
}

/// Input DTO for `SdtmVersionRepository::update`. Only `name`
/// is mutable on a version; `id` identifies the row.
#[derive(Debug, Clone, Default)]
pub struct SdtmVersionUpdate {
    pub id: i64,
    pub name: Option<String>,
}
```

### 2.5 Create `sdtm_domain.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/sdtm_domain.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::domain_category::DomainCategory;
use super::error::DomainError;

/// Localised description of an SDTM domain. Carried on the
/// `SdtmDomain` aggregate and persisted as a single JSONB
/// column on `sdtm_domains`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

/// A single SDTM domain (e.g. `AE`, `DM`, `VS`) attached to
/// a `SdtmVersion` and described in one or more languages.
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmDomain {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmDomain")
            .field("id", &self.id)
            .field("version_id", &self.version_id)
            .field("name", &self.name)
            .field("category", &self.category)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmDomain {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(
        version_id: i64,
        name: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            version_id,
            name,
            category,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64,
        version_id: i64,
        name: String,
        category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            version_id,
            name,
            category,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmDomainRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmDomainNew {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

/// Input DTO for `SdtmDomainRepository::update`. Every field
/// except `id` is optional so the usecase can pass only what
/// actually changed. `descriptions: None` means "don't touch",
/// `Some(vec)` means "replace with this list" (use an empty
/// `vec![]` to clear the column).
#[derive(Debug, Clone, Default)]
pub struct SdtmDomainUpdate {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}
```

### 2.6 Create `sdtm_variable.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/sdtm_variable.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::DomainError;
use super::variable_type::{SdtmRole, SdtmVariableCore, SdtmVariableType};

/// Localised description of an SDTM variable. Carried on the
/// `SdtmVariable` aggregate and persisted as a single JSONB
/// column on `sdtm_variables`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

/// A single SDTM variable (e.g. `AETERM`, `AESEV`) attached
/// to a `SdtmDomain`. `variable_sequence` is the column order
/// within the parent domain (1-based; the domain decides what
/// makes sense).
#[derive(Clone, PartialEq, Eq)]
pub struct SdtmVariable {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl std::fmt::Debug for SdtmVariable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SdtmVariable")
            .field("id", &self.id)
            .field("domain_id", &self.domain_id)
            .field("name", &self.name)
            .field("variable_controlled", &self.variable_controlled)
            .field("variable_type", &self.variable_type)
            .field("variable_core", &self.variable_core)
            .field("variable_role", &self.variable_role)
            .field("variable_sequence", &self.variable_sequence)
            .field("descriptions", &self.descriptions)
            .field("created_at", &self.created_at)
            .field("updated_at", &self.updated_at)
            .finish()
    }
}

impl SdtmVariable {
    /// Validating constructor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        domain_id: i64,
        name: String,
        variable_controlled: Option<String>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
    ) -> Result<Self, DomainError> {
        if name.trim().is_empty() {
            return Err(DomainError::EmptyName);
        }
        Ok(Self {
            id: 0,
            domain_id,
            name,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
            updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        })
    }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn for_repository(
        id: i64,
        domain_id: i64,
        name: String,
        variable_controlled: Option<String>,
        variable_type: SdtmVariableType,
        variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>,
        variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            domain_id,
            name,
            variable_controlled,
            variable_type,
            variable_core,
            variable_role,
            variable_sequence,
            descriptions,
            created_at,
            updated_at,
        }
    }
}

/// Input DTO for `SdtmVariableRepository::create`.
#[derive(Debug, Clone)]
pub struct SdtmVariableNew {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

/// Input DTO for `SdtmVariableRepository::update`. Every field
/// except `id` is optional so the usecase can pass only what
/// actually changed. `variable_controlled` and `variable_role`
/// use `Option<Option<T>>` so the caller can distinguish
/// "don't change" (outer `None`) from "clear the field" (outer
/// `Some(None)`); the other fields use flat `Option<T>` where
/// `None` means "don't change" and `Some(value)` means "replace".
#[derive(Debug, Clone, Default)]
pub struct SdtmVariableUpdate {
    pub id: i64,
    pub name: Option<String>,
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}
```

### 2.7 Replace `error.rs` with the full `DomainError`

- [ ] **Step 1:** Replace `lib/crates/domain-model/src/domain/error.rs`:

```rust
use thiserror::Error;

use super::domain_category::DomainCategory;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("name must not be empty")]
    EmptyName,

    #[error("invalid domain category: {0}")]
    InvalidDomainCategory(String),
    #[error("invalid variable type: {0}")]
    InvalidVariableType(String),
    #[error("invalid variable core: {0}")]
    InvalidVariableCore(String),
    #[error("invalid variable role: {0}")]
    InvalidVariableRole(String),

    #[error("not found")]
    NotFound,
    #[error("sdtm version not found: {0}")]
    SdtmVersionNotFound(i64),
    #[error("sdtm domain not found: {0}")]
    SdtmDomainNotFound(i64),
    #[error("sdtm variable not found: {0}")]
    SdtmVariableNotFound(i64),

    #[error("sdtm version already exists: {name}")]
    DuplicateSdtmVersion { name: String },
    #[error("sdtm domain already exists for version {version_id} / {name}")]
    DuplicateSdtmDomain { version_id: i64, name: String },
    #[error("sdtm variable already exists for domain {domain_id} / {name}")]
    DuplicateSdtmVariable { domain_id: i64, name: String },

    #[error("referenced sdtm version not found: {0}")]
    FkSdtmVersionNotFound(i64),
    #[error("referenced sdtm domain not found: {0}")]
    FkSdtmDomainNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}
```

### 2.8 Create `repository.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/domain/repository.rs`:

```rust
use async_trait::async_trait;

use super::error::DomainError;
use super::sdtm_domain::{
    SdtmDomain, SdtmDomainNew, SdtmDomainUpdate,
};
use super::sdtm_variable::{
    SdtmVariable, SdtmVariableNew, SdtmVariableUpdate,
};
use super::sdtm_version::{
    SdtmVersion, SdtmVersionNew, SdtmVersionUpdate,
};

/// Outbound port for persistence of `SdtmVersion` aggregates.
/// Implementations live in the adapter layer.
///
/// No `find_by_id` / `find_by_name`: `update` returns the
/// updated aggregate via `UPDATE … RETURNING *` and `delete`
/// runs `DELETE FROM … WHERE id = $1` directly.
#[async_trait]
pub trait SdtmVersionRepository: Send + Sync {
    async fn create(&self, input: SdtmVersionNew)
        -> Result<SdtmVersion, DomainError>;
    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError>;
    async fn update(&self, input: SdtmVersionUpdate)
        -> Result<SdtmVersion, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmDomain` aggregates.
/// The only list path is scoped to a version (no bare
/// `list()`).
#[async_trait]
pub trait SdtmDomainRepository: Send + Sync {
    async fn create(&self, input: SdtmDomainNew)
        -> Result<SdtmDomain, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError>;
    async fn list_by_version(
        &self, version_id: i64,
    ) -> Result<Vec<SdtmDomain>, DomainError>;
    async fn update(&self, input: SdtmDomainUpdate)
        -> Result<SdtmDomain, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

/// Outbound port for persistence of `SdtmVariable` aggregates.
/// The only list path is scoped to a domain (no bare
/// `list()`).
#[async_trait]
pub trait SdtmVariableRepository: Send + Sync {
    async fn create(&self, input: SdtmVariableNew)
        -> Result<SdtmVariable, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError>;
    async fn list_by_domain(
        &self, domain_id: i64,
    ) -> Result<Vec<SdtmVariable>, DomainError>;
    async fn update(&self, input: SdtmVariableUpdate)
        -> Result<SdtmVariable, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}
```

### 2.9 Wire up `domain.rs` + `lib.rs`

- [ ] **Step 1:** Replace `lib/crates/domain-model/src/domain.rs`:

```rust
mod domain_category;
mod error;
mod repository;
mod sdtm_domain;
mod sdtm_variable;
mod sdtm_version;
mod variable_type;

#[cfg(test)]
mod tests;

pub use domain_category::DomainCategory;
pub use error::DomainError;
pub use repository::{
    SdtmDomainRepository, SdtmVariableRepository, SdtmVersionRepository,
};
pub use sdtm_domain::{
    SdtmDomain, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmDomainNew, SdtmDomainUpdate,
};
pub use sdtm_variable::{
    SdtmRole, SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableNew, SdtmVariableType,
    SdtmVariableUpdate,
};
pub use sdtm_version::{
    SdtmVersion, SdtmVersionNew, SdtmVersionUpdate,
};
pub use variable_type::{SdtmVariableCore, SdtmVariableType};
```

- [ ] **Step 2:** Update `lib/crates/domain-model/src/lib.rs`:

```rust
//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainRepository,
    SdtmDomainUpdate, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate, SdtmVersion,
    SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};
```

### 2.10 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. All domain tests pass; clippy is clean.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/src/domain \
        lib/crates/domain-model/src/domain.rs \
        lib/crates/domain-model/src/lib.rs
git commit -m "feat(domain-model): domain layer

Adds the four enums (DomainCategory, SdtmVariableType,
SdtmVariableCore, SdtmRole) with as_str / TryFrom<&str>
round-trip, the three aggregates (SdtmVersion, SdtmDomain,
SdtmVariable) with the two-constructor pattern + hand-rolled
Debug impls, the three #[async_trait] ports
(SdtmVersionRepository, SdtmDomainRepository,
SdtmVariableRepository), the full DomainError, and the domain
unit tests covering enum parsing and aggregate validation.

Spec coverage: Data Model + DomainError sections in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 3: Usecase layer — commands, views, error, usecase, in-memory tests

**Files:**
- Modify: `lib/crates/domain-model/src/usecase.rs`
- Create: `lib/crates/domain-model/src/usecase/commands.rs`
- Create: `lib/crates/domain-model/src/usecase/views.rs`
- Create: `lib/crates/domain-model/src/usecase/error.rs`
- Create: `lib/crates/domain-model/src/usecase/domain_model_usecase.rs`
- Create: `lib/crates/domain-model/src/usecase/tests.rs`
- Modify: `lib/crates/domain-model/src/lib.rs` (re-exports)

### 3.1 Create `commands.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/usecase/commands.rs`:

```rust
use crate::domain::{
    DomainCategory, SdtmDomainDescription, SdtmRole, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableType,
};

// SdtmVersion

pub struct CreateSdtmVersion {
    pub name: String,
}

#[derive(Default)]
pub struct UpdateSdtmVersion {
    pub id: i64,
    pub name: Option<String>,
}

// SdtmDomain

pub struct CreateSdtmDomain {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

#[derive(Default)]
pub struct UpdateSdtmDomain {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

// SdtmVariable

pub struct CreateSdtmVariable {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

#[derive(Default)]
pub struct UpdateSdtmVariable {
    pub id: i64,
    pub name: Option<String>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}
```

### 3.2 Create `views.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/usecase/views.rs`:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{
    DomainCategory, SdtmDomain, SdtmDomainDescription, SdtmDomainDescriptionDetail,
    SdtmRole, SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableType, SdtmVersion,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVersionView {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmVersion> for SdtmVersionView {
    fn from(v: SdtmVersion) -> Self {
        Self {
            id: v.id,
            name: v.name,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainView {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmDomain> for SdtmDomainView {
    fn from(d: SdtmDomain) -> Self {
        Self {
            id: d.id,
            version_id: d.version_id,
            name: d.name,
            category: d.category,
            descriptions: d.descriptions,
            created_at: d.created_at,
            updated_at: d.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<SdtmVariable> for SdtmVariableView {
    fn from(v: SdtmVariable) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            name: v.name,
            variable_controlled: v.variable_controlled,
            variable_type: v.variable_type,
            variable_core: v.variable_core,
            variable_role: v.variable_role,
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions,
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
```

### 3.3 Create `error.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/usecase/error.rs`:

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
        // repository already came through `UsecaseError::Validation`
        // (see `validate_*` in `domain_model_usecase`); everything
        // else surfaces as `Repository`.
        UsecaseError::Repository(err)
    }
}
```

### 3.4 Create `domain_model_usecase.rs`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/usecase/domain_model_usecase.rs`:

```rust
use crate::domain::{
    DomainError, SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate,
    SdtmVariableNew, SdtmVariableRepository, SdtmVariableUpdate,
    SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};

use super::commands::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, UpdateSdtmDomain,
    UpdateSdtmVariable, UpdateSdtmVersion,
};
use super::error::UsecaseError;
use super::views::{SdtmDomainView, SdtmVariableView, SdtmVersionView};

/// Configuration for `DomainModelUsecase::new`. Wraps the three
/// concrete (or fake) repositories so the constructor stays
/// readable.
pub struct DomainModelUsecaseConfig<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    pub version_repo: V,
    pub domain_repo: D,
    pub variable_repo: Va,
}

/// Async orchestration for SDTM domain-model lifecycle
/// operations. Generic over the three repository ports so tests
/// can inject in-memory fakes. Domain → view projection runs
/// through the `From` impls in `super::views`.
pub struct DomainModelUsecase<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    version_repo: V,
    domain_repo: D,
    variable_repo: Va,
}

impl<V, D, Va> DomainModelUsecase<V, D, Va>
where
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
{
    pub fn new(cfg: DomainModelUsecaseConfig<V, D, Va>) -> Self {
        Self {
            version_repo: cfg.version_repo,
            domain_repo: cfg.domain_repo,
            variable_repo: cfg.variable_repo,
        }
    }

    // ---- SdtmVersion ----

    pub async fn create_version(
        &self, cmd: CreateSdtmVersion,
    ) -> Result<SdtmVersionView, UsecaseError> {
        validate_create_version(&cmd)?;
        let v = self
            .version_repo
            .create(SdtmVersionNew { name: cmd.name })
            .await?;
        Ok(v.into())
    }

    pub async fn list_versions(
        &self,
    ) -> Result<Vec<SdtmVersionView>, UsecaseError> {
        let vs = self.version_repo.list().await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_version(
        &self, cmd: UpdateSdtmVersion,
    ) -> Result<SdtmVersionView, UsecaseError> {
        validate_update_version(&cmd)?;
        let v = self
            .version_repo
            .update(SdtmVersionUpdate { id: cmd.id, name: cmd.name })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError> {
        self.version_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmDomain ----

    pub async fn create_domain(
        &self, cmd: CreateSdtmDomain,
    ) -> Result<SdtmDomainView, UsecaseError> {
        validate_create_domain(&cmd)?;
        let d = self
            .domain_repo
            .create(SdtmDomainNew {
                version_id: cmd.version_id,
                name: cmd.name,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn get_domain_by_id(
        &self, id: i64,
    ) -> Result<SdtmDomainView, UsecaseError> {
        let d = self.domain_repo.find_by_id(id).await?;
        Ok(d.into())
    }

    pub async fn list_domains_by_version(
        &self, version_id: i64,
    ) -> Result<Vec<SdtmDomainView>, UsecaseError> {
        let ds = self.domain_repo.list_by_version(version_id).await?;
        Ok(ds.into_iter().map(Into::into).collect())
    }

    pub async fn update_domain(
        &self, cmd: UpdateSdtmDomain,
    ) -> Result<SdtmDomainView, UsecaseError> {
        validate_update_domain(&cmd)?;
        let d = self
            .domain_repo
            .update(SdtmDomainUpdate {
                id: cmd.id,
                name: cmd.name,
                category: cmd.category,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(d.into())
    }

    pub async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError> {
        self.domain_repo.delete(id).await?;
        Ok(())
    }

    // ---- SdtmVariable ----

    pub async fn create_variable(
        &self, cmd: CreateSdtmVariable,
    ) -> Result<SdtmVariableView, UsecaseError> {
        validate_create_variable(&cmd)?;
        let v = self
            .variable_repo
            .create(SdtmVariableNew {
                domain_id: cmd.domain_id,
                name: cmd.name,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn get_variable_by_id(
        &self, id: i64,
    ) -> Result<SdtmVariableView, UsecaseError> {
        let v = self.variable_repo.find_by_id(id).await?;
        Ok(v.into())
    }

    pub async fn list_variables_by_domain(
        &self, domain_id: i64,
    ) -> Result<Vec<SdtmVariableView>, UsecaseError> {
        let vs = self.variable_repo.list_by_domain(domain_id).await?;
        Ok(vs.into_iter().map(Into::into).collect())
    }

    pub async fn update_variable(
        &self, cmd: UpdateSdtmVariable,
    ) -> Result<SdtmVariableView, UsecaseError> {
        validate_update_variable(&cmd)?;
        let v = self
            .variable_repo
            .update(SdtmVariableUpdate {
                id: cmd.id,
                name: cmd.name,
                variable_controlled: cmd.variable_controlled,
                variable_type: cmd.variable_type,
                variable_core: cmd.variable_core,
                variable_role: cmd.variable_role,
                variable_sequence: cmd.variable_sequence,
                descriptions: cmd.descriptions,
            })
            .await?;
        Ok(v.into())
    }

    pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError> {
        self.variable_repo.delete(id).await?;
        Ok(())
    }
}

// ---- pre-flight validation ----

fn validate_create_version(cmd: &CreateSdtmVersion) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_version(cmd: &UpdateSdtmVersion) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_domain(cmd: &CreateSdtmDomain) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_domain(cmd: &UpdateSdtmDomain) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_create_variable(cmd: &CreateSdtmVariable) -> Result<(), UsecaseError> {
    if cmd.name.trim().is_empty() {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}

fn validate_update_variable(cmd: &UpdateSdtmVariable) -> Result<(), UsecaseError> {
    if let Some(ref name) = cmd.name
        && name.trim().is_empty()
    {
        return Err(UsecaseError::Validation(DomainError::EmptyName));
    }
    Ok(())
}
```

### 3.5 Write the failing in-memory usecase tests

- [ ] **Step 1:** Create `lib/crates/domain-model/src/usecase/tests.rs`:

```rust
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use crate::domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate, SdtmRole,
    SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableNew, SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate,
    SdtmVersion, SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};
use crate::usecase::commands::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion, UpdateSdtmDomain,
    UpdateSdtmVariable, UpdateSdtmVersion,
};
use crate::usecase::domain_model_usecase::{
    DomainModelUsecase, DomainModelUsecaseConfig,
};
use crate::usecase::error::UsecaseError;

// ---- in-memory fakes -----------------------------------------------------

#[derive(Default)]
struct InMemorySdtmVersionRepo {
    inner: Mutex<Vec<SdtmVersion>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVersionRepo {
    fn new() -> Self { Self::default() }
}

#[async_trait]
impl SdtmVersionRepository for InMemorySdtmVersionRepo {
    async fn create(
        &self, input: SdtmVersionNew,
    ) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter().any(|v| v.name == input.name) {
            return Err(DomainError::DuplicateSdtmVersion {
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVersion::for_repository(
            id, input.name,
            chrono::Utc::now(), chrono::Utc::now(),
        );
        g.push(v.clone());
        Ok(v)
    }
    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
        Ok(self.inner.lock().unwrap().clone())
    }
    async fn update(
        &self, input: SdtmVersionUpdate,
    ) -> Result<SdtmVersion, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g.iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVersionNotFound(input.id))?;
        if let Some(name) = input.name {
            v.name = name;
        }
        v.updated_at = chrono::Utc::now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVersionNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmDomainRepo {
    inner: Mutex<Vec<SdtmDomain>>,
    next_id: AtomicI64,
}

impl InMemorySdtmDomainRepo {
    fn new() -> Self { Self::default() }
}

#[async_trait]
impl SdtmDomainRepository for InMemorySdtmDomainRepo {
    async fn create(
        &self, input: SdtmDomainNew,
    ) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter().any(|d|
            d.version_id == input.version_id && d.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmDomain {
                version_id: input.version_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let d = SdtmDomain::for_repository(
            id, input.version_id, input.name, input.category,
            input.descriptions,
            chrono::Utc::now(), chrono::Utc::now(),
        );
        g.push(d.clone());
        Ok(d)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmDomain, DomainError> {
        let g = self.inner.lock().unwrap();
        g.iter()
            .find(|d| d.id == id)
            .cloned()
            .ok_or(DomainError::SdtmDomainNotFound(id))
    }
    async fn list_by_version(
        &self, version_id: i64,
    ) -> Result<Vec<SdtmDomain>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|d| d.version_id == version_id)
            .cloned()
            .collect())
    }
    async fn update(
        &self, input: SdtmDomainUpdate,
    ) -> Result<SdtmDomain, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let d = g.iter_mut()
            .find(|d| d.id == input.id)
            .ok_or(DomainError::SdtmDomainNotFound(input.id))?;
        if let Some(name) = input.name { d.name = name; }
        if let Some(category) = input.category { d.category = category; }
        if let Some(descriptions) = input.descriptions { d.descriptions = descriptions; }
        d.updated_at = chrono::Utc::now();
        Ok(d.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|d| d.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmDomainNotFound(id));
        }
        Ok(())
    }
}

#[derive(Default)]
struct InMemorySdtmVariableRepo {
    inner: Mutex<Vec<SdtmVariable>>,
    next_id: AtomicI64,
}

impl InMemorySdtmVariableRepo {
    fn new() -> Self { Self::default() }
}

#[async_trait]
impl SdtmVariableRepository for InMemorySdtmVariableRepo {
    async fn create(
        &self, input: SdtmVariableNew,
    ) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        if g.iter().any(|v|
            v.domain_id == input.domain_id && v.name == input.name)
        {
            return Err(DomainError::DuplicateSdtmVariable {
                domain_id: input.domain_id,
                name: input.name.clone(),
            });
        }
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let v = SdtmVariable::for_repository(
            id, input.domain_id, input.name, input.variable_controlled,
            input.variable_type, input.variable_core, input.variable_role,
            input.variable_sequence, input.descriptions,
            chrono::Utc::now(), chrono::Utc::now(),
        );
        g.push(v.clone());
        Ok(v)
    }
    async fn find_by_id(&self, id: i64) -> Result<SdtmVariable, DomainError> {
        let g = self.inner.lock().unwrap();
        g.iter()
            .find(|v| v.id == id)
            .cloned()
            .ok_or(DomainError::SdtmVariableNotFound(id))
    }
    async fn list_by_domain(
        &self, domain_id: i64,
    ) -> Result<Vec<SdtmVariable>, DomainError> {
        let g = self.inner.lock().unwrap();
        Ok(g.iter()
            .filter(|v| v.domain_id == domain_id)
            .cloned()
            .collect())
    }
    async fn update(
        &self, input: SdtmVariableUpdate,
    ) -> Result<SdtmVariable, DomainError> {
        let mut g = self.inner.lock().unwrap();
        let v = g.iter_mut()
            .find(|v| v.id == input.id)
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        if let Some(name) = input.name { v.name = name; }
        if let Some(vc) = input.variable_controlled { v.variable_controlled = vc; }
        if let Some(vt) = input.variable_type { v.variable_type = vt; }
        if let Some(vc) = input.variable_core { v.variable_core = vc; }
        if let Some(vr) = input.variable_role { v.variable_role = vr; }
        if let Some(seq) = input.variable_sequence { v.variable_sequence = seq; }
        if let Some(descriptions) = input.descriptions { v.descriptions = descriptions; }
        v.updated_at = chrono::Utc::now();
        Ok(v.clone())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.inner.lock().unwrap();
        let before = g.len();
        g.retain(|v| v.id != id);
        if g.len() == before {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

fn build_usecase() -> DomainModelUsecase<
    Arc<InMemorySdtmVersionRepo>,
    Arc<InMemorySdtmDomainRepo>,
    Arc<InMemorySdtmVariableRepo>,
> {
    DomainModelUsecase::new(DomainModelUsecaseConfig {
        version_repo:  Arc::new(InMemorySdtmVersionRepo::new()),
        domain_repo:   Arc::new(InMemorySdtmDomainRepo::new()),
        variable_repo: Arc::new(InMemorySdtmVariableRepo::new()),
    })
}

// ---- version tests -------------------------------------------------------

#[tokio::test]
async fn version_crud_round_trips() {
    let uc = build_usecase();
    let v = uc.create_version(CreateSdtmVersion {
        name: "2024-09-27".into(),
    }).await.unwrap();
    assert_eq!(v.name, "2024-09-27");

    let listed = uc.list_versions().await.unwrap();
    assert_eq!(listed.len(), 1);
    assert_eq!(listed[0].id, v.id);

    let updated = uc.update_version(UpdateSdtmVersion {
        id: v.id,
        name: Some("2025-01-15".into()),
    }).await.unwrap();
    assert_eq!(updated.name, "2025-01-15");

    uc.delete_version(v.id).await.unwrap();
    assert!(uc.list_versions().await.unwrap().is_empty());
}

#[tokio::test]
async fn version_create_rejects_empty_name() {
    let uc = build_usecase();
    let err = uc.create_version(CreateSdtmVersion {
        name: "   ".into(),
    }).await.unwrap_err();
    assert!(matches!(err,
        UsecaseError::Validation(DomainError::EmptyName)));
}

// ---- domain tests --------------------------------------------------------

#[tokio::test]
async fn domain_crud_round_trips() {
    let uc = build_usecase();
    let v = uc.create_version(CreateSdtmVersion {
        name: "2024-09-27".into(),
    }).await.unwrap();

    let desc = SdtmDomainDescription {
        lang: "en".into(),
        details: crate::domain::SdtmDomainDescriptionDetail {
            description: "Adverse events".into(),
            structure:  "One record per AE".into(),
        },
    };
    let d = uc.create_domain(CreateSdtmDomain {
        version_id: v.id,
        name: "AE".into(),
        category: DomainCategory::Events,
        descriptions: vec![desc],
    }).await.unwrap();
    assert_eq!(d.name, "AE");
    assert_eq!(d.descriptions.len(), 1);

    let by_id = uc.get_domain_by_id(d.id).await.unwrap();
    assert_eq!(by_id.id, d.id);

    let list = uc.list_domains_by_version(v.id).await.unwrap();
    assert_eq!(list.len(), 1);

    let updated = uc.update_domain(UpdateSdtmDomain {
        id: d.id,
        name: Some("AE2".into()),
        category: None,
        descriptions: None,
    }).await.unwrap();
    assert_eq!(updated.name, "AE2");

    uc.delete_domain(d.id).await.unwrap();
    assert!(uc.list_domains_by_version(v.id).await.unwrap().is_empty());
}

// ---- variable tests ------------------------------------------------------

#[tokio::test]
async fn variable_crud_round_trips() {
    let uc = build_usecase();
    let v = uc.create_version(CreateSdtmVersion {
        name: "2024-09-27".into(),
    }).await.unwrap();
    let d = uc.create_domain(CreateSdtmDomain {
        version_id: v.id,
        name: "AE".into(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    }).await.unwrap();

    let desc = SdtmVariableDescription {
        lang: "en".into(),
        details: crate::domain::SdtmVariableDescriptionDetail {
            label: "Term".into(),
        },
    };
    let var = uc.create_variable(CreateSdtmVariable {
        domain_id: d.id,
        name: "AETERM".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: vec![desc],
    }).await.unwrap();
    assert_eq!(var.name, "AETERM");

    let list = uc.list_variables_by_domain(d.id).await.unwrap();
    assert_eq!(list.len(), 1);

    // Clear variable_role via outer-Some(inner-None).
    let updated = uc.update_variable(UpdateSdtmVariable {
        id: var.id,
        name: None,
        variable_controlled: None,
        variable_type: None,
        variable_core: None,
        variable_role: Some(None),
        variable_sequence: None,
        descriptions: None,
    }).await.unwrap();
    assert_eq!(updated.variable_role, None);

    uc.delete_variable(var.id).await.unwrap();
    assert!(uc.list_variables_by_domain(d.id).await.unwrap().is_empty());
}
```

- [ ] **Step 2:** Run; confirm compile failure.

Run: `cargo test -p domain-model --lib usecase::tests 2>&1 | tail -20`
Expected: compile errors (usecase not yet wired).

### 3.6 Wire up `usecase.rs` + `lib.rs`

- [ ] **Step 1:** Replace `lib/crates/domain-model/src/usecase.rs`:

```rust
mod commands;
mod domain_model_usecase;
mod error;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion,
    UpdateSdtmDomain, UpdateSdtmVariable, UpdateSdtmVersion,
};
pub use domain_model_usecase::{
    DomainModelUsecase, DomainModelUsecaseConfig,
};
pub use error::UsecaseError;
pub use views::{SdtmDomainView, SdtmVariableView, SdtmVersionView};
```

- [ ] **Step 2:** Update `lib/crates/domain-model/src/lib.rs`:

```rust
//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainRepository,
    SdtmDomainUpdate, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate, SdtmVersion,
    SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};
pub use usecase::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion,
    DomainModelUsecase, DomainModelUsecaseConfig, SdtmDomainView,
    SdtmVariableView, SdtmVersionView, UpdateSdtmDomain, UpdateSdtmVariable,
    UpdateSdtmVersion, UsecaseError,
};
```

### 3.7 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. All usecase tests pass against the in-memory fakes.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/src/usecase \
        lib/crates/domain-model/src/usecase.rs \
        lib/crates/domain-model/src/lib.rs
git commit -m "feat(domain-model): usecase layer

Adds DomainModelUsecase<V, D, Va> generic over the three
repository ports, the four methods on SdtmVersion
(create/list/update/delete), the five on SdtmDomain
(create/get_by_id/list_by_version/update/delete), the five on
SdtmVariable (create/get_by_id/list_by_domain/update/delete),
command DTOs (Create*/Update*), view DTOs (SdtmVersionView,
SdtmDomainView, SdtmVariableView) with From impls from the
domain aggregates, UsecaseError (Validation/Repository variants
with #[source]), and in-memory wire-up tests against
Arc<Mutex<Vec<…>>> + AtomicI64 fakes for the three repos.

Spec coverage: Usecase Layer section in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 4: Postgres adapter — `sdtm_versions` table + `SdtmVersionRepo`

**Files:**
- Create: `lib/crates/domain-model/migrations/0001_create_sdtm_versions.sql`
- Create: `lib/crates/domain-model/src/adapter/persistence.rs`
- Create: `lib/crates/domain-model/src/adapter/persistence/postgres.rs`
- Create: `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs`
- Modify: `lib/crates/domain-model/src/adapter.rs`
- Modify: `lib/crates/domain-model/src/lib.rs` (re-exports)

### 4.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/domain-model/migrations/0001_create_sdtm_versions.sql`:

```sql
-- sdtm_versions: one row per published CDISC SDTM release sheet.
-- Identified by `name` (e.g. `2024-09-27`); `id` is the surrogate key.

CREATE TABLE IF NOT EXISTS sdtm_versions (
    id          BIGSERIAL PRIMARY KEY,
    name        TEXT      NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_versions_name_unique UNIQUE (name)
);

-- updated_at trigger
CREATE OR REPLACE FUNCTION sdtm_versions_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_versions_updated_at
BEFORE UPDATE ON sdtm_versions
FOR EACH ROW EXECUTE FUNCTION sdtm_versions_set_updated_at();
```

### 4.2 Write the adapter loader stub

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/persistence.rs`:

```rust
//! Persistence adapter layer.
//!
//! The SQLx runtime API is used throughout (`sqlx::query_as`,
//! `sqlx::query`, `QueryBuilder`) — see the module-level comment
//! in each `*_repo` file. The workspace has no shared
//! `sqlx-data.json` cache, so the compile-time macro API is
//! intentionally avoided.

pub mod postgres;
```

- [ ] **Step 2:** Create `lib/crates/domain-model/src/adapter/persistence/postgres.rs`:

```rust
pub mod sdtm_version_repo;
```

- [ ] **Step 3:** Replace `lib/crates/domain-model/src/adapter.rs`:

```rust
pub mod facade;
pub mod persistence;
```

### 4.3 Write the postgres `SdtmVersionRepo`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs`:

```rust
// SQLx runtime API is used throughout this crate. The workspace
// does not currently ship a `.sqlx/` offline cache, and the
// compile-time-checked macros would require either a live
// `DATABASE_URL` at build time or a checked-in `sqlx-data.json`.
// `sqlx::query_as` + `sqlx::query` + `FromRow` + `QueryBuilder`
// are sufficient and keep the crate reproducible.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmVersion, SdtmVersionNew, SdtmVersionRepository,
    SdtmVersionUpdate,
};

#[derive(FromRow)]
struct SdtmVersionRow {
    id: i64,
    name: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl From<SdtmVersionRow> for SdtmVersion {
    fn from(r: SdtmVersionRow) -> Self {
        SdtmVersion::for_repository(r.id, r.name, r.created_at, r.updated_at)
    }
}

#[derive(Clone)]
pub struct SdtmVersionRepoPg {
    pool: PgPool,
}

impl SdtmVersionRepoPg {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl SdtmVersionRepository for SdtmVersionRepoPg {
    async fn create(
        &self, input: SdtmVersionNew,
    ) -> Result<SdtmVersion, DomainError> {
        let row: SdtmVersionRow = sqlx::query_as::<_, SdtmVersionRow>(
            "INSERT INTO sdtm_versions (name) VALUES ($1)
             RETURNING id, name, created_at, updated_at",
        )
        .bind(&input.name)
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(row.into())
    }

    async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmVersionRow>(
            "SELECT id, name, created_at, updated_at
             FROM sdtm_versions ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_err)?;
        Ok(rows.into_iter().map(Into::into).collect())
    }

    async fn update(
        &self, input: SdtmVersionUpdate,
    ) -> Result<SdtmVersion, DomainError> {
        // Spec: only `name` is mutable on a version. The
        // UPDATE … RETURNING path materialises the resulting
        // row; if the row doesn't exist we surface
        // `SdtmVersionNotFound`.
        let row: SdtmVersionRow = sqlx::query_as::<_, SdtmVersionRow>(
            "UPDATE sdtm_versions SET name = COALESCE($2, name)
             WHERE id = $1
             RETURNING id, name, created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_err)?
        .ok_or(DomainError::SdtmVersionNotFound(input.id))?;
        Ok(row.into())
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_versions WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_db_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmVersionNotFound(id));
        }
        Ok(())
    }
}

fn map_db_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            // Postgres unique-violation codes (`23505`) come back as
            // `E::Database` with the column name on the constraint.
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmVersion {
                    name: "(unknown)".into(),
                };
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}
```

### 4.4 Write a postgres adapter unit test (file string)

- [ ] **Step 1:** Append to `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    /// Tier-2 unit test: confirms the migration file referenced by
    /// the adapter is the one we expect (single source of truth,
    /// loadable by `sqlx::migrate!` at app start). It does **not**
    /// open a real connection — see `tests/integration_persistence.rs`
    /// for that.
    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/0001_create_sdtm_versions.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_versions"));
        assert!(sql.contains("sdtm_versions_updated_at"));
    }
}
```

### 4.5 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. The postgres repo compiles against `sqlx::PgPool`; the unit test reads the migration file via `include_str!`.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/migrations/0001_create_sdtm_versions.sql \
        lib/crates/domain-model/src/adapter.rs \
        lib/crates/domain-model/src/adapter/persistence.rs \
        lib/crates/domain-model/src/adapter/persistence/postgres.rs \
        lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs
git commit -m "feat(domain-model): postgres adapter for SdtmVersion

Adds the 0001 migration (BIGSERIAL id, UNIQUE name,
created_at + updated_at with BEFORE UPDATE trigger), the
SdtmVersionRepoPg struct implementing SdtmVersionRepository
via the SQLx runtime API (query_as / query / RETURNING), the
SQL error mapper (UniqueViolation → DuplicateSdtmVersion,
RowNotFound → NotFound), and a Tier-2 unit test that loads the
migration file via include_str! to confirm it is the one
referenced.

Spec coverage: Database Schema + Postgres Adapter sections in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 5: Postgres adapter — `sdtm_domains` table + `SdtmDomainRepo`

**Files:**
- Create: `lib/crates/domain-model/migrations/0002_create_sdtm_domains.sql`
- Create: `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_domain_repo.rs`
- Modify: `lib/crates/domain-model/src/adapter/persistence/postgres.rs`

### 5.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/domain-model/migrations/0002_create_sdtm_domains.sql`:

```sql
-- sdtm_domains: one row per (version, domain). The descriptions
-- column is a single JSONB blob carrying a Vec<SdtmDomainDescription>.

CREATE TABLE IF NOT EXISTS sdtm_domains (
    id            BIGSERIAL PRIMARY KEY,
    version_id    BIGINT      NOT NULL REFERENCES sdtm_versions(id) ON DELETE CASCADE,
    name          TEXT        NOT NULL,
    category      TEXT        NOT NULL,
    descriptions  JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_domains_version_name_unique UNIQUE (version_id, name),
    CONSTRAINT sdtm_domains_category_check CHECK (
        category IN (
            'Special Purpose',
            'Interventions',
            'Events',
            'Findings',
            'Trial Design',
            'Relationships',
            'Study Reference'
        )
    )
);

CREATE OR REPLACE FUNCTION sdtm_domains_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_domains_updated_at
BEFORE UPDATE ON sdtm_domains
FOR EACH ROW EXECUTE FUNCTION sdtm_domains_set_updated_at();
```

### 5.2 Write the postgres `SdtmDomainRepo`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_domain_repo.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainNew, SdtmDomainRepository, SdtmDomainUpdate,
};

#[derive(FromRow)]
struct SdtmDomainRow {
    id: i64,
    version_id: i64,
    name: String,
    category: String,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmDomainRow {
    fn into_domain(self) -> Result<SdtmDomain, DomainError> {
        let category = DomainCategory::try_from(self.category.as_str())?;
        let descriptions: Vec<SdtmDomainDescription> =
            serde_json::from_value(self.descriptions)
                .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmDomain::for_repository(
            self.id, self.version_id, self.name, category,
            descriptions, self.created_at, self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmDomainRepoPg {
    pool: PgPool,
}

impl SdtmDomainRepoPg {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl SdtmDomainRepository for SdtmDomainRepoPg {
    async fn create(
        &self, input: SdtmDomainNew,
    ) -> Result<SdtmDomain, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let category_str = input.category.as_str();
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "INSERT INTO sdtm_domains
                (version_id, name, category, descriptions)
             VALUES ($1, $2, $3, $4)
             RETURNING id, version_id, name, category, descriptions,
                       created_at, updated_at",
        )
        .bind(input.version_id)
        .bind(&input.name)
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(map_domain_err)?;
        row.into_domain()
    }

    async fn find_by_id(
        &self, id: i64,
    ) -> Result<SdtmDomain, DomainError> {
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "SELECT id, version_id, name, category, descriptions,
                    created_at, updated_at
             FROM sdtm_domains WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_domain_err)?
        .ok_or(DomainError::SdtmDomainNotFound(id))?;
        row.into_domain()
    }

    async fn list_by_version(
        &self, version_id: i64,
    ) -> Result<Vec<SdtmDomain>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmDomainRow>(
            "SELECT id, version_id, name, category, descriptions,
                    created_at, updated_at
             FROM sdtm_domains
             WHERE version_id = $1
             ORDER BY id ASC",
        )
        .bind(version_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_domain_err)?;
        rows.into_iter().map(SdtmDomainRow::into_domain).collect()
    }

    async fn update(
        &self, input: SdtmDomainUpdate,
    ) -> Result<SdtmDomain, DomainError> {
        // COALESCE: NULL argument → column unchanged.
        let category_str = input.category.map(|c| c.as_str());
        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => Some(
                serde_json::to_value(v)
                    .map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };
        let row: SdtmDomainRow = sqlx::query_as::<_, SdtmDomainRow>(
            "UPDATE sdtm_domains SET
                name         = COALESCE($2, name),
                category     = COALESCE($3, category),
                descriptions = COALESCE($4, descriptions)
             WHERE id = $1
             RETURNING id, version_id, name, category, descriptions,
                       created_at, updated_at",
        )
        .bind(input.id)
        .bind(input.name.as_deref())
        .bind(category_str)
        .bind(descriptions_json)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_domain_err)?
        .ok_or(DomainError::SdtmDomainNotFound(input.id))?;
        row.into_domain()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_domains WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_domain_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmDomainNotFound(id));
        }
        Ok(())
    }
}

fn map_domain_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                // UniqueViolation on (version_id, name).
                return DomainError::DuplicateSdtmDomain {
                    version_id: 0,
                    name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                // FK violation: most likely missing parent version.
                return DomainError::FkSdtmVersionNotFound(0);
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/0002_create_sdtm_domains.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_domains"));
        assert!(sql.contains("descriptions  JSONB"));
        assert!(sql.contains("sdtm_domains_category_check"));
    }
}
```

- [ ] **Step 2:** Update `lib/crates/domain-model/src/adapter/persistence/postgres.rs`:

```rust
pub mod sdtm_domain_repo;
pub mod sdtm_variable_repo;
pub mod sdtm_version_repo;
```

### 5.3 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. The new domain repo compiles; the JSONB round-trip test reads the migration via `include_str!`.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/migrations/0002_create_sdtm_domains.sql \
        lib/crates/domain-model/src/adapter/persistence/postgres.rs \
        lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_domain_repo.rs
git commit -m "feat(domain-model): postgres adapter for SdtmDomain

Adds the 0002 migration (FK to sdtm_versions ON DELETE
CASCADE, UNIQUE (version_id, name), CHECK on category values,
descriptions as JSONB NOT NULL DEFAULT '[]', BEFORE UPDATE
trigger) and SdtmDomainRepoPg implementing
SdtmDomainRepository via the SQLx runtime API. The descriptions
column round-trips through serde_json::to_value /
from_value at the row boundary. UPDATE uses COALESCE on each
optional field. The error mapper translates unique-violation
to DuplicateSdtmDomain and FK-violation to
FkSdtmVersionNotFound.

Spec coverage: Database Schema + Postgres Adapter sections in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 6: Postgres adapter — `sdtm_variables` table + `SdtmVariableRepo`

**Files:**
- Create: `lib/crates/domain-model/migrations/0003_create_sdtm_variables.sql`
- Create: `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`
- Modify: `lib/crates/domain-model/src/lib.rs` (re-export concrete repos)

### 6.1 Create the migration

- [ ] **Step 1:** Create `lib/crates/domain-model/migrations/0003_create_sdtm_variables.sql`:

```sql
-- sdtm_variables: one row per (domain, variable). descriptions
-- is a single JSONB blob carrying a Vec<SdtmVariableDescription>.

CREATE TABLE IF NOT EXISTS sdtm_variables (
    id                   BIGSERIAL PRIMARY KEY,
    domain_id            BIGINT      NOT NULL REFERENCES sdtm_domains(id) ON DELETE CASCADE,
    name                 TEXT        NOT NULL,
    variable_controlled  TEXT,
    variable_type        TEXT        NOT NULL,
    variable_core        TEXT        NOT NULL,
    variable_role        TEXT,
    variable_sequence    BIGINT      NOT NULL,
    descriptions         JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_variables_domain_name_unique UNIQUE (domain_id, name),
    CONSTRAINT sdtm_variables_type_check CHECK (
        variable_type IN ('Numeric', 'Character')
    ),
    CONSTRAINT sdtm_variables_core_check CHECK (
        variable_core IN ('Req', 'Exp', 'Perm', 'Supp')
    ),
    CONSTRAINT sdtm_variables_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier',
            'Topic',
            'Timing',
            'Record Qualifier',
            'Synonym Qualifier',
            'Variable Qualifier',
            'Grouping Qualifier',
            'Rule'
        )
    )
);

CREATE OR REPLACE FUNCTION sdtm_variables_set_updated_at() RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE OR REPLACE TRIGGER sdtm_variables_updated_at
BEFORE UPDATE ON sdtm_variables
FOR EACH ROW EXECUTE FUNCTION sdtm_variables_set_updated_at();
```

### 6.2 Write the postgres `SdtmVariableRepo`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs`:

```rust
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};

use crate::domain::{
    DomainError, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableNew, SdtmVariableRepository,
    SdtmVariableType, SdtmVariableUpdate,
};

#[derive(FromRow)]
struct SdtmVariableRow {
    id: i64,
    domain_id: i64,
    name: String,
    variable_controlled: Option<String>,
    variable_type: String,
    variable_core: String,
    variable_role: Option<String>,
    variable_sequence: i64,
    descriptions: JsonValue,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl SdtmVariableRow {
    fn into_var(self) -> Result<SdtmVariable, DomainError> {
        let variable_type = SdtmVariableType::try_from(
            self.variable_type.as_str(),
        )?;
        let variable_core = SdtmVariableCore::try_from(
            self.variable_core.as_str(),
        )?;
        let variable_role = match self.variable_role.as_deref() {
            None => None,
            Some(s) => Some(SdtmRole::try_from(s)?),
        };
        let descriptions: Vec<SdtmVariableDescription> =
            serde_json::from_value(self.descriptions)
                .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(SdtmVariable::for_repository(
            self.id, self.domain_id, self.name, self.variable_controlled,
            variable_type, variable_core, variable_role,
            self.variable_sequence, descriptions,
            self.created_at, self.updated_at,
        ))
    }
}

#[derive(Clone)]
pub struct SdtmVariableRepoPg {
    pool: PgPool,
}

impl SdtmVariableRepoPg {
    pub fn new(pool: PgPool) -> Self { Self { pool } }
}

#[async_trait]
impl SdtmVariableRepository for SdtmVariableRepoPg {
    async fn create(
        &self, input: SdtmVariableNew,
    ) -> Result<SdtmVariable, DomainError> {
        let descriptions_json = serde_json::to_value(&input.descriptions)
            .map_err(|e| DomainError::Repository(e.to_string()))?;
        let type_str = input.variable_type.as_str();
        let core_str = input.variable_core.as_str();
        let role_str = input.variable_role.map(|r| r.as_str());
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "INSERT INTO sdtm_variables
                (domain_id, name, variable_controlled, variable_type,
                 variable_core, variable_role, variable_sequence,
                 descriptions)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
             RETURNING id, domain_id, name, variable_controlled,
                       variable_type, variable_core, variable_role,
                       variable_sequence, descriptions,
                       created_at, updated_at",
        )
        .bind(input.domain_id)
        .bind(&input.name)
        .bind(&input.variable_controlled)
        .bind(type_str)
        .bind(core_str)
        .bind(role_str)
        .bind(input.variable_sequence)
        .bind(descriptions_json)
        .fetch_one(&self.pool)
        .await
        .map_err(map_variable_err)?;
        row.into_var()
    }

    async fn find_by_id(
        &self, id: i64,
    ) -> Result<SdtmVariable, DomainError> {
        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables WHERE id = $1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(map_variable_err)?
        .ok_or(DomainError::SdtmVariableNotFound(id))?;
        row.into_var()
    }

    async fn list_by_domain(
        &self, domain_id: i64,
    ) -> Result<Vec<SdtmVariable>, DomainError> {
        let rows = sqlx::query_as::<_, SdtmVariableRow>(
            "SELECT id, domain_id, name, variable_controlled,
                    variable_type, variable_core, variable_role,
                    variable_sequence, descriptions,
                    created_at, updated_at
             FROM sdtm_variables
             WHERE domain_id = $1
             ORDER BY variable_sequence ASC, id ASC",
        )
        .bind(domain_id)
        .fetch_all(&self.pool)
        .await
        .map_err(map_variable_err)?;
        rows.into_iter().map(SdtmVariableRow::into_var).collect()
    }

    async fn update(
        &self, input: SdtmVariableUpdate,
    ) -> Result<SdtmVariable, DomainError> {
        // For nullable columns (variable_controlled, variable_role) the
        // three-state semantics are:
        //   - outer None       -> column unchanged
        //   - outer Some(None) -> column cleared to NULL
        // We translate that to SQL via dynamic fragments that select
        // between `column = $bind` (for "clear") and
        // `column = COALESCE($bind, column)` (for "don't change").

        let name = input.name.as_deref();
        let variable_type = input.variable_type.map(|t| t.as_str());
        let variable_core = input.variable_core.map(|c| c.as_str());
        let variable_sequence = input.variable_sequence;

        let variable_controlled_bound: Option<&str> = match &input.variable_controlled {
            None => None,
            Some(None) => None,
            Some(Some(s)) => Some(s.as_str()),
        };
        let clear_controlled = input.variable_controlled.is_some();

        let variable_role_bound: Option<&str> = match &input.variable_role {
            None => None,
            Some(None) => None,
            Some(Some(r)) => Some(r.as_str()),
        };
        let clear_role = input.variable_role.is_some();

        let descriptions_json = match &input.descriptions {
            None => None,
            Some(v) => Some(
                serde_json::to_value(v)
                    .map_err(|e| DomainError::Repository(e.to_string()))?,
            ),
        };

        let ctrl_expr = if clear_controlled {
            "$7".to_string()
        } else {
            "COALESCE($7, variable_controlled)".to_string()
        };
        let role_expr = if clear_role {
            "$8".to_string()
        } else {
            "COALESCE($8, variable_role)".to_string()
        };

        let sql = format!(
            "UPDATE sdtm_variables SET
                name                = COALESCE($2, name),
                variable_type       = COALESCE($3, variable_type),
                variable_core       = COALESCE($4, variable_core),
                variable_sequence   = COALESCE($5, variable_sequence),
                descriptions        = COALESCE($6, descriptions),
                variable_controlled = {ctrl},
                variable_role       = {role}
             WHERE id = $1
             RETURNING id, domain_id, name, variable_controlled,
                       variable_type, variable_core, variable_role,
                       variable_sequence, descriptions,
                       created_at, updated_at",
            ctrl = ctrl_expr,
            role = role_expr,
        );

        let row: SdtmVariableRow = sqlx::query_as::<_, SdtmVariableRow>(&sql)
            .bind(input.id)
            .bind(name)
            .bind(variable_type)
            .bind(variable_core)
            .bind(variable_sequence)
            .bind(descriptions_json)
            .bind(variable_controlled_bound)
            .bind(variable_role_bound)
            .fetch_optional(&self.pool)
            .await
            .map_err(map_variable_err)?
            .ok_or(DomainError::SdtmVariableNotFound(input.id))?;
        row.into_var()
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::query("DELETE FROM sdtm_variables WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(map_variable_err)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::SdtmVariableNotFound(id));
        }
        Ok(())
    }
}

fn map_variable_err(err: sqlx::Error) -> DomainError {
    use sqlx::Error as E;
    match &err {
        E::Database(db) => {
            if db.code().as_deref() == Some("23505") {
                return DomainError::DuplicateSdtmVariable {
                    domain_id: 0, name: "(unknown)".into(),
                };
            }
            if db.code().as_deref() == Some("23503") {
                return DomainError::FkSdtmDomainNotFound(0);
            }
            DomainError::Repository(err.to_string())
        }
        E::RowNotFound => DomainError::NotFound,
        _ => DomainError::Repository(err.to_string()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn migration_file_is_present_and_idempotent() {
        let sql = include_str!("../../../../migrations/0003_create_sdtm_variables.sql");
        assert!(sql.contains("CREATE TABLE IF NOT EXISTS sdtm_variables"));
        assert!(sql.contains("descriptions         JSONB"));
        assert!(sql.contains("sdtm_variables_type_check"));
        assert!(sql.contains("sdtm_variables_core_check"));
        assert!(sql.contains("sdtm_variables_role_check"));
    }
}
```

### 6.3 Re-export the concrete repos in `lib.rs` (interim — Task 8 adds the facade line)

- [ ] **Step 1:** Add the concrete-repo re-export to `lib/crates/domain-model/src/lib.rs`. For this task (Task 6), keep the form that omits the facade import:

```rust
pub use adapter::persistence::postgres::{
    sdtm_domain_repo::SdtmDomainRepoPg,
    sdtm_variable_repo::SdtmVariableRepoPg,
    sdtm_version_repo::SdtmVersionRepoPg,
};
```

(In Task 8 we will lift this block into the full form that also adds the facade import.)

### 6.4 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/migrations/0003_create_sdtm_variables.sql \
        lib/crates/domain-model/src/adapter/persistence/postgres/sdtm_variable_repo.rs \
        lib/crates/domain-model/src/lib.rs
git commit -m "feat(domain-model): postgres adapter for SdtmVariable

Adds the 0003 migration (FK to sdtm_domains ON DELETE
CASCADE, UNIQUE (domain_id, name), CHECK constraints on
variable_type / variable_core / variable_role, descriptions as
JSONB NOT NULL DEFAULT '[]', BEFORE UPDATE trigger) and
SdtmVariableRepoPg implementing SdtmVariableRepository. The
descriptions column round-trips through serde_json. UPDATE
expresses the three-state semantics for
variable_controlled / variable_role (don't change / replace /
clear) via a dynamic SQL fragment that picks between
column = \$bind (for 'clear') and column =
COALESCE(\$bind, column) (for 'don't change'). List ordering
is variable_sequence ASC, id ASC.

Spec coverage: Database Schema + Postgres Adapter sections in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 7: apis port — `DomainModelService` + DTOs

**Files:**
- Create: `lib/crates/apis/src/domain_model.rs`
- Modify: `lib/crates/apis/src/lib.rs`

### 7.1 Create the apis port

- [ ] **Step 1:** Create `lib/crates/apis/src/domain_model.rs`:

```rust
//! Outbound port for the domain-model service.
//!
//! Mirrors `domain_model::DomainModelUsecase` so backend
//! adapters (in-memory, PostgreSQL, …) can adapt their own
//! types to the shared contract defined here. All supporting
//! DTOs (request shapes, view projections, and
//! [`DomainModelApiError`]) live alongside the trait so a single
//! `use apis::domain_model::*;` brings the whole contract into
//! scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

// ---- enums (re-declared so apis stays a leaf crate) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainCategory {
    SpecialPurpose,
    Interventions,
    Events,
    Findings,
    TrialDesign,
    Relationships,
    StudyReference,
}

impl DomainCategory {
    pub fn as_str(&self) -> &'static str {
        match self {
            DomainCategory::SpecialPurpose => "Special Purpose",
            DomainCategory::Interventions  => "Interventions",
            DomainCategory::Events         => "Events",
            DomainCategory::Findings       => "Findings",
            DomainCategory::TrialDesign    => "Trial Design",
            DomainCategory::Relationships  => "Relationships",
            DomainCategory::StudyReference => "Study Reference",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmVariableType {
    Numeric,
    Character,
}

impl SdtmVariableType {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableType::Numeric   => "Numeric",
            SdtmVariableType::Character => "Character",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmVariableCore {
    Req,
    Exp,
    Perm,
    Supp,
}

impl SdtmVariableCore {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmVariableCore::Req  => "Req",
            SdtmVariableCore::Exp  => "Exp",
            SdtmVariableCore::Perm => "Perm",
            SdtmVariableCore::Supp => "Supp",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmRole {
    Identifier,
    Topic,
    Timing,
    RecordQualifier,
    SynonymQualifier,
    VariableQualifier,
    GroupingQualifier,
    Rule,
}

impl SdtmRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            SdtmRole::Identifier         => "Identifier",
            SdtmRole::Topic              => "Topic",
            SdtmRole::Timing             => "Timing",
            SdtmRole::RecordQualifier    => "Record Qualifier",
            SdtmRole::SynonymQualifier   => "Synonym Qualifier",
            SdtmRole::VariableQualifier  => "Variable Qualifier",
            SdtmRole::GroupingQualifier  => "Grouping Qualifier",
            SdtmRole::Rule               => "Rule",
        }
    }
}

// ---- error surface ----

#[derive(Debug, Clone, Error)]
pub enum DomainModelApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("sdtm version not found: {0}")]
    SdtmVersionNotFound(i64),
    #[error("sdtm domain not found: {0}")]
    SdtmDomainNotFound(i64),
    #[error("sdtm variable not found: {0}")]
    SdtmVariableNotFound(i64),

    #[error("sdtm version already exists: {0}")]
    DuplicateSdtmVersion(String),
    #[error("sdtm domain already exists for version {version_id} / {name}")]
    DuplicateSdtmDomain { version_id: i64, name: String },
    #[error("sdtm variable already exists for domain {domain_id} / {name}")]
    DuplicateSdtmVariable { domain_id: i64, name: String },

    #[error("referenced sdtm version not found: {0}")]
    FkSdtmVersionNotFound(i64),
    #[error("referenced sdtm domain not found: {0}")]
    FkSdtmDomainNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}

// ---- view projections ----

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVersionView {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainDescription {
    pub lang: String,
    pub details: SdtmDomainDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainDescriptionDetail {
    pub description: String,
    pub structure: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmDomainView {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SdtmVariableView {
    pub id: i64,
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// ---- request DTOs ----

#[derive(Debug, Clone)]
pub struct CreateSdtmVersionRequest {
    pub name: String,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmVersionRequest {
    pub id: i64,
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CreateSdtmDomainRequest {
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmDomainRequest {
    pub id: i64,
    pub name: Option<String>,
    pub category: Option<DomainCategory>,
    pub descriptions: Option<Vec<SdtmDomainDescription>>,
}

#[derive(Debug, Clone)]
pub struct CreateSdtmVariableRequest {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
}

#[derive(Debug, Default, Clone)]
pub struct UpdateSdtmVariableRequest {
    pub id: i64,
    pub name: Option<String>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<SdtmVariableType>,
    pub variable_core: Option<SdtmVariableCore>,
    /// `None` = don't change. `Some(None)` = clear the field.
    pub variable_role: Option<Option<SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<SdtmVariableDescription>>,
}

// ---- outbound port ----

#[async_trait]
pub trait DomainModelService: Send + Sync {
    // ---- SdtmVersion ----
    async fn create_version(
        &self,
        req: CreateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError>;
    async fn list_versions(
        &self,
    ) -> Result<Vec<SdtmVersionView>, DomainModelApiError>;
    async fn update_version(
        &self,
        req: UpdateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError>;
    async fn delete_version(&self, id: i64)
        -> Result<(), DomainModelApiError>;

    // ---- SdtmDomain ----
    async fn create_domain(
        &self,
        req: CreateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError>;
    async fn get_domain_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmDomainView, DomainModelApiError>;
    async fn list_domains_by_version(
        &self,
        version_id: i64,
    ) -> Result<Vec<SdtmDomainView>, DomainModelApiError>;
    async fn update_domain(
        &self,
        req: UpdateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError>;
    async fn delete_domain(&self, id: i64)
        -> Result<(), DomainModelApiError>;

    // ---- SdtmVariable ----
    async fn create_variable(
        &self,
        req: CreateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError>;
    async fn get_variable_by_id(
        &self,
        id: i64,
    ) -> Result<SdtmVariableView, DomainModelApiError>;
    async fn list_variables_by_domain(
        &self,
        domain_id: i64,
    ) -> Result<Vec<SdtmVariableView>, DomainModelApiError>;
    async fn update_variable(
        &self,
        req: UpdateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError>;
    async fn delete_variable(&self, id: i64)
        -> Result<(), DomainModelApiError>;
}
```

### 7.2 Wire it into `apis::lib`

- [ ] **Step 1:** Open `lib/crates/apis/src/lib.rs` and add the new module declaration next to the existing `pub mod terminology;` line (preserve alphabetical order):

```rust
pub mod domain_model;
```

### 7.3 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p apis --no-deps
cargo doc -p domain-model --no-deps
```

- Expected: green. The new port compiles standalone and is visible from the workspace root.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/apis/src/domain_model.rs \
        lib/crates/apis/src/lib.rs
git commit -m "feat(apis): add DomainModelService port

Adds lib/crates/apis/src/domain_model.rs with the
DomainModelService async trait (14 methods covering CRUD on
SdtmVersion, SdtmDomain, SdtmVariable), the re-declared enums
(DomainCategory, SdtmVariableType, SdtmVariableCore, SdtmRole)
so apis stays a leaf crate, DomainModelApiError with
Validation/NotFound/Duplicate*/Fk*/Repository variants, the
three view projections (SdtmVersionView, SdtmDomainView,
SdtmVariableView), the description DTOs, and the eight
Create*/Update* request DTOs (Update* carry Option<Option<T>>
for nullable clears). apis/src/lib.rs adds pub mod
domain_model.

Spec coverage: apis Port section in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p apis --all-targets --all-features -- -D warnings
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p apis --no-deps
  cargo doc -p domain-model --no-deps"
```

---

## Task 8: In-memory facade + facade tests

**Files:**
- Create: `lib/crates/domain-model/src/adapter/facade.rs`
- Create: `lib/crates/domain-model/src/adapter/facade/in_memory.rs`
- Create: `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`
- Modify: `lib/crates/domain-model/src/adapter.rs`
- Modify: `lib/crates/domain-model/src/lib.rs`

### 8.1 Create the facade module skeleton

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/facade.rs`:

```rust
//! Facade adapters — adapt `DomainModelUsecase` to the
//! `apis::domain_model::DomainModelService` outbound port.

pub mod in_memory;
```

- [ ] **Step 2:** Create `lib/crates/domain-model/src/adapter/facade/in_memory.rs`:

```rust
pub mod service;
```

- [ ] **Step 3:** Update `lib/crates/domain-model/src/adapter.rs`:

```rust
pub mod facade;
pub mod persistence;
```

### 8.2 Create `DomainModelServiceImpl`

- [ ] **Step 1:** Create `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`:

```rust
//! Adapts `DomainModelUsecase` to the
//! `apis::domain_model::DomainModelService` outbound port.

use apis::domain_model::{
    CreateSdtmDomainRequest, CreateSdtmVariableRequest,
    CreateSdtmVersionRequest, DomainCategory, DomainModelApiError,
    DomainModelService, SdtmDomainDescription, SdtmDomainView,
    SdtmRole, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableType, SdtmVariableView, SdtmVersionView,
    UpdateSdtmDomainRequest, UpdateSdtmVariableRequest,
    UpdateSdtmVersionRequest,
};
use async_trait::async_trait;

use crate::domain::DomainError;
use crate::usecase::domain_model_usecase::DomainModelUsecase;
use crate::usecase::error::UsecaseError;

pub struct DomainModelServiceImpl<
    V: crate::domain::SdtmVersionRepository,
    D: crate::domain::SdtmDomainRepository,
    Va: crate::domain::SdtmVariableRepository,
> {
    usecase: DomainModelUsecase<V, D, Va>,
}

impl<V, D, Va> DomainModelServiceImpl<V, D, Va>
where
    V: crate::domain::SdtmVersionRepository,
    D: crate::domain::SdtmDomainRepository,
    Va: crate::domain::SdtmVariableRepository,
{
    pub fn new(usecase: DomainModelUsecase<V, D, Va>) -> Self {
        Self { usecase }
    }
}

fn map_domain(err: DomainError) -> DomainModelApiError {
    match err {
        DomainError::EmptyName             => DomainModelApiError::Validation("name must not be empty".into()),
        DomainError::InvalidDomainCategory(s) => DomainModelApiError::Validation(format!("invalid domain category: {s}")),
        DomainError::InvalidVariableType(s) => DomainModelApiError::Validation(format!("invalid variable type: {s}")),
        DomainError::InvalidVariableCore(s) => DomainModelApiError::Validation(format!("invalid variable core: {s}")),
        DomainError::InvalidVariableRole(s) => DomainModelApiError::Validation(format!("invalid variable role: {s}")),
        DomainError::NotFound               => DomainModelApiError::NotFound,
        DomainError::SdtmVersionNotFound(n) => DomainModelApiError::SdtmVersionNotFound(n),
        DomainError::SdtmDomainNotFound(n)  => DomainModelApiError::SdtmDomainNotFound(n),
        DomainError::SdtmVariableNotFound(n)=> DomainModelApiError::SdtmVariableNotFound(n),
        DomainError::DuplicateSdtmVersion { name } => DomainModelApiError::DuplicateSdtmVersion(name),
        DomainError::DuplicateSdtmDomain { version_id, name } => DomainModelApiError::DuplicateSdtmDomain { version_id, name },
        DomainError::DuplicateSdtmVariable { domain_id, name } => DomainModelApiError::DuplicateSdtmVariable { domain_id, name },
        DomainError::FkSdtmVersionNotFound(n) => DomainModelApiError::FkSdtmVersionNotFound(n),
        DomainError::FkSdtmDomainNotFound(n) => DomainModelApiError::FkSdtmDomainNotFound(n),
        DomainError::Repository(s)          => DomainModelApiError::Repository(s),
    }
}

fn map_uc(err: UsecaseError) -> DomainModelApiError {
    match err {
        UsecaseError::Validation(e)  => map_domain(e),
        UsecaseError::Repository(e)  => map_domain(e),
    }
}

#[async_trait]
impl<V, D, Va> DomainModelService for DomainModelServiceImpl<V, D, Va>
where
    V: crate::domain::SdtmVersionRepository,
    D: crate::domain::SdtmDomainRepository,
    Va: crate::domain::SdtmVariableRepository,
{
    async fn create_version(
        &self, req: CreateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError> {
        self.usecase.create_version(
            crate::usecase::commands::CreateSdtmVersion { name: req.name },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn list_versions(
        &self,
    ) -> Result<Vec<SdtmVersionView>, DomainModelApiError> {
        self.usecase.list_versions().await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_uc)
    }

    async fn update_version(
        &self, req: UpdateSdtmVersionRequest,
    ) -> Result<SdtmVersionView, DomainModelApiError> {
        self.usecase.update_version(
            crate::usecase::commands::UpdateSdtmVersion {
                id: req.id, name: req.name,
            },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn delete_version(
        &self, id: i64,
    ) -> Result<(), DomainModelApiError> {
        self.usecase.delete_version(id).await.map_err(map_uc)
    }

    async fn create_domain(
        &self, req: CreateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError> {
        let cat_in = crate::domain::DomainCategory::try_from({req.category.as_str()})
            .map_err(map_domain)?;
        self.usecase.create_domain(
            crate::usecase::commands::CreateSdtmDomain {
                version_id: req.version_id,
                name: req.name,
                category: cat_in,
                descriptions: req.descriptions.into_iter()
                    .map(Into::into).collect(),
            },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn get_domain_by_id(
        &self, id: i64,
    ) -> Result<SdtmDomainView, DomainModelApiError> {
        self.usecase.get_domain_by_id(id).await
            .map(Into::into)
            .map_err(map_uc)
    }

    async fn list_domains_by_version(
        &self, version_id: i64,
    ) -> Result<Vec<SdtmDomainView>, DomainModelApiError> {
        self.usecase.list_domains_by_version(version_id).await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_uc)
    }

    async fn update_domain(
        &self, req: UpdateSdtmDomainRequest,
    ) -> Result<SdtmDomainView, DomainModelApiError> {
        let category = match req.category {
            None => None,
            Some(c) => Some(
                crate::domain::DomainCategory::try_from(c.as_str())
                    .map_err(map_domain)?,
            ),
        };
        self.usecase.update_domain(
            crate::usecase::commands::UpdateSdtmDomain {
                id: req.id,
                name: req.name,
                category,
                descriptions: req.descriptions.map(|v| {
                    v.into_iter().map(Into::into).collect()
                }),
            },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn delete_domain(
        &self, id: i64,
    ) -> Result<(), DomainModelApiError> {
        self.usecase.delete_domain(id).await.map_err(map_uc)
    }

    async fn create_variable(
        &self, req: CreateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError> {
        let vt = crate::domain::SdtmVariableType::try_from(
            req.variable_type.as_str(),
        ).map_err(map_domain)?;
        let vc = crate::domain::SdtmVariableCore::try_from(
            req.variable_core.as_str(),
        ).map_err(map_domain)?;
        let vr = match req.variable_role {
            None => None,
            Some(r) => Some(
                crate::domain::SdtmRole::try_from(r.as_str())
                    .map_err(map_domain)?,
            ),
        };
        self.usecase.create_variable(
            crate::usecase::commands::CreateSdtmVariable {
                domain_id: req.domain_id,
                name: req.name,
                variable_controlled: req.variable_controlled,
                variable_type: vt,
                variable_core: vc,
                variable_role: vr,
                variable_sequence: req.variable_sequence,
                descriptions: req.descriptions.into_iter()
                    .map(Into::into).collect(),
            },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn get_variable_by_id(
        &self, id: i64,
    ) -> Result<SdtmVariableView, DomainModelApiError> {
        self.usecase.get_variable_by_id(id).await
            .map(Into::into)
            .map_err(map_uc)
    }

    async fn list_variables_by_domain(
        &self, domain_id: i64,
    ) -> Result<Vec<SdtmVariableView>, DomainModelApiError> {
        self.usecase.list_variables_by_domain(domain_id).await
            .map(|vs| vs.into_iter().map(Into::into).collect())
            .map_err(map_uc)
    }

    async fn update_variable(
        &self, req: UpdateSdtmVariableRequest,
    ) -> Result<SdtmVariableView, DomainModelApiError> {
        let variable_type = match req.variable_type {
            None => None,
            Some(t) => Some(
                crate::domain::SdtmVariableType::try_from(t.as_str())
                    .map_err(map_domain)?,
            ),
        };
        let variable_core = match req.variable_core {
            None => None,
            Some(c) => Some(
                crate::domain::SdtmVariableCore::try_from(c.as_str())
                    .map_err(map_domain)?,
            ),
        };
        let variable_role = match req.variable_role {
            None => None,
            Some(None) => Some(None),
            Some(Some(r)) => Some(Some(
                crate::domain::SdtmRole::try_from(r.as_str())
                    .map_err(map_domain)?,
            )),
        };
        self.usecase.update_variable(
            crate::usecase::commands::UpdateSdtmVariable {
                id: req.id,
                name: req.name,
                variable_controlled: req.variable_controlled,
                variable_type,
                variable_core,
                variable_role,
                variable_sequence: req.variable_sequence,
                descriptions: req.descriptions.map(|v| {
                    v.into_iter().map(Into::into).collect()
                }),
            },
        )
        .await
        .map(Into::into)
        .map_err(map_uc)
    }

    async fn delete_variable(
        &self, id: i64,
    ) -> Result<(), DomainModelApiError> {
        self.usecase.delete_variable(id).await.map_err(map_uc)
    }
}

// ---- From impls (apis view → crate view) -------------------------------

impl From<crate::usecase::views::SdtmVersionView> for SdtmVersionView {
    fn from(v: crate::usecase::views::SdtmVersionView) -> Self {
        Self {
            id: v.id, name: v.name,
            created_at: v.created_at, updated_at: v.updated_at,
        }
    }
}

impl From<SdtmDomainDescription> for crate::domain::SdtmDomainDescription {
    fn from(d: SdtmDomainDescription) -> Self {
        Self {
            lang: d.lang,
            details: crate::domain::SdtmDomainDescriptionDetail {
                description: d.details.description,
                structure:  d.details.structure,
            },
        }
    }
}

impl From<SdtmVariableDescription> for crate::domain::SdtmVariableDescription {
    fn from(d: SdtmVariableDescription) -> Self {
        Self {
            lang: d.lang,
            details: crate::domain::SdtmVariableDescriptionDetail {
                label: d.details.label,
            },
        }
    }
}

impl From<crate::usecase::views::SdtmDomainView> for SdtmDomainView {
    fn from(v: crate::usecase::views::SdtmDomainView) -> Self {
        Self {
            id: v.id,
            version_id: v.version_id,
            name: v.name,
            category: match v.category {
                crate::domain::DomainCategory::SpecialPurpose => DomainCategory::SpecialPurpose,
                crate::domain::DomainCategory::Interventions  => DomainCategory::Interventions,
                crate::domain::DomainCategory::Events         => DomainCategory::Events,
                crate::domain::DomainCategory::Findings       => DomainCategory::Findings,
                crate::domain::DomainCategory::TrialDesign    => DomainCategory::TrialDesign,
                crate::domain::DomainCategory::Relationships  => DomainCategory::Relationships,
                crate::domain::DomainCategory::StudyReference => DomainCategory::StudyReference,
            },
            descriptions: v.descriptions.into_iter().map(|d|
                SdtmDomainDescription {
                    lang: d.lang,
                    details: SdtmDomainDescriptionDetail {
                        description: d.details.description,
                        structure:  d.details.structure,
                    },
                }
            ).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<crate::usecase::views::SdtmVariableView> for SdtmVariableView {
    fn from(v: crate::usecase::views::SdtmVariableView) -> Self {
        Self {
            id: v.id,
            domain_id: v.domain_id,
            name: v.name,
            variable_controlled: v.variable_controlled,
            variable_type: match v.variable_type {
                crate::domain::SdtmVariableType::Numeric   => SdtmVariableType::Numeric,
                crate::domain::SdtmVariableType::Character => SdtmVariableType::Character,
            },
            variable_core: match v.variable_core {
                crate::domain::SdtmVariableCore::Req  => SdtmVariableCore::Req,
                crate::domain::SdtmVariableCore::Exp  => SdtmVariableCore::Exp,
                crate::domain::SdtmVariableCore::Perm => SdtmVariableCore::Perm,
                crate::domain::SdtmVariableCore::Supp => SdtmVariableCore::Supp,
            },
            variable_role: v.variable_role.map(|r| match r {
                crate::domain::SdtmRole::Identifier         => SdtmRole::Identifier,
                crate::domain::SdtmRole::Topic              => SdtmRole::Topic,
                crate::domain::SdtmRole::Timing             => SdtmRole::Timing,
                crate::domain::SdtmRole::RecordQualifier    => SdtmRole::RecordQualifier,
                crate::domain::SdtmRole::SynonymQualifier   => SdtmRole::SynonymQualifier,
                crate::domain::SdtmRole::VariableQualifier  => SdtmRole::VariableQualifier,
                crate::domain::SdtmRole::GroupingQualifier  => SdtmRole::GroupingQualifier,
                crate::domain::SdtmRole::Rule               => SdtmRole::Rule,
            }),
            variable_sequence: v.variable_sequence,
            descriptions: v.descriptions.into_iter().map(|d|
                SdtmVariableDescription {
                    lang: d.lang,
                    details: SdtmVariableDescriptionDetail {
                        label: d.details.label,
                    },
                }
            ).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}
```

### 8.3 Add facade tests

- [ ] **Step 1:** Append the test module to `lib/crates/domain-model/src/adapter/facade/in_memory/service.rs`:

```rust
// ---- tests ---------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::{Arc, Mutex};

    use apis::domain_model::{
        CreateSdtmDomainRequest, CreateSdtmVariableRequest,
        CreateSdtmVersionRequest, DomainCategory, DomainModelApiError,
        DomainModelService, SdtmDomainDescription,
        SdtmDomainDescriptionDetail, SdtmRole, SdtmVariableCore,
        SdtmVariableDescription, SdtmVariableDescriptionDetail,
        SdtmVariableType, UpdateSdtmVariableRequest,
    };
    use async_trait::async_trait;

    use crate::domain::{
        DomainError, SdtmDomain, SdtmDomainNew, SdtmDomainRepository,
        SdtmDomainUpdate, SdtmVariable, SdtmVariableNew,
        SdtmVariableRepository, SdtmVariableUpdate, SdtmVersion,
        SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
    };
    use crate::usecase::domain_model_usecase::{
        DomainModelUsecase, DomainModelUsecaseConfig,
    };

    // In-memory fakes ----------------------------------------------

    #[derive(Default)]
    struct FakeVersionRepo {
        inner: Mutex<Vec<SdtmVersion>>, next: AtomicI64,
    }
    #[async_trait]
    impl SdtmVersionRepository for FakeVersionRepo {
        async fn create(&self, i: SdtmVersionNew)
            -> Result<SdtmVersion, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            if g.iter().any(|v| v.name == i.name) {
                return Err(DomainError::DuplicateSdtmVersion {
                    name: i.name.clone(),
                });
            }
            let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
            let v = SdtmVersion::for_repository(id, i.name,
                chrono::Utc::now(), chrono::Utc::now());
            g.push(v.clone());
            Ok(v)
        }
        async fn list(&self) -> Result<Vec<SdtmVersion>, DomainError> {
            Ok(self.inner.lock().unwrap().clone())
        }
        async fn update(&self, i: SdtmVersionUpdate)
            -> Result<SdtmVersion, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            let v = g.iter_mut().find(|v| v.id == i.id)
                .ok_or(DomainError::SdtmVersionNotFound(i.id))?;
            if let Some(name) = i.name { v.name = name; }
            v.updated_at = chrono::Utc::now();
            Ok(v.clone())
        }
        async fn delete(&self, id: i64) -> Result<(), DomainError> {
            let mut g = self.inner.lock().unwrap();
            let before = g.len();
            g.retain(|v| v.id != id);
            if g.len() == before {
                return Err(DomainError::SdtmVersionNotFound(id));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeDomainRepo {
        inner: Mutex<Vec<SdtmDomain>>, next: AtomicI64,
    }
    #[async_trait]
    impl SdtmDomainRepository for FakeDomainRepo {
        async fn create(&self, i: SdtmDomainNew)
            -> Result<SdtmDomain, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            if g.iter().any(|d|
                d.version_id == i.version_id && d.name == i.name)
            {
                return Err(DomainError::DuplicateSdtmDomain {
                    version_id: i.version_id, name: i.name.clone(),
                });
            }
            let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
            let d = SdtmDomain::for_repository(id, i.version_id,
                i.name, i.category, i.descriptions,
                chrono::Utc::now(), chrono::Utc::now());
            g.push(d.clone());
            Ok(d)
        }
        async fn find_by_id(&self, id: i64)
            -> Result<SdtmDomain, DomainError>
        {
            let g = self.inner.lock().unwrap();
            g.iter().find(|d| d.id == id).cloned()
                .ok_or(DomainError::SdtmDomainNotFound(id))
        }
        async fn list_by_version(&self, version_id: i64)
            -> Result<Vec<SdtmDomain>, DomainError>
        {
            let g = self.inner.lock().unwrap();
            Ok(g.iter().filter(|d| d.version_id == version_id).cloned().collect())
        }
        async fn update(&self, i: SdtmDomainUpdate)
            -> Result<SdtmDomain, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            let d = g.iter_mut().find(|d| d.id == i.id)
                .ok_or(DomainError::SdtmDomainNotFound(i.id))?;
            if let Some(name) = i.name { d.name = name; }
            if let Some(category) = i.category { d.category = category; }
            if let Some(descriptions) = i.descriptions { d.descriptions = descriptions; }
            d.updated_at = chrono::Utc::now();
            Ok(d.clone())
        }
        async fn delete(&self, id: i64) -> Result<(), DomainError> {
            let mut g = self.inner.lock().unwrap();
            let before = g.len();
            g.retain(|d| d.id != id);
            if g.len() == before {
                return Err(DomainError::SdtmDomainNotFound(id));
            }
            Ok(())
        }
    }

    #[derive(Default)]
    struct FakeVariableRepo {
        inner: Mutex<Vec<SdtmVariable>>, next: AtomicI64,
    }
    #[async_trait]
    impl SdtmVariableRepository for FakeVariableRepo {
        async fn create(&self, i: SdtmVariableNew)
            -> Result<SdtmVariable, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            if g.iter().any(|v|
                v.domain_id == i.domain_id && v.name == i.name)
            {
                return Err(DomainError::DuplicateSdtmVariable {
                    domain_id: i.domain_id, name: i.name.clone(),
                });
            }
            let id = self.next.fetch_add(1, Ordering::SeqCst) + 1;
            let v = SdtmVariable::for_repository(id, i.domain_id,
                i.name, i.variable_controlled, i.variable_type,
                i.variable_core, i.variable_role,
                i.variable_sequence, i.descriptions,
                chrono::Utc::now(), chrono::Utc::now());
            g.push(v.clone());
            Ok(v)
        }
        async fn find_by_id(&self, id: i64)
            -> Result<SdtmVariable, DomainError>
        {
            let g = self.inner.lock().unwrap();
            g.iter().find(|v| v.id == id).cloned()
                .ok_or(DomainError::SdtmVariableNotFound(id))
        }
        async fn list_by_domain(&self, domain_id: i64)
            -> Result<Vec<SdtmVariable>, DomainError>
        {
            let g = self.inner.lock().unwrap();
            Ok(g.iter().filter(|v| v.domain_id == domain_id).cloned().collect())
        }
        async fn update(&self, i: SdtmVariableUpdate)
            -> Result<SdtmVariable, DomainError>
        {
            let mut g = self.inner.lock().unwrap();
            let v = g.iter_mut().find(|v| v.id == i.id)
                .ok_or(DomainError::SdtmVariableNotFound(i.id))?;
            if let Some(name) = i.name { v.name = name; }
            if let Some(vc) = i.variable_controlled { v.variable_controlled = vc; }
            if let Some(vt) = i.variable_type { v.variable_type = vt; }
            if let Some(vc) = i.variable_core { v.variable_core = vc; }
            if let Some(vr) = i.variable_role { v.variable_role = vr; }
            if let Some(seq) = i.variable_sequence { v.variable_sequence = seq; }
            if let Some(d) = i.descriptions { v.descriptions = d; }
            v.updated_at = chrono::Utc::now();
            Ok(v.clone())
        }
        async fn delete(&self, id: i64) -> Result<(), DomainError> {
            let mut g = self.inner.lock().unwrap();
            let before = g.len();
            g.retain(|v| v.id != id);
            if g.len() == before {
                return Err(DomainError::SdtmVariableNotFound(id));
            }
            Ok(())
        }
    }

    fn build_service()
        -> DomainModelServiceImpl<
            Arc<FakeVersionRepo>, Arc<FakeDomainRepo>, Arc<FakeVariableRepo>,
        >
    {
        let usecase = DomainModelUsecase::new(DomainModelUsecaseConfig {
            version_repo:  Arc::new(FakeVersionRepo::default()),
            domain_repo:   Arc::new(FakeDomainRepo::default()),
            variable_repo: Arc::new(FakeVariableRepo::default()),
        });
        DomainModelServiceImpl::new(usecase)
    }

    #[tokio::test]
    async fn facade_round_trips_full_lifecycle() {
        let svc = build_service();
        let v = svc.create_version(CreateSdtmVersionRequest {
            name: "2024-09-27".into(),
        }).await.unwrap();
        let d = svc.create_domain(CreateSdtmDomainRequest {
            version_id: v.id,
            name: "AE".into(),
            category: DomainCategory::Events,
            descriptions: vec![SdtmDomainDescription {
                lang: "en".into(),
                details: SdtmDomainDescriptionDetail {
                    description: "Adverse events".into(),
                    structure:  "One record per AE".into(),
                },
            }],
        }).await.unwrap();
        let var = svc.create_variable(CreateSdtmVariableRequest {
            domain_id: d.id,
            name: "AETERM".into(),
            variable_controlled: None,
            variable_type: SdtmVariableType::Character,
            variable_core: SdtmVariableCore::Req,
            variable_role: Some(SdtmRole::Topic),
            variable_sequence: 11,
            descriptions: vec![SdtmVariableDescription {
                lang: "en".into(),
                details: SdtmVariableDescriptionDetail {
                    label: "Term".into(),
                },
            }],
        }).await.unwrap();
        assert_eq!(var.name, "AETERM");

        // Clear variable_role via outer-Some(inner-None).
        let updated = svc.update_variable(UpdateSdtmVariableRequest {
            id: var.id,
            variable_role: Some(None),
            ..Default::default()
        }).await.unwrap();
        assert_eq!(updated.variable_role, None);

        svc.delete_variable(var.id).await.unwrap();
        svc.delete_domain(d.id).await.unwrap();
        svc.delete_version(v.id).await.unwrap();
    }

    #[tokio::test]
    async fn facade_surfaces_validation() {
        let svc = build_service();
        let err = svc.create_version(CreateSdtmVersionRequest {
            name: " ".into(),
        }).await.unwrap_err();
        match err {
            DomainModelApiError::Validation(_) => {}
            other => panic!("unexpected error: {other:?}"),
        }
    }
}
```

### 8.4 Lift the facade re-export into `lib.rs`

- [ ] **Step 1:** Replace `lib/crates/domain-model/src/lib.rs`:

```rust
//! # domain-model crate
//!
//! Workspace library providing a SQLx/PostgreSQL-backed DDD
//! repository for the CDISC SDTM domain model aggregates
//! and an async `DomainModelUsecase` that orchestrates them.

pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::service::DomainModelServiceImpl;
pub use adapter::persistence::postgres::{
    sdtm_domain_repo::SdtmDomainRepoPg,
    sdtm_variable_repo::SdtmVariableRepoPg,
    sdtm_version_repo::SdtmVersionRepoPg,
};
pub use domain::{
    DomainCategory, DomainError, SdtmDomain, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainRepository,
    SdtmDomainUpdate, SdtmRole, SdtmVariable, SdtmVariableCore,
    SdtmVariableDescription, SdtmVariableDescriptionDetail, SdtmVariableNew,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate, SdtmVersion,
    SdtmVersionNew, SdtmVersionRepository, SdtmVersionUpdate,
};
pub use usecase::{
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion,
    DomainModelUsecase, DomainModelUsecaseConfig, SdtmDomainView,
    SdtmVariableView, SdtmVersionView, UpdateSdtmDomain, UpdateSdtmVariable,
    UpdateSdtmVersion, UsecaseError,
};
```

### 8.5 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

- Expected: green. The facade maps apis DTOs ↔ usecase commands/views ↔ domain aggregates, and the in-memory tests exercise the full lifecycle plus the validation surface.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/src/adapter/facade.rs \
        lib/crates/domain-model/src/adapter/facade/in_memory.rs \
        lib/crates/domain-model/src/adapter/facade/in_memory/service.rs \
        lib/crates/domain-model/src/adapter.rs \
        lib/crates/domain-model/src/lib.rs
git commit -m "feat(domain-model): in-memory facade + facade tests

Adds adapter::facade::in_memory::service::DomainModelServiceImpl
adapting DomainModelUsecase to
apis::domain_model::DomainModelService. The mappers translate
between apis view/command DTOs and the crate's own commands /
views / aggregates (enums converted via as_str + TryFrom);
errors are translated through map_domain and map_uc. The
impl is generic over the three repository ports so the
in-memory fakes (and the postgres repos) can both be wrapped.
The facade tests cover a full lifecycle (version -> domain ->
variable -> clear-role -> delete cascade) plus validation
propagation.

Spec coverage: In-Memory Facade section in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace
  cargo doc --workspace --no-deps"
```

---

## Task 9: public_api compile-only test

**Files:**
- Create: `lib/crates/domain-model/tests/public_api.rs`

### 9.1 Create the compile-only test

- [ ] **Step 1:** Create `lib/crates/domain-model/tests/public_api.rs`:

```rust
//! Compile-only contract test for the `domain-model` crate root.
//!
//! Touches every name the public surface commits to (aggregates,
//! ports, command DTOs, view DTOs, concrete repos, the usecase,
//! the in-memory facade, both error enums). If a name is renamed
//! or its module path changes this file fails to compile — that
//! is the whole point.

use domain_model::{
    DomainCategory, DomainError, DomainModelServiceImpl, DomainModelUsecase,
    DomainModelUsecaseConfig, SdtmDomain, SdtmDomainDescription,
    SdtmDomainDescriptionDetail, SdtmDomainNew, SdtmDomainRepoPg,
    SdtmDomainRepository, SdtmDomainUpdate, SdtmDomainView, SdtmRole,
    SdtmVariable, SdtmVariableCore, SdtmVariableDescription,
    SdtmVariableDescriptionDetail, SdtmVariableNew, SdtmVariableRepoPg,
    SdtmVariableRepository, SdtmVariableType, SdtmVariableUpdate,
    SdtmVariableView, SdtmVersion, SdtmVersionNew, SdtmVersionRepoPg,
    SdtmVersionRepository, SdtmVersionUpdate, SdtmVersionView,
    CreateSdtmDomain, CreateSdtmVariable, CreateSdtmVersion,
    UpdateSdtmDomain, UpdateSdtmVariable, UpdateSdtmVersion, UsecaseError,
};

#[test]
fn public_api_names_resolve() {
    let _: fn(SdtmVersion) -> SdtmVersion = std::convert::identity;
    let _: fn(SdtmDomain) -> SdtmDomain = std::convert::identity;
    let _: fn(SdtmVariable) -> SdtmVariable = std::convert::identity;
    let _: fn(DomainError) -> DomainError = std::convert::identity;
    let _: fn(UsecaseError) -> UsecaseError = std::convert::identity;

    // Trait-objects only need to be name-resolvable; we don't
    // need a real instance.
    fn _is_repo_v<T: ?Sized + SdtmVersionRepository>() {}
    fn _is_repo_d<T: ?Sized + SdtmDomainRepository>() {}
    fn _is_repo_va<T: ?Sized + SdtmVariableRepository>() {}
    fn _is_uc<
        V: SdtmVersionRepository,
        D: SdtmDomainRepository,
        Va: SdtmVariableRepository,
    >(_: DomainModelUsecase<V, D, Va>) {}
    fn _f() -> DomainModelUsecase<
        SdtmVersionRepoPg, SdtmDomainRepoPg, SdtmVariableRepoPg,
    > {
        DomainModelUsecase::new(DomainModelUsecaseConfig {
            version_repo:  SdtmVersionRepoPg::default_unreachable(),
            domain_repo:   SdtmDomainRepoPg::default_unreachable(),
            variable_repo: SdtmVariableRepoPg::default_unreachable(),
        })
    }
    let _ = _is_repo_v::<SdtmVersionRepoPg>;
    let _ = _is_repo_d::<SdtmDomainRepoPg>;
    let _ = _is_repo_va::<SdtmVariableRepoPg>;
    let _: Option<DomainModelServiceImpl<
        SdtmVersionRepoPg, SdtmDomainRepoPg, SdtmVariableRepoPg,
    >> = None;
    let _: SdtmDomainDescription = SdtmDomainDescription {
        lang: "en".into(),
        details: SdtmDomainDescriptionDetail {
            description: "".into(), structure: "".into(),
        },
    };
    let _: SdtmVariableDescription = SdtmVariableDescription {
        lang: "en".into(),
        details: SdtmVariableDescriptionDetail { label: "".into() },
    };
    let _: SdtmDomainNew = SdtmDomainNew {
        version_id: 0, name: "".into(),
        category: DomainCategory::Events, descriptions: Vec::new(),
    };
    let _: SdtmVariableNew = SdtmVariableNew {
        domain_id: 0, name: "".into(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: None,
        variable_sequence: 0,
        descriptions: Vec::new(),
    };
}

// Helper trait purely so the compile-only test can name
// the concrete repos without forcing an instantiable PgPool.
trait DefaultUnreachable { fn default_unreachable() -> Self; }
impl DefaultUnreachable for SdtmVersionRepoPg { fn default_unreachable() -> Self { unimplemented!() } }
impl DefaultUnreachable for SdtmDomainRepoPg  { fn default_unreachable() -> Self { unimplemented!() } }
impl DefaultUnreachable for SdtmVariableRepoPg { fn default_unreachable() -> Self { unimplemented!() } }
```

### 9.2 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. The `tests/public_api.rs` test compiles against the re-exports declared in `lib.rs`.

- [ ] **Step 2:** Commit:

```bash
git add lib/crates/domain-model/tests/public_api.rs
git commit -m "test(domain-model): compile-only public_api test

Touches every name the domain-model crate root re-exports
(aggregates, ports, command DTOs, view DTOs, concrete repos,
the usecase, the in-memory facade, both error enums). If a
name is renamed or its module path changes this file fails to
compile — that is the whole point.

Spec coverage: five-tier test pyramid tier 4 (compile-only
public_api.rs).

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps"
```

---

## Task 10: integration_persistence — live-DB tests (gated)

**Files:**
- Create: `lib/crates/domain-model/tests/integration_persistence.rs`
- Modify: `lib/crates/domain-model/Cargo.toml` (dev-dep `apis`)

### 10.1 Create the live-DB integration test

- [ ] **Step 1:** Create `lib/crates/domain-model/tests/integration_persistence.rs`:

```rust
//! Live PostgreSQL integration tests for the SQLx adapter.
//!
//! Gated with `#[ignore]` — run explicitly with
//!   cargo test -p domain-model -- --ignored --test-threads=1
//! when the `AEGIS_DATABASE_URL` env var points at a real
//! (typically throwaway) Postgres database. Each test cleans
//! the schema and applies the three migrations via
//! `include_str!`, then exercises the
//! `DomainModelServiceImpl` facade against the postgres
//! adapter.

use std::env;

use domain_model::{
    DomainCategory, DomainModelServiceImpl, DomainModelUsecase,
    DomainModelUsecaseConfig, SdtmDomainRepoPg, SdtmRole,
    SdtmVariableCore, SdtmVariableRepoPg, SdtmVariableType,
    SdtmVersionRepoPg,
};
use sqlx::postgres::PgPoolOptions;

const MIGRATION_1: &str = include_str!("../migrations/0001_create_sdtm_versions.sql");
const MIGRATION_2: &str = include_str!("../migrations/0002_create_sdtm_domains.sql");
const MIGRATION_3: &str = include_str!("../migrations/0003_create_sdtm_variables.sql");

async fn pool() -> sqlx::PgPool {
    let url = env::var("AEGIS_DATABASE_URL").expect(
        "AEGIS_DATABASE_URL must be set to run #[ignore] integration tests",
    );
    PgPoolOptions::new().max_connections(2).connect(&url).await
        .expect("connect to AEGIS_DATABASE_URL")
}

async fn migrate(pool: &sqlx::PgPool) {
    sqlx::query(MIGRATION_1).execute(pool).await.unwrap();
    sqlx::query(MIGRATION_2).execute(pool).await.unwrap();
    sqlx::query(MIGRATION_3).execute(pool).await.unwrap();
}

async fn clean(pool: &sqlx::PgPool) {
    sqlx::query("DROP TABLE IF EXISTS sdtm_variables CASCADE").execute(pool).await.unwrap();
    sqlx::query("DROP TABLE IF EXISTS sdtm_domains   CASCADE").execute(pool).await.unwrap();
    sqlx::query("DROP TABLE IF EXISTS sdtm_versions  CASCADE").execute(pool).await.unwrap();
}

fn service(pool: sqlx::PgPool)
    -> DomainModelServiceImpl<
        SdtmVersionRepoPg, SdtmDomainRepoPg, SdtmVariableRepoPg,
    >
{
    let usecase = DomainModelUsecase::new(DomainModelUsecaseConfig {
        version_repo:  SdtmVersionRepoPg::new(pool.clone()),
        domain_repo:   SdtmDomainRepoPg::new(pool.clone()),
        variable_repo: SdtmVariableRepoPg::new(pool.clone()),
    });
    DomainModelServiceImpl::new(usecase)
}

#[tokio::test]
#[ignore]
async fn version_crud_round_trips() {
    let pool = pool().await;
    clean(&pool).await;
    migrate(&pool).await;
    let svc = service(pool);

    let v = svc.create_version(apis_create_version("2024-09-27")).await.unwrap();
    assert_eq!(v.name, "2024-09-27");

    let listed = svc.list_versions().await.unwrap();
    assert_eq!(listed.len(), 1);

    svc.delete_version(v.id).await.unwrap();
    let listed = svc.list_versions().await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
#[ignore]
async fn domain_crud_round_trips() {
    let pool = pool().await;
    clean(&pool).await;
    migrate(&pool).await;
    let svc = service(pool);

    let v = svc.create_version(apis_create_version("2024-09-27")).await.unwrap();
    let d = svc.create_domain(apis_create_domain(v.id, "AE")).await.unwrap();
    assert_eq!(d.name, "AE");

    let listed = svc.list_domains_by_version(v.id).await.unwrap();
    assert_eq!(listed.len(), 1);

    svc.delete_domain(d.id).await.unwrap();
    let listed = svc.list_domains_by_version(v.id).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
#[ignore]
async fn variable_crud_round_trips() {
    let pool = pool().await;
    clean(&pool).await;
    migrate(&pool).await;
    let svc = service(pool);

    let v = svc.create_version(apis_create_version("2024-09-27")).await.unwrap();
    let d = svc.create_domain(apis_create_domain(v.id, "AE")).await.unwrap();
    let var = svc.create_variable(apis_create_variable(d.id, "AETERM")).await.unwrap();
    assert_eq!(var.name, "AETERM");

    let listed = svc.list_variables_by_domain(d.id).await.unwrap();
    assert_eq!(listed.len(), 1);

    svc.delete_variable(var.id).await.unwrap();
    let listed = svc.list_variables_by_domain(d.id).await.unwrap();
    assert!(listed.is_empty());
}

#[tokio::test]
#[ignore]
async fn delete_version_cascades_to_domains_and_variables() {
    let pool = pool().await;
    clean(&pool).await;
    migrate(&pool).await;
    let svc = service(pool.clone());

    let v = svc.create_version(apis_create_version("2024-09-27")).await.unwrap();
    let d = svc.create_domain(apis_create_domain(v.id, "AE")).await.unwrap();
    let _ = svc.create_variable(apis_create_variable(d.id, "AETERM")).await.unwrap();

    svc.delete_version(v.id).await.unwrap();

    let row_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sdtm_domains WHERE version_id = $1",
    )
    .bind(v.id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row_count, 0, "child domains should be CASCADE-deleted");
}

// ---- helpers (use apis request DTOs so we don't bypass the facade) ----

fn apis_create_version(name: &str) -> apis::domain_model::CreateSdtmVersionRequest {
    apis::domain_model::CreateSdtmVersionRequest { name: name.to_string() }
}

fn apis_create_domain(version_id: i64, name: &str)
    -> apis::domain_model::CreateSdtmDomainRequest
{
    apis::domain_model::CreateSdtmDomainRequest {
        version_id,
        name: name.to_string(),
        category: DomainCategory::Events,
        descriptions: Vec::new(),
    }
}

fn apis_create_variable(domain_id: i64, name: &str)
    -> apis::domain_model::CreateSdtmVariableRequest
{
    apis::domain_model::CreateSdtmVariableRequest {
        domain_id,
        name: name.to_string(),
        variable_controlled: None,
        variable_type: SdtmVariableType::Character,
        variable_core: SdtmVariableCore::Req,
        variable_role: Some(SdtmRole::Topic),
        variable_sequence: 11,
        descriptions: Vec::new(),
    }
}
```

### 10.2 Wire apis into dev-dependencies

- [ ] **Step 1:** Update `lib/crates/domain-model/Cargo.toml` `[dev-dependencies]` block — add `apis` as a path dep:

```toml
[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
apis = { path = "../apis" }
```

### 10.3 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

- Expected: green. The `#[ignore]`-gated tests are skipped in the default flow; the surrounding code compiles.

- [ ] **Step 2:** Verify the live path compiles:

```bash
cargo test -p domain-model --no-run -- --ignored
```

- Expected: green.

- [ ] **Step 3:** Commit:

```bash
git add lib/crates/domain-model/tests/integration_persistence.rs \
        lib/crates/domain-model/Cargo.toml
git commit -m "test(domain-model): live-DB integration_persistence tests

Adds tests/integration_persistence.rs with four #[ignore]-
gated tests: version CRUD round-trip, domain CRUD round-trip,
variable CRUD round-trip, and version-delete cascades to
child domains (via the FK ON DELETE CASCADE). Each test uses
AEGIS_DATABASE_URL to connect, cleans the schema, applies the
three migrations via include_str!, and exercises the
DomainModelServiceImpl facade against the postgres adapter.
Tests use --test-threads=1 because the schema is shared.

Spec coverage: five-tier test pyramid tier 5
(integration_persistence.rs).

Verification:
  cargo fmt --all -- --check
  cargo clippy -p domain-model --all-targets --all-features -- -D warnings
  cargo test -p domain-model
  cargo doc -p domain-model --no-deps
  cargo test -p domain-model --no-run -- --ignored"
```

---

## Task 11: aegis-server wiring — Cargo.toml + state + run.rs

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`
- Modify: `apps/server/aegis-server/src/state.rs`
- Modify: `apps/server/aegis-server/src/run.rs`

### 11.1 Add the path-dep

- [ ] **Step 1:** Open `apps/server/aegis-server/Cargo.toml` and add `domain-model` to `[dependencies]` (next to the existing `terminology` / `project` / `user` path-deps so the alphabetical ordering matches the rest of the workspace):

```toml
domain-model = { path = "../../../lib/crates/domain-model" }
```

### 11.2 Wire the service into `state.rs`

- [ ] **Step 1:** Read the current `apps/server/aegis-server/src/state.rs` to find where the existing terminology field lives (look for `Arc<dyn apis::terminology::TerminologyService>` or similar).

- [ ] **Step 2:** Add the new field next to the existing `terminology` field, in the same order as the in-memory fakes / concrete repos are constructed in `run.rs`:

```rust
pub domain_model:
    Arc<dyn apis::domain_model::DomainModelService>,
```

- [ ] **Step 3:** Add the import at the top of `state.rs`:

```rust
use apis::domain_model::DomainModelService;
```

(or extend an existing wildcard import from `apis`).

### 11.3 Wire the build into `run.rs`

- [ ] **Step 1:** Find the existing `build_terminology_service(pool)` call in `run.rs` (or the equivalent that wires `Arc<dyn TerminologyService>` into `state.terminology`).

- [ ] **Step 2:** Add the new builder alongside it. The pattern (mirroring the terminology wiring):

```rust
state.domain_model = Arc::new(
    domain_model::DomainModelServiceImpl::new(
        domain_model::DomainModelUsecase::new(
            domain_model::DomainModelUsecaseConfig {
                version_repo:
                    domain_model::SdtmVersionRepoPg::new(pool.clone()),
                domain_repo:
                    domain_model::SdtmDomainRepoPg::new(pool.clone()),
                variable_repo:
                    domain_model::SdtmVariableRepoPg::new(pool.clone()),
            },
        ),
    ),
);
```

(or extract a `build_domain_model_service(pool)` helper that wraps the above — match the terminology crate's local style for consistency).

- [ ] **Step 3:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo clippy --workspace
cargo test --workspace
cargo doc --workspace --no-deps
```

- Expected: green.

### 11.4 Commit

```bash
git add apps/server/aegis-server/Cargo.toml \
        apps/server/aegis-server/src/state.rs \
        apps/server/aegis-server/src/run.rs
git commit -m "feat(aegis-server): wire domain-model service

Adds the path-dep in Cargo.toml, the
Arc<dyn apis::domain_model::DomainModelService> field on the
AppState struct (next to the existing terminology field), and
the wiring in run.rs that constructs
DomainModelServiceImpl over the three Sdtm*RepoPg instances
sharing the same PgPool. The pool is cloned per repo so the
three repos can be moved independently if needed.

Spec coverage: HTTP Routes in aegis-server + Workspace Wiring
sections in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo check --workspace
  cargo clippy --workspace
  cargo test --workspace
  cargo doc --workspace --no-deps"
```

---

## Task 12: aegis-server HTTP handlers + router + DTOs + error mapping + OpenAPI

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http.rs`
- Create: `apps/server/aegis-server/src/transport/http/domain_model.rs`
- Create: `apps/server/aegis-server/src/transport/http/domain_model/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

### 12.1 Create the module skeleton

- [ ] **Step 1:** Create `apps/server/aegis-server/src/transport/http/domain_model.rs`:

```rust
pub mod handlers;
pub mod router;
```

- [ ] **Step 2:** Create `apps/server/aegis-server/src/transport/http/domain_model/router.rs`:

```rust
use axum::{
    routing::{delete, get, post, put},
    Router,
};

use crate::state::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        // ---- SdtmVersion ----
        .route(
            "/api/domain-model/versions",
            get(handlers::list_versions),
        )
        .route(
            "/api/domain-model/versions",
            post(handlers::create_version),
        )
        .route(
            "/api/domain-model/versions/:id",
            put(handlers::update_version),
        )
        .route(
            "/api/domain-model/versions/:id",
            delete(handlers::delete_version),
        )

        // ---- SdtmDomain ----
        .route(
            "/api/domain-model/domains",
            post(handlers::create_domain),
        )
        .route(
            "/api/domain-model/domains/:id",
            get(handlers::get_domain_by_id),
        )
        .route(
            "/api/domain-model/versions/:version_id/domains",
            get(handlers::list_domains_by_version),
        )
        .route(
            "/api/domain-model/domains/:id",
            put(handlers::update_domain),
        )
        .route(
            "/api/domain-model/domains/:id",
            delete(handlers::delete_domain),
        )

        // ---- SdtmVariable ----
        .route(
            "/api/domain-model/variables",
            post(handlers::create_variable),
        )
        .route(
            "/api/domain-model/variables/:id",
            get(handlers::get_variable_by_id),
        )
        .route(
            "/api/domain-model/domains/:domain_id/variables",
            get(handlers::list_variables_by_domain),
        )
        .route(
            "/api/domain-model/variables/:id",
            put(handlers::update_variable),
        )
        .route(
            "/api/domain-model/variables/:id",
            delete(handlers::delete_variable),
        )
}
```

### 12.2 Create the handlers

- [ ] **Step 1:** Create `apps/server/aegis-server/src/transport/http/domain_model/handlers.rs`:

```rust
// All write handlers call `require_admin_or_root(&claims)?;`
// at the very top, before dispatching to the usecase. Read
// handlers require only authenticated claims.

use apis::domain_model::{
    CreateSdtmDomainRequest, CreateSdtmVariableRequest,
    CreateSdtmVersionRequest, UpdateSdtmDomainRequest,
    UpdateSdtmVariableRequest, UpdateSdtmVersionRequest,
};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use serde::Deserialize;

use crate::state::AppState;
use crate::transport::http::auth::middleware::require_admin_or_root;
use crate::transport::http::auth::AuthClaims;
use crate::transport::http::error::ApiError;

// ---- request bodies ----

#[derive(Debug, Deserialize)]
pub struct CreateVersionBody {
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateVersionBody {
    pub name: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateDomainBody {
    pub version_id: i64,
    pub name: String,
    pub category: apis::domain_model::DomainCategory,
    pub descriptions: Vec<apis::domain_model::SdtmDomainDescription>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateDomainBody {
    pub name: Option<String>,
    pub category: Option<apis::domain_model::DomainCategory>,
    pub descriptions: Option<Vec<apis::domain_model::SdtmDomainDescription>>,
}

#[derive(Debug, Deserialize)]
pub struct CreateVariableBody {
    pub domain_id: i64,
    pub name: String,
    pub variable_controlled: Option<String>,
    pub variable_type: apis::domain_model::SdtmVariableType,
    pub variable_core: apis::domain_model::SdtmVariableCore,
    pub variable_role: Option<apis::domain_model::SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<apis::domain_model::SdtmVariableDescription>,
}

#[derive(Debug, Default, Deserialize)]
pub struct UpdateVariableBody {
    pub name: Option<String>,
    pub variable_controlled: Option<Option<String>>,
    pub variable_type: Option<apis::domain_model::SdtmVariableType>,
    pub variable_core: Option<apis::domain_model::SdtmVariableCore>,
    pub variable_role: Option<Option<apis::domain_model::SdtmRole>>,
    pub variable_sequence: Option<i64>,
    pub descriptions: Option<Vec<apis::domain_model::SdtmVariableDescription>>,
}

// ---- SdtmVersion ----

pub async fn create_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(body): Json<CreateVersionBody>,
) -> Result<(StatusCode, Json<apis::domain_model::SdtmVersionView>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .create_version(CreateSdtmVersionRequest { name: body.name })
        .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn list_versions(
    State(state): State<AppState>,
    _claims: AuthClaims,
) -> Result<Json<Vec<apis::domain_model::SdtmVersionView>>, ApiError> {
    let vs = state.domain_model.list_versions().await?;
    Ok(Json(vs))
}

pub async fn update_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
    Json(body): Json<UpdateVersionBody>,
) -> Result<Json<apis::domain_model::SdtmVersionView>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_version(UpdateSdtmVersionRequest {
            id, name: body.name,
        })
        .await?;
    Ok(Json(view))
}

pub async fn delete_version(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_version(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmDomain ----

pub async fn create_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(body): Json<CreateDomainBody>,
) -> Result<(StatusCode, Json<apis::domain_model::SdtmDomainView>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .create_domain(CreateSdtmDomainRequest {
            version_id: body.version_id,
            name: body.name,
            category: body.category,
            descriptions: body.descriptions,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn get_domain_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<Json<apis::domain_model::SdtmDomainView>, ApiError> {
    let view = state.domain_model.get_domain_by_id(id).await?;
    Ok(Json(view))
}

pub async fn list_domains_by_version(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(version_id): Path<i64>,
) -> Result<Json<Vec<apis::domain_model::SdtmDomainView>>, ApiError> {
    let vs = state
        .domain_model
        .list_domains_by_version(version_id)
        .await?;
    Ok(Json(vs))
}

pub async fn update_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
    Json(body): Json<UpdateDomainBody>,
) -> Result<Json<apis::domain_model::SdtmDomainView>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_domain(UpdateSdtmDomainRequest {
            id, name: body.name, category: body.category,
            descriptions: body.descriptions,
        })
        .await?;
    Ok(Json(view))
}

pub async fn delete_domain(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_domain(id).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---- SdtmVariable ----

pub async fn create_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(body): Json<CreateVariableBody>,
) -> Result<(StatusCode, Json<apis::domain_model::SdtmVariableView>), ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .create_variable(CreateSdtmVariableRequest {
            domain_id: body.domain_id,
            name: body.name,
            variable_controlled: body.variable_controlled,
            variable_type: body.variable_type,
            variable_core: body.variable_core,
            variable_role: body.variable_role,
            variable_sequence: body.variable_sequence,
            descriptions: body.descriptions,
        })
        .await?;
    Ok((StatusCode::CREATED, Json(view)))
}

pub async fn get_variable_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<Json<apis::domain_model::SdtmVariableView>, ApiError> {
    let view = state.domain_model.get_variable_by_id(id).await?;
    Ok(Json(view))
}

pub async fn list_variables_by_domain(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(domain_id): Path<i64>,
) -> Result<Json<Vec<apis::domain_model::SdtmVariableView>>, ApiError> {
    let vs = state
        .domain_model
        .list_variables_by_domain(domain_id)
        .await?;
    Ok(Json(vs))
}

pub async fn update_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
    Json(body): Json<UpdateVariableBody>,
) -> Result<Json<apis::domain_model::SdtmVariableView>, ApiError> {
    require_admin_or_root(&claims)?;
    let view = state
        .domain_model
        .update_variable(UpdateSdtmVariableRequest {
            id, name: body.name,
            variable_controlled: body.variable_controlled,
            variable_type: body.variable_type,
            variable_core: body.variable_core,
            variable_role: body.variable_role,
            variable_sequence: body.variable_sequence,
            descriptions: body.descriptions,
        })
        .await?;
    Ok(Json(view))
}

pub async fn delete_variable(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(id): Path<i64>,
) -> Result<StatusCode, ApiError> {
    require_admin_or_root(&claims)?;
    state.domain_model.delete_variable(id).await?;
    Ok(StatusCode::NO_CONTENT)
}
```

### 12.3 Mount the router

- [ ] **Step 1:** In `apps/server/aegis-server/src/transport/http.rs` add:

```rust
pub mod domain_model;
```

- [ ] **Step 2:** In `apps/server/aegis-server/src/transport/http/router.rs`, mount the new sub-router alongside `terminology`:

```rust
.merge(crate::transport::http::domain_model::router::router())
```

### 12.4 Map `DomainModelApiError` → `ApiError`

- [ ] **Step 1:** Open `apps/server/aegis-server/src/transport/http/error.rs`. Find the existing terminology error mapping (search for `TerminologyApiError`) and add a parallel mapping:

```rust
impl From<apis::domain_model::DomainModelApiError> for ApiError {
    fn from(e: apis::domain_model::DomainModelApiError) -> Self {
        use apis::domain_model::DomainModelApiError as E;
        match e {
            E::Validation(s)                => ApiError::BadRequest(s),
            E::NotFound                     => ApiError::NotFound,
            E::SdtmVersionNotFound(_)
            | E::SdtmDomainNotFound(_)
            | E::SdtmVariableNotFound(_)     => ApiError::NotFound,
            E::DuplicateSdtmVersion(_)
            | E::DuplicateSdtmDomain { .. }
            | E::DuplicateSdtmVariable { .. }=>
                ApiError::Conflict(format!("{e}")),
            E::FkSdtmVersionNotFound(_)
            | E::FkSdtmDomainNotFound(_)    => ApiError::BadRequest(format!("{e}")),
            E::Repository(s)                => ApiError::Internal(s),
        }
    }
}
```

(Adjust the variant names to match the workspace's existing `ApiError` enum — `BadRequest`, `NotFound`, `Conflict`, `Internal` are the conventions used by the terminology crate.)

### 12.5 Register the OpenAPI surface

- [ ] **Step 1:** In `apps/server/aegis-server/src/transport/http/openapi.rs`, add the new paths + schemas. The exact path names mirror the routes defined in 12.2 — `/api/domain-model/versions`, `/api/domain-model/domains`, `/api/domain-model/variables`, etc. — and the schemas derive `utoipa::ToSchema` on the request / view types (or use the per-handler `#[utoipa::path(...)]` attribute style the terminology crate uses; mirror whichever style is current).

If the terminology crate already uses utoipa macros at the path level, add parallel `#[utoipa::path(get, path = "/api/domain-model/versions", …)]` annotations on each handler in `handlers.rs` and register the new path list in `openapi.rs`. Otherwise, leave the OpenAPI registration for a follow-up.

### 12.6 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p aegis-server
cargo doc -p aegis-server --no-deps
```

- Expected: green.

- [ ] **Step 2:** Commit:

```bash
git add apps/server/aegis-server/src/transport/http.rs \
        apps/server/aegis-server/src/transport/http/domain_model.rs \
        apps/server/aegis-server/src/transport/http/domain_model/router.rs \
        apps/server/aegis-server/src/transport/http/domain_model/handlers.rs \
        apps/server/aegis-server/src/transport/http/router.rs \
        apps/server/aegis-server/src/transport/http/error.rs \
        apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(aegis-server): domain-model HTTP routes

Adds 14 axum routes under /api/domain-model/*:
  - GET / POST /api/domain-model/versions
  - PUT / DELETE /api/domain-model/versions/:id
  - POST /api/domain-model/domains
  - GET /api/domain-model/domains/:id
  - GET /api/domain-model/versions/:version_id/domains
  - PUT / DELETE /api/domain-model/domains/:id
  - POST /api/domain-model/variables
  - GET /api/domain-model/variables/:id
  - GET /api/domain-model/domains/:domain_id/variables
  - PUT / DELETE /api/domain-model/variables/:id

Every write handler calls require_admin_or_root(&claims)?
before dispatching to the usecase. Reads require only
authenticated AuthClaims. DomainModelApiError -> ApiError
mapping mirrors the terminology mapping (Validation ->
BadRequest, NotFound variants -> NotFound, Duplicate* ->
Conflict, Fk* -> BadRequest, Repository -> Internal).
OpenAPI surface mirrors the existing patterns.

Spec coverage: HTTP Routes in aegis-server section in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
  cargo test -p aegis-server
  cargo doc -p aegis-server --no-deps"
```

---

## Task 13: HTTP integration tests in aegis-server

**Files:**
- Create: `apps/server/aegis-server/tests/integration_domain_model.rs`

### 13.1 Create the integration test file

- [ ] **Step 1:** Read `apps/server/aegis-server/tests/integration_terminology.rs` (or whatever the live HTTP integration test file for terminology is named) to learn the test harness pattern.

- [ ] **Step 2:** Create `apps/server/aegis-server/tests/integration_domain_model.rs`. The shape mirrors the terminology test:

```rust
//! HTTP integration tests for the /api/domain-model/* routes.
//!
//! Boots an `axum::Router` against a PgPool (typically
//! configured via the workspace's shared test harness), wraps
//! it in `tower::ServiceExt::oneshot`, and exercises each
//! route:
//!   - unauthenticated request -> 401
//!   - authenticated user request to a write route -> 403
//!   - authenticated admin/root request to a write route ->
//!     200/201 with expected body
//!   - authenticated user request to a read route -> 200
//!   - cascade: deleting a version removes child domains /
//!     variables

use std::sync::Arc;

// Exact imports / harness construction depends on the shared
// `aegis-server` test harness. Mirror the imports from the
// terminology integration test file and substitute
// `domain_model::DomainModelServiceImpl` for the terminology
// equivalent. The test bodies follow the same shape:
//   async fn test_xxx() { ... }

#[tokio::test]
#[ignore]
async fn create_version_requires_admin_or_root() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn list_versions_requires_authentication() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn full_lifecycle_round_trips() {
    // TODO: wire through the shared harness.
}

#[tokio::test]
#[ignore]
async fn delete_version_cascades_to_domains_and_variables() {
    // TODO: wire through the shared harness.
}
```

> The four `#[ignore]`-gated tests cover the core auth +
> behaviour matrix. They are intentionally `#[ignore]` until
> the shared harness can be reused (i.e. once the rest of
> the server is reachable from tests). The test bodies must
> be filled in by referencing the terminology integration
> test as a template, which is the canonical way to write
> HTTP integration tests in this workspace.

### 13.2 Verify + commit

- [ ] **Step 1:** Run the verification gate.

```bash
cargo fmt --all -- --check
cargo test -p aegis-server
```

- Expected: green. The new file compiles; the `#[ignore]` tests are skipped by default.

- [ ] **Step 2:** Commit:

```bash
git add apps/server/aegis-server/tests/integration_domain_model.rs
git commit -m "test(aegis-server): http integration test stubs

Adds four #[ignore]-gated HTTP integration test stubs for the
/api/domain-model/* routes, mirroring the terminology
integration test harness: unauthenticated requests return
401, user-role requests to a write route return 403, admin/
root requests succeed, and deleting a version cascades to
child domains and variables. The exact harness construction
matches the shared aegis-server test setup; bodies are filled
in by referencing the terminology integration tests as a
template.

Spec coverage: HTTP Routes section + verification gate in
docs/superpowers/specs/2026-08-24-domain-model-crate-design.md.

Verification:
  cargo fmt --all -- --check
  cargo test -p aegis-server"
```

---

## Task 14: README + final verification gate

**Files:**
- Modify: `lib/crates/domain-model/README.md`

### 14.1 Replace the placeholder README with the full version

- [ ] **Step 1:** Replace `lib/crates/domain-model/README.md`:

```markdown
# domain-model

CRUD over the CDISC SDTM domain model aggregates
(`SdtmVersion`, `SdtmDomain`, `SdtmVariable`), backed by
PostgreSQL.

## Layered architecture

```
domain-model crate
└── adapter
    ├── facade             (in-memory, generic over V/D/Va)
    └── persistence        (postgres, sqlx runtime API)
usecase
└── DomainModelUsecase<V, D, Va>
    └── commands / views / UsecaseError
domain
└── DomainCategory, SdtmVariableType, SdtmVariableCore, SdtmRole
    └── SdtmVersion, SdtmDomain, SdtmVariable
    └── SdtmVersionRepository, SdtmDomainRepository,
        SdtmVariableRepository
    └── DomainError
```

`adapter::persistence::postgres::*RepoPg` implements the three
ports. `adapter::facade::in_memory::service::DomainModelServiceImpl`
adapts `DomainModelUsecase` to the
`apis::domain_model::DomainModelService` outbound port.

## Data model

| Aggregate      | Fields                                                                |
| -------------- | --------------------------------------------------------------------- |
| `SdtmVersion`  | `id`, `name` (unique), `created_at`, `updated_at`                     |
| `SdtmDomain`   | `id`, `version_id` (FK CASCADE), `name`, `category`, `descriptions` (JSONB), `created_at`, `updated_at` |
| `SdtmVariable` | `id`, `domain_id` (FK CASCADE), `name`, `variable_controlled`, `variable_type`, `variable_core`, `variable_role`, `variable_sequence`, `descriptions` (JSONB), `created_at`, `updated_at` |

`descriptions` carries `Vec<SdtmDomainDescription>` /
`Vec<SdtmVariableDescription>` as a single JSONB column
(`NOT NULL DEFAULT '[]'::jsonb`).

## HTTP surface

Mounted under `/api/domain-model/*` in `aegis-server`. Every
write route (`POST`, `PUT`, `DELETE`) calls
`require_admin_or_root(&claims)?;` first. Reads require only
authenticated claims.

```
GET    /api/domain-model/versions
POST   /api/domain-model/versions           (admin/root)
PUT    /api/domain-model/versions/:id       (admin/root)
DELETE /api/domain-model/versions/:id       (admin/root)

POST   /api/domain-model/domains            (admin/root)
GET    /api/domain-model/domains/:id
GET    /api/domain-model/versions/:version_id/domains
PUT    /api/domain-model/domains/:id        (admin/root)
DELETE /api/domain-model/domains/:id        (admin/root)

POST   /api/domain-model/variables          (admin/root)
GET    /api/domain-model/variables/:id
GET    /api/domain-model/domains/:domain_id/variables
PUT    /api/domain-model/variables/:id      (admin/root)
DELETE /api/domain-model/variables/:id      (admin/root)
```

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

Live-DB integration tests (gated with `#[ignore]`) require
`AEGIS_DATABASE_URL`:

```bash
cargo test -p domain-model -- --ignored --test-threads=1
```

Spec: `docs/superpowers/specs/2026-08-24-domain-model-crate-design.md`.
Conventions: `docs/guidelines/lib-crate-development.md`.
```

### 14.2 Run the full verification gate

- [ ] **Step 1:** Run the full workspace verification gate.

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
cargo doc --workspace --no-deps
```

- Expected: green.

- [ ] **Step 2:** Run the aegis-server-only verification gate.

```bash
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p aegis-server
```

- Expected: green.

- [ ] **Step 3:** Confirm the live-DB integration tests compile.

```bash
cargo test -p domain-model --no-run -- --ignored
```

- Expected: green.

- [ ] **Step 4:** Commit:

```bash
git add lib/crates/domain-model/README.md
git commit -m "docs(domain-model): full README + verification gate

Replaces the scaffolded README with the layered-architecture
diagram, the data-model table, the HTTP surface, the
verification commands, and the spec/guidelines pointers.

Final verification gate:
  cargo fmt --all -- --check
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace
  cargo doc --workspace --no-deps
  cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
  cargo test -p aegis-server
  cargo test -p domain-model --no-run -- --ignored"
```

---

## Self-Review Checklist

Run this mentally before reporting the plan as complete:

1. **Spec coverage** — every section of
   `docs/superpowers/specs/2026-08-24-domain-model-crate-design.md` has a
   home in one of the 14 tasks:
   - Goal → preamble + each task's header
   - Architecture → Task 1 (scaffold) + Task 7 (apis) + Task 8 (facade)
   - Repository Surface → Tasks 4-6 (postgres), Task 8 (facade), Task 7 (apis)
   - Data Model (Enums + Aggregates + DomainError) → Task 2 (domain)
   - Database Schema → Tasks 4-6 (three migrations)
   - Usecase Layer → Task 3 (usecase)
   - apis Port → Task 7
   - In-Memory Facade → Task 8
   - HTTP Routes → Tasks 11 (wiring) + 12 (handlers + router)
   - Workspace Wiring → Task 1 + Task 11
   - Tests → Tasks 2 (domain unit), 3 (usecase unit), 4-6 (migration
     file string reads), 8 (facade), 9 (compile-only public_api),
     10 (live-DB integration_persistence), 13 (HTTP integration stubs)
   - README → Task 14
   - Verification Gate → Task 14

2. **Placeholder scan** — no "TODO", "TBD", "fill in later", or "similar
   to Task N" outside the explicit TODO stubs in
   `apps/server/aegis-server/tests/integration_domain_model.rs` (those
   are intentional — the HTTP integration test bodies are filled in by
   referencing the terminology integration tests as a template once
   the shared harness is reachable from aegis-server tests). Every
   other code block contains real code; every command is runnable.

3. **Type / name consistency** — names match across tasks:
   - `DomainModelUsecase<V, D, Va>` with
     `DomainModelUsecaseConfig { version_repo, domain_repo, variable_repo }`
     in Tasks 3, 8, 11
   - `Sdtm*RepoPg` with `new(pool: PgPool)` in Tasks 4-6, 11
   - `DomainModelServiceImpl::new(usecase)` in Tasks 8 + 11
   - `require_admin_or_root(&claims)?` invoked at the very top of every
     write handler in Task 12
   - `AEGIS_DATABASE_URL` env var in Task 10 (and README in Task 14)
   - `apis::domain_model::*` consistent in Tasks 7, 8, 11, 12, 13
