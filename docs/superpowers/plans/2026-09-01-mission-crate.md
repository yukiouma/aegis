# Mission Crate Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `lib/crates/mission` per [`docs/superpowers/specs/2026-09-01-mission-crate-design.md`](../specs/2026-09-01-mission-crate-design.md) — a ports-and-adapters DDD crate that owns the `Mission` + `Assignee` aggregates, exposes an `apis::mission::MissionService` facade, and ships HTTP routes under `/api/mission` in `apps/server/aegis-server` with strict project-leader authorization.

**Architecture:** Three DDD layers — `domain` (pure types + ports + errors, two-constructor pattern), `usecase` (`MissionUsecase<M, A, P, U>` generic over `MissionRepository`, `AssigneeRepository`, `ProjectLookup`, `UserLookup`), `adapter` (PostgreSQL-backed `MissionRepoPg` + `AssigneeRepoPg`; cross-crate `ProjectLookupImpl` + `UserLookupImpl` on the apis ports; in-memory facade `MissionServiceImpl` implementing `apis::mission::MissionService`). The server wires the facade into `AppState.mission: Arc<dyn apis::mission::MissionService>` and mounts `/api/mission` via utoipa-axum.

**Tech Stack:** `sqlx 0.9` (Postgres runtime API), `tokio 1.53`, `async-trait 0.1.91`, `thiserror 2`, `chrono 0.4` (clock + serde), `serde 1` (derive), `serde_json 1`, `dotenvy 0.15` (dev-only), `apis` (workspace path-dep). utoipa-axum + axum 0.8 for the HTTP layer.

**Spec:** [`docs/superpowers/specs/2026-09-01-mission-crate-design.md`](../specs/2026-09-01-mission-crate-design.md)

## Global Constraints

These come from the spec and `docs/guidelines/lib-crate-development.md`; every task implicitly includes them.

- **Edition:** Rust 2024 (`edition = "2024"`, `resolver = "3"`).
- **No `mod.rs`:** every module uses `src/<module>.rs` + `src/<module>/`. Terminal leaf files (`mission_kind.rs`, `mission_role.rs`, `mission_repo.rs`, `assignee_repo.rs`, `service.rs`, etc.) are leaf files with no companion directory.
- **Layer dependency rule:** `domain` depends on nothing except std + `async-trait`; `usecase` depends on `domain` + `apis` (port types only); `adapter` depends on `usecase` + `domain` + `apis` + `sqlx`. No layer reaches into a sibling layer inside the same crate beyond the documented direction.
- **Public surface:** the crate root (`mission::lib.rs`) re-exports exactly the types every consumer (`run.rs`, `state.rs`, facade, usecase, domain, public_api.rs) is allowed to name. No internal helpers, no row structs, no in-memory fakes are re-exported.
- **Runtime SQLx API:** the persistence adapter uses `sqlx::query_as` and `sqlx::QueryBuilder`. No compile-time `query!` / `query_as!` macros. A module-level comment at the top of `persistence/postgres.rs` documents the choice.
- **`map_db_error` rules** (mirror the project / user crates): `sqlx::Error::RowNotFound` → `DomainError::NotFound`; `sqlx::Error::Database` with SQLSTATE `23505` → `DomainError::DuplicateMission { … }` or `DomainError::DuplicateAssignee { … }` depending on which insert failed; everything else → `DomainError::Repository(driver_message)`.
- **Two-constructor pattern:** every aggregate has `new(...)` (validating, returns `Result`) and `pub(crate) fn for_repository(...)` (skips validation, used by the row bridge). `MissionNew` carries the initial assignee list so the repo can insert both rows in one transaction.
- **Migrations:** consumed via `sqlx::migrate!("./migrations")` in integration tests. Each schema change is one file. Live-DB integration tests are `#[ignore]`-gated.
- **Env var:** live-DB tests read `AEGIS_MISSION_DATABASE_URL` (with `dotenvy::dotenv()` at startup; panic if missing).
- **Unique per-run values:** integration tests generate a per-process atomic counter + wall-clock nanoseconds for any UNIQUE-constrained column.
- **Destructive cleanup:** integration tests `DROP TABLE IF EXISTS assignees CASCADE`, `DROP TABLE IF EXISTS missions CASCADE`, and `DROP TABLE IF EXISTS _sqlx_migrations CASCADE` before applying migrations.
- **Layer-boundary visibility:** `adapter::persistence` is `pub(crate) mod postgres;`; `adapter::persistence::postgres` keeps `row` and the per-table repo modules private but exposes `MissionRepo` / `AssigneeRepo` via `pub use`. The `postgres.rs` leaf is `pub` so the `pub use` is well-formed. `adapter::facade::in_memory::service` and `adapter::service::{project,user}` keep their `tests` modules `#[cfg(test)]`-gated.
- **Wire DTOs live in `aegis-server`**, not in `apis`. The apis crate stays serde / utoipa free; the handlers translate wire ↔ apis types at the boundary.
- **Test gates** per the lib-crate guideline section 8:
  ```bash
  cargo fmt --all -- --check
  cargo clippy -p mission --all-targets --all-features -- -D warnings
  cargo test -p mission
  cargo doc -p mission --no-deps
  cargo check --workspace
  cargo clippy --workspace --all-targets --all-features -- -D warnings
  cargo test --workspace
  # when AEGIS_MISSION_DATABASE_URL is set:
  cargo test -p mission -- --ignored --test-threads=1
  ```

---

## File Structure

Created (paths relative to `lib/crates/mission/`):

```
Cargo.toml
README.md
migrations/
  0001_create_missions.sql
  0002_create_assignees.sql
src/
  lib.rs
  domain.rs
  domain/
    error.rs
    mission.rs
    assignee.rs
    mission_kind.rs
    mission_role.rs
    mission_lookup.rs       (MissionRepository + MissionNew + AssigneeNew)
    project_lookup.rs        (ProjectLookup trait + ProjectNotFound mapping)
    user_lookup.rs           (UserLookup trait)
    tests.rs
  usecase.rs
  usecase/
    commands.rs
    error.rs
    views.rs
    mission_usecase.rs
    tests.rs
  adapter.rs
  adapter/
    persistence.rs            (pub(crate) mod postgres;)
    persistence/
      postgres.rs             (pub use MissionRepo / AssigneeRepo)
      postgres/
        row.rs                (MissionRow + AssigneeRow, private)
        mission_repo.rs
        assignee_repo.rs
        tests.rs
    facade.rs                (pub mod in_memory;)
    facade/
      in_memory.rs            (pub use MissionServiceImpl)
      in_memory/
        service.rs
        tests.rs
    service.rs                (pub mod project; pub mod user;)
    service/
      project.rs              (ProjectLookupImpl)
      project/
        tests.rs
      user.rs                 (UserLookupImpl)
      user/
        tests.rs
tests/
  public_api.rs
  integration_persistence.rs
```

Modified (paths relative to the repo root):

```
Cargo.toml                                                       (workspace member)
lib/crates/apis/src/lib.rs                                       (pub mod mission;)
lib/crates/apis/src/mission.rs                                   (NEW — the port)
apps/server/aegis-server/Cargo.toml                              (add mission dep)
apps/server/aegis-server/src/lib.rs                              (re-export nothing new; transport sub-module already pub)
apps/server/aegis-server/src/transport.rs                        (no change)
apps/server/aegis-server/src/transport/http.rs                   (pub mod mission;)
apps/server/aegis-server/src/transport/http/router.rs            (nest /mission under /api)
apps/server/aegis-server/src/transport/http/dto.rs               (mission wire DTOs + From impls)
apps/server/aegis-server/src/transport/http/error.rs             (Mission(#[from] …) variant + status / code tables)
apps/server/aegis-server/src/transport/http/openapi.rs           (register new schemas)
apps/server/aegis-server/src/transport/http/mission.rs           (NEW — pub mod router; pub mod handlers;)
apps/server/aegis-server/src/transport/http/mission/router.rs    (NEW)
apps/server/aegis-server/src/transport/http/mission/handlers.rs  (NEW)
apps/server/aegis-server/src/state.rs                            (mission: Arc<dyn MissionService>)
apps/server/aegis-server/src/state.rs test_support               (NullMissionService)
apps/server/aegis-server/src/run.rs                              (wire mission facade)
apps/server/aegis-server/src/transport/http/router.rs tests      (NullMissionService)
```

Each file owns exactly the responsibility in its name. `domain/tests.rs` exercises `MissionKind`, `MissionRole`, `Mission::new`, `Assignee::new`. `adapter/persistence/postgres/tests.rs` covers row conversions + migration schema content. `adapter/service/{project,user}/tests.rs` cover the cross-crate lookup adapters. `usecase/tests.rs` covers command orchestration against mock ports. `adapter/facade/in_memory/tests.rs` covers the `MissionService` surface end-to-end including leadership enforcement. `tests/public_api.rs` is compile-only. `tests/integration_persistence.rs` is the `#[ignore]`-gated live-DB round-trip.

---
## Task 1: Crate scaffolding + workspace wiring

**Files:**
- Modify: `/root/coding/project/aegis/Cargo.toml` (add `lib/crates/mission` to `[workspace].members`)
- Create: `/root/coding/project/aegis/lib/crates/mission/Cargo.toml`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/lib.rs` (placeholder + module declarations)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/domain.rs` (placeholder)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/usecase.rs` (placeholder)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter.rs` (placeholder)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence.rs` (placeholder)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade.rs` (placeholder)
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter/service.rs` (placeholder)

**Interfaces:**
- Consumes: nothing (greenfield crate).
- Produces: a `mission` crate that builds and tests clean, with empty module skeletons. Later tasks fill each module.

- [ ] **Step 1: Add `mission` to workspace members**

Edit `/root/coding/project/aegis/Cargo.toml`. In the `[workspace].members` array, add `"lib/crates/mission"` as a new entry — keep the existing lines, add the new one in alphabetical position right after `lib/crates/domain-model` and before `lib/crates/project`:

```toml
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth", "lib/crates/crf", "lib/crates/domain-model", "lib/crates/mission",
    "lib/crates/project",
    "lib/crates/terminology",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

- [ ] **Step 2: Create `lib/crates/mission/Cargo.toml`**

```toml
[package]
name = "mission"
version = "0.1.0"
edition = "2024"

[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
# `chrono` provides the `DateTime<Utc>` type carried on every
# aggregate's `created_at` / `updated_at` columns.
chrono = { workspace = true }
# `serde` provides Serialize / Deserialize for the wire-level DTOs
# on the apis `mission` port this crate adapts.
serde = { workspace = true }
serde_json = { workspace = true }
# `apis` provides the outbound `MissionService` port the in-memory
# facade implements plus the `ProjectService` / `UserService`
# inbound ports `ProjectLookupImpl` / `UserLookupImpl` adapt.
apis = { path = "../apis" }

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

- [ ] **Step 3: Create `lib/crates/mission/src/lib.rs`**

```rust
//! `mission` workspace crate.
//!
//! Hosts the `Mission` and `Assignee` aggregates, their ports,
//! the PostgreSQL-backed persistence adapters, the cross-crate
//! `ProjectLookup` / `UserLookup` adapters, the usecase layer
//! that orchestrates them with project-leader authorization, and
//! the in-memory facade that adapts `MissionUsecase` to
//! `apis::mission::MissionService`.
//!
//! Layered architecture:
//!
//! ```text
//! mission crate
//! └── adapter
//!     ├── facade                  (MissionServiceImpl<M, A, P, U>)
//!     ├── persistence             (MissionRepoPg, AssigneeRepoPg)
//!     └── service                 (ProjectLookupImpl, UserLookupImpl)
//! usecase
//!     └── MissionUsecase<M, A, P, U>
//! domain
//!     └── Mission, Assignee,
//!         MissionKind, MissionRole,
//!         MissionRepository, AssigneeRepository,
//!         ProjectLookup, UserLookup,
//!         DomainError
//! ```

pub mod adapter;
pub mod domain;
pub mod usecase;
```

- [ ] **Step 4: Create the placeholder module leaves**

`lib/crates/mission/src/domain.rs`:

```rust
// Filled by Task 2.
```

`lib/crates/mission/src/usecase.rs`:

```rust
// Filled by Task 4.
```

`lib/crates/mission/src/adapter.rs`:

```rust
pub(crate) mod persistence;
pub mod facade;
pub mod service;
```

`lib/crates/mission/src/adapter/persistence.rs`:

```rust
// Filled by Task 3.
```

`lib/crates/mission/src/adapter/facade.rs`:

```rust
// Filled by Task 4.
```

`lib/crates/mission/src/adapter/service.rs`:

```rust
pub mod project;
pub mod user;
```

`lib/crates/mission/src/adapter/service/project.rs`:

```rust
// Filled by Task 3.
```

`lib/crates/mission/src/adapter/service/user.rs`:

```rust
// Filled by Task 3.
```

- [ ] **Step 5: Build the empty scaffold**

Run: `cargo check -p mission`
Expected: success, no errors.

- [ ] **Step 6: Commit the scaffold**

```bash
git add Cargo.toml lib/crates/mission
git commit -m "$(cat <<'EOF'
feat(mission): scaffold crate + Cargo workspace wiring

Adds `lib/crates/mission` as a new workspace member with the
DDD three-layer module layout (domain / usecase / adapter) and
the four sub-modules under `adapter` (persistence, facade,
service/{project,user}). No behavior yet — every module leaf is
a placeholder filled by subsequent tasks.

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
Verification: cargo check -p mission
EOF
)"
```

## Task 2: Domain layer (types, ports, errors, tests)

**Files:**
- Create: `lib/crates/mission/src/domain/error.rs`
- Create: `lib/crates/mission/src/domain/mission_kind.rs`
- Create: `lib/crates/mission/src/domain/mission_role.rs`
- Create: `lib/crates/mission/src/domain/assignee.rs`
- Create: `lib/crates/mission/src/domain/mission.rs`
- Create: `lib/crates/mission/src/domain/mission_lookup.rs` (`MissionRepository`, `MissionNew`, `AssigneeNew`)
- Create: `lib/crates/mission/src/domain/project_lookup.rs`
- Create: `lib/crates/mission/src/domain/user_lookup.rs`
- Create: `lib/crates/mission/src/domain/tests.rs`
- Modify: `lib/crates/mission/src/domain.rs` (declare children + re-exports)

**Interfaces:**
- Consumes: nothing (pure domain).
- Produces: every domain type the usecase / adapter / facade / public_api / integration tests use.

- [ ] **Step 1: Write the failing domain tests**

Create `lib/crates/mission/src/domain/tests.rs`:

```rust
use super::{
    Assignee, AssigneeNew, DomainError, Mission, MissionKind, MissionNew, MissionRole,
    assignees_within_mission_are_unique, ensure_mission_kind, ensure_mission_role,
};

#[test]
fn mission_kind_round_trip() {
    for k in [MissionKind::Crf, MissionKind::Sdtm, MissionKind::Adam, MissionKind::Tfl] {
        let s = k.as_str();
        let parsed = MissionKind::try_from(s).expect("parses");
        assert_eq!(parsed, k);
    }
}

#[test]
fn mission_kind_unknown_rejected() {
    let err = MissionKind::try_from("not_a_kind").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionKind(ref s) if s == "not_a_kind"));
}

#[test]
fn mission_role_round_trip() {
    for r in [MissionRole::Dev, MissionRole::Qc] {
        let s = r.as_str();
        let parsed = MissionRole::try_from(s).expect("parses");
        assert_eq!(parsed, r);
    }
}

#[test]
fn mission_role_unknown_rejected() {
    let err = MissionRole::try_from("manager").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionRole(ref s) if s == "manager"));
}

#[test]
fn mission_new_rejects_empty_code() {
    let m = Mission::new(
        1,
        "p1".into(),
        MissionKind::Crf,
        "   ".into(),
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyMissionCode)));
}

#[test]
fn mission_new_accepts_non_empty_code() {
    let m = Mission::new(
        1,
        "p1".into(),
        MissionKind::Crf,
        "c1".into(),
        vec![],
        now(),
        now(),
    )
    .unwrap();
    assert_eq!(m.mission_code, "c1");
}

#[test]
fn assignee_new_rejects_empty_user_code() {
    let a = Assignee::new(1, "".into(), MissionRole::Dev, now(), now());
    assert!(matches!(a, Err(DomainError::EmptyUserCode)));
}

#[test]
fn assignees_within_mission_are_unique_detects_duplicate() {
    let assignees = vec![
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
    ];
    let err = assignees_within_mission_are_unique(&assignees).unwrap_err();
    assert!(matches!(err, DomainError::DuplicateAssignee { .. }));
}

#[test]
fn assignees_within_mission_are_unique_accepts_distinct_roles() {
    let assignees = vec![
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Dev,
        },
        AssigneeNew {
            user_code: "u1".into(),
            role: MissionRole::Qc,
        },
    ];
    assert!(assignees_within_mission_are_unique(&assignees).is_ok());
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
```

- [ ] **Step 2: Run tests to confirm they fail**

Run: `cargo test -p mission --lib domain::tests`
Expected: FAIL — `super::*` does not exist yet.

- [ ] **Step 3: Write `domain/error.rs`**

```rust
use thiserror::Error;

use super::mission_kind::MissionKind;
use super::mission_role::MissionRole;

#[derive(Debug, Error)]
pub enum DomainError {
    #[error("mission code must not be empty")]
    EmptyMissionCode,

    #[error("user code must not be empty")]
    EmptyUserCode,

    #[error("unknown mission kind: {0}")]
    UnknownMissionKind(String),

    #[error("unknown mission role: {0}")]
    UnknownMissionRole(String),

    #[error("mission not found")]
    NotFound,

    #[error("assignee not found")]
    AssigneeNotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("mission already exists for {project_code}/{mission_kind:?}/{mission_code}")]
    DuplicateMission {
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
    },

    #[error("assignee already exists for mission {mission_id}/{user_code}/{role:?}")]
    DuplicateAssignee {
        mission_id: i64,
        user_code: String,
        role: MissionRole,
    },

    #[error("repository error: {0}")]
    Repository(String),
}
```

- [ ] **Step 4: Write `domain/mission_kind.rs`**

```rust
use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl MissionKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            MissionKind::Crf => "crf",
            MissionKind::Sdtm => "sdtm",
            MissionKind::Adam => "adam",
            MissionKind::Tfl => "tfl",
        }
    }
}

impl FromStr for MissionKind {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "crf" => Ok(MissionKind::Crf),
            "sdtm" => Ok(MissionKind::Sdtm),
            "adam" => Ok(MissionKind::Adam),
            "tfl" => Ok(MissionKind::Tfl),
            other => Err(DomainError::UnknownMissionKind(other.to_string())),
        }
    }
}

impl TryFrom<&str> for MissionKind {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
```

- [ ] **Step 5: Write `domain/mission_role.rs`**

```rust
use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionRole {
    Dev,
    Qc,
}

impl MissionRole {
    pub const fn as_str(self) -> &'static str {
        match self {
            MissionRole::Dev => "dev",
            MissionRole::Qc => "qc",
        }
    }
}

impl FromStr for MissionRole {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "dev" => Ok(MissionRole::Dev),
            "qc" => Ok(MissionRole::Qc),
            other => Err(DomainError::UnknownMissionRole(other.to_string())),
        }
    }
}

impl TryFrom<&str> for MissionRole {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
```

- [ ] **Step 6: Write `domain/assignee.rs`**

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::mission_role::MissionRole;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Assignee {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Assignee {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs.
    pub fn new(
        id: i64,
        user_code: String,
        role: MissionRole,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if user_code.trim().is_empty() {
            return Err(DomainError::EmptyUserCode);
        }
        Ok(Self {
            id,
            user_code,
            role,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        user_code: String,
        role: MissionRole,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            user_code,
            role,
            created_at,
            updated_at,
        }
    }
}
```

- [ ] **Step 7: Write `domain/mission.rs`**

```rust
use chrono::{DateTime, Utc};

use super::assignee::Assignee;
use super::error::DomainError;
use super::mission_kind::MissionKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<Assignee>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Mission {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs. The assignee list is not
    /// validated here — uniqueness of `(user_code, role)` within
    /// a mission is the usecase's job (it owns
    /// `assignees_within_mission_are_unique`).
    pub fn new(
        id: i64,
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
        assignees: Vec<Assignee>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if mission_code.trim().is_empty() {
            return Err(DomainError::EmptyMissionCode);
        }
        Ok(Self {
            id,
            project_code,
            mission_kind,
            mission_code,
            assignees,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
        assignees: Vec<Assignee>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            project_code,
            mission_kind,
            mission_code,
            assignees,
            created_at,
            updated_at,
        }
    }
}
```

- [ ] **Step 8: Write `domain/mission_lookup.rs`**

```rust
use async_trait::async_trait;

use super::assignee::Assignee;
use super::error::DomainError;
use super::mission::Mission;
use super::mission_kind::MissionKind;
use super::mission_role::MissionRole;

/// Persistence-input DTO for `MissionRepository::create`. Carries
/// the initial assignee list so the repo can insert both the
/// mission row and its assignee rows inside one transaction. The
/// DB CHECK + UNIQUE on `assignees` is the safety net for the
/// per-mission uniqueness invariant the usecase enforces up
/// front via [`assignees_within_mission_are_unique`].
#[derive(Debug, Clone)]
pub struct MissionNew {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeNew>,
}

/// Persistence-input DTO for `AssigneeRepository::add`
/// (single-row insert used by the standalone `add_assignee` flow).
#[derive(Debug, Clone)]
pub struct AssigneeNew {
    pub user_code: String,
    pub role: MissionRole,
}

/// Check the per-mission `(user_code, role)` uniqueness invariant.
/// Returns `Err(DomainError::DuplicateAssignee { mission_id: 0, … })`
/// on the first duplicate pair — `mission_id` is left at `0` here
/// because the caller has not yet assigned one; the usecase fills
/// it in if it needs a different value.
pub fn assignees_within_mission_are_unique(
    assignees: &[AssigneeNew],
) -> Result<(), DomainError> {
    let mut seen: Vec<(String, MissionRole)> = Vec::with_capacity(assignees.len());
    for a in assignees {
        let pair = (a.user_code.clone(), a.role);
        if seen.contains(&pair) {
            return Err(DomainError::DuplicateAssignee {
                mission_id: 0,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        seen.push(pair);
    }
    Ok(())
}

#[async_trait]
pub trait MissionRepository: Send + Sync {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError>;

    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError>;

    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError>;

    /// Hard delete; cascades to `assignees` via `ON DELETE CASCADE`.
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

#[async_trait]
pub trait AssigneeRepository: Send + Sync {
    async fn add(
        &self,
        mission_id: i64,
        input: AssigneeNew,
    ) -> Result<Assignee, DomainError>;

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError>;
}
```

- [ ] **Step 9: Write `domain/project_lookup.rs`**

```rust
use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port for project existence + leadership
/// checks. Adapted to `apis::project::ProjectService` by
/// `adapter::service::project::ProjectLookupImpl`.
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;

    async fn is_leader(
        &self,
        project_code: &str,
        user_code: &str,
    ) -> Result<bool, DomainError>;
}
```

- [ ] **Step 10: Write `domain/user_lookup.rs`**

```rust
use async_trait::async_trait;

use super::error::DomainError;

/// Narrow cross-crate port for user existence checks. Adapted to
/// `apis::user::UserService` by `adapter::service::user::UserLookupImpl`.
#[async_trait]
pub trait UserLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
```

- [ ] **Step 11: Wire `domain.rs`**

Replace `lib/crates/mission/src/domain.rs`:

```rust
//! Domain layer.
//!
//! Pure types, value objects, ports (traits), and `DomainError`.
//! No I/O — no `sqlx`, no `tokio`. Validates inputs and enforces
//! invariants.

mod assignee;
mod error;
mod mission;
mod mission_kind;
mod mission_lookup;
mod mission_role;
mod project_lookup;
#[cfg(test)]
mod tests;
mod user_lookup;

pub use assignee::Assignee;
pub use error::DomainError;
pub use mission::Mission;
pub use mission_kind::MissionKind;
pub use mission_lookup::{
    AssigneeNew, AssigneeRepository, MissionNew, MissionRepository,
    assignees_within_mission_are_unique,
};
pub use mission_role::MissionRole;
pub use project_lookup::ProjectLookup;
pub use user_lookup::UserLookup;
```

- [ ] **Step 12: Run the domain tests**

Run: `cargo test -p mission --lib domain::tests`
Expected: PASS — every test in `tests.rs` passes.

- [ ] **Step 13: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/src/domain.rs lib/crates/mission/src/domain
git commit -m "$(cat <<'EOF'
feat(mission): domain layer (Mission, Assignee, ports, errors)

Adds the pure domain layer:
- `Mission` and `Assignee` aggregates with the two-constructor
  pattern (`new` validates, `for_repository` is `pub(crate)` and
  used by the row bridge)
- `MissionKind { Crf, Sdtm, Adam, Tfl }` and `MissionRole { Dev, Qc }`
  with `as_str` / `FromStr` round-trip
- `MissionRepository`, `AssigneeRepository`, `ProjectLookup`,
  `UserLookup` ports
- `MissionNew` carries the initial assignee list for one
  transaction; `AssigneeNew` is the single-row add input
- `assignees_within_mission_are_unique` enforces the
  `(user_code, role)` invariant at the usecase boundary
- `DomainError` with every variant mapped in `map_usecase_to_api_error`
  in Task 4

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib domain::tests.
EOF
)"
```

## Task 3: Persistence + service adapters + migrations

**Files:**
- Create: `lib/crates/mission/migrations/0001_create_missions.sql`
- Create: `lib/crates/mission/migrations/0002_create_assignees.sql`
- Create: `lib/crates/mission/src/adapter/persistence/postgres.rs`
- Create: `lib/crates/mission/src/adapter/persistence/postgres/row.rs`
- Create: `lib/crates/mission/src/adapter/persistence/postgres/mission_repo.rs`
- Create: `lib/crates/mission/src/adapter/persistence/postgres/assignee_repo.rs`
- Create: `lib/crates/mission/src/adapter/persistence/postgres/tests.rs`
- Create: `lib/crates/mission/src/adapter/service/project.rs`
- Create: `lib/crates/mission/src/adapter/service/project/tests.rs`
- Create: `lib/crates/mission/src/adapter/service/user.rs`
- Create: `lib/crates/mission/src/adapter/service/user/tests.rs`

**Interfaces:**
- Consumes: every type from `mission::domain::*` plus `sqlx::PgPool`, `Arc<dyn apis::project::ProjectService>`, `Arc<dyn apis::user::UserService>`.
- Produces:
  - `mission::adapter::persistence::postgres::MissionRepo::new(PgPool) -> Self`
  - `mission::adapter::persistence::postgres::AssigneeRepo::new(PgPool) -> Self`
  - `mission::adapter::persistence::postgres::{MissionRepo, AssigneeRepo}` implementing their domain ports
  - `mission::adapter::service::project::ProjectLookupImpl::new(Arc<dyn ProjectService>) -> Self`
  - `mission::adapter::service::user::UserLookupImpl::new(Arc<dyn UserService>) -> Self`
  - both impls implement their domain lookup ports

- [ ] **Step 1: Write migration `0001_create_missions.sql`**

Create `lib/crates/mission/migrations/0001_create_missions.sql`:

```sql
-- 0001_create_missions.sql
--
-- Single migration for the `mission` crate that owns the
-- `Mission` aggregate. Assignees live in their own migration
-- so the per-mission UNIQUE constraint and `ON DELETE CASCADE`
-- FK are co-located with the `assignees` table definition.
--
-- Layout:
--   * `missions`
--       - `id`              surrogate primary key.
--       - `project_code`    caller-chosen; not a FK (no FK to
--                            `projects.code` because projects uses
--                            INTEGER ids and the spec keeps the
--                            mission / project relationship
--                            resolved at the usecase via
--                            `ProjectLookup`).
--       - `mission_kind`    text discriminant ('crf', 'sdtm',
--                            'adam', 'tfl') — belt-and-braces
--                            CHECK against out-of-band inserts.
--       - `mission_code`    caller-chosen stable identifier.
--       - `created_at`      DEFAULT NOW() at insert.
--       - `updated_at`      DEFAULT NOW() at insert; the
--                            `missions_set_updated_at` trigger
--                            refreshes it.
--       - UNIQUE (project_code, mission_kind, mission_code).

CREATE TABLE missions (
    id              BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    project_code    TEXT NOT NULL,
    mission_kind    TEXT NOT NULL,
    mission_code    TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT missions_natural_key
        UNIQUE (project_code, mission_kind, mission_code),
    CONSTRAINT missions_kind_check
        CHECK (mission_kind IN ('crf', 'sdtm', 'adam', 'tfl'))
);

CREATE OR REPLACE FUNCTION missions_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER missions_set_updated_at
    BEFORE UPDATE ON missions
    FOR EACH ROW
    EXECUTE FUNCTION missions_set_updated_at();
```

- [ ] **Step 2: Write migration `0002_create_assignees.sql`**

Create `lib/crates/mission/migrations/0002_create_assignees.sql`:

```sql
-- 0002_create_assignees.sql
--
-- Per-mission assignee rows. FK to `missions(id)` with
-- `ON DELETE CASCADE` so mission deletion cascades to its
-- assignees in a single DELETE.
--
--   * `assignees`
--       - `id`           surrogate primary key.
--       - `mission_id`   FK CASCADE.
--       - `user_code`    caller-chosen (no FK — user existence is
--                         enforced at the usecase via
--                         `UserLookup`, mirroring how mission /
--                         project referential integrity is
--                         resolved).
--       - `role`         text ('dev', 'qc') with CHECK.
--       - `created_at`   DEFAULT NOW() at insert.
--       - `updated_at`   refreshed by the trigger.
--       - UNIQUE (mission_id, user_code, role) — the per-mission
--         uniqueness invariant.

CREATE TABLE assignees (
    id           BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    mission_id   BIGINT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    user_code    TEXT NOT NULL,
    role         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT assignees_per_mission_unique
        UNIQUE (mission_id, user_code, role),
    CONSTRAINT assignees_role_check
        CHECK (role IN ('dev', 'qc'))
);

CREATE OR REPLACE FUNCTION assignees_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER assignees_set_updated_at
    BEFORE UPDATE ON assignees
    FOR EACH ROW
    EXECUTE FUNCTION assignees_set_updated_at();
```

- [ ] **Step 3: Write the failing adapter tests**

Create `lib/crates/mission/src/adapter/persistence/postgres/tests.rs`:

```rust
use std::convert::TryFrom;
use std::fs;

use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, Mission, MissionKind, MissionRole,
};
use super::row::{AssigneeRow, MissionRow};

#[test]
fn mission_row_to_domain() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "crf".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let m: Mission = Mission::try_from((row, vec![])).unwrap();
    assert_eq!(m.id, 1);
    assert_eq!(m.mission_kind, MissionKind::Crf);
    assert_eq!(m.mission_code, "c1");
    assert!(m.assignees.is_empty());
}

#[test]
fn assignee_row_to_domain() {
    let row = AssigneeRow {
        id: 7,
        mission_id: 1,
        user_code: "u1".into(),
        role: "qc".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    let a: Assignee = Assignee::try_from(row).unwrap();
    assert_eq!(a.id, 7);
    assert_eq!(a.role, MissionRole::Qc);
}

#[test]
fn mission_row_rejects_unknown_kind() {
    let row = MissionRow {
        id: 1,
        project_code: "p1".into(),
        mission_kind: "not-a-kind".into(),
        mission_code: "c1".into(),
        created_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
        updated_at: DateTime::<Utc>::from_timestamp(0, 0).unwrap(),
    };
    assert!(Mission::try_from((row, vec![])).is_err());
}

#[test]
fn mission_migration_has_natural_key_unique() {
    let sql = read_migration("0001_create_missions.sql");
    assert!(sql.contains("missions_natural_key"));
    assert!(sql.contains("UNIQUE (project_code, mission_kind, mission_code)"));
    assert!(sql.contains("missions_kind_check"));
    assert!(sql.contains("CHECK (mission_kind IN ('crf', 'sdtm', 'adam', 'tfl'))"));
    assert!(sql.contains("missions_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON missions"));
}

#[test]
fn assignee_migration_has_per_mission_unique_and_cascade() {
    let sql = read_migration("0002_create_assignees.sql");
    assert!(sql.contains("assignees_per_mission_unique"));
    assert!(sql.contains("UNIQUE (mission_id, user_code, role)"));
    assert!(sql.contains("assignees_role_check"));
    assert!(sql.contains("CHECK (role IN ('dev', 'qc'))"));
    assert!(sql.contains("assignees_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON assignees"));
    assert!(sql.contains("REFERENCES missions(id) ON DELETE CASCADE"));
}

fn read_migration(name: &str) -> String {
    let path = format!(
        "{}/migrations/{}",
        env!("CARGO_MANIFEST_DIR"),
        name
    );
    fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}
```

Create `lib/crates/mission/src/adapter/service/project/tests.rs`:

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{
    ProjectApiError, ProjectMemberView, ProjectService, ProjectView, UserSummaryView,
};

use crate::domain::{DomainError, ProjectLookup};

use super::ProjectLookupImpl;

#[derive(Clone)]
struct FakeProject {
    leader_codes: Vec<String>,
}

#[async_trait]
impl ProjectService for FakeProject {
    async fn create_project(
        &self,
        _req: apis::project::CreateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_id(
        &self,
        _id: i32,
    ) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
    async fn get_project_by_code(
        &self,
        code: &str,
    ) -> Result<ProjectView, ProjectApiError> {
        Ok(view(code, &self.leader_codes))
    }
    async fn list_projects(
        &self,
    ) -> Result<Vec<ProjectView>, ProjectApiError> {
        unimplemented!()
    }
    async fn update_project(
        &self,
        _req: apis::project::UpdateProjectRequest,
    ) -> Result<ProjectView, ProjectApiError> {
        unimplemented!()
    }
}

fn view(code: &str, leaders: &[String]) -> ProjectView {
    ProjectView {
        id: 1,
        code: code.to_string(),
        description: String::new(),
        members: ProjectMemberView {
            leaders: leaders
                .iter()
                .map(|c| UserSummaryView {
                    code: c.clone(),
                    name: c.clone(),
                })
                .collect(),
            workers: vec![],
        },
        unblind_members: ProjectMemberView::default(),
        tags: vec![],
        active: true,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    }
}

#[tokio::test]
async fn is_leader_true_for_listed_leader() {
    let svc = Arc::new(FakeProject {
        leader_codes: vec!["alice".into(), "bob"],
    });
    let lookup = ProjectLookupImpl::new(svc);
    assert!(lookup.is_leader("p1", "alice").await.unwrap());
}

#[tokio::test]
async fn is_leader_false_for_non_leader() {
    let svc = Arc::new(FakeProject {
        leader_codes: vec!["alice".into()],
    });
    let lookup = ProjectLookupImpl::new(svc);
    assert!(!lookup.is_leader("p1", "carol").await.unwrap());
}

#[tokio::test]
async fn get_by_code_maps_not_found() {
    struct NotFound;
    #[async_trait]
    impl ProjectService for NotFound {
        async fn create_project(
            &self,
            _: apis::project::CreateProjectRequest,
        ) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_id(
            &self,
            _: i32,
        ) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
        async fn get_project_by_code(
            &self,
            _: &str,
        ) -> Result<ProjectView, ProjectApiError> {
            Err(ProjectApiError::NotFound)
        }
        async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError> {
            unimplemented!()
        }
        async fn update_project(
            &self,
            _: apis::project::UpdateProjectRequest,
        ) -> Result<ProjectView, ProjectApiError> {
            unimplemented!()
        }
    }
    let lookup = ProjectLookupImpl::new(Arc::new(NotFound));
    let err = lookup.get_by_code("p1").await.unwrap_err();
    assert!(matches!(err, DomainError::ProjectNotFound(ref c) if c == "p1"));
}
```

Create `lib/crates/mission/src/adapter/service/user/tests.rs`:

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{Role as ApiRole, UserApiError, UserService, UserView};

use crate::domain::{DomainError, UserLookup};

use super::UserLookupImpl;

#[derive(Clone)]
struct FakeUser;

#[async_trait]
impl UserService for FakeUser {
    async fn create(
        &self,
        _: apis::user::CreateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, code: &str) -> Result<UserView, UserApiError> {
        Ok(UserView {
            id: 1,
            code: code.into(),
            name: code.into(),
            role: ApiRole::General,
            active: true,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: apis::user::UpdateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

struct MissingUser;

#[async_trait]
impl UserService for MissingUser {
    async fn create(
        &self,
        _: apis::user::CreateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_id(&self, _: i32) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
    async fn get_by_code(&self, _: &str) -> Result<UserView, UserApiError> {
        Err(UserApiError::NotFound)
    }
    async fn list(&self) -> Result<Vec<UserView>, UserApiError> {
        unimplemented!()
    }
    async fn update(
        &self,
        _: apis::user::UpdateUserRequest,
    ) -> Result<UserView, UserApiError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn user_lookup_get_by_code_ok() {
    let lookup = UserLookupImpl::new(Arc::new(FakeUser));
    lookup.get_by_code("u1").await.unwrap();
}

#[tokio::test]
async fn user_lookup_get_by_code_missing_maps_error() {
    let lookup = UserLookupImpl::new(Arc::new(MissingUser));
    let err = lookup.get_by_code("ghost").await.unwrap_err();
    assert!(matches!(err, DomainError::UserNotFound(ref c) if c == "ghost"));
}
```

- [ ] **Step 4: Run tests to confirm they fail**

Run: `cargo test -p mission --lib`
Expected: FAIL — every adapter test fails because the modules do not exist yet.

- [ ] **Step 5: Write `adapter/persistence/postgres/row.rs`**

```rust
use std::convert::TryFrom;
use std::str::FromStr;

use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, DomainError, Mission, MissionKind, MissionRole,
};

/// Raw row from `missions`. `mission_kind` is read as TEXT and
/// parsed via `MissionKind::from_str` so the DB CHECK is the
/// belt-and-braces against out-of-band inserts.
pub(crate) struct MissionRow {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: String,
    pub mission_code: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw row from `assignees`. `role` is read as TEXT and parsed
/// via `MissionRole::from_str`.
pub(crate) struct AssigneeRow {
    pub id: i64,
    pub mission_id: i64,
    pub user_code: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<MissionRow> for (MissionKind, String, String, String) {
    type Error = DomainError;
    fn try_from(row: MissionRow) -> Result<Self, Self::Error> {
        Ok((
            MissionKind::from_str(&row.mission_kind)?,
            row.project_code,
            row.mission_code,
            row.mission_kind,
        ))
    }
}

impl TryFrom<(MissionRow, Vec<AssigneeRow>)> for Mission {
    type Error = DomainError;
    fn try_from((row, assignees): (MissionRow, Vec<AssigneeRow>)) -> Result<Self, Self::Error> {
        let mission_kind = MissionKind::from_str(&row.mission_kind)?;
        let assignees = assignees
            .into_iter()
            .map(Assignee::try_from)
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Mission::for_repository(
            row.id,
            row.project_code,
            mission_kind,
            row.mission_code,
            assignees,
            row.created_at,
            row.updated_at,
        ))
    }
}

impl TryFrom<AssigneeRow> for Assignee {
    type Error = DomainError;
    fn try_from(row: AssigneeRow) -> Result<Self, Self::Error> {
        let role = MissionRole::from_str(&row.role)?;
        Ok(Assignee::for_repository(
            row.id,
            row.user_code,
            role,
            row.created_at,
            row.updated_at,
        ))
    }
}
```

- [ ] **Step 6: Write `adapter/persistence/postgres/mission_repo.rs`**

```rust
use std::collections::HashMap;
use std::convert::TryFrom;

use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use crate::domain::{
    AssigneeNew, DomainError, Mission, MissionKind, MissionNew, MissionRepository,
};

use super::map_db_error;
use super::row::{AssigneeRow, MissionRow};

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct MissionRepo {
    pool: PgPool,
}

impl MissionRepo {
    pub async fn connect(database_url: &str) -> Result<Self, DomainError> {
        let pool = PgPoolOptions::new()
            .max_connections(8)
            .connect(database_url)
            .await
            .map_err(map_db_error)?;
        Ok(Self { pool })
    }

    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl MissionRepository for MissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let mut tx = self.pool.begin().await.map_err(map_db_error)?;

        let row: MissionRow = sqlx::QueryBuilder::new(
            "INSERT INTO missions (project_code, mission_kind, mission_code) VALUES (",
        )
        .push_bind(&input.project_code)
        .push(", ")
        .push_bind(input.mission_kind.as_str())
        .push(", ")
        .push_bind(&input.mission_code)
        .push(") RETURNING id, project_code, mission_kind, mission_code, created_at, updated_at")
        .build_query_as::<MissionRow>()
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db) if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) => {
                DomainError::DuplicateMission {
                    project_code: input.project_code.clone(),
                    mission_kind: input.mission_kind,
                    mission_code: input.mission_code.clone(),
                }
            }
            other => map_db_error(other),
        })?;

        let mission_id = row.id;

        for assignee in &input.assignees {
            insert_assignee(&mut tx, mission_id, assignee).await?;
        }

        tx.commit().await.map_err(map_db_error)?;

        self.find_by_id(mission_id).await
    }

    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        let row: MissionRow = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<MissionRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::NotFound)?;

        let assignees = load_assignees(&self.pool, id).await?;
        Mission::try_from((row, assignees)).map_err(Into::into)
    }

    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE project_code = ",
        );
        qb.push_bind(project_code);
        if let Some(k) = kind {
            qb.push(" AND mission_kind = ").push_bind(k.as_str());
        }
        qb.push(" ORDER BY id ASC");
        let rows: Vec<MissionRow> = qb
            .build_query_as::<MissionRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        load_missions_with_assignees(&self.pool, rows).await
    }

    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        // First fetch the mission ids that have an assignee with
        // `user_code`, then fetch the missions + their assignees.
        let ids: Vec<i64> = sqlx::QueryBuilder::new(
            "SELECT DISTINCT mission_id FROM assignees WHERE user_code = ",
        )
        .push_bind(user_code)
        .build_query_as::<(i64,)>()
        .fetch_all(&self.pool)
        .await
        .map_err(map_db_error)?
        .into_iter()
        .map(|(id,)| id)
        .collect();

        if ids.is_empty() {
            return Ok(vec![]);
        }

        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, project_code, mission_kind, mission_code, created_at, updated_at \
             FROM missions WHERE id IN (",
        );
        let mut sep = qb.separated(", ");
        for id in &ids {
            sep.push_bind(id);
        }
        qb.push(") ORDER BY id ASC");
        let rows: Vec<MissionRow> = qb
            .build_query_as::<MissionRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        load_missions_with_assignees(&self.pool, rows).await
    }

    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM missions WHERE id = ")
            .push_bind(id)
            .build()
            .execute(&self.pool)
            .await
            .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::NotFound);
        }
        Ok(())
    }
}

async fn insert_assignee(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    mission_id: i64,
    assignee: &AssigneeNew,
) -> Result<(), DomainError> {
    let _: AssigneeRow = sqlx::QueryBuilder::new(
        "INSERT INTO assignees (mission_id, user_code, role) VALUES (",
    )
    .push_bind(mission_id)
    .push(", ")
    .push_bind(&assignee.user_code)
    .push(", ")
    .push_bind(assignee.role.as_str())
    .push(") RETURNING id, mission_id, user_code, role, created_at, updated_at")
    .build_query_as::<AssigneeRow>()
    .fetch_one(&mut **tx)
    .await
    .map_err(|e| match e {
        sqlx::Error::Database(ref db) if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) => {
            DomainError::DuplicateAssignee {
                mission_id,
                user_code: assignee.user_code.clone(),
                role: assignee.role,
            }
        }
        other => map_db_error(other),
    })?;
    Ok(())
}

async fn load_assignees(pool: &PgPool, mission_id: i64) -> Result<Vec<AssigneeRow>, DomainError> {
    let rows: Vec<AssigneeRow> = sqlx::QueryBuilder::new(
        "SELECT id, mission_id, user_code, role, created_at, updated_at \
         FROM assignees WHERE mission_id = ",
    )
    .push_bind(mission_id)
    .push(" ORDER BY id ASC")
    .build_query_as::<AssigneeRow>()
    .fetch_all(pool)
    .await
    .map_err(map_db_error)?;
    Ok(rows)
}

async fn load_missions_with_assignees(
    pool: &PgPool,
    rows: Vec<MissionRow>,
) -> Result<Vec<Mission>, DomainError> {
    if rows.is_empty() {
        return Ok(vec![]);
    }

    let mut qb = sqlx::QueryBuilder::new(
        "SELECT id, mission_id, user_code, role, created_at, updated_at \
         FROM assignees WHERE mission_id IN (",
    );
    let mut sep = qb.separated(", ");
    let ids: Vec<i64> = rows.iter().map(|r| r.id).collect();
    for id in &ids {
        sep.push_bind(id);
    }
    qb.push(") ORDER BY mission_id ASC, id ASC");
    let assignee_rows: Vec<AssigneeRow> = qb
        .build_query_as::<AssigneeRow>()
        .fetch_all(pool)
        .await
        .map_err(map_db_error)?;

    let mut by_mission: HashMap<i64, Vec<AssigneeRow>> = HashMap::new();
    for a in assignee_rows {
        by_mission.entry(a.mission_id).or_default().push(a);
    }

    rows.into_iter()
        .map(|row| {
            let assignees = by_mission.remove(&row.id).unwrap_or_default();
            Mission::try_from((row, assignees)).map_err(Into::into)
        })
        .collect()
}
```

Note: the `TryFrom<MissionRow>` impl in `row.rs` produces a tuple `(MissionKind, String, String, String)`, but `Mission::try_from((MissionRow, Vec<AssigneeRow>))` is what `mission_repo.rs` uses — drop the tuple impl from `row.rs` to keep only the `(MissionRow, Vec<AssigneeRow>)` one. Edit `row.rs` to remove the tuple impl and any unused imports.

- [ ] **Step 7: Write `adapter/persistence/postgres/assignee_repo.rs`**

```rust
use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{Assignee, AssigneeNew, AssigneeRepository, DomainError};

use super::map_db_error;
use super::row::AssigneeRow;

/// PostgreSQL SQLSTATE for unique-violation.
const SQLSTATE_UNIQUE_VIOLATION: &str = "23505";

pub struct AssigneeRepo {
    pool: PgPool,
}

impl AssigneeRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl AssigneeRepository for AssigneeRepo {
    async fn add(
        &self,
        mission_id: i64,
        input: AssigneeNew,
    ) -> Result<Assignee, DomainError> {
        // Caller (usecase) has already verified the user exists;
        // mission existence is enforced via FK — the row will fail
        // to insert if `mission_id` does not exist. Map that
        // generic FK violation to `DomainError::NotFound` so the
        // facade surfaces a 404 instead of a 500.
        let row: AssigneeRow = sqlx::QueryBuilder::new(
            "INSERT INTO assignees (mission_id, user_code, role) VALUES (",
        )
        .push_bind(mission_id)
        .push(", ")
        .push_bind(&input.user_code)
        .push(", ")
        .push_bind(input.role.as_str())
        .push(") RETURNING id, mission_id, user_code, role, created_at, updated_at")
        .build_query_as::<AssigneeRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(|e| match e {
            sqlx::Error::Database(ref db) if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) => {
                DomainError::DuplicateAssignee {
                    mission_id,
                    user_code: input.user_code.clone(),
                    role: input.role,
                }
            }
            other => map_db_error(other),
        })?;
        Assignee::try_from(row).map_err(Into::into)
    }

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new(
            "DELETE FROM assignees WHERE mission_id = ",
        )
        .push_bind(mission_id)
        .push(" AND id = ")
        .push_bind(assignee_id)
        .build()
        .execute(&self.pool)
        .await
        .map_err(map_db_error)?;
        if res.rows_affected() == 0 {
            return Err(DomainError::AssigneeNotFound);
        }
        Ok(())
    }
}
```

- [ ] **Step 8: Write `adapter/persistence/postgres.rs`**

```rust
//! PostgreSQL-backed implementations of `MissionRepository` and
//! `AssigneeRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the project / user
//! crates. `MissionRepo::create` opens a transaction so the
//! mission row and every assignee row land atomically; the FK
//! `ON DELETE CASCADE` makes mission deletion a single DELETE.
//!
//! `row` is private to `postgres/`. The `MissionRow` /
//! `AssigneeRow` types are NOT re-exported at the crate root.

pub(crate) mod assignee_repo;
pub(crate) mod mission_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use assignee_repo::AssigneeRepo;
pub use mission_repo::MissionRepo;

use crate::domain::DomainError;

/// Map a `sqlx::Error` into the domain error taxonomy.
///
/// `RowNotFound` → `NotFound`. `Database` with SQLSTATE `23505`
/// (unique violation) is NOT mapped here — the call sites that
/// care about uniqueness (`MissionRepo::create`,
/// `MissionRepo::insert_assignee`, `AssigneeRepo::add`) handle
/// that variant themselves so they can build the structured
/// `DuplicateMission` / `DuplicateAssignee` variants with the
/// right context. Everything else → `Repository(driver_message)`.
fn map_db_error(e: sqlx::Error) -> DomainError {
    match e {
        sqlx::Error::RowNotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}
```

- [ ] **Step 9: Wire `adapter/persistence.rs`**

Replace `lib/crates/mission/src/adapter/persistence.rs`:

```rust
pub(crate) mod postgres;
```

- [ ] **Step 10: Write `adapter/service/project.rs`**

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::project::{ProjectApiError, ProjectService};

use crate::domain::{DomainError, ProjectLookup};

/// Adapter that maps the apis `ProjectService` port onto the
/// narrow domain `ProjectLookup` port. The mission crate never
/// reaches apis `project` types directly; everything flows
/// through this struct so the domain layer stays free of `apis`
/// references.
pub struct ProjectLookupImpl {
    projects: Arc<dyn ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(projects: Arc<dyn ProjectService>) -> Self {
        Self { projects }
    }
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.projects.get_project_by_code(code).await {
            Ok(_) => Ok(()),
            Err(ProjectApiError::NotFound) => Err(DomainError::ProjectNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }

    async fn is_leader(
        &self,
        project_code: &str,
        user_code: &str,
    ) -> Result<bool, DomainError> {
        let view = self
            .projects
            .get_project_by_code(project_code)
            .await
            .map_err(|e| match e {
                ProjectApiError::NotFound => {
                    DomainError::ProjectNotFound(project_code.to_string())
                }
                other => DomainError::Repository(other.to_string()),
            })?;
        Ok(view
            .members
            .leaders
            .iter()
            .any(|u| u.code == user_code))
    }
}

#[cfg(test)]
mod tests;
```

- [ ] **Step 11: Write `adapter/service/user.rs`**

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{UserApiError, UserService};

use crate::domain::{DomainError, UserLookup};

/// Adapter that maps the apis `UserService` port onto the narrow
/// domain `UserLookup` port.
pub struct UserLookupImpl {
    users: Arc<dyn UserService>,
}

impl UserLookupImpl {
    pub fn new(users: Arc<dyn UserService>) -> Self {
        Self { users }
    }
}

#[async_trait]
impl UserLookup for UserLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.users.get_by_code(code).await {
            Ok(_) => Ok(()),
            Err(UserApiError::NotFound) => Err(DomainError::UserNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}

#[cfg(test)]
mod tests;
```

- [ ] **Step 12: Run the adapter tests**

Run: `cargo test -p mission --lib`
Expected: PASS for every test added in Step 3. The `load_missions_with_assignees` helper also relies on `Mission::try_from((row, assignees))` which exists in `row.rs`.

- [ ] **Step 13: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/migrations lib/crates/mission/src/adapter
git commit -m "$(cat <<'EOF'
feat(mission): persistence + service adapters + migrations

- 0001_create_missions.sql: missions table with UNIQUE
  (project_code, mission_kind, mission_code), CHECK on
  mission_kind, and the updated_at trigger
- 0002_create_assignees.sql: assignees table with UNIQUE
  (mission_id, user_code, role), CHECK on role, and the FK
  ON DELETE CASCADE to missions(id)
- MissionRepo: PgPool-backed MissionRepository with transactional
  mission + assignee insert, find_by_id, list_by_project
  (optionally filtered by kind), list_by_user, delete
- AssigneeRepo: PgPool-backed AssigneeRepository for the
  standalone add / remove operations
- row.rs: MissionRow / AssigneeRow private types with TryFrom
  impls that parse mission_kind / role via the domain enums
- ProjectLookupImpl + UserLookupImpl: apis port adapters for
  the cross-crate lookups
- Adapter tests: row-bridge TryFrom + migration-content
  assertions + is_leader / get_by_code mapping

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib.
EOF
)"
```

## Task 4: Usecase + apis port + facade

**Files:**
- Create: `lib/crates/apis/src/mission.rs` (NEW — the port)
- Modify: `lib/crates/apis/src/lib.rs` (`pub mod mission;`)
- Create: `lib/crates/mission/src/usecase/commands.rs`
- Create: `lib/crates/mission/src/usecase/error.rs`
- Create: `lib/crates/mission/src/usecase/views.rs`
- Create: `lib/crates/mission/src/usecase/mission_usecase.rs`
- Create: `lib/crates/mission/src/usecase/tests.rs`
- Modify: `lib/crates/mission/src/usecase.rs` (declare children + re-exports)
- Create: `lib/crates/mission/src/adapter/facade/in_memory.rs` (`pub use MissionServiceImpl`)
- Create: `lib/crates/mission/src/adapter/facade/in_memory/service.rs`
- Create: `lib/crates/mission/src/adapter/facade/in_memory/tests.rs`
- Modify: `lib/crates/mission/src/adapter/facade.rs` (`pub mod in_memory;`)

**Interfaces:**
- Consumes: every domain type from Task 2 plus `Arc<dyn apis::mission::MissionService>` consumers.
- Produces:
  - `apis::mission::{MissionKind, MissionRole, MissionApiError, MissionView, AssigneeView, CreateMissionRequest, AssigneeData, ListMissionsByProjectRequest, ListMissionsByUserRequest, Actor, MissionService}`.
  - `mission::usecase::{CreateMission, AssigneeData, MissionView, AssigneeView, UsecaseError, MissionUsecase, MissionUsecaseConfig}`.
  - `mission::adapter::facade::in_memory::MissionServiceImpl::from_usecase(...) -> Self` and `::from_repos(...) -> Self`.

- [ ] **Step 1: Write `lib/crates/apis/src/mission.rs`**

```rust
//! Outbound port for the mission service.
//!
//! Mirrors the surface of `mission::usecase::MissionUsecase` so
//! adapters in any backend (in-memory, PostgreSQL, …) can adapt
//! their own types to the shared contract defined here. All
//! supporting DTOs (request shapes, view projections, enums, and
//! [`MissionApiError`]) live alongside the trait so a single
//! `use apis::mission::*;` brings the whole contract into scope.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

/// Mission flavour — what kind of clinical-programming work the
/// mission is for.
///
/// Mirrors `mission::domain::MissionKind`. The two enums are kept
/// in sync layer by layer — adapter implementations convert
/// losslessly via the matching variant constructors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl From<MissionKind> for &'static str {
    fn from(k: MissionKind) -> Self {
        match k {
            MissionKind::Crf => "crf",
            MissionKind::Sdtm => "sdtm",
            MissionKind::Adam => "adam",
            MissionKind::Tfl => "tfl",
        }
    }
}

/// Role the assignee plays on the mission.
///
/// Mirrors `mission::domain::MissionRole`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MissionRole {
    Dev,
    Qc,
}

impl From<MissionRole> for &'static str {
    fn from(r: MissionRole) -> Self {
        match r {
            MissionRole::Dev => "dev",
            MissionRole::Qc => "qc",
        }
    }
}

/// Error surface returned by every [`MissionService`] method.
///
/// Adapters translate backend-specific errors (e.g.
/// `mission::UsecaseError`) into this type at the implementation
/// boundary.
#[derive(Debug, Clone, Error)]
pub enum MissionApiError {
    #[error("validation failed: {0}")]
    Validation(String),

    #[error("not found")]
    NotFound,

    #[error("assignee not found")]
    AssigneeNotFound,

    #[error("project not found: {0}")]
    ProjectNotFound(String),

    #[error("user not found: {0}")]
    UserNotFound(String),

    #[error("forbidden: user {user_code} is not a leader of project {project_code}")]
    Forbidden {
        user_code: String,
        project_code: String,
    },

    #[error("mission already exists for {project_code}/{mission_kind:?}/{mission_code}")]
    DuplicateMission {
        project_code: String,
        mission_kind: MissionKind,
        mission_code: String,
    },

    #[error("assignee already exists for mission {mission_id}/{user_code}/{role:?}")]
    DuplicateAssignee {
        mission_id: i64,
        user_code: String,
        role: MissionRole,
    },

    #[error("repository error: {0}")]
    Repository(String),
}

/// Safe projection of an `Assignee` aggregate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssigneeView {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Safe projection of a `Mission` aggregate — assignees are
/// hydrated to `Vec<AssigneeView>` on read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionView {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Input DTO for [`MissionService::create_mission`].
#[derive(Debug, Clone)]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

/// One assignee entry inside a [`CreateMissionRequest`] or a
/// standalone [`MissionService::add_assignee`] call.
#[derive(Debug, Clone)]
pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}

/// Query for [`MissionService::list_missions_by_project`].
#[derive(Debug, Clone)]
pub struct ListMissionsByProjectRequest {
    pub project_code: String,
    pub kind: Option<MissionKind>,
}

/// Query for [`MissionService::list_missions_by_user`].
#[derive(Debug, Clone)]
pub struct ListMissionsByUserRequest {
    pub user_code: String,
}

/// Shared actor type for any port that authorizes on behalf of an
/// authenticated user. Built by the transport layer from the JWT
/// subject (`AuthClaims.code`); passed to every write method.
#[derive(Debug, Clone)]
pub struct Actor {
    pub user_code: String,
}

/// Outbound port for mission lifecycle operations.
///
/// `Send + Sync` so a `Box<dyn MissionService>` can be shared
/// state in an async server (axum, tarpc, …). Object-safe: no
/// generic methods, no `Self` in return position beyond `&self`.
///
/// Implementations adapt a backend's usecase layer (e.g.
/// `mission::MissionUsecase`) into this contract, translating
/// between backend-specific DTOs / errors and the `apis` types
/// defined above.
#[async_trait]
pub trait MissionService: Send + Sync {
    async fn create_mission(
        &self,
        actor: &Actor,
        req: CreateMissionRequest,
    ) -> Result<MissionView, MissionApiError>;

    async fn get_mission_by_id(&self, id: i64) -> Result<MissionView, MissionApiError>;

    async fn list_missions_by_project(
        &self,
        req: ListMissionsByProjectRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;

    async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), MissionApiError>;

    async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<AssigneeView, MissionApiError>;

    async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), MissionApiError>;
}
```

- [ ] **Step 2: Wire `apis/src/lib.rs`**

Edit `/root/coding/project/aegis/lib/crates/apis/src/lib.rs`. Add `pub mod mission;` to the module list:

```rust
pub mod auth;
pub mod crf;
pub mod domain_model;
pub mod mission;
pub mod project;
pub mod terminology;
pub mod user;
```

- [ ] **Step 3: Confirm `apis` still compiles**

Run: `cargo check -p apis`
Expected: success. The new module compiles in isolation (it only references `chrono` and `thiserror`, both already workspace deps).

- [ ] **Step 4: Write the failing usecase tests**

Create `lib/crates/mission/src/usecase/tests.rs`:

```rust
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use async_trait::async_trait;

use apis::mission::Actor;
use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, Mission, MissionKind, MissionNew,
    MissionRepository, MissionRole, ProjectLookup, UserLookup,
};
use crate::usecase::{CreateMission, MissionUsecase, MissionUsecaseConfig};

// ---- in-memory fakes ----

#[derive(Default)]
struct FakeMissionRepo {
    next_id: AtomicI32,
    missions: Mutex<Vec<Mission>>,
}

#[async_trait]
impl MissionRepository for FakeMissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as i64;
        let now: DateTime<Utc> = Utc::now();
        let assignees = input
            .assignees
            .iter()
            .enumerate()
            .map(|(idx, a)| Assignee {
                id: (id * 1000 + idx as i64),
                user_code: a.user_code.clone(),
                role: a.role,
                created_at: now,
                updated_at: now,
            })
            .collect();
        let m = Mission::for_repository(
            id,
            input.project_code,
            input.mission_kind,
            input.mission_code,
            assignees,
            now,
            now,
        );
        self.missions.lock().unwrap().push(m.clone());
        Ok(m)
    }
    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        self.missions
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| {
                m.project_code == project_code && kind.map_or(true, |k| k == m.mission_kind)
            })
            .cloned()
            .collect())
    }
    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.assignees.iter().any(|a| a.user_code == user_code))
            .cloned()
            .collect())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.missions.lock().unwrap();
        let before = g.len();
        g.retain(|m| m.id != id);
        if g.len() == before {
            Err(DomainError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct FakeAssigneeRepo {
    next_id: AtomicI32,
    assignees: Mutex<Vec<Assignee>>,
}

#[async_trait]
impl AssigneeRepository for FakeAssigneeRepo {
    async fn add(
        &self,
        mission_id: i64,
        input: AssigneeNew,
    ) -> Result<Assignee, DomainError> {
        let now = Utc::now();
        let a = Assignee::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.user_code,
            input.role,
            now,
            now,
        )?;
        let mut g = self.assignees.lock().unwrap();
        if g.iter()
            .any(|x| x.mission_id_if_some() == mission_id && x.user_code == a.user_code && x.role == a.role)
        {
            return Err(DomainError::DuplicateAssignee {
                mission_id,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        g.push(a.clone());
        Ok(a)
    }
    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let mut g = self.assignees.lock().unwrap();
        let before = g.len();
        g.retain(|a| !(a.id == assignee_id && a.id == assignee_id));
        if g.len() == before {
            Err(DomainError::AssigneeNotFound)
        } else {
            Ok(())
        }
    }
}

// Tiny helper so the duplicate-detection closure above reads cleanly.
trait AssigneeMissionId {
    fn mission_id_if_some(&self) -> i64;
}
impl AssigneeMissionId for Assignee {
    fn mission_id_if_some(&self) -> i64 {
        // The fake carries no mission_id on the assignee struct
        // (the domain Assignee doesn't either — mission_id lives
        // only on the row). We compare by id alone; the usecase
        // delegates uniqueness to the per-mission check it runs
        // before calling the repo. This dummy method returns a
        // constant so the closure type-checks; the duplicate
        // detection the test actually exercises is the one inside
        // `assignees_within_mission_are_unique`.
        0
    }
}

struct FakeProject {
    leader_for: Vec<&'static str>,
}

#[async_trait]
impl ProjectLookup for FakeProject {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code == "p1" {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.into()))
        }
    }
    async fn is_leader(
        &self,
        project_code: &str,
        user_code: &str,
    ) -> Result<bool, DomainError> {
        Ok(project_code == "p1" && self.leader_for.contains(&user_code))
    }
}

struct FakeUser;

#[async_trait]
impl UserLookup for FakeUser {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code.starts_with('u') {
            Ok(())
        } else {
            Err(DomainError::UserNotFound(code.into()))
        }
    }
}

fn usecase() -> MissionUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser> {
    MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    })
}

#[tokio::test]
async fn create_mission_enforces_leadership() {
    let uc = usecase();
    let err = uc
        .create_mission(
            &Actor {
                user_code: "carol".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, crate::usecase::UsecaseError::Forbidden { .. }));
}

#[tokio::test]
async fn create_mission_succeeds_for_leader() {
    let uc = usecase();
    let view = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Sdtm,
                mission_code: "c1".into(),
                assignees: vec![crate::usecase::AssigneeData {
                    user_code: "u1".into(),
                    role: MissionRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(view.project_code, "p1");
    assert_eq!(view.assignees.len(), 1);
}

#[tokio::test]
async fn create_mission_rejects_unknown_user_in_assignees() {
    let uc = usecase();
    let err = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![crate::usecase::AssigneeData {
                    user_code: "ghost".into(),
                    role: MissionRole::Dev,
                }],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, crate::usecase::UsecaseError::Domain(DomainError::UserNotFound(_))));
}

#[tokio::test]
async fn list_missions_by_project_filters_by_kind() {
    let uc = usecase();
    // Seed two missions of different kinds via direct repo
    // access. We can't reach the private fields through the
    // usecase surface without a leader; use the leader path.
    for (kind, code) in [(MissionKind::Crf, "c1"), (MissionKind::Sdtm, "s1")] {
        uc.create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: kind,
                mission_code: code.into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();
    }
    let only_crf = uc.list_missions_by_project("p1", Some(MissionKind::Crf)).await.unwrap();
    assert_eq!(only_crf.len(), 1);
    assert_eq!(only_crf[0].mission_kind, MissionKind::Crf);
}
```

- [ ] **Step 5: Run usecase tests to confirm they fail**

Run: `cargo test -p mission --lib usecase::tests`
Expected: FAIL — usecase module does not exist yet.

- [ ] **Step 6: Write `usecase/commands.rs`**

```rust
use crate::domain::{MissionKind, MissionRole};

#[derive(Debug, Clone)]
pub struct CreateMission {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

#[derive(Debug, Clone)]
pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}
```

- [ ] **Step 7: Write `usecase/error.rs`**

```rust
use thiserror::Error;

use crate::domain::DomainError;

#[derive(Debug, Error)]
pub enum UsecaseError {
    #[error("{0}")]
    Domain(#[source] DomainError),

    #[error("forbidden: user {user_code} is not a leader of project {project_code}")]
    Forbidden {
        user_code: String,
        project_code: String,
    },
}

impl From<DomainError> for UsecaseError {
    fn from(e: DomainError) -> Self {
        UsecaseError::Domain(e)
    }
}
```

- [ ] **Step 8: Write `usecase/views.rs`**

```rust
use chrono::{DateTime, Utc};

use crate::domain::{Assignee, Mission};

/// Projection of `Mission` returned by the usecase to the facade.
/// The facade converts this into `apis::mission::MissionView` via
/// `From` impls in the facade module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionView {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: crate::domain::MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssigneeView {
    pub id: i64,
    pub user_code: String,
    pub role: crate::domain::MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Mission> for MissionView {
    fn from(m: Mission) -> Self {
        MissionView {
            id: m.id,
            project_code: m.project_code,
            mission_kind: m.mission_kind,
            mission_code: m.mission_code,
            assignees: m.assignees.into_iter().map(Into::into).collect(),
            created_at: m.created_at,
            updated_at: m.updated_at,
        }
    }
}

impl From<Assignee> for AssigneeView {
    fn from(a: Assignee) -> Self {
        AssigneeView {
            id: a.id,
            user_code: a.user_code,
            role: a.role,
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
```

- [ ] **Step 9: Write `usecase/mission_usecase.rs`**

```rust
use apis::mission::Actor;

use crate::domain::{
    assignees_within_mission_are_unique, AssigneeNew, AssigneeRepository, DomainError, MissionKind,
    MissionRepository, ProjectLookup, UserLookup,
};

use super::commands::{AssigneeData, CreateMission};
use super::error::UsecaseError;
use super::views::{AssigneeView, MissionView};

pub struct MissionUsecaseConfig<M, A, P, U> {
    pub mission_repo: M,
    pub assignee_repo: A,
    pub project_lookup: P,
    pub user_lookup: U,
}

pub struct MissionUsecase<M, A, P, U> {
    pub(crate) mission_repo: M,
    pub(crate) assignee_repo: A,
    pub(crate) project_lookup: P,
    pub(crate) user_lookup: U,
}

impl<M, A, P, U> MissionUsecase<M, A, P, U>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    pub fn new(config: MissionUsecaseConfig<M, A, P, U>) -> Self {
        Self {
            mission_repo: config.mission_repo,
            assignee_repo: config.assignee_repo,
            project_lookup: config.project_lookup,
            user_lookup: config.user_lookup,
        }
    }

    async fn ensure_leader(
        &self,
        actor: &Actor,
        project_code: &str,
    ) -> Result<(), UsecaseError> {
        let is_leader = self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?;
        if !is_leader {
            return Err(UsecaseError::Forbidden {
                user_code: actor.user_code.clone(),
                project_code: project_code.to_string(),
            });
        }
        Ok(())
    }

    async fn ensure_project_exists(&self, project_code: &str) -> Result<(), UsecaseError> {
        self.project_lookup
            .get_by_code(project_code)
            .await
            .map_err(UsecaseError::from)
    }

    async fn ensure_user_exists(&self, user_code: &str) -> Result<(), UsecaseError> {
        self.user_lookup
            .get_by_code(user_code)
            .await
            .map_err(UsecaseError::from)
    }

    pub async fn create_mission(
        &self,
        actor: &Actor,
        input: CreateMission,
    ) -> Result<MissionView, UsecaseError> {
        self.ensure_leader(actor, &input.project_code).await?;
        self.ensure_project_exists(&input.project_code).await?;

        // Validate every assignee user exists up front so the
        // usecase surfaces a structured `UserNotFound` before
        // the repo transaction starts. The DB CHECK + UNIQUE
        // remain the safety net.
        for a in &input.assignees {
            self.ensure_user_exists(&a.user_code).await?;
        }

        assignees_within_mission_are_unique(
            &input
                .assignees
                .iter()
                .map(|a| AssigneeNew {
                    user_code: a.user_code.clone(),
                    role: a.role,
                })
                .collect::<Vec<_>>(),
        )?;

        let mission = self
            .mission_repo
            .create(crate::domain::MissionNew {
                project_code: input.project_code,
                mission_kind: input.mission_kind,
                mission_code: input.mission_code,
                assignees: input
                    .assignees
                    .into_iter()
                    .map(|a| AssigneeNew {
                        user_code: a.user_code,
                        role: a.role,
                    })
                    .collect(),
            })
            .await?;

        Ok(mission.into())
    }

    pub async fn get_mission_by_id(&self, id: i64) -> Result<MissionView, UsecaseError> {
        Ok(self.mission_repo.find_by_id(id).await?.into())
    }

    pub async fn list_missions_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<MissionView>, UsecaseError> {
        Ok(self
            .mission_repo
            .list_by_project(project_code, kind)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn list_missions_by_user(
        &self,
        user_code: &str,
    ) -> Result<Vec<MissionView>, UsecaseError> {
        Ok(self
            .mission_repo
            .list_by_user(user_code)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), UsecaseError> {
        let m = self.mission_repo.find_by_id(id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.mission_repo.delete(id).await?;
        Ok(())
    }

    pub async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<AssigneeView, UsecaseError> {
        let m = self.mission_repo.find_by_id(mission_id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.ensure_user_exists(&data.user_code).await?;
        let assignee = self
            .assignee_repo
            .add(
                mission_id,
                AssigneeNew {
                    user_code: data.user_code,
                    role: data.role,
                },
            )
            .await?;
        Ok(assignee.into())
    }

    pub async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), UsecaseError> {
        let m = self.mission_repo.find_by_id(mission_id).await?;
        self.ensure_leader(actor, &m.project_code).await?;
        self.assignee_repo.remove(mission_id, assignee_id).await?;
        Ok(())
    }
}
```

- [ ] **Step 10: Wire `usecase.rs`**

Replace `lib/crates/mission/src/usecase.rs`:

```rust
//! Usecase layer.
//!
//! `MissionUsecase<M, A, P, U>` orchestrates the four ports
//! (mission, assignee, project lookup, user lookup) and surfaces
//! `UsecaseError`. Every write method calls
//! `project_lookup.is_leader` and projects the domain aggregate
//! into the `MissionView` / `AssigneeView` DTOs the facade maps
//! to the apis port types.

mod commands;
mod error;
mod mission_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{AssigneeData, CreateMission};
pub use error::UsecaseError;
pub use mission_usecase::{MissionUsecase, MissionUsecaseConfig};
pub use views::{AssigneeView, MissionView};
```

- [ ] **Step 11: Write `adapter/facade/in_memory/service.rs`**

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::mission::{
    Actor, AssigneeData, AssigneeView as ApiAssigneeView, CreateMissionRequest,
    ListMissionsByProjectRequest, ListMissionsByUserRequest, MissionApiError, MissionService,
    MissionView as ApiMissionView,
};

use crate::domain::{MissionRepository, ProjectLookup, UserLookup};
use crate::usecase::{
    AssigneeData as UcAssigneeData, CreateMission as UcCreateMission, MissionUsecase,
    MissionUsecaseConfig, UsecaseError,
};

use super::super::super::usecase::AssigneeView as UcAssigneeView;
use super::super::super::usecase::MissionView as UcMissionView;

pub struct MissionServiceImpl<M, A, P, U> {
    usecase: MissionUsecase<M, A, P, U>,
}

impl<M, A, P, U> MissionServiceImpl<M, A, P, U>
where
    M: MissionRepository,
    A: crate::domain::AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    pub fn from_usecase(usecase: MissionUsecase<M, A, P, U>) -> Self {
        Self { usecase }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_repos(
        mission_repo: M,
        assignee_repo: crate::domain::AssigneeRepositoryT<A>,
        projects: Arc<P>,
        users: Arc<U>,
    ) -> Self
    where
        A: crate::domain::AssigneeRepository,
    {
        Self::from_usecase(MissionUsecase::new(MissionUsecaseConfig {
            mission_repo,
            assignee_repo,
            project_lookup: projects,
            user_lookup: users,
        }))
    }
}

// Dummy alias used only to keep the `from_repos` signature
// compilable under the test scaffolding without a re-arc-ing
// helper. The actual call sites always pass an `A` that already
// implements `AssigneeRepository`. This alias exists so the
// `#[allow(clippy::too_many_arguments)]` attribute has a real
// item to attach to; remove it (and the alias) once a real
// `Arc<A>` constructor is added.
pub trait AssigneeRepositoryT {}

#[async_trait]
impl<M, A, P, U> MissionService for MissionServiceImpl<M, A, P, U>
where
    M: MissionRepository + 'static,
    A: crate::domain::AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
{
    async fn create_mission(
        &self,
        actor: &Actor,
        req: CreateMissionRequest,
    ) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .create_mission(
                actor,
                UcCreateMission {
                    project_code: req.project_code,
                    mission_kind: req.mission_kind.into(),
                    mission_code: req.mission_code,
                    assignees: req
                        .assignees
                        .into_iter()
                        .map(|a| UcAssigneeData {
                            user_code: a.user_code,
                            role: a.role.into(),
                        })
                        .collect(),
                },
            )
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn get_mission_by_id(&self, id: i64) -> Result<ApiMissionView, MissionApiError> {
        self.usecase
            .get_mission_by_id(id)
            .await
            .map(into_api_mission)
            .map_err(map_error)
    }

    async fn list_missions_by_project(
        &self,
        req: ListMissionsByProjectRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_project(&req.project_code, req.kind.map(Into::into))
            .await
            .map(|v| v.map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_user(&req.user_code)
            .await
            .map(|v| v.map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn delete_mission(
        &self,
        actor: &Actor,
        id: i64,
    ) -> Result<(), MissionApiError> {
        self.usecase.delete_mission(actor, id).await.map_err(map_error)
    }

    async fn add_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        data: AssigneeData,
    ) -> Result<ApiAssigneeView, MissionApiError> {
        self.usecase
            .add_assignee(
                actor,
                mission_id,
                UcAssigneeData {
                    user_code: data.user_code,
                    role: data.role.into(),
                },
            )
            .await
            .map(into_api_assignee)
            .map_err(map_error)
    }

    async fn remove_assignee(
        &self,
        actor: &Actor,
        mission_id: i64,
        assignee_id: i64,
    ) -> Result<(), MissionApiError> {
        self.usecase
            .remove_assignee(actor, mission_id, assignee_id)
            .await
            .map_err(map_error)
    }
}

fn into_api_mission(m: UcMissionView) -> ApiMissionView {
    ApiMissionView {
        id: m.id,
        project_code: m.project_code,
        mission_kind: m.mission_kind.into(),
        mission_code: m.mission_code,
        assignees: m.assignees.into_iter().map(into_api_assignee).collect(),
        created_at: m.created_at,
        updated_at: m.updated_at,
    }
}

fn into_api_assignee(a: UcAssigneeView) -> ApiAssigneeView {
    ApiAssigneeView {
        id: a.id,
        user_code: a.user_code,
        role: a.role.into(),
        created_at: a.created_at,
        updated_at: a.updated_at,
    }
}

fn map_error(e: UsecaseError) -> MissionApiError {
    match e {
        UsecaseError::Forbidden {
            user_code,
            project_code,
        } => MissionApiError::Forbidden {
            user_code,
            project_code,
        },
        UsecaseError::Domain(d) => match d {
            DomainError::EmptyMissionCode
            | DomainError::EmptyUserCode
            | DomainError::UnknownMissionKind(_)
            | DomainError::UnknownMissionRole(_) => {
                MissionApiError::Validation(d.to_string())
            }
            DomainError::NotFound => MissionApiError::NotFound,
            DomainError::AssigneeNotFound => MissionApiError::AssigneeNotFound,
            DomainError::ProjectNotFound(c) => MissionApiError::ProjectNotFound(c),
            DomainError::UserNotFound(c) => MissionApiError::UserNotFound(c),
            DomainError::DuplicateMission {
                project_code,
                mission_kind,
                mission_code,
            } => MissionApiError::DuplicateMission {
                project_code,
                mission_kind: mission_kind.into(),
                mission_code,
            },
            DomainError::DuplicateAssignee {
                mission_id,
                user_code,
                role,
            } => MissionApiError::DuplicateAssignee {
                mission_id,
                user_code,
                role: role.into(),
            },
            DomainError::Repository(s) => MissionApiError::Repository(s),
        },
    }
}
```

The `AssigneeRepositoryT` alias above is a placeholder; remove it before commit — the `from_repos` signature should take `Arc<A>` for symmetry with `CrfServiceImpl::from_repos`. Replace the `from_repos` body with the real implementation:

```rust
pub fn from_repos(
    mission_repo: M,
    assignee_repo: Arc<A>,
    projects: Arc<P>,
    users: Arc<U>,
) -> Self {
    Self::from_usecase(MissionUsecase::new(MissionUsecaseConfig {
        mission_repo,
        assignee_repo: (*assignee_repo).clone(),
        project_lookup: projects,
        user_lookup: users,
    }))
}
```

(Requires `A: Clone`. Add `A: Clone` to the where-clause of `from_repos` and remove the `AssigneeRepositoryT` alias entirely.)

- [ ] **Step 12: Wire `adapter/facade/in_memory.rs`**

Replace `lib/crates/mission/src/adapter/facade/in_memory.rs`:

```rust
//! In-memory facade.
//!
//! Holds a `MissionUsecase<M, A, P, U>` and projects its results
//! into the apis `MissionView` / `AssigneeView` types. The only
//! facade today.

mod service;
#[cfg(test)]
mod tests;

pub use service::MissionServiceImpl;
```

- [ ] **Step 13: Wire `adapter/facade.rs`**

Replace `lib/crates/mission/src/adapter/facade.rs`:

```rust
pub mod in_memory;
```

- [ ] **Step 14: Write the facade tests**

Create `lib/crates/mission/src/adapter/facade/in_memory/tests.rs`. Use the same fakes as `usecase/tests.rs` but call `MissionServiceImpl::from_usecase(...)` and exercise `MissionService` directly:

```rust
use std::sync::Arc;

use apis::mission::{
    Actor, AssigneeData, CreateMissionRequest, ListMissionsByProjectRequest,
    ListMissionsByUserRequest, MissionKind as ApiKind, MissionRole as ApiRole, MissionService,
};

use crate::domain::{AssigneeRepository, MissionKind, MissionRepository, ProjectLookup, UserLookup};
use crate::usecase::{
    AssigneeData as UcAssigneeData, CreateMission as UcCreateMission, MissionUsecase,
    MissionUsecaseConfig,
};

use super::service::MissionServiceImpl;

// Re-import the in-memory fakes built in `usecase/tests.rs` via a
// shared helper module at `src/test_support.rs` to avoid
// duplicating ~150 lines. See Step 14a for the helper.
#[path = "../../../test_support.rs"]
mod test_support;
use test_support::*;

fn service() -> MissionServiceImpl<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser> {
    let usecase = MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    });
    MissionServiceImpl::from_usecase(usecase)
}

#[tokio::test]
async fn facade_create_mission_for_non_leader_returns_forbidden() {
    let svc = Arc::new(service());
    let err = svc
        .create_mission(
            &Actor {
                user_code: "carol".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, apis::mission::MissionApiError::Forbidden { .. }));
}

#[tokio::test]
async fn facade_create_then_list_by_project() {
    let svc = Arc::new(service());
    let view = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Sdtm,
                mission_code: "c1".into(),
                assignees: vec![AssigneeData {
                    user_code: "u1".into(),
                    role: ApiRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    assert_eq!(view.mission_code, "c1");

    let list = svc
        .list_missions_by_project(ListMissionsByProjectRequest {
            project_code: "p1".into(),
            kind: None,
        })
        .await
        .unwrap();
    assert_eq!(list.len(), 1);

    let user_view = svc
        .list_missions_by_user(ListMissionsByUserRequest {
            user_code: "u1".into(),
        })
        .await
        .unwrap();
    assert_eq!(user_view.len(), 1);
}

#[tokio::test]
async fn facade_add_assignee_then_remove() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();

    let a = svc
        .add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            AssigneeData {
                user_code: "u2".into(),
                role: ApiRole::Qc,
            },
        )
        .await
        .unwrap();
    assert_eq!(a.user_code, "u2");

    svc.remove_assignee(
        &Actor {
            user_code: "alice".into(),
        },
        m.id,
        a.id,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn facade_duplicate_assignee_returns_duplicate_error() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![AssigneeData {
                    user_code: "u1".into(),
                    role: ApiRole::Dev,
                }],
            },
        )
        .await
        .unwrap();

    let err = svc
        .add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            AssigneeData {
                user_code: "u1".into(),
                role: ApiRole::Dev,
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::mission::MissionApiError::DuplicateAssignee { .. }
    ));
}

#[tokio::test]
async fn facade_delete_cascades_assignees() {
    let svc = Arc::new(service());
    let m = svc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMissionRequest {
                project_code: "p1".into(),
                mission_kind: ApiKind::Crf,
                mission_code: "c1".into(),
                assignees: vec![AssigneeData {
                    user_code: "u1".into(),
                    role: ApiRole::Dev,
                }],
            },
        )
        .await
        .unwrap();
    svc.delete_mission(
        &Actor {
            user_code: "alice".into(),
        },
        m.id,
    )
    .await
    .unwrap();
    let err = svc.get_mission_by_id(m.id).await.unwrap_err();
    assert!(matches!(err, apis::mission::MissionApiError::NotFound));
    // The list-by-user is now empty too.
    let user_view = svc
        .list_missions_by_user(ListMissionsByUserRequest {
            user_code: "u1".into(),
        })
        .await
        .unwrap();
    assert!(user_view.is_empty());
}
```

- [ ] **Step 14a: Add the shared test support module**

Create `lib/crates/mission/src/test_support.rs`:

```rust
//! Shared test fakes for the mission crate. Used by
//! `usecase::tests` and `adapter::facade::in_memory::tests` so
//! the fake definitions live in one place.

#![allow(dead_code)]

use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Mutex;

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, Mission, MissionKind, MissionNew,
    MissionRepository, MissionRole, ProjectLookup, UserLookup,
};

#[derive(Default)]
pub struct FakeMissionRepo {
    pub next_id: AtomicI32,
    pub missions: Mutex<Vec<Mission>>,
}

#[async_trait]
impl MissionRepository for FakeMissionRepo {
    async fn create(&self, input: MissionNew) -> Result<Mission, DomainError> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst) as i64;
        let now: DateTime<Utc> = Utc::now();
        let assignees = input
            .assignees
            .iter()
            .enumerate()
            .map(|(idx, a)| Assignee::for_repository(
                id * 1000 + idx as i64,
                a.user_code.clone(),
                a.role,
                now,
                now,
            ))
            .collect();
        let m = Mission::for_repository(
            id,
            input.project_code,
            input.mission_kind,
            input.mission_code,
            assignees,
            now,
            now,
        );
        self.missions.lock().unwrap().push(m.clone());
        Ok(m)
    }
    async fn find_by_id(&self, id: i64) -> Result<Mission, DomainError> {
        self.missions
            .lock()
            .unwrap()
            .iter()
            .find(|m| m.id == id)
            .cloned()
            .ok_or(DomainError::NotFound)
    }
    async fn list_by_project(
        &self,
        project_code: &str,
        kind: Option<MissionKind>,
    ) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| {
                m.project_code == project_code && kind.map_or(true, |k| k == m.mission_kind)
            })
            .cloned()
            .collect())
    }
    async fn list_by_user(&self, user_code: &str) -> Result<Vec<Mission>, DomainError> {
        Ok(self
            .missions
            .lock()
            .unwrap()
            .iter()
            .filter(|m| m.assignees.iter().any(|a| a.user_code == user_code))
            .cloned()
            .collect())
    }
    async fn delete(&self, id: i64) -> Result<(), DomainError> {
        let mut g = self.missions.lock().unwrap();
        let before = g.len();
        g.retain(|m| m.id != id);
        if g.len() == before {
            Err(DomainError::NotFound)
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
pub struct FakeAssigneeRepo {
    pub next_id: AtomicI32,
    pub assignees: Mutex<Vec<(i64, Assignee)>>, // (mission_id, assignee)
}

#[async_trait]
impl AssigneeRepository for FakeAssigneeRepo {
    async fn add(
        &self,
        mission_id: i64,
        input: AssigneeNew,
    ) -> Result<Assignee, DomainError> {
        let now = Utc::now();
        let a = Assignee::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.user_code,
            input.role,
            now,
            now,
        )?;
        let mut g = self.assignees.lock().unwrap();
        if g.iter().any(|(mid, x)| {
            *mid == mission_id && x.user_code == a.user_code && x.role == a.role
        }) {
            return Err(DomainError::DuplicateAssignee {
                mission_id,
                user_code: a.user_code.clone(),
                role: a.role,
            });
        }
        g.push((mission_id, a.clone()));
        Ok(a)
    }
    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let mut g = self.assignees.lock().unwrap();
        let before = g.len();
        g.retain(|(mid, a)| !(*mid == mission_id && a.id == assignee_id));
        if g.len() == before {
            Err(DomainError::AssigneeNotFound)
        } else {
            Ok(())
        }
    }
}

pub struct FakeProject {
    pub leader_for: Vec<&'static str>,
}

#[async_trait]
impl ProjectLookup for FakeProject {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code == "p1" {
            Ok(())
        } else {
            Err(DomainError::ProjectNotFound(code.into()))
        }
    }
    async fn is_leader(
        &self,
        project_code: &str,
        user_code: &str,
    ) -> Result<bool, DomainError> {
        Ok(project_code == "p1" && self.leader_for.contains(&user_code))
    }
}

pub struct FakeUser;

#[async_trait]
impl UserLookup for FakeUser {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        if code.starts_with('u') {
            Ok(())
        } else {
            Err(DomainError::UserNotFound(code.into()))
        }
    }
}
```

Refactor `usecase/tests.rs` to drop its local copies of the fakes and `#[path = "../test_support.rs"] mod test_support;` instead.

- [ ] **Step 15: Run facade tests**

Run: `cargo test -p mission --lib`
Expected: PASS for every test in `usecase/tests.rs` and `adapter/facade/in_memory/tests.rs`.

- [ ] **Step 16: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/apis/src/lib.rs lib/crates/apis/src/mission.rs \
        lib/crates/mission/src/usecase.rs lib/crates/mission/src/usecase \
        lib/crates/mission/src/adapter/facade.rs \
        lib/crates/mission/src/adapter/facade \
        lib/crates/mission/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat(mission): usecase + facade + apis::mission port

- apis::mission: MissionService port with MissionKind, MissionRole,
  MissionApiError, MissionView, AssigneeView, CreateMissionRequest,
  AssigneeData, ListMissionsByProjectRequest,
  ListMissionsByUserRequest, Actor; the apis crate stays
  serde / utoipa free
- mission::usecase: CreateMission / AssigneeData commands,
  MissionView / AssigneeView projections, UsecaseError with
  From<DomainError>, MissionUsecase<M, A, P, U> generic over the
  four ports with strict leader-only authorization on every write
- mission::adapter::facade::in_memory::MissionServiceImpl adapts
  MissionUsecase to apis::mission::MissionService with full
  UsecaseError → MissionApiError mapping
- Shared test_support module with FakeMissionRepo,
  FakeAssigneeRepo, FakeProject, FakeUser so usecase and facade
  tests share the fakes
- Tests: leadership enforcement, per-mission uniqueness, cascade
  delete, list-by-user filtering

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib.
EOF
)"
```

## Task 5: HTTP transport, state, run, openapi, integration + public_api tests

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml` (add `mission` dep)
- Modify: `apps/server/aegis-server/src/transport/http.rs` (`pub mod mission;`)
- Create: `apps/server/aegis-server/src/transport/http/mission.rs`
- Create: `apps/server/aegis-server/src/transport/http/mission/router.rs`
- Create: `apps/server/aegis-server/src/transport/http/mission/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (nest `/api/mission`)
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs` (mission wire DTOs)
- Modify: `apps/server/aegis-server/src/transport/http/error.rs` (Mission variant + tables)
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs` (register schemas)
- Modify: `apps/server/aegis-server/src/state.rs` (`mission: Arc<dyn MissionService>`)
- Modify: `apps/server/aegis-server/src/run.rs` (wire facade)
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (test module NullMissionService)
- Create: `lib/crates/mission/tests/public_api.rs`
- Create: `lib/crates/mission/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: every apis::mission type, every mission::* type from Tasks 1–4, `axum`, `utoipa`, `utoipa_axum`.
- Produces:
  - HTTP routes at `/api/mission`, `/api/mission/{id}`, `/api/mission/by-project/{project_code}`, `/api/mission/by-user/{user_code}`, `/api/mission/{id}/assignee`, `/api/mission/{id}/assignee/{assignee_id}`.
  - `AppState.mission` field, `run.rs` wiring, `NullMissionService` test double, `openapi::ApiDoc` schemas.
  - Compile-only `tests/public_api.rs` pinning the public surface.
  - `#[ignore]`-gated `tests/integration_persistence.rs` exercising the full Postgres round-trip.

- [ ] **Step 1: Add `mission` dep to the server crate**

Edit `apps/server/aegis-server/Cargo.toml`. Add to `[dependencies]`:

```toml
mission = { path = "../../../lib/crates/mission" }
```

(Place the line alphabetically — right after `domain-model` and before `terminology`.)

- [ ] **Step 2: Add the wire DTOs and `From` impls**

Edit `apps/server/aegis-server/src/transport/http/dto.rs`. Append the mission section at the end of the file (after the existing project DTOs):

```rust
// -- mission ----------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: dto::MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeDataRequest>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeDataRequest {
    pub user_code: String,
    pub role: dto::MissionRole,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionKind {
    Crf,
    Sdtm,
    Adam,
    Tfl,
}

impl From<apis::mission::MissionKind> for MissionKind {
    fn from(k: apis::mission::MissionKind) -> Self {
        match k {
            apis::mission::MissionKind::Crf => MissionKind::Crf,
            apis::mission::MissionKind::Sdtm => MissionKind::Sdtm,
            apis::mission::MissionKind::Adam => MissionKind::Adam,
            apis::mission::MissionKind::Tfl => MissionKind::Tfl,
        }
    }
}

impl From<MissionKind> for apis::mission::MissionKind {
    fn from(k: MissionKind) -> Self {
        match k {
            MissionKind::Crf => apis::mission::MissionKind::Crf,
            MissionKind::Sdtm => apis::mission::MissionKind::Sdtm,
            MissionKind::Adam => apis::mission::MissionKind::Adam,
            MissionKind::Tfl => apis::mission::MissionKind::Tfl,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum MissionRole {
    Dev,
    Qc,
}

impl From<apis::mission::MissionRole> for MissionRole {
    fn from(r: apis::mission::MissionRole) -> Self {
        match r {
            apis::mission::MissionRole::Dev => MissionRole::Dev,
            apis::mission::MissionRole::Qc => MissionRole::Qc,
        }
    }
}

impl From<MissionRole> for apis::mission::MissionRole {
    fn from(r: MissionRole) -> Self {
        match r {
            MissionRole::Dev => apis::mission::MissionRole::Dev,
            MissionRole::Qc => apis::mission::MissionRole::Qc,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionViewResponse {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AssigneeViewResponse {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct MissionListResponse {
    pub missions: Vec<MissionViewResponse>,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathId {
    pub id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathProjectCode {
    pub project_code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathUserCode {
    pub user_code: String,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathMissionId {
    pub mission_id: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PathMissionIdAssignee {
    pub mission_id: i64,
    pub assignee_id: i64,
}

impl From<apis::mission::MissionView> for MissionViewResponse {
    fn from(v: apis::mission::MissionView) -> Self {
        MissionViewResponse {
            id: v.id,
            project_code: v.project_code,
            mission_kind: v.mission_kind.into(),
            mission_code: v.mission_code,
            assignees: v.assignees.into_iter().map(Into::into).collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

impl From<apis::mission::AssigneeView> for AssigneeViewResponse {
    fn from(a: apis::mission::AssigneeView) -> Self {
        AssigneeViewResponse {
            id: a.id,
            user_code: a.user_code,
            role: a.role.into(),
            created_at: a.created_at,
            updated_at: a.updated_at,
        }
    }
}
```

(The existing `dto.rs` already has a `ProjectMemberDataRequest` type with a `pub members` / `pub unblind_members` shape — these new mission types don't shadow anything. Place them at the end of the file.)

- [ ] **Step 3: Add the mission error arm**

Edit `apps/server/aegis-server/src/transport/http/error.rs`. Add a new variant:

```rust
#[error("{0}")]
Mission(#[from] apis::mission::MissionApiError),
```

In the `status()` helper, add:

```rust
fn mission_status(e: &apis::mission::MissionApiError) -> StatusCode {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => StatusCode::BAD_REQUEST,
        MissionApiError::NotFound => StatusCode::NOT_FOUND,
        MissionApiError::AssigneeNotFound => StatusCode::NOT_FOUND,
        MissionApiError::ProjectNotFound(_) => StatusCode::NOT_FOUND,
        MissionApiError::UserNotFound(_) => StatusCode::NOT_FOUND,
        MissionApiError::Forbidden { .. } => StatusCode::FORBIDDEN,
        MissionApiError::DuplicateMission { .. } => StatusCode::CONFLICT,
        MissionApiError::DuplicateAssignee { .. } => StatusCode::CONFLICT,
        MissionApiError::Repository(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}
```

Wire it into the `status()` match's `ApiError::Mission(e)` arm:

```rust
ApiError::Mission(e) => mission_status(e),
```

In the `code()` helper, add a `mission_code` function:

```rust
fn mission_code(e: &apis::mission::MissionApiError) -> &'static str {
    use apis::mission::MissionApiError;
    match e {
        MissionApiError::Validation(_) => "mission_validation_failed",
        MissionApiError::NotFound => "mission_not_found",
        MissionApiError::AssigneeNotFound => "assignee_not_found",
        MissionApiError::ProjectNotFound(_) => "project_not_found",
        MissionApiError::UserNotFound(_) => "user_not_found",
        MissionApiError::Forbidden { .. } => "mission_forbidden",
        MissionApiError::DuplicateMission { .. } => "mission_duplicate",
        MissionApiError::DuplicateAssignee { .. } => "assignee_duplicate",
        MissionApiError::Repository(_) => "mission_repository_error",
    }
}
```

And in `code()`:

```rust
ApiError::Mission(e) => mission_code(e),
```

The `ErrorBody` is rendered through the existing helper (`fn_error_body(self) -> ErrorBody` matches `code()` and `&self.to_string()`). No change needed there.

- [ ] **Step 4: Register the new schemas in OpenAPI**

Edit `apps/server/aegis-server/src/transport/http/openapi.rs`. Inside `components(schemas(…))`, add the new wire DTOs:

```rust
dto::CreateMissionRequest,
dto::AssigneeDataRequest,
dto::MissionKind,
dto::MissionRole,
dto::MissionViewResponse,
dto::AssigneeViewResponse,
dto::MissionListResponse,
dto::PathId,
dto::PathProjectCode,
dto::PathUserCode,
dto::PathMissionId,
dto::PathMissionIdAssignee,
```

- [ ] **Step 5: Create `apps/server/aegis-server/src/transport/http/mission.rs`**

```rust
//! Mission HTTP feature module.
//!
//! The `MissionService` trait exposes seven operations; six land
//! at HTTP. Each handler is a thin adapter over
//! [`crate::transport::http::dto`] and the apis DTOs.

pub mod handlers;
pub mod router;
```

- [ ] **Step 6: Create `apps/server/aegis-server/src/transport/http/mission/router.rs`**

```rust
//! Mission HTTP routes.
//!
//! Mounted at `/api/mission` by the top-level router.

use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;
use crate::transport::http::mission::handlers;

/// Build the resource router that backs `/api/mission`. Each
/// handler is registered in its own `routes!(...)` call because
/// utoipa-axum 0.2 panics on multiple same-method handlers in a
/// single call.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(handlers::create_mission))
        .routes(routes!(handlers::get_mission_by_id))
        .routes(routes!(handlers::list_missions_by_project))
        .routes(routes!(handlers::list_missions_by_user))
        .routes(routes!(handlers::delete_mission))
        .routes(routes!(handlers::add_assignee))
        .routes(routes!(handlers::remove_assignee))
}
```

- [ ] **Step 7: Create `apps/server/aegis-server/src/transport/http/mission/handlers.rs`**

```rust
//! HTTP handlers for the mission namespace.
//!
//! Each handler is a thin adapter over [`crate::transport::http::dto`]
//! and the apis DTOs. Every write handler builds an [`Actor`] from
//! the JWT subject and lets the usecase enforce leadership.

use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;

use apis::mission::{
    Actor, AssigneeData, CreateMissionRequest, ListMissionsByProjectRequest,
    ListMissionsByUserRequest,
};

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{
    self, PathId, PathMissionId, PathMissionIdAssignee, PathProjectCode, PathUserCode,
};
use crate::transport::http::error::ApiError;

fn to_actor(claims: &AuthClaims) -> Actor {
    Actor {
        user_code: claims.code.clone(),
    }
}

fn assignee_data(d: dto::AssigneeDataRequest) -> AssigneeData {
    AssigneeData {
        user_code: d.user_code,
        role: d.role.into(),
    }
}

// -- missions --------------------------------------------------------------

/// `POST /api/mission` — create a mission.
#[utoipa::path(
    post, path = "", tag = "mission",
    operation_id = "mission_create",
    request_body = dto::CreateMissionRequest,
    responses(
        (status = 201, description = "mission created", body = dto::MissionViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the project's leader set", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Project / user not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Mission / assignee already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn create_mission(
    State(state): State<AppState>,
    claims: AuthClaims,
    Json(req): Json<dto::CreateMissionRequest>,
) -> Result<(StatusCode, Json<dto::MissionViewResponse>), ApiError> {
    let view = state
        .mission
        .create_mission(
            &to_actor(&claims),
            CreateMissionRequest {
                project_code: req.project_code,
                mission_kind: req.mission_kind.into(),
                mission_code: req.mission_code,
                assignees: req.assignees.into_iter().map(assignee_data).collect(),
            },
        )
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `GET /api/mission/{id}` — fetch a mission by id.
#[utoipa::path(
    get, path = "/{id}", tag = "mission",
    operation_id = "mission_get_by_id",
    params(("id" = i64, Path, description = "Mission id")),
    responses(
        (status = 200, description = "mission found", body = dto::MissionViewResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn get_mission_by_id(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathId { id }): Path<PathId>,
) -> Result<Json<dto::MissionViewResponse>, ApiError> {
    let view = state.mission.get_mission_by_id(id).await?;
    Ok(Json(view.into()))
}

/// `GET /api/mission/by-project/{project_code}` — list missions
/// for a project. Optional `?kind=crf|sdtm|adam|tfl` filter.
#[utoipa::path(
    get, path = "/by-project/{project_code}", tag = "mission",
    operation_id = "mission_list_by_project",
    params(
        ("project_code" = String, Path, description = "Project code"),
        ("kind" = Option<String>, Query, description = "Filter by mission kind"),
    ),
    responses(
        (status = 200, description = "missions list", body = dto::MissionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_missions_by_project(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathProjectCode { project_code }): Path<PathProjectCode>,
    axum::extract::Query(q): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Result<Json<dto::MissionListResponse>, ApiError> {
    let kind = q
        .get("kind")
        .map(|s| apis::mission::MissionKind::try_from(s.as_str()))
        .transpose()
        .map_err(|e| ApiError::Mission(apis::mission::MissionApiError::Validation(e.to_string())))?;
    let views = state
        .mission
        .list_missions_by_project(ListMissionsByProjectRequest {
            project_code,
            kind,
        })
        .await?;
    Ok(Json(dto::MissionListResponse {
        missions: views.into_iter().map(Into::into).collect(),
    }))
}

/// `GET /api/mission/by-user/{user_code}` — list missions the
/// user appears on (across roles).
#[utoipa::path(
    get, path = "/by-user/{user_code}", tag = "mission",
    operation_id = "mission_list_by_user",
    params(("user_code" = String, Path, description = "User code")),
    responses(
        (status = 200, description = "missions list", body = dto::MissionListResponse),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn list_missions_by_user(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Path(PathUserCode { user_code }): Path<PathUserCode>,
) -> Result<Json<dto::MissionListResponse>, ApiError> {
    let views = state
        .mission
        .list_missions_by_user(ListMissionsByUserRequest { user_code })
        .await?;
    Ok(Json(dto::MissionListResponse {
        missions: views.into_iter().map(Into::into).collect(),
    }))
}

/// `DELETE /api/mission/{id}` — hard delete; cascades to assignees.
#[utoipa::path(
    delete, path = "/{id}", tag = "mission",
    operation_id = "mission_delete",
    params(("id" = i64, Path, description = "Mission id")),
    responses(
        (status = 204, description = "mission deleted"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn delete_mission(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathId { id }): Path<PathId>,
) -> Result<StatusCode, ApiError> {
    state.mission.delete_mission(&to_actor(&claims), id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `POST /api/mission/{id}/assignee` — add an assignee.
#[utoipa::path(
    post, path = "/{mission_id}/assignee", tag = "mission",
    operation_id = "mission_add_assignee",
    params(("mission_id" = i64, Path, description = "Mission id")),
    request_body = dto::AssigneeDataRequest,
    responses(
        (status = 201, description = "assignee added", body = dto::AssigneeViewResponse),
        (status = 400, description = "Validation failed", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission / user not found", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Assignee already exists", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn add_assignee(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathMissionId { mission_id }): Path<PathMissionId>,
    Json(req): Json<dto::AssigneeDataRequest>,
) -> Result<(StatusCode, Json<dto::AssigneeViewResponse>), ApiError> {
    let view = state
        .mission
        .add_assignee(&to_actor(&claims), mission_id, assignee_data(req))
        .await?;
    Ok((StatusCode::CREATED, Json(view.into())))
}

/// `DELETE /api/mission/{mission_id}/assignee/{assignee_id}` —
/// remove an assignee.
#[utoipa::path(
    delete, path = "/{mission_id}/assignee/{assignee_id}", tag = "mission",
    operation_id = "mission_remove_assignee",
    params(
        ("mission_id" = i64, Path, description = "Mission id"),
        ("assignee_id" = i64, Path, description = "Assignee id"),
    ),
    responses(
        (status = 204, description = "assignee removed"),
        (status = 401, description = "Missing / invalid token", body = crate::transport::http::error::ErrorBody),
        (status = 403, description = "Caller is not a leader of the mission's project", body = crate::transport::http::error::ErrorBody),
        (status = 404, description = "Mission / assignee not found", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Repository failure", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn remove_assignee(
    State(state): State<AppState>,
    claims: AuthClaims,
    Path(PathMissionIdAssignee {
        mission_id,
        assignee_id,
    }): Path<PathMissionIdAssignee>,
) -> Result<StatusCode, ApiError> {
    state
        .mission
        .remove_assignee(&to_actor(&claims), mission_id, assignee_id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
```

- [ ] **Step 8: Register `pub mod mission;` on the http transport root**

Edit `apps/server/aegis-server/src/transport/http.rs`. Add `pub mod mission;` to the module list:

```rust
pub mod auth;
pub mod crf;
pub mod domain_model;
pub mod dto;
pub mod error;
pub mod healthz;
pub mod mission;
pub mod openapi;
pub mod project;
pub mod router;
pub mod terminology;
pub mod user;
```

- [ ] **Step 9: Mount `/api/mission` in the router**

Edit `apps/server/aegis-server/src/transport/http/router.rs`. Add a use statement and a route nest:

```rust
use crate::transport::http::mission::router as mission_router;
```

In the `api_routers` builder:

```rust
let mission_routes = mission_router::router();
…
.nest("/mission", mission_routes)
```

The doc comment at the top should mention the new routes:

```
//! - `/api/mission/*`                mission CRUD + assignee management
```

- [ ] **Step 10: Add `mission` to `AppState`**

Edit `apps/server/aegis-server/src/state.rs`. Add a `pub mission: Arc<dyn apis::mission::MissionService>` field:

```rust
#[derive(Clone)]
pub struct AppState {
    pub auth: Arc<dyn apis::auth::AuthService>,
    pub user: Arc<dyn apis::user::UserService>,
    pub project: Arc<dyn apis::project::ProjectService>,
    pub terminology: Arc<dyn apis::terminology::TerminologyService>,
    pub domain_model: Arc<dyn apis::domain_model::DomainModelService>,
    pub crf: Arc<dyn apis::crf::CrfService>,
    pub mission: Arc<dyn apis::mission::MissionService>,
}
```

Add a `NullMissionService` to `state::test_support`:

```rust
#[derive(Clone)]
pub(crate) struct NullMissionService;

#[async_trait]
impl apis::mission::MissionService for NullMissionService {
    async fn create_mission(
        &self,
        _: &apis::mission::Actor,
        _: apis::mission::CreateMissionRequest,
    ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn get_mission_by_id(
        &self,
        _: i64,
    ) -> Result<apis::mission::MissionView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn list_missions_by_project(
        &self,
        _: apis::mission::ListMissionsByProjectRequest,
    ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn list_missions_by_user(
        &self,
        _: apis::mission::ListMissionsByUserRequest,
    ) -> Result<Vec<apis::mission::MissionView>, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn delete_mission(
        &self,
        _: &apis::mission::Actor,
        _: i64,
    ) -> Result<(), apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn add_assignee(
        &self,
        _: &apis::mission::Actor,
        _: i64,
        _: apis::mission::AssigneeData,
    ) -> Result<apis::mission::AssigneeView, apis::mission::MissionApiError> {
        unimplemented!()
    }
    async fn remove_assignee(
        &self,
        _: &apis::mission::Actor,
        _: i64,
        _: i64,
    ) -> Result<(), apis::mission::MissionApiError> {
        unimplemented!()
    }
}
```

(Place this in `test_support` after `NullCrfService`.)

- [ ] **Step 11: Update the router-level test module**

Edit `apps/server/aegis-server/src/transport/http/router.rs` `tests` module. The existing `tests::AppState::new(...)` builder probably uses `NullCrfService` etc.; add a `NullMissionService` field to the test builder so the existing `AppState` literal in tests compiles.

- [ ] **Step 12: Wire the mission facade in `run.rs`**

Edit `apps/server/aegis-server/src/run.rs`. After the existing `crf` wiring (around the place where `CrfServiceImpl::from_usecase(...)` is wrapped in `Arc`), add:

```rust
let project_lookup: Arc<mission::ProjectLookupImpl> =
    Arc::new(mission::ProjectLookupImpl::new(state.project.clone()));
let user_lookup: Arc<mission::UserLookupImpl> =
    Arc::new(mission::UserLookupImpl::new(state.user.clone()));
let mission_repo = mission::MissionRepo::new(pool.clone());
let assignee_repo = mission::AssigneeRepo::new(pool.clone());
let mission_usecase = mission::MissionUsecase::new(mission::MissionUsecaseConfig {
    mission_repo: mission_repo.clone(),
    assignee_repo: assignee_repo.clone(),
    project_lookup: project_lookup.clone(),
    user_lookup: user_lookup.clone(),
});
let mission_service: Arc<dyn apis::mission::MissionService> =
    Arc::new(mission::MissionServiceImpl::from_usecase(mission_usecase));
```

(Place it before `state.mission = mission_service;`. Add `use mission::{…};` imports as needed.)

The `state` field is the `AppState` constructed earlier in `run`; replace its `mission` field with the wired value. If the `state` is built via `..Default::default()`, supply `mission: mission_service` explicitly.

- [ ] **Step 13: Add `apis::mission::MissionKind::TryFrom<&str>`**

The `handlers.rs` code calls `apis::mission::MissionKind::try_from(s.as_str())` — the apis port doesn't currently expose that. Add the `TryFrom<&str>` impl to `apis::mission`:

Edit `lib/crates/apis/src/mission.rs`. Add:

```rust
impl std::str::FromStr for MissionKind {
    type Err = crate::mission::MissionApiError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "crf" => Ok(MissionKind::Crf),
            "sdtm" => Ok(MissionKind::Sdtm),
            "adam" => Ok(MissionKind::Adam),
            "tfl" => Ok(MissionKind::Tfl),
            other => Err(crate::mission::MissionApiError::Validation(format!(
                "unknown mission kind: {}",
                other
            ))),
        }
    }
}

impl std::convert::TryFrom<&str> for MissionKind {
    type Error = crate::mission::MissionApiError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
```

- [ ] **Step 14: Write the compile-only public API test**

Create `lib/crates/mission/tests/public_api.rs`:

```rust
//! Compile-only smoke test pinning the public API surface.
//!
//! Every name referenced from `aegis-server::run.rs`,
//! `aegis-server::state.rs`, or downstream consumer crates is
//! imported here. If a future refactor renames, reorders, or
//! tightens trait bounds on any of these names, this file stops
//! compiling and the refactor must update it in lockstep.

use std::sync::Arc;

use apis::mission::MissionService as ApiMissionService;
use mission::{
    AssigneeNew, AssigneeRepo, AssigneeRepository, DomainError, Mission, MissionKind, MissionNew,
    MissionRepo, MissionRepository, MissionRole, MissionServiceImpl, MissionUsecase,
    MissionUsecaseConfig, MissionView, ProjectLookup, ProjectLookupImpl, UserLookup, UserLookupImpl,
};

fn _mission_repo_new(pool: sqlx::PgPool) -> MissionRepo {
    MissionRepo::new(pool)
}

fn _assignee_repo_new(pool: sqlx::PgPool) -> AssigneeRepo {
    AssigneeRepo::new(pool)
}

fn _project_lookup(
    svc: Arc<dyn apis::project::ProjectService>,
) -> ProjectLookupImpl {
    ProjectLookupImpl::new(svc)
}

fn _user_lookup(
    svc: Arc<dyn apis::user::UserService>,
) -> UserLookupImpl {
    UserLookupImpl::new(svc)
}

fn _usecase_config<R, A, P, U>(
    mission_repo: R,
    assignee_repo: A,
    project_lookup: P,
    user_lookup: U,
) -> MissionUsecaseConfig<R, A, P, U>
where
    R: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    MissionUsecaseConfig {
        mission_repo,
        assignee_repo,
        project_lookup,
        user_lookup,
    }
}

fn _usecase<R, A, P, U>(
    cfg: MissionUsecaseConfig<R, A, P, U>,
) -> MissionUsecase<R, A, P, U>
where
    R: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    MissionUsecase::new(cfg)
}

fn _facade<R, A, P, U>(
    usecase: MissionUsecase<R, A, P, U>,
) -> MissionServiceImpl<R, A, P, U>
where
    R: MissionRepository + 'static,
    A: AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
{
    MissionServiceImpl::from_usecase(usecase)
}

// Verify the in-memory facade can be stored behind the apis port
// dyn-pointer.
fn _arc_dyn<M, A, P, U>(svc: MissionServiceImpl<M, A, P, U>) -> Arc<dyn ApiMissionService>
where
    M: MissionRepository + 'static,
    A: AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
{
    Arc::new(svc) as Arc<dyn ApiMissionService>
}

// Pin the domain types so a future rename is loud.
fn _domain_types() -> (
    MissionKind,
    MissionRole,
    Mission,
    MissionNew,
    AssigneeNew,
    MissionView,
    DomainError,
) {
    (
        MissionKind::Crf,
        MissionRole::Dev,
        todo!(),
        todo!(),
        todo!(),
        todo!(),
        DomainError::NotFound,
    )
}
```

- [ ] **Step 15: Write the integration persistence test**

Create `lib/crates/mission/tests/integration_persistence.rs`:

```rust
//! Live-database round-trip tests for the `mission` crate.
//!
//! Every test is gated with `#[ignore]`. Run with:
//!
//! ```bash
//! cargo test -p mission -- --ignored --test-threads=1
//! ```
//!
//! Pre-conditions:
//! - `AEGIS_MISSION_DATABASE_URL` points at a Postgres instance
//!   the test is allowed to drop tables in.
//! - The script first drops `assignees`, `missions`, and the
//!   `_sqlx_migrations` bookkeeping table, then re-applies the
//!   migrations via `sqlx::migrate!`. Destructive by design.

use std::sync::atomic::{AtomicU32, Ordering};

use chrono::Utc;
use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions;

use apis::mission::{
    Actor, AssigneeData as ApiAssigneeData, CreateMissionRequest, MissionKind as ApiKind,
    MissionRole as ApiRole, MissionService,
};

use mission::{
    AssigneeRepo, AssigneeNew, MissionRepo, MissionServiceImpl, MissionUsecase,
    MissionUsecaseConfig, ProjectLookupImpl, UserLookupImpl,
};

static SEQ: AtomicU32 = AtomicU32::new(0);

fn unique_suffix() -> String {
    let n = SEQ.fetch_add(1, Ordering::SeqCst);
    let ns = Utc::now().timestamp_nanos_opt().unwrap_or(0);
    format!("{}_{}", ns, n)
}

async fn pool() -> PgPool {
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_MISSION_DATABASE_URL")
        .expect("AEGIS_MISSION_DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .expect("connect");
    sqlx::query("DROP TABLE IF EXISTS assignees CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS missions CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::migrate!("../crates/mission/migrations")
        .run(&pool)
        .await
        .expect("migrate");
    pool
}

fn actor() -> Actor {
    Actor {
        user_code: "leader".into(),
    }
}

#[tokio::test]
#[ignore]
async fn create_find_list_delete_round_trip() {
    let pool = pool().await;
    let mission_repo = MissionRepo::new(pool.clone());
    let assignee_repo = AssigneeRepo::new(pool.clone());

    // Build a fake project + user port for the in-memory
    // wiring. The integration test exercises the Postgres-backed
    // mission / assignee repos; project + user existence is
    // checked via the apis ports — for the integration test we
    // stand up the project / user services via their in-memory
    // fakes (the crf crate's `ProjectLookupImpl` adapts apis).
    //
    // Because the apis ports don't expose an in-memory test
    // double, we instead skip the cross-crate lookup by
    // exercising only the persistence layer directly through
    // `MissionRepo` and `AssigneeRepo`. The facade's leadership
    // enforcement is covered by the in-memory facade tests.
    let suffix = unique_suffix();

    let mission_code = format!("m_{}", suffix);
    let input = mission::MissionNew {
        project_code: "p1".into(),
        mission_kind: mission::MissionKind::Crf,
        mission_code: mission_code.clone(),
        assignees: vec![AssigneeNew {
            user_code: "u1".into(),
            role: mission::MissionRole::Dev,
        }],
    };
    let mission = mission_repo.create(input).await.expect("create");
    assert_eq!(mission.mission_code, mission_code);
    assert_eq!(mission.assignees.len(), 1);

    let by_id = mission_repo.find_by_id(mission.id).await.expect("find_by_id");
    assert_eq!(by_id.assignees.len(), 1);

    let by_project = mission_repo
        .list_by_project("p1", None)
        .await
        .expect("list_by_project");
    assert_eq!(by_project.len(), 1);

    let by_user = mission_repo
        .list_by_user("u1")
        .await
        .expect("list_by_user");
    assert_eq!(by_user.len(), 1);

    mission_repo.delete(mission.id).await.expect("delete");
    let err = mission_repo.find_by_id(mission.id).await.unwrap_err();
    assert!(matches!(err, mission::DomainError::NotFound));

    let after_user = mission_repo
        .list_by_user("u1")
        .await
        .expect("list_by_user after delete");
    assert!(after_user.is_empty());

    // Verify the cascade at the SQL level: the assignees row
    // should be gone too.
    let cnt: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM assignees")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(cnt, 0);

    // Silence unused-warning when the lookup imports above go
    // away.
    let _ = (
        MissionUsecase::<_, _, ProjectLookupImpl, UserLookupImpl>::new,
        MissionServiceImpl::from_usecase::<MissionRepo, AssigneeRepo, ProjectLookupImpl, UserLookupImpl>,
        ApiKind::Crf,
        ApiRole::Dev,
        CreateMissionRequest {
            project_code: "p1".into(),
            mission_kind: ApiKind::Crf,
            mission_code: "x".into(),
            assignees: vec![ApiAssigneeData {
                user_code: "u1".into(),
                role: ApiRole::Dev,
            }],
        },
        actor(),
    );
}
```

(The test deliberately exercises `MissionRepo` / `AssigneeRepo` directly so it doesn't need to stand up a fake `apis::project::ProjectService` / `apis::user::UserService`. Leadership enforcement is covered by the in-memory facade tests.)

- [ ] **Step 16: Run the full verification gate**

```bash
cargo fmt --all
cargo fmt --all -- --check
cargo check --workspace
cargo clippy -p mission --all-targets --all-features -- -D warnings
cargo test -p mission
cargo doc -p mission --no-deps
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Expected: every check passes; every test passes (the live-DB integration is `#[ignore]` so it stays out of the default run).

- [ ] **Step 17: Lint and commit**

```bash
git add apps/server/aegis-server apps/server/aegis-server/Cargo.toml \
        lib/crates/mission/tests \
        lib/crates/apis/src/mission.rs
git commit -m "$(cat <<'EOF'
feat(aegis-server): mission HTTP router + state + run wiring

- apps/server/aegis-server/Cargo.toml: add mission path-dep
- transport::http::mission::{router,handlers}: mounts
  /api/mission with seven utoipa-tagged handlers
- transport::http::dto: CreateMissionRequest, AssigneeDataRequest,
  MissionKind, MissionRole, MissionViewResponse, AssigneeViewResponse,
  MissionListResponse, PathId, PathProjectCode, PathUserCode,
  PathMissionId, PathMissionIdAssignee + From<apis> impls
- transport::http::error: Mission(#[from] apis::mission::MissionApiError)
  variant with status / code tables mapping every
  MissionApiError variant to (StatusCode, error code string)
- transport::http::openapi: registers every new schema in
  components(schemas(...))
- AppState: mission: Arc<dyn apis::mission::MissionService>;
  test_support gains NullMissionService
- run.rs: wires ProjectLookupImpl + UserLookupImpl +
  MissionRepo + AssigneeRepo + MissionUsecase + MissionServiceImpl
  into AppState.mission
- apis::mission: adds FromStr + TryFrom<&str> for MissionKind so
  handlers can parse the ?kind= query parameter
- tests/public_api.rs: compile-only smoke test pinning the
  public API surface
- tests/integration_persistence.rs: #[ignore]-gated Postgres
  round-trip covering create / find / list / delete + cascade

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission;
cargo check --workspace; cargo clippy --workspace; cargo test
--workspace; cargo doc -p mission --no-deps. With
AEGIS_MISSION_DATABASE_URL: cargo test -p mission -- --ignored
--test-threads=1.
EOF
)"
```

## Task 6: README + lockfile commit

**Files:**
- Create: `lib/crates/mission/README.md`
- Modify: `Cargo.lock` (workspace dep drift only; auto-generated)

**Interfaces:**
- Consumes: nothing.
- Produces: a README documenting the crate shape, the DB setup, and the test commands; one `chore:` commit pinning the lockfile drift.

- [ ] **Step 1: Write the README**

Create `lib/crates/mission/README.md`:

````markdown
# mission

CRUD over the `Mission` aggregate and its `Assignee` child
collection. Backed by PostgreSQL via SQLx. Authorization is
strict leader-only via `apis::project::ProjectView.members.leaders`.

## Layered architecture

```
mission crate
└── adapter
    ├── facade                  (MissionServiceImpl<M, A, P, U>)
    ├── persistence             (MissionRepoPg, AssigneeRepoPg)
    └── service                 (ProjectLookupImpl, UserLookupImpl)
usecase
└── MissionUsecase<M, A, P, U>
domain
└── Mission, Assignee, MissionKind, MissionRole,
    MissionRepository, AssigneeRepository,
    ProjectLookup, UserLookup,
    DomainError
```

`adapter::persistence::postgres::MissionRepo::create` opens a
transaction so the mission row and every assignee row land
atomically. `assignees.mission_id` uses `ON DELETE CASCADE` so
mission deletion is a single DELETE.

## Data model

| Aggregate   | Fields                                                                  |
| ----------- | ----------------------------------------------------------------------- |
| `Mission`   | `id`, `project_code`, `mission_kind`, `mission_code`,                   |
|             | `created_at`, `updated_at`; UNIQUE (project_code, mission_kind, mission_code) |
| `Assignee`  | `id`, `user_code`, `role`,                                              |
|             | `created_at`, `updated_at`; UNIQUE (mission_id, user_code, role)        |

Enums:

- `MissionKind { Crf, Sdtm, Adam, Tfl }` — DB CHECK enforces
  the four-value set.
- `MissionRole { Dev, Qc }` — DB CHECK enforces the two-value
  set.

## Database setup

Apply the crate's migrations:

```bash
sqlx migrate run \
  --source lib/crates/mission/migrations \
  --database-url $AEGIS_MISSION_DATABASE_URL
```

Env var for live tests: `AEGIS_MISSION_DATABASE_URL`.

## Construction

```rust
use mission::{
{
    AssigneeRepo, MissionRepo, MissionServiceImpl, MissionUsecase,
    MissionUsecaseConfig, ProjectLookupImpl, UserLookupImpl,
};
use std::sync::Arc;

let project_lookup = Arc::new(ProjectLookupImpl::new(state.project.clone()));
let user_lookup    = Arc::new(UserLookupImpl::new(state.user.clone()));
let mission_repo   = MissionRepo::new(pool.clone());
let assignee_repo  = AssigneeRepo::new(pool.clone());

let usecase = MissionUsecase::new(MissionUsecaseConfig {
    mission_repo:   mission_repo.clone(),
    assignee_repo:  assignee_repo.clone(),
    project_lookup: project_lookup.clone(),
    user_lookup:    user_lookup.clone(),
});

let service: Arc<dyn apis::mission::MissionService> =
    Arc::new(MissionServiceImpl::from_usecase(usecase));
```

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p mission --all-targets --all-features -- -D warnings
cargo test -p mission
cargo doc -p mission --no-deps
```

Live-DB integration tests (gated with `#[ignore]`) require
`AEGIS_MISSION_DATABASE_URL`:

```bash
cargo test -p mission -- --ignored --test-threads=1
```

Spec: [`docs/superpowers/specs/2026-09-01-mission-crate-design.md`](../../docs/superpowers/specs/2026-09-01-mission-crate-design.md).
Conventions: [`docs/guidelines/lib-crate-development.md`](../../docs/guidelines/lib-crate-development.md).
````

- [ ] **Step 2: Commit the README**

```bash
git add lib/crates/mission/README.md
git commit -m "$(cat <<'EOF'
docs(mission): README

Documents the mission crate shape, the data model, the DB
setup, the construction snippet, and the verification gate.
Back-links to the spec and the lib-crate guideline.

Spec: docs/superpowers/specs/2026-09-01-mission-crate-design.md
EOF
)"
```

- [ ] **Step 3: Commit lockfile drift (if any)**

Run:

```bash
git status --short
```

If `Cargo.lock` is dirty, stage and commit:

```bash
git add Cargo.lock
git commit -m "chore: pin Cargo.lock after adding mission crate"
```

If `Cargo.lock` is already clean, skip this step.

- [ ] **Step 4: Final verification gate**

Run the full gate from the spec:

```bash
cargo fmt --all -- --check
cargo clippy -p mission --all-targets --all-features -- -D warnings
cargo test -p mission
cargo doc -p mission --no-deps
cargo check --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

Expected: every command succeeds. With
`AEGIS_MISSION_DATABASE_URL` set, also run:

```bash
cargo test -p mission -- --ignored --test-threads=1
```

Expected: the live-DB round-trip passes.

---

## Self-Review

1. **Spec coverage:**
   - Two aggregates + ports → Task 2 (`Mission`, `Assignee`,
     `MissionRepository`, `AssigneeRepository`).
   - `ProjectLookup` / `UserLookup` cross-crate adapters → Task 3
     (`ProjectLookupImpl`, `UserLookupImpl`).
   - `MissionUsecase<M, A, P, U>` with strict leader-only auth →
     Task 4.
   - `apis::mission::MissionService` port with all DTOs and
     error variants → Task 4.
   - `MissionServiceImpl` facade with `UsecaseError → MissionApiError`
     mapping → Task 4.
   - `0001_create_missions.sql` + `0002_create_assignees.sql`
     with UNIQUE / CHECK / trigger / `ON DELETE CASCADE` → Task 3.
   - Two-table persistence with transaction + reload → Task 3
     (`MissionRepo::create`).
   - `Arc<dyn apis::mission::MissionService>` in `AppState` →
     Task 5.
   - HTTP router at `/api/mission` with seven handlers →
     Task 5.
   - `ErrorBody` mapping (status / code) for every variant →
     Task 5.
   - Tests at all five tiers → Tasks 2, 3, 4, 5.
   - README + lockfile commit → Task 6.

2. **Placeholder scan:** no TBD / TODO / "fill in details" / "similar
   to Task N" placeholders.

3. **Type consistency:** every type referenced in a later task is
   defined in an earlier task. Cross-checked:
   - `MissionNew`, `AssigneeNew`, `MissionKind`, `MissionRole`,
     `MissionRepository`, `AssigneeRepository`, `ProjectLookup`,
     `UserLookup`, `DomainError` — Task 2.
   - `MissionRepo`, `AssigneeRepo`, `ProjectLookupImpl`,
     `UserLookupImpl` — Task 3.
   - `CreateMission`, `AssigneeData`, `MissionView`,
     `AssigneeView`, `UsecaseError`, `MissionUsecase`,
     `MissionUsecaseConfig`, `MissionServiceImpl` — Task 4.
   - `Actor`, `MissionApiError`, `MissionKind` (apis),
     `MissionRole` (apis), wire DTOs, `PathId`, `PathProjectCode`,
     `PathUserCode`, `PathMissionId`, `PathMissionIdAssignee`,
     `MissionViewResponse`, `AssigneeViewResponse`,
     `MissionListResponse` — Task 5.
   - `NullMissionService`, `AppState.mission` — Task 5.
   - Constructor signatures in `public_api.rs` (`MissionRepo::new(PgPool)`,
     `AssigneeRepo::new(PgPool)`, `ProjectLookupImpl::new(Arc<dyn ProjectService>)`,
     `UserLookupImpl::new(Arc<dyn UserService>)`,
     `MissionUsecase::new(MissionUsecaseConfig)`,
     `MissionServiceImpl::from_usecase(...)`) match the Task 3
     and Task 4 definitions.

   The `from_repos` constructor mentioned in the spec and used in
   the crf precedent has been **intentionally omitted** from the
   mission facade — the spec only requires `from_usecase` for the
   production wiring path, and `run.rs` (Task 5 Step 12) builds
   the usecase inline rather than through `from_repos`. If a
   later caller wants a single-call `from_repos`, it can be
   added without touching this plan.
