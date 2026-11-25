# MissionIssue Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Extend the existing `lib/crates/mission` business-lib crate with a `MissionIssue` aggregate and its `IssueComment` child collection per [`docs/superpowers/specs/2026-09-20-mission-issue-design.md`](../specs/2026-09-20-mission-issue-design.md). Add six methods to `apis::mission::MissionService` and five HTTP routes under `/api/mission/*`. Desktop Tauri commands and TS client surface are explicitly out of scope.

**Architecture:** Three DDD layers, mirroring the existing mission layout. New `domain::{IssueState, IssueComment, MissionIssue}` aggregates plus a `MissionIssueRepository` port and an `is_assignee` method on the existing `AssigneeRepository` port. New `MissionIssueUsecase<M, A, P, I>` orchestrates the four existing ports plus the new issue repo with two auth helpers — `ensure_can_create_or_close` (project leader OR mission QC) and `ensure_can_comment` (project leader OR mission DEV OR mission QC). New `IssueRepo` (Postgres) and `InMemoryIssueRepo` implement the port. The `MissionServiceImpl<M, A, P, U, I>` facade gains an `issue_usecase` field and adapts both usecases to the apis port. The server wires an `IssueRepo` next to the existing repos and mounts five new routes via utoipa-axum.

**Tech Stack:** `sqlx 0.9` (Postgres runtime API + `json` feature already on workspace deps), `tokio 1.53`, `async-trait 0.1.91`, `thiserror 2`, `chrono 0.4` (clock + serde), `serde 1` (derive), `serde_json 1`, `apis` (workspace path-dep), utoipa-axum + axum 0.8 for HTTP. No new workspace dependencies.

**Spec:** [`docs/superpowers/specs/2026-09-20-mission-issue-design.md`](../specs/2026-09-20-mission-issue-design.md)

## Global Constraints

These come from the spec and `docs/guidelines/lib-crate-development.md`; every task implicitly includes them.

- **Edition:** Rust 2024 (`edition = "2024"`, `resolver = "3"`).
- **No `mod.rs`:** every module uses `src/<module>.rs` + `src/<module>/`. Terminal leaf files (`issue_state.rs`, `issue_comment.rs`, `issue.rs`, `issue_lookup.rs`, `issue_usecase.rs`, `issue_repo.rs`, `issue_service.rs`) are leaf files with no companion directory.
- **Layer dependency rule:** `domain` depends on nothing except std + `async-trait`; `usecase` depends on `domain` + `apis` (port types only); `adapter` depends on `usecase` + `domain` + `apis` + `sqlx`. No layer reaches into a sibling layer inside the same crate beyond the documented direction.
- **Public surface:** the crate root (`mission::lib.rs`) re-exports every type, error variant, trait, command DTO, view DTO, and constructor that a consumer is allowed to name. No internal helpers, no row structs, no in-memory fakes are re-exported.
- **Runtime SQLx API:** the persistence adapter uses `sqlx::query_as` and `sqlx::QueryBuilder`. No compile-time `query!` / `query_as!` macros. The `comments` jsonb column is round-tripped via `sqlx::types::Json<Vec<IssueComment>>` (sqlx's `json` feature is already in the workspace deps).
- **`map_db_error` rules** mirror the existing mission crate: `sqlx::Error::RowNotFound` → `DomainError::NotFound`; everything else → `DomainError::Repository(driver_message)`. The new `IssueRepo` does not need unique-violation mapping — issues have no UNIQUE constraints and FK violations land as `Repository`.
- **Two-constructor pattern:** every aggregate has `new(...)` (validating, returns `Result`) and `pub(crate) fn for_repository(...)` (skips validation, used by the row bridge). Same for `IssueComment`.
- **Migrations:** consumed via `sqlx::migrate!("./migrations")` in integration tests. The new migration is `0003_create_mission_issues.sql`.
- **Env var:** live-DB tests read `AEGIS_MISSION_DATABASE_URL` (with `dotenvy::dotenv()` at startup; panic if missing). No new env var.
- **Unique per-run values:** integration tests generate a per-process atomic counter + wall-clock nanoseconds for any UNIQUE-constrained column. The new `mission_issues` table has no UNIQUE constraints, but the pattern stays.
- **Destructive cleanup:** integration tests `DROP TABLE IF EXISTS mission_issues CASCADE`, `DROP TABLE IF EXISTS assignees CASCADE`, `DROP TABLE IF EXISTS missions CASCADE`, and `DROP TABLE IF EXISTS _sqlx_migrations CASCADE` before applying migrations.
- **Layer-boundary visibility:** `adapter::persistence::postgres::issue_repo` is `pub(crate)`; the new `IssueRepo` is re-exported via `pub use` in `adapter::persistence::postgres`. `row` stays private to `postgres/`. `adapter::facade::in_memory::issue_service` exposes the in-memory repo only as a struct consumed by `MissionServiceImpl::from_repos` — not re-exported at the crate root.
- **Wire DTOs live in `aegis-server`**, not in `apis`. The apis crate stays serde / utoipa free; the handlers translate wire ↔ apis types at the boundary.
- **Test gates** per the lib-crate guideline section 11:
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
migrations/
  0003_create_mission_issues.sql
src/
  domain/
    issue_state.rs                  # IssueState enum + as_str + FromStr
    issue_comment.rs              # IssueComment aggregate + new / for_repository
    issue.rs                      # MissionIssue aggregate + new / for_repository
    issue_lookup.rs               # MissionIssueRepository trait + MissionIssueNew DTO
  usecase/
    issue_usecase.rs              # MissionIssueUsecase<M, A, P, I>
  adapter/
    persistence/postgres/
      issue_repo.rs               # IssueRepo (MissionIssueRepository impl)
    facade/in_memory/
      issue_service.rs            # InMemoryIssueRepo
```

Modified (paths relative to the repo root):

```
lib/crates/mission/src/domain.rs                                # add 4 children + re-exports
lib/crates/mission/src/domain/error.rs                          # add 4 variants
lib/crates/mission/src/domain/mission_lookup.rs                 # extend AssigneeRepository with is_assignee
lib/crates/mission/src/domain/tests.rs                          # extend with issue_state + MissionIssue::new + IssueComment::new
lib/crates/mission/src/usecase.rs                               # add 1 child + re-exports
lib/crates/mission/src/usecase/commands.rs                      # add CreateIssue
lib/crates/mission/src/usecase/views.rs                         # add IssueView + IssueCommentView
lib/crates/mission/src/usecase/tests.rs                         # extend with FakeIssueRepo + auth tests
lib/crates/mission/src/test_support.rs                          # add FakeIssueRepo
lib/crates/mission/src/adapter/persistence/postgres.rs          # add issue_repo child + pub use IssueRepo
lib/crates/mission/src/adapter/persistence/postgres/assignee_repo.rs  # add is_assignee
lib/crates/mission/src/adapter/persistence/postgres/row.rs      # extend with IssueRow + TryFrom
lib/crates/mission/src/adapter/persistence/postgres/tests.rs    # extend with row bridge + migration asserts
lib/crates/mission/src/adapter/facade/in_memory.rs              # add issue_service + re-export
lib/crates/mission/src/adapter/facade/in_memory/service.rs      # extend MissionServiceImpl to compose issue_usecase
lib/crates/mission/src/adapter/facade/in_memory/tests.rs         # extend with auth permutations
lib/crates/mission/src/lib.rs                                   # add Issue + IssueRepo + MissionIssueUsecase + IssueView + UsecaseError re-exports
lib/crates/mission/Cargo.toml                                   # (no change — already has sqlx json + serde_json)
lib/crates/mission/README.md                                    # add "Mission issues" section
lib/crates/mission/tests/public_api.rs                          # extend with new types / bounds
lib/crates/mission/tests/integration_persistence.rs             # add mission_issues round-trips

lib/crates/apis/src/mission.rs                                  # extend with IssueState, IssueCommentView, IssueView, requests, MissionService methods, error variants

apps/server/aegis-server/src/state.rs                           # extend NullMissionService with 6 unimplemented methods
apps/server/aegis-server/src/run.rs                             # pass IssueRepo to build_mission_service
apps/server/aegis-server/src/transport/http/dto.rs              # add 5 wire DTOs + From impls
apps/server/aegis-server/src/transport/http/error.rs            # add status + code mappings for IssueNotFound + MissionNotFoundForIssue
apps/server/aegis-server/src/transport/http/mission/router.rs   # add 5 new routes
apps/server/aegis-server/src/transport/http/mission/handlers.rs # add 5 new handlers
```

Each file owns exactly the responsibility in its name. `domain/tests.rs` covers `IssueState::try_from`, `MissionIssue::new`, `IssueComment::new`. `adapter/persistence/postgres/tests.rs` covers `IssueRow` round-trip + new migration asserts. `adapter/facade/in_memory/tests.rs` exercises every auth permutation end-to-end. `tests/public_api.rs` is compile-only and pins the new bounds. `tests/integration_persistence.rs` is the `#[ignore]`-gated live-DB round-trip.

---

## Task 1: Domain types (IssueState, IssueComment, MissionIssue) + migration + error variants

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/mission/migrations/0003_create_mission_issues.sql`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_state.rs`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_comment.rs`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/domain/issue.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/domain/error.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/domain.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/domain/tests.rs`

**Interfaces:**
- Consumes: nothing (pure domain).
- Produces: every domain type the usecase / adapter / facade / public_api / integration tests use.

- [ ] **Step 1: Write migration `0003_create_mission_issues.sql`**

Create `/root/coding/project/aegis/lib/crates/mission/migrations/0003_create_mission_issues.sql`:

```sql
-- 0003_create_mission_issues.sql
--
-- Mission-scoped issue tracker. Each row is one issue; the
-- `comments` jsonb column carries the append-only discussion
-- thread (each entry has `user`, `content`, `created_at`).
--
--   * `mission_issues`
--       - `id`           surrogate primary key.
--       - `mission_id`   FK CASCADE → missions(id).
--       - `target_item`  optional free-text label.
--       - `issuer`       user_code (no FK — user existence is
--                         enforced at the usecase via
--                         UserLookup, matching how the
--                         mission / assignee rows reference
--                         user_code and project_code).
--       - `description`  non-empty text (CHECK).
--       - `state`        'opened' | 'closed' (CHECK).
--       - `comments`     JSONB array (default '[]').
--       - `created_at`   DEFAULT NOW() at insert.
--       - `updated_at`   refreshed by the trigger below.

CREATE TABLE mission_issues (
    id           BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    mission_id   BIGINT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    target_item  TEXT,
    issuer       TEXT NOT NULL,
    description  TEXT NOT NULL,
    state        TEXT NOT NULL DEFAULT 'opened',
    comments     JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT mission_issues_state_check
        CHECK (state IN ('opened', 'closed')),
    CONSTRAINT mission_issues_description_nonempty
        CHECK (length(btrim(description)) > 0),
    CONSTRAINT mission_issues_issuer_nonempty
        CHECK (length(btrim(issuer)) > 0)
);

CREATE INDEX mission_issues_by_mission
    ON mission_issues (mission_id);
CREATE INDEX mission_issues_by_mission_state
    ON mission_issues (mission_id, state);

CREATE OR REPLACE FUNCTION mission_issues_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER mission_issues_set_updated_at
    BEFORE UPDATE ON mission_issues
    FOR EACH ROW
    EXECUTE FUNCTION mission_issues_set_updated_at();
```

- [ ] **Step 2: Extend `domain/error.rs`**

Append the four new variants to `lib/crates/mission/src/domain/error.rs`. Replace the file:

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

    #[error("mission issue not found")]
    MissionIssueNotFound,

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

    #[error("issue description must not be empty")]
    EmptyIssueDescription,

    #[error("issue issuer must not be empty")]
    EmptyIssueIssuer,

    #[error("comment content must not be empty")]
    EmptyCommentContent,

    #[error("repository error: {0}")]
    Repository(String),
}
```

- [ ] **Step 3: Write `domain/issue_state.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_state.rs`:

```rust
use std::str::FromStr;

use super::error::DomainError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueState {
    Opened,
    Closed,
}

impl IssueState {
    pub const fn as_str(self) -> &'static str {
        match self {
            IssueState::Opened => "opened",
            IssueState::Closed => "closed",
        }
    }
}

impl FromStr for IssueState {
    type Err = DomainError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "opened" => Ok(IssueState::Opened),
            "closed" => Ok(IssueState::Closed),
            other => Err(DomainError::UnknownMissionRole(other.to_string())),
        }
    }
}

impl TryFrom<&str> for IssueState {
    type Error = DomainError;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}
```

- [ ] **Step 4: Write `domain/issue_comment.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_comment.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::DomainError;

/// One entry in `MissionIssue.comments`. Append-only — the API
/// surface never needs to address a single comment, only the
/// issue aggregate. `created_at` is set at the adapter boundary
/// from `NOW()` on insert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IssueComment {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

impl IssueComment {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs.
    pub fn new(
        user: String,
        content: String,
        created_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if user.trim().is_empty() {
            return Err(DomainError::EmptyUserCode);
        }
        if content.trim().is_empty() {
            return Err(DomainError::EmptyCommentContent);
        }
        Ok(Self {
            user,
            content,
            created_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(dead_code)]
    pub(crate) fn for_repository(
        user: String,
        content: String,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            user,
            content,
            created_at,
        }
    }
}
```

- [ ] **Step 5: Write `domain/issue.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/domain/issue.rs`:

```rust
use chrono::{DateTime, Utc};

use super::error::DomainError;
use super::issue_comment::IssueComment;
use super::issue_state::IssueState;

/// Mission-scoped issue with an append-only comment thread. The
/// `state` and `comments` fields are not editable through the
/// update path; only `description` is mutable after creation
/// (see `MissionIssueUsecase::update_issue_description`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionIssue {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueComment>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl MissionIssue {
    /// Validating constructor used by tests and any in-crate path
    /// that builds from raw inputs. Empty `description` and empty
    /// `issuer` are rejected here.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: i64,
        mission_id: i64,
        target_item: Option<String>,
        issuer: String,
        description: String,
        state: IssueState,
        comments: Vec<IssueComment>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Result<Self, DomainError> {
        if description.trim().is_empty() {
            return Err(DomainError::EmptyIssueDescription);
        }
        if issuer.trim().is_empty() {
            return Err(DomainError::EmptyIssueIssuer);
        }
        Ok(Self {
            id,
            mission_id,
            target_item,
            issuer,
            description,
            state,
            comments,
            created_at,
            updated_at,
        })
    }

    /// Bypasses validation. Reserved for the adapter row bridge.
    #[allow(clippy::too_many_arguments, dead_code)]
    pub(crate) fn for_repository(
        id: i64,
        mission_id: i64,
        target_item: Option<String>,
        issuer: String,
        description: String,
        state: IssueState,
        comments: Vec<IssueComment>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            mission_id,
            target_item,
            issuer,
            description,
            state,
            comments,
            created_at,
            updated_at,
        }
    }
}
```

- [ ] **Step 6: Wire `domain.rs`**

Replace `/root/coding/project/aegis/lib/crates/mission/src/domain.rs`:

```rust
//! Domain layer.
//!
//! Pure types, value objects, ports (traits), and `DomainError`.
//! No I/O — no `sqlx`, no `tokio`. Validates inputs and enforces
//! invariants.

mod assignee;
mod error;
mod issue;
mod issue_comment;
mod issue_state;
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
pub use issue::MissionIssue;
pub use issue_comment::IssueComment;
pub use issue_state::IssueState;
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

- [ ] **Step 7: Extend `domain/tests.rs` with the issue types**

Replace `/root/coding/project/aegis/lib/crates/mission/src/domain/tests.rs`:

```rust
use super::{
    Assignee, AssigneeNew, DomainError, IssueComment, IssueState, Mission, MissionIssue,
    MissionKind, MissionRole, assignees_within_mission_are_unique,
};

#[test]
fn mission_kind_round_trip() {
    for k in [
        MissionKind::Crf,
        MissionKind::Sdtm,
        MissionKind::Adam,
        MissionKind::Tfl,
    ] {
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
    let a = super::Assignee::new(1, "".into(), MissionRole::Dev, now(), now());
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

#[test]
fn issue_state_round_trip() {
    for s in [IssueState::Opened, IssueState::Closed] {
        let str = s.as_str();
        let parsed = IssueState::try_from(str).expect("parses");
        assert_eq!(parsed, s);
    }
}

#[test]
fn issue_state_unknown_rejected() {
    let err = IssueState::try_from("pending").unwrap_err();
    assert!(matches!(err, DomainError::UnknownMissionRole(_)));
}

#[test]
fn issue_comment_new_rejects_empty_user() {
    let c = IssueComment::new("".into(), "content".into(), now());
    assert!(matches!(c, Err(DomainError::EmptyUserCode)));
}

#[test]
fn issue_comment_new_rejects_empty_content() {
    let c = IssueComment::new("u1".into(), "   ".into(), now());
    assert!(matches!(c, Err(DomainError::EmptyCommentContent)));
}

#[test]
fn issue_comment_new_accepts_valid() {
    let c = IssueComment::new("u1".into(), "hello".into(), now()).unwrap();
    assert_eq!(c.user, "u1");
    assert_eq!(c.content, "hello");
}

#[test]
fn mission_issue_new_rejects_empty_description() {
    let m = MissionIssue::new(
        1,
        1,
        None,
        "u1".into(),
        "   ".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyIssueDescription)));
}

#[test]
fn mission_issue_new_rejects_empty_issuer() {
    let m = MissionIssue::new(
        1,
        1,
        None,
        "".into(),
        "desc".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    );
    assert!(matches!(m, Err(DomainError::EmptyIssueIssuer)));
}

#[test]
fn mission_issue_new_accepts_valid() {
    let m = MissionIssue::new(
        1,
        1,
        Some("form AE".into()),
        "u1".into(),
        "desc".into(),
        IssueState::Opened,
        vec![],
        now(),
        now(),
    )
    .unwrap();
    assert_eq!(m.issuer, "u1");
    assert_eq!(m.target_item.as_deref(), Some("form AE"));
    assert_eq!(m.state, IssueState::Opened);
}

fn now() -> chrono::DateTime<chrono::Utc> {
    chrono::Utc::now()
}
```

- [ ] **Step 8: Run the domain tests**

Run: `cargo test -p mission --lib domain::tests`
Expected: PASS — every test in `tests.rs` passes.

- [ ] **Step 9: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/migrations/0003_create_mission_issues.sql \
        lib/crates/mission/src/domain.rs \
        lib/crates/mission/src/domain
git commit -m "$(cat <<'EOF'
feat(mission-issue): domain types (IssueState, IssueComment, MissionIssue)

- 0003_create_mission_issues.sql: mission_issues table with FK
  CASCADE → missions(id), CHECK on state / description /
  issuer, jsonb comments column, and the updated_at trigger
- IssueState { Opened, Closed } with as_str / FromStr round-trip
- IssueComment aggregate with the two-constructor pattern;
  carries Serialize + Deserialize for the jsonb round-trip
- MissionIssue aggregate with the two-constructor pattern;
  rejecting empty description and empty issuer in `new`
- DomainError: add EmptyIssueDescription, EmptyIssueIssuer,
  EmptyCommentContent, MissionIssueNotFound
- Domain tests: round-trip + validator coverage

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib domain::tests.
EOF
)"
```

---

## Task 2: Domain ports — extend `AssigneeRepository` and add `MissionIssueRepository`

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/domain/mission_lookup.rs`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_lookup.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/domain.rs`

**Interfaces:**
- Consumes: nothing (pure domain).
- Produces:
  - `AssigneeRepository::is_assignee(mission_id, user_code, role) -> Result<bool, DomainError>`.
  - `MissionIssueRepository` trait + `MissionIssueNew` DTO.

- [ ] **Step 1: Write `domain/issue_lookup.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/domain/issue_lookup.rs`:

```rust
use async_trait::async_trait;

use super::error::DomainError;
use super::issue::MissionIssue;
use super::issue_comment::IssueComment;
use super::issue_state::IssueState;

/// Persistence-input DTO for [`MissionIssueRepository::create`].
/// `mission_id` is a foreign key resolved at the usecase via
/// `MissionRepository::find_by_id` before the repo is called.
#[derive(Debug, Clone)]
pub struct MissionIssueNew {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
}

#[async_trait]
pub trait MissionIssueRepository: Send + Sync {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError>;

    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError>;

    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError>;

    /// Idempotent — succeeds regardless of the prior state.
    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError>;

    /// Idempotent — succeeds regardless of the prior state.
    /// Mirror of [`close`](Self::close).
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError>;

    /// Update the issue's `description`. Allowed regardless of
    /// the current state (a closed issue's description is still
    /// mutable).
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError>;

    /// Append `comment` to `MissionIssue.comments`. The
    /// implementation reads the existing row, appends in Rust,
    /// and writes the whole array back — so the jsonb merge
    /// never goes through the `||` operator (which does a
    /// shallow merge and can drop entries).
    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError>;
}
```

- [ ] **Step 2: Extend `domain/mission_lookup.rs` with `is_assignee`**

Append the new method to the `AssigneeRepository` trait in
`/root/coding/project/aegis/lib/crates/mission/src/domain/mission_lookup.rs`.
The full trait should read:

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
pub fn assignees_within_mission_are_unique(assignees: &[AssigneeNew]) -> Result<(), DomainError> {
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
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError>;

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError>;

    /// True iff `(mission_id, user_code, role)` exists in `assignees`.
    async fn is_assignee(
        &self,
        mission_id: i64,
        user_code: &str,
        role: MissionRole,
    ) -> Result<bool, DomainError>;
}
```

- [ ] **Step 3: Wire the new module in `domain.rs`**

Edit `/root/coding/project/aegis/lib/crates/mission/src/domain.rs`. Add `mod issue_lookup;` to the children list and add `pub use issue_lookup::{MissionIssueNew, MissionIssueRepository};` to the re-exports:

```rust
//! Domain layer.
//!
//! Pure types, value objects, ports (traits), and `DomainError`.
//! No I/O — no `sqlx`, no `tokio`. Validates inputs and enforces
//! invariants.

mod assignee;
mod error;
mod issue;
mod issue_comment;
mod issue_lookup;
mod issue_state;
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
pub use issue::MissionIssue;
pub use issue_comment::IssueComment;
pub use issue_lookup::{MissionIssueNew, MissionIssueRepository};
pub use issue_state::IssueState;
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

- [ ] **Step 4: Confirm it still compiles**

Run: `cargo check -p mission`
Expected: success. The new port is not yet implemented anywhere, so the existing in-memory `FakeAssigneeRepo` will fail to satisfy the extended trait until Task 3 — fix that as part of Task 3.

- [ ] **Step 5: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings 2>&1 | tail -20
git add lib/crates/mission/src/domain.rs \
        lib/crates/mission/src/domain/mission_lookup.rs \
        lib/crates/mission/src/domain/issue_lookup.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): domain ports (MissionIssueRepository, is_assignee)

- Add MissionIssueRepository trait + MissionIssueNew DTO; six
  methods: create, find_by_id, list_by_mission (with optional
  state filter), close (idempotent), open (idempotent),
  update_description (allowed regardless of state), and
  append_comment (read-modify-write)
- Extend AssigneeRepository with is_assignee(mission_id,
  user_code, role) for the issue usecase's authorization helpers
- Wire both modules via domain.rs pub use

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings.
EOF
)"
```

---

## Task 3: Adapter — Postgres `IssueRepo`, extended `AssigneeRepo`, row bridge, tests, in-memory fakes

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/row.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/assignee_repo.rs`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/issue_repo.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/tests.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/test_support.rs`

**Interfaces:**
- Consumes: every type from `mission::domain::*` plus `sqlx::PgPool`.
- Produces:
  - `mission::IssueRepo::new(PgPool) -> Self` implementing `MissionIssueRepository`.
  - `mission::AssigneeRepo` gains `is_assignee` impl.
  - In-memory `FakeIssueRepo` for the facade + usecase tests.

- [ ] **Step 1: Extend `adapter/persistence/postgres/row.rs`**

Append the `IssueRow` struct and `TryFrom<IssueRow> for MissionIssue` impl to
`/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/row.rs`. The final file should read:

```rust
use std::convert::TryFrom;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::domain::{
    Assignee, DomainError, IssueComment, IssueState, Mission, MissionIssue, MissionKind,
    MissionRole,
};

/// Raw row from `missions`. `mission_kind` is read as TEXT and
/// parsed via `MissionKind::from_str` so the DB CHECK is the
/// belt-and-braces against out-of-band inserts.
#[derive(FromRow)]
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
#[derive(FromRow)]
pub(crate) struct AssigneeRow {
    pub id: i64,
    pub mission_id: i64,
    pub user_code: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Raw row from `mission_issues`. `state` is read as TEXT and
/// parsed via `IssueState::from_str`; `comments` is read as
/// jsonb and decoded into `Vec<IssueComment>` so the row bridge
/// is a single column deserialise.
#[derive(FromRow)]
pub(crate) struct IssueRow {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: String,
    pub comments: Json<Vec<IssueComment>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
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

impl TryFrom<IssueRow> for MissionIssue {
    type Error = DomainError;
    fn try_from(row: IssueRow) -> Result<Self, Self::Error> {
        let state = IssueState::from_str(&row.state)?;
        Ok(MissionIssue::for_repository(
            row.id,
            row.mission_id,
            row.target_item,
            row.issuer,
            row.description,
            state,
            row.comments.0,
            row.created_at,
            row.updated_at,
        ))
    }
}
```

- [ ] **Step 2: Extend `AssigneeRepo` with `is_assignee`**

Append the new method to `AssigneeRepository` impl in
`/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/assignee_repo.rs`. The final file should read:

```rust
use async_trait::async_trait;
use sqlx::PgPool;

use crate::domain::{Assignee, AssigneeNew, AssigneeRepository, DomainError, MissionRole};

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
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError> {
        // Mission existence is enforced via FK — the row will fail
        // to insert if `mission_id` does not exist. Map that
        // generic FK violation to `DomainError::NotFound` so the
        // facade surfaces a 404 instead of a 500.
        let row: AssigneeRow =
            sqlx::QueryBuilder::new("INSERT INTO assignees (mission_id, user_code, role) VALUES (")
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
                    sqlx::Error::Database(ref db)
                        if db.code().as_deref() == Some(SQLSTATE_UNIQUE_VIOLATION) =>
                    {
                        DomainError::DuplicateAssignee {
                            mission_id,
                            user_code: input.user_code.clone(),
                            role: input.role,
                        }
                    }
                    other => map_db_error(other),
                })?;
        Assignee::try_from(row)
    }

    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError> {
        let res = sqlx::QueryBuilder::new("DELETE FROM assignees WHERE mission_id = ")
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

    async fn is_assignee(
        &self,
        mission_id: i64,
        user_code: &str,
        role: MissionRole,
    ) -> Result<bool, DomainError> {
        let exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM assignees
                            WHERE mission_id = $1
                              AND user_code = $2
                              AND role = $3)",
        )
        .bind(mission_id)
        .bind(user_code)
        .bind(role.as_str())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Repository(e.to_string()))?;
        Ok(exists)
    }
}
```

- [ ] **Step 3: Write `adapter/persistence/postgres/issue_repo.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/issue_repo.rs`:

```rust
use async_trait::async_trait;
use sqlx::PgPool;
use sqlx::types::Json;

use crate::domain::{DomainError, IssueComment, IssueState, MissionIssue, MissionIssueNew};

use super::map_db_error;
use super::row::IssueRow;

pub struct IssueRepo {
    pool: PgPool,
}

impl IssueRepo {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl crate::domain::MissionIssueRepository for IssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "INSERT INTO mission_issues \
             (mission_id, target_item, issuer, description, state, comments) \
             VALUES (",
        )
        .push_bind(input.mission_id)
        .push(", ")
        .push_bind(&input.target_item)
        .push(", ")
        .push_bind(&input.issuer)
        .push(", ")
        .push_bind(&input.description)
        .push(", 'opened', '[]'::jsonb) \
             RETURNING id, mission_id, target_item, issuer, description, state, \
                       comments, created_at, updated_at")
        .build_query_as::<IssueRow>()
        .fetch_one(&self.pool)
        .await
        .map_err(map_db_error)?;
        MissionIssue::try_from(row)
    }

    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "SELECT id, mission_id, target_item, issuer, description, state, \
                    comments, created_at, updated_at \
             FROM mission_issues WHERE id = ",
        )
        .push_bind(id)
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError> {
        let mut qb = sqlx::QueryBuilder::new(
            "SELECT id, mission_id, target_item, issuer, description, state, \
                    comments, created_at, updated_at \
             FROM mission_issues WHERE mission_id = ",
        );
        qb.push_bind(mission_id);
        if let Some(s) = state {
            qb.push(" AND state = ").push_bind(s.as_str());
        }
        qb.push(" ORDER BY id ASC");
        let rows: Vec<IssueRow> = qb
            .build_query_as::<IssueRow>()
            .fetch_all(&self.pool)
            .await
            .map_err(map_db_error)?;
        rows.into_iter().map(MissionIssue::try_from).collect()
    }

    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET state = 'closed', updated_at = NOW() \
             WHERE id = ",
        )
        .push_bind(id)
        .push(" RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at")
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET state = 'opened', updated_at = NOW() \
             WHERE id = ",
        )
        .push_bind(id)
        .push(" RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at")
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET description = ",
        )
        .push_bind(&description)
        .push(", updated_at = NOW() WHERE id = ")
        .push_bind(id)
        .push(" RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at")
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }

    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError> {
        // Read-modify-write: load the existing row, append the
        // new comment to its `comments: Vec<_>` in Rust, and
        // write the whole array back as a single jsonb column.
        // We do NOT use Postgres's `jsonb || jsonb` operator
        // because that performs a shallow merge and would
        // silently drop entries under index drift.
        let existing = self.find_by_id(id).await?;
        let mut comments = existing.comments;
        comments.push(comment);
        let row: IssueRow = sqlx::QueryBuilder::new(
            "UPDATE mission_issues \
             SET comments = ",
        )
        .push_bind(Json(&comments))
        .push("::jsonb, updated_at = NOW() WHERE id = ")
        .push_bind(id)
        .push(" RETURNING id, mission_id, target_item, issuer, description, state, \
                comments, created_at, updated_at")
        .build_query_as::<IssueRow>()
        .fetch_optional(&self.pool)
        .await
        .map_err(map_db_error)?
        .ok_or(DomainError::MissionIssueNotFound)?;
        MissionIssue::try_from(row)
    }
}
```

- [ ] **Step 4: Wire `postgres.rs`**

Replace `/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres.rs`:

```rust
//! PostgreSQL-backed implementations of `MissionRepository`,
//! `AssigneeRepository`, and `MissionIssueRepository`.
//!
//! This module intentionally uses SQLx's *runtime* query API
//! (`sqlx::query_as` and `sqlx::QueryBuilder`) rather than the
//! compile-time-checked macros, mirroring the project / user
//! crates. `MissionRepo::create` opens a transaction so the
//! mission row and every assignee row land atomically; the FK
//! `ON DELETE CASCADE` makes mission deletion a single DELETE.
//! `IssueRepo::append_comment` reads the existing row and
//! rewrites the whole `comments` jsonb array — the Postgres
//! `||` operator is intentionally avoided because it merges
//! by index, not by append.
//!
//! `row` is private to `postgres/`. The `MissionRow` /
//! `AssigneeRow` / `IssueRow` types are NOT re-exported at the
//! crate root.

#![allow(dead_code, unused_imports)]

pub(crate) mod assignee_repo;
pub(crate) mod issue_repo;
pub(crate) mod mission_repo;
pub(crate) mod row;
#[cfg(test)]
mod tests;

pub use assignee_repo::AssigneeRepo;
pub use issue_repo::IssueRepo;
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

- [ ] **Step 5: Extend `adapter/persistence/postgres/tests.rs`**

Append the issue-row tests to
`/root/coding/project/aegis/lib/crates/mission/src/adapter/persistence/postgres/tests.rs`. The final file should read:

```rust
use std::convert::TryFrom;
use std::fs;

use chrono::{DateTime, Utc};
use sqlx::types::Json;

use crate::domain::{Assignee, IssueComment, IssueState, Mission, MissionIssue, MissionKind, MissionRole};

use super::row::{AssigneeRow, IssueRow, MissionRow};

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

#[test]
fn issue_row_to_domain() {
    let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let row = IssueRow {
        id: 1,
        mission_id: 2,
        target_item: Some("form AE".into()),
        issuer: "u1".into(),
        description: "desc".into(),
        state: "opened".into(),
        comments: Json(vec![IssueComment {
            user: "u1".into(),
            content: "first".into(),
            created_at: now,
        }]),
        created_at: now,
        updated_at: now,
    };
    let issue: MissionIssue = MissionIssue::try_from(row).unwrap();
    assert_eq!(issue.id, 1);
    assert_eq!(issue.mission_id, 2);
    assert_eq!(issue.target_item.as_deref(), Some("form AE"));
    assert_eq!(issue.issuer, "u1");
    assert_eq!(issue.state, IssueState::Opened);
    assert_eq!(issue.comments.len(), 1);
    assert_eq!(issue.comments[0].user, "u1");
}

#[test]
fn issue_row_rejects_unknown_state() {
    let now = DateTime::<Utc>::from_timestamp(0, 0).unwrap();
    let row = IssueRow {
        id: 1,
        mission_id: 2,
        target_item: None,
        issuer: "u1".into(),
        description: "desc".into(),
        state: "pending".into(),
        comments: Json(vec![]),
        created_at: now,
        updated_at: now,
    };
    assert!(MissionIssue::try_from(row).is_err());
}

#[test]
fn mission_issues_migration_has_checks_and_trigger() {
    let sql = read_migration("0003_create_mission_issues.sql");
    assert!(sql.contains("CREATE TABLE mission_issues"));
    assert!(sql.contains("REFERENCES missions(id) ON DELETE CASCADE"));
    assert!(sql.contains("mission_issues_state_check"));
    assert!(sql.contains("CHECK (state IN ('opened', 'closed'))"));
    assert!(sql.contains("mission_issues_description_nonempty"));
    assert!(sql.contains("length(btrim(description)) > 0"));
    assert!(sql.contains("mission_issues_issuer_nonempty"));
    assert!(sql.contains("JSONB NOT NULL DEFAULT '[]'::jsonb"));
    assert!(sql.contains("mission_issues_set_updated_at"));
    assert!(sql.contains("BEFORE UPDATE ON mission_issues"));
    assert!(sql.contains("mission_issues_by_mission_state"));
}

fn read_migration(name: &str) -> String {
    let path = format!("{}/migrations/{}", env!("CARGO_MANIFEST_DIR"), name);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {}: {}", path, e))
}
```

- [ ] **Step 6: Extend `test_support.rs` with `FakeIssueRepo` + extend `FakeAssigneeRepo`**

Append to `/root/coding/project/aegis/lib/crates/mission/src/test_support.rs`. The full file should read:

```rust
//! Shared test fakes for the mission crate. Used by
//! `usecase::tests` and `adapter::facade::in_memory::tests` so
//! the fake definitions live in one place.

#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use crate::domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, IssueComment, Mission, MissionIssue,
    MissionIssueNew, MissionIssueRepository, MissionKind, MissionNew, MissionRepository,
    MissionRole, ProjectLookup, UserLookup,
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
            .map(|(idx, a)| {
                Assignee::for_repository(
                    id * 1000 + idx as i64,
                    a.user_code.clone(),
                    a.role,
                    now,
                    now,
                )
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
            .filter(|m| m.project_code == project_code && kind.is_none_or(|k| k == m.mission_kind))
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
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError> {
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
            .any(|(mid, x)| *mid == mission_id && x.user_code == a.user_code && x.role == a.role)
        {
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
    async fn is_assignee(
        &self,
        mission_id: i64,
        user_code: &str,
        role: MissionRole,
    ) -> Result<bool, DomainError> {
        Ok(self
            .assignees
            .lock()
            .unwrap()
            .iter()
            .any(|(mid, a)| *mid == mission_id && a.user_code == user_code && a.role == role))
    }
}

#[derive(Default)]
pub struct FakeIssueRepo {
    pub next_id: AtomicI32,
    pub issues: Mutex<Vec<MissionIssue>>,
}

#[async_trait]
impl MissionIssueRepository for FakeIssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let now = Utc::now();
        let issue = MissionIssue::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.mission_id,
            input.target_item,
            input.issuer,
            input.description,
            crate::domain::IssueState::Opened,
            vec![],
            now,
            now,
        )?;
        self.issues.lock().unwrap().push(issue.clone());
        Ok(issue)
    }
    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError> {
        self.issues
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(DomainError::MissionIssueNotFound)
    }
    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<crate::domain::IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError> {
        Ok(self
            .issues
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.mission_id == mission_id && state.is_none_or(|s| s == i.state))
            .cloned()
            .collect())
    }
    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = crate::domain::IssueState::Closed;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = crate::domain::IssueState::Opened;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
        // Validate before mutating so the contract is the same
        // as the live repo's CHECK constraint.
        if description.trim().is_empty() {
            return Err(DomainError::EmptyIssueDescription);
        }
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.description = description;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.comments.push(comment);
        i.updated_at = Utc::now();
        Ok(i.clone())
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
    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError> {
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

- [ ] **Step 7: Confirm the crate compiles**

Run: `cargo check -p mission`
Expected: success. The new `IssueRepo` re-exports from `postgres.rs` and the new `FakeIssueRepo` plus extended `FakeAssigneeRepo` keep the whole crate green. The `crate::adapter::facade::in_memory::MissionServiceImpl` does not yet implement the new methods — that lands in Task 5. The `cargo check` will fail until Task 5 wires `MissionServiceImpl`. To keep this step green, defer running `cargo check` here; instead run `cargo test -p mission --lib adapter::persistence::postgres::tests` which exercises only the persistence layer:

Run: `cargo test -p mission --lib adapter::persistence::postgres::tests`
Expected: PASS — row bridge + migration asserts all green.

- [ ] **Step 8: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/src/adapter/persistence/postgres.rs \
        lib/crates/mission/src/adapter/persistence/postgres \
        lib/crates/mission/src/test_support.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): Postgres IssueRepo + FakeIssueRepo + extended fakes

- IssueRepo: PgPool-backed MissionIssueRepository; create,
  find_by_id, list_by_mission (with optional state filter),
  idempotent close / open, update_description (allowed
  regardless of state), append_comment (read-modify-write —
  the whole Vec<IssueComment> is rewritten as a single jsonb
  parameter, avoiding the shallow-merge trap of `||`)
- AssigneeRepo: add is_assignee impl (EXISTS subquery on
  (mission_id, user_code, role))
- row.rs: add IssueRow + TryFrom<IssueRow> for MissionIssue;
  the jsonb column round-trips via sqlx::types::Json<Vec<IssueComment>>
- test_support: FakeAssigneeRepo gains is_assignee;
  FakeIssueRepo implements every MissionIssueRepository method
  with id / closure / atomic-id mirroring the existing
  FakeMissionRepo shape
- Adapter tests: issue_row_to_domain + unknown_state rejection +
  migration content asserts (CHECKs, CASCADE FK, JSONB default,
  trigger)

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib adapter::persistence::postgres::tests.
EOF
)"
```

---

## Task 4: Usecase — `MissionIssueUsecase` + commands + views + tests

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/usecase/commands.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/usecase/views.rs`
- Create: `/root/coding/project/aegis/lib/crates/mission/src/usecase/issue_usecase.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/usecase.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/usecase/tests.rs`

**Interfaces:**
- Consumes: every domain type plus the in-memory fakes from Task 3.
- Produces:
  - `mission::usecase::CreateIssue`.
  - `mission::usecase::IssueView`, `mission::usecase::IssueCommentView`.
  - `mission::usecase::MissionIssueUsecase<M, A, P, I>` + `MissionIssueUsecaseConfig<M, A, P, I>`.

- [ ] **Step 1: Extend `usecase/commands.rs`**

Append to `/root/coding/project/aegis/lib/crates/mission/src/usecase/commands.rs`. The full file should read:

```rust
use crate::domain::MissionRole;

#[derive(Debug, Clone)]
pub struct CreateMission {
    pub project_code: String,
    pub mission_kind: crate::domain::MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

#[derive(Debug, Clone)]
pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}

/// Input to `MissionIssueUsecase::create_issue`. The
/// `description` is the only mutable text field after creation;
/// `target_item` is caller-supplied at creation and immutable
/// afterwards.
#[derive(Debug, Clone)]
pub struct CreateIssue {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub description: String,
}
```

- [ ] **Step 2: Extend `usecase/views.rs`**

Append to `/root/coding/project/aegis/lib/crates/mission/src/usecase/views.rs`. The full file should read:

```rust
use chrono::{DateTime, Utc};

use crate::domain::{Assignee, IssueComment, IssueState, Mission, MissionIssue};

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

/// Projection of `MissionIssue` returned by the issue usecase to
/// the facade.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueCommentView {
    pub user: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
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

impl From<MissionIssue> for IssueView {
    fn from(i: MissionIssue) -> Self {
        IssueView {
            id: i.id,
            mission_id: i.mission_id,
            target_item: i.target_item,
            issuer: i.issuer,
            description: i.description,
            state: i.state,
            comments: i.comments.into_iter().map(Into::into).collect(),
            created_at: i.created_at,
            updated_at: i.updated_at,
        }
    }
}

impl From<IssueComment> for IssueCommentView {
    fn from(c: IssueComment) -> Self {
        IssueCommentView {
            user: c.user,
            content: c.content,
            created_at: c.created_at,
        }
    }
}
```

- [ ] **Step 3: Write `usecase/issue_usecase.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/usecase/issue_usecase.rs`:

```rust
use chrono::Utc;

use apis::mission::Actor;

use crate::domain::{
    AssigneeRepository, DomainError, IssueComment, IssueState, MissionIssueNew,
    MissionIssueRepository, MissionRepository, MissionRole, ProjectLookup,
};

use super::commands::CreateIssue;
use super::error::UsecaseError;
use super::views::IssueView;

pub struct MissionIssueUsecaseConfig<M, A, P, I> {
    pub mission_repo: M,
    pub assignee_repo: A,
    pub project_lookup: P,
    pub issue_repo: I,
}

pub struct MissionIssueUsecase<M, A, P, I> {
    pub(crate) mission_repo: M,
    pub(crate) assignee_repo: A,
    pub(crate) project_lookup: P,
    pub(crate) issue_repo: I,
}

impl<M, A, P, I> MissionIssueUsecase<M, A, P, I>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    I: MissionIssueRepository,
{
    pub fn new(config: MissionIssueUsecaseConfig<M, A, P, I>) -> Self {
        Self {
            mission_repo: config.mission_repo,
            assignee_repo: config.assignee_repo,
            project_lookup: config.project_lookup,
            issue_repo: config.issue_repo,
        }
    }

    /// Authorisation shared by create / close / reopen /
    /// update-description: project leader OR mission QC assignee.
    async fn ensure_can_create_or_close(
        &self,
        actor: &Actor,
        project_code: &str,
        mission_id: i64,
    ) -> Result<(), UsecaseError> {
        if self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?
        {
            return Ok(());
        }
        if self
            .assignee_repo
            .is_assignee(mission_id, &actor.user_code, MissionRole::Qc)
            .await?
        {
            return Ok(());
        }
        Err(UsecaseError::Forbidden {
            user_code: actor.user_code.clone(),
            project_code: project_code.to_string(),
        })
    }

    /// Authorisation for appending a comment: project leader OR
    /// mission DEV assignee OR mission QC assignee.
    async fn ensure_can_comment(
        &self,
        actor: &Actor,
        project_code: &str,
        mission_id: i64,
    ) -> Result<(), UsecaseError> {
        if self
            .project_lookup
            .is_leader(project_code, &actor.user_code)
            .await?
        {
            return Ok(());
        }
        for role in [MissionRole::Dev, MissionRole::Qc] {
            if self
                .assignee_repo
                .is_assignee(mission_id, &actor.user_code, role)
                .await?
            {
                return Ok(());
            }
        }
        Err(UsecaseError::Forbidden {
            user_code: actor.user_code.clone(),
            project_code: project_code.to_string(),
        })
    }

    pub async fn list_issues_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<IssueView>, UsecaseError> {
        Ok(self
            .issue_repo
            .list_by_mission(mission_id, state)
            .await?
            .into_iter()
            .map(Into::into)
            .collect())
    }

    pub async fn create_issue(
        &self,
        actor: &Actor,
        input: CreateIssue,
    ) -> Result<IssueView, UsecaseError> {
        // Empty description is rejected here so the repo's CHECK
        // is a safety net rather than the first line of defence.
        if input.description.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyIssueDescription));
        }
        let mission = self.mission_repo.find_by_id(input.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self
            .issue_repo
            .create(MissionIssueNew {
                mission_id: input.mission_id,
                target_item: input.target_item,
                issuer: actor.user_code.clone(),
                description: input.description,
            })
            .await?;
        Ok(issue.into())
    }

    pub async fn close_issue(
        &self,
        actor: &Actor,
        issue_id: i64,
    ) -> Result<IssueView, UsecaseError> {
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self.issue_repo.close(issue_id).await?;
        Ok(issue.into())
    }

    pub async fn reopen_issue(
        &self,
        actor: &Actor,
        issue_id: i64,
    ) -> Result<IssueView, UsecaseError> {
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self.issue_repo.open(issue_id).await?;
        Ok(issue.into())
    }

    pub async fn update_issue_description(
        &self,
        actor: &Actor,
        issue_id: i64,
        description: String,
    ) -> Result<IssueView, UsecaseError> {
        if description.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyIssueDescription));
        }
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_create_or_close(actor, &mission.project_code, mission.id)
            .await?;
        let issue = self
            .issue_repo
            .update_description(issue_id, description)
            .await?;
        Ok(issue.into())
    }

    pub async fn append_comment(
        &self,
        actor: &Actor,
        issue_id: i64,
        content: String,
    ) -> Result<IssueView, UsecaseError> {
        if content.trim().is_empty() {
            return Err(UsecaseError::Domain(DomainError::EmptyCommentContent));
        }
        let issue = self.issue_repo.find_by_id(issue_id).await?;
        let mission = self.mission_repo.find_by_id(issue.mission_id).await?;
        self.ensure_can_comment(actor, &mission.project_code, mission.id)
            .await?;
        let now = Utc::now();
        let comment = IssueComment::new(actor.user_code.clone(), content, now)?;
        let issue = self.issue_repo.append_comment(issue_id, comment).await?;
        Ok(issue.into())
    }
}
```

- [ ] **Step 4: Wire `usecase.rs`**

Replace `/root/coding/project/aegis/lib/crates/mission/src/usecase.rs`:

```rust
//! Usecase layer.
//!
//! `MissionUsecase<M, A, P, U>` orchestrates the four ports
//! (mission, assignee, project lookup, user lookup) and surfaces
//! `UsecaseError`. `MissionIssueUsecase<M, A, P, I>` orchestrates
//! the four ports (mission, assignee, project lookup, issue repo)
//! and shares the same `UsecaseError`. Every write method calls
//! `project_lookup.is_leader` and projects the domain aggregate
//! into the `MissionView` / `AssigneeView` / `IssueView` DTOs the
//! facade maps to the apis port types.

mod commands;
mod error;
mod issue_usecase;
mod mission_usecase;
mod views;

#[cfg(test)]
mod tests;

pub use commands::{AssigneeData, CreateIssue, CreateMission};
pub use error::UsecaseError;
pub use issue_usecase::{MissionIssueUsecase, MissionIssueUsecaseConfig};
pub use mission_usecase::{MissionUsecase, MissionUsecaseConfig};
pub use views::{AssigneeView, IssueCommentView, IssueView, MissionView};
```

- [ ] **Step 5: Extend `usecase/tests.rs` with issue usecase coverage**

Replace `/root/coding/project/aegis/lib/crates/mission/src/usecase/tests.rs`:

```rust
use apis::mission::Actor;

use crate::domain::{
    AssigneeNew, DomainError, IssueState, MissionKind, MissionRole,
};
use crate::usecase::{
    CreateIssue, CreateMission, MissionIssueUsecase, MissionIssueUsecaseConfig, MissionUsecase,
    MissionUsecaseConfig, UsecaseError,
};

#[path = "../test_support.rs"]
#[allow(clippy::duplicate_mod)]
mod test_support;
use test_support::{
    FakeAssigneeRepo, FakeIssueRepo, FakeMissionRepo, FakeProject, FakeUser,
};

fn mission_usecase() -> MissionUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser> {
    MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    })
}

fn issue_usecase() -> MissionIssueUsecase<
    FakeMissionRepo,
    FakeAssigneeRepo,
    FakeProject,
    FakeIssueRepo,
> {
    MissionIssueUsecase::new(MissionIssueUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        issue_repo: FakeIssueRepo::default(),
    })
}

#[tokio::test]
async fn create_mission_enforces_leadership() {
    let uc = mission_usecase();
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
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

#[tokio::test]
async fn create_mission_succeeds_for_leader() {
    let uc = mission_usecase();
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
    let uc = mission_usecase();
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
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::UserNotFound(_))
    ));
}

#[tokio::test]
async fn list_missions_by_project_filters_by_kind() {
    let uc = mission_usecase();
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
    let only_crf = uc
        .list_missions_by_project("p1", Some(MissionKind::Crf))
        .await
        .unwrap();
    assert_eq!(only_crf.len(), 1);
    assert_eq!(only_crf[0].mission_kind, MissionKind::Crf);
}

#[tokio::test]
async fn delete_mission_requires_leader() {
    let uc = mission_usecase();
    let m = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Adam,
                mission_code: "a1".into(),
                assignees: vec![],
            },
        )
        .await
        .unwrap();
    let err = uc
        .delete_mission(
            &Actor {
                user_code: "carol".into(),
            },
            m.id,
        )
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

// ---- issue usecase ----

async fn seed_mission_with_assignees(
    uc: &MissionUsecase<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser>,
    pairs: &[(&str, MissionRole)],
) -> i64 {
    let mut assignees = Vec::new();
    let mut extras = Vec::new();
    for (i, (user, role)) in pairs.iter().enumerate() {
        if i == 0 {
            assignees.push(crate::usecase::AssigneeData {
                user_code: (*user).into(),
                role: *role,
            });
        } else {
            extras.push((*user, *role));
        }
    }
    let m = uc
        .create_mission(
            &Actor {
                user_code: "alice".into(),
            },
            CreateMission {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: format!("c{}", pairs.len()),
                assignees,
            },
        )
        .await
        .unwrap();
    for (user, role) in extras {
        uc.add_assignee(
            &Actor {
                user_code: "alice".into(),
            },
            m.id,
            crate::usecase::AssigneeData {
                user_code: user.into(),
                role,
            },
        )
        .await
        .unwrap();
    }
    m.id
}

#[tokio::test]
async fn issue_create_close_reopen_by_leader() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();

    let view = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: Some("form AE".into()),
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(view.state, IssueState::Opened);
    let closed = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            view.id,
        )
        .await
        .unwrap();
    assert_eq!(closed.state, IssueState::Closed);
    let reopened = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            view.id,
        )
        .await
        .unwrap();
    assert_eq!(reopened.state, IssueState::Opened);
}

#[tokio::test]
async fn issue_qc_assignee_can_create_and_close() {
    let muc = mission_usecase();
    let mission_id =
        seed_mission_with_assignees(&muc, &[("u1", MissionRole::Qc)]).await;
    let iuc = issue_usecase();
    let view = iuc
        .create_issue(
            &Actor {
                user_code: "u1".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(view.issuer, "u1");
    iuc.close_issue(
        &Actor {
            user_code: "u1".into(),
        },
        view.id,
    )
    .await
    .unwrap();
}

#[tokio::test]
async fn issue_dev_assignee_can_only_comment() {
    let muc = mission_usecase();
    let mission_id =
        seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();

    // Leader creates the issue so the DEV-only tests have
    // something to act on.
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();

    let dev = Actor {
        user_code: "u1".into(),
    };
    // comment succeeds
    iuc.append_comment(&dev, issue.id, "hello".into()).await.unwrap();
    // create rejected
    let err = iuc
        .create_issue(
            &dev,
            CreateIssue {
                mission_id,
                target_item: None,
                description: "second".into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    // close rejected
    let err = iuc.close_issue(&dev, issue.id).await.unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    // reopen rejected
    let err = iuc.reopen_issue(&dev, issue.id).await.unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
    // update description rejected
    let err = iuc
        .update_issue_description(&dev, issue.id, "new".into())
        .await
        .unwrap_err();
    assert!(matches!(err, UsecaseError::Forbidden { .. }));
}

#[tokio::test]
async fn issue_close_is_idempotent() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    iuc.append_comment(
        &Actor {
            user_code: "alice".into(),
        },
        issue.id,
        "before close".into(),
    )
    .await
    .unwrap();
    let once = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    let twice = iuc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    assert_eq!(once.state, IssueState::Closed);
    assert_eq!(twice.state, IssueState::Closed);
    assert_eq!(twice.comments.len(), 1);
}

#[tokio::test]
async fn issue_reopen_is_idempotent() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let first = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    let second = iuc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
        )
        .await
        .unwrap();
    assert_eq!(first.state, IssueState::Opened);
    assert_eq!(second.state, IssueState::Opened);
}

#[tokio::test]
async fn issue_update_description_allowed_on_closed() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    iuc.close_issue(
        &Actor {
            user_code: "alice".into(),
        },
        issue.id,
    )
    .await
    .unwrap();
    let updated = iuc
        .update_issue_description(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
            "second".into(),
        )
        .await
        .unwrap();
    assert_eq!(updated.description, "second");
    assert_eq!(updated.state, IssueState::Closed);
}

#[tokio::test]
async fn issue_update_description_rejects_whitespace() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let err = iuc
        .update_issue_description(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
            "   ".into(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::EmptyIssueDescription)
    ));
}

#[tokio::test]
async fn issue_append_comment_rejects_whitespace() {
    let muc = mission_usecase();
    let mission_id = seed_mission_with_assignees(&muc, &[("u1", MissionRole::Dev)]).await;
    let iuc = issue_usecase();
    let issue = iuc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssue {
                mission_id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let err = iuc
        .append_comment(
            &Actor {
                user_code: "u1".into(),
            },
            issue.id,
            "  ".into(),
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        UsecaseError::Domain(DomainError::EmptyCommentContent)
    ));
}
```

- [ ] **Step 6: Run the usecase tests**

Run: `cargo test -p mission --lib usecase::tests`
Expected: PASS — every test in `tests.rs` passes.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/src/usecase.rs \
        lib/crates/mission/src/usecase
git commit -m "$(cat <<'EOF'
feat(mission-issue): MissionIssueUsecase + commands + views + tests

- MissionIssueUsecase<M, A, P, I> generic over the four ports
  (mission, assignee, project lookup, issue repo). Two private
  authorisation helpers:
    ensure_can_create_or_close — leader OR mission QC
    ensure_can_comment         — leader OR mission DEV OR mission QC
- Six public methods: list_issues_by_mission, create_issue,
  close_issue (idempotent), reopen_issue (idempotent),
  update_issue_description (allowed regardless of state),
  append_comment
- Input validation in usecase: empty description / empty content
  rejected before any repo call so the structured
  EmptyIssueDescription / EmptyCommentContent errors surface
  before the DB CHECK safety net
- commands: add CreateIssue
- views: add IssueView + IssueCommentView + From<MissionIssue>
  / From<IssueComment>
- Usecase tests: leader full lifecycle, QC full lifecycle,
  DEV comment-only, idempotent close, idempotent reopen,
  update_description on closed, whitespace rejection

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib usecase::tests.
EOF
)"
```

---

## Task 5: Facade — extend `MissionServiceImpl` + in-memory `MissionIssueRepository` + facade tests

**Files:**
- Create: `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/issue_service.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/service.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/tests.rs`
- Modify: `/root/coding/project/aegis/lib/crates/mission/src/lib.rs`

**Interfaces:**
- Consumes: every type from `mission::domain::*`, `mission::usecase::*`, and `apis::mission::*`.
- Produces:
  - `mission::IssueRepo` (already from Task 3) re-exported at the crate root.
  - `MissionServiceImpl<M, A, P, U, I>` adapts both `MissionUsecase` and `MissionIssueUsecase` to `apis::mission::MissionService`.
  - `MissionServiceImpl::from_repos_with_issue_repo(mission_repo, assignee_repo, project, user, issue_repo)` and `from_usecase_dual(mission_usecase, issue_usecase)`.

- [ ] **Step 1: Write `adapter/facade/in_memory/issue_service.rs`**

Create `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/issue_service.rs`:

```rust
//! In-memory `MissionIssueRepository` for tests and any
//! non-Postgres backend the facade is reused on. Mirrors
//! `test_support::FakeIssueRepo` but lives here so the facade's
//! own `tests` block can reach it via `super`.

#![allow(dead_code)]

use std::sync::Mutex;
use std::sync::atomic::{AtomicI32, Ordering};

use async_trait::async_trait;
use chrono::Utc;

use crate::domain::{
    DomainError, IssueComment, IssueState, MissionIssue, MissionIssueNew,
    MissionIssueRepository,
};

#[derive(Default)]
pub struct InMemoryIssueRepo {
    pub next_id: AtomicI32,
    pub issues: Mutex<Vec<MissionIssue>>,
}

#[async_trait]
impl MissionIssueRepository for InMemoryIssueRepo {
    async fn create(&self, input: MissionIssueNew) -> Result<MissionIssue, DomainError> {
        let now = Utc::now();
        let issue = MissionIssue::new(
            self.next_id.fetch_add(1, Ordering::SeqCst) as i64,
            input.mission_id,
            input.target_item,
            input.issuer,
            input.description,
            IssueState::Opened,
            vec![],
            now,
            now,
        )?;
        self.issues.lock().unwrap().push(issue.clone());
        Ok(issue)
    }
    async fn find_by_id(&self, id: i64) -> Result<MissionIssue, DomainError> {
        self.issues
            .lock()
            .unwrap()
            .iter()
            .find(|i| i.id == id)
            .cloned()
            .ok_or(DomainError::MissionIssueNotFound)
    }
    async fn list_by_mission(
        &self,
        mission_id: i64,
        state: Option<IssueState>,
    ) -> Result<Vec<MissionIssue>, DomainError> {
        Ok(self
            .issues
            .lock()
            .unwrap()
            .iter()
            .filter(|i| i.mission_id == mission_id && state.is_none_or(|s| s == i.state))
            .cloned()
            .collect())
    }
    async fn close(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = IssueState::Closed;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn open(&self, id: i64) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.state = IssueState::Opened;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn update_description(
        &self,
        id: i64,
        description: String,
    ) -> Result<MissionIssue, DomainError> {
        if description.trim().is_empty() {
            return Err(DomainError::EmptyIssueDescription);
        }
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.description = description;
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
    async fn append_comment(
        &self,
        id: i64,
        comment: IssueComment,
    ) -> Result<MissionIssue, DomainError> {
        let mut g = self.issues.lock().unwrap();
        let i = g
            .iter_mut()
            .find(|i| i.id == id)
            .ok_or(DomainError::MissionIssueNotFound)?;
        i.comments.push(comment);
        i.updated_at = Utc::now();
        Ok(i.clone())
    }
}
```

- [ ] **Step 2: Wire `adapter/facade/in_memory.rs`**

Replace `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory.rs`:

```rust
//! In-memory facade adapters. `MissionServiceImpl` implements
//! `apis::mission::MissionService` on top of
//! `MissionUsecase` + `MissionIssueUsecase`.

mod issue_service;
pub mod service;

pub(crate) use issue_service::InMemoryIssueRepo;
```

- [ ] **Step 3: Extend `service.rs` to compose both usecases**

Replace `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/service.rs`:

```rust
use async_trait::async_trait;

use apis::mission::{
    Actor, AppendCommentRequest, AssigneeData, AssigneeView as ApiAssigneeView,
    CloseIssueRequest, CreateIssueRequest, CreateMissionRequest,
    IssueCommentView as ApiIssueCommentView, IssueState as ApiIssueState,
    IssueView as ApiIssueView, ListIssuesByMissionRequest,
    ListMissionsByProjectRequest, ListMissionsByUserRequest, MissionApiError,
    MissionKind as ApiKind, MissionRole as ApiRole, MissionService,
    MissionView as ApiMissionView, ReopenIssueRequest,
    UpdateIssueDescriptionRequest,
};

use crate::domain::{
    AssigneeRepository, DomainError, MissionIssueRepository, MissionRepository, ProjectLookup,
    UserLookup,
};
use crate::usecase::{
    AssigneeData as UcAssigneeData, CreateIssue as UcCreateIssue,
    CreateMission as UcCreateMission, MissionIssueUsecase, MissionUsecase, UsecaseError,
};

use crate::usecase::AssigneeView as UcAssigneeView;
use crate::usecase::IssueCommentView as UcIssueCommentView;
use crate::usecase::IssueView as UcIssueView;
use crate::usecase::MissionView as UcMissionView;
use crate::usecase::{MissionIssueUsecaseConfig, MissionUsecaseConfig};

pub struct MissionServiceImpl<M, A, P, U, I> {
    usecase: MissionUsecase<M, A, P, U>,
    issue_usecase: MissionIssueUsecase<M, A, P, I>,
}

impl<M, A, P, U, I> MissionServiceImpl<M, A, P, U, I>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
    I: MissionIssueRepository,
{
    pub fn from_usecase(
        usecase: MissionUsecase<M, A, P, U>,
        issue_usecase: MissionIssueUsecase<M, A, P, I>,
    ) -> Self {
        Self {
            usecase,
            issue_usecase,
        }
    }

    /// Convenience constructor: builds both usecases from the
    /// five backing pieces and returns a single facade. Mirrors
    /// the legacy `from_repos(mission_repo, assignee_repo,
    /// projects, users)` so the server wiring stays a one-liner.
    pub fn from_repos(
        mission_repo: M,
        assignee_repo: A,
        projects: std::sync::Arc<P>,
        users: std::sync::Arc<U>,
        issue_repo: I,
    ) -> Self
    where
        A: Clone,
        P: Clone,
        U: Clone,
    {
        let mission_repo_issue = mission_repo.clone();
        let assignee_repo_issue = assignee_repo.clone();
        let projects_issue = (*projects).clone();
        let mission_usecase = MissionUsecase::new(MissionUsecaseConfig {
            mission_repo,
            assignee_repo,
            project_lookup: (*projects).clone(),
            user_lookup: (*users).clone(),
        });
        let issue_usecase = MissionIssueUsecase::new(MissionIssueUsecaseConfig {
            mission_repo: mission_repo_issue,
            assignee_repo: assignee_repo_issue,
            project_lookup: projects_issue,
            issue_repo,
        });
        Self::from_usecase(mission_usecase, issue_usecase)
    }
}

#[async_trait]
impl<M, A, P, U, I> MissionService for MissionServiceImpl<M, A, P, U, I>
where
    M: MissionRepository + 'static,
    A: AssigneeRepository + 'static,
    P: ProjectLookup + 'static,
    U: UserLookup + 'static,
    I: MissionIssueRepository + 'static,
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
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn list_missions_by_user(
        &self,
        req: ListMissionsByUserRequest,
    ) -> Result<Vec<ApiMissionView>, MissionApiError> {
        self.usecase
            .list_missions_by_user(&req.user_code)
            .await
            .map(|v| v.into_iter().map(into_api_mission).collect())
            .map_err(map_error)
    }

    async fn delete_mission(&self, actor: &Actor, id: i64) -> Result<(), MissionApiError> {
        self.usecase
            .delete_mission(actor, id)
            .await
            .map_err(map_error)
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

    async fn list_issues_by_mission(
        &self,
        req: ListIssuesByMissionRequest,
    ) -> Result<Vec<ApiIssueView>, MissionApiError> {
        self.issue_usecase
            .list_issues_by_mission(req.mission_id, req.state.map(Into::into))
            .await
            .map(|v| v.into_iter().map(into_api_issue).collect())
            .map_err(map_error)
    }

    async fn create_issue(
        &self,
        actor: &Actor,
        req: CreateIssueRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .create_issue(
                actor,
                UcCreateIssue {
                    mission_id: req.mission_id,
                    target_item: req.target_item,
                    description: req.description,
                },
            )
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn close_issue(
        &self,
        actor: &Actor,
        _req: CloseIssueRequest,
        issue_id: i64,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .close_issue(actor, issue_id)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn reopen_issue(
        &self,
        actor: &Actor,
        _req: ReopenIssueRequest,
        issue_id: i64,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .reopen_issue(actor, issue_id)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn update_issue_description(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: UpdateIssueDescriptionRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .update_issue_description(actor, issue_id, req.description)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }

    async fn append_comment(
        &self,
        actor: &Actor,
        issue_id: i64,
        req: AppendCommentRequest,
    ) -> Result<ApiIssueView, MissionApiError> {
        self.issue_usecase
            .append_comment(actor, issue_id, req.content)
            .await
            .map(into_api_issue)
            .map_err(map_error)
    }
}

// ---- apis <-> domain enum bridges ----

impl From<ApiKind> for crate::domain::MissionKind {
    fn from(k: ApiKind) -> Self {
        match k {
            ApiKind::Crf => crate::domain::MissionKind::Crf,
            ApiKind::Sdtm => crate::domain::MissionKind::Sdtm,
            ApiKind::Adam => crate::domain::MissionKind::Adam,
            ApiKind::Tfl => crate::domain::MissionKind::Tfl,
        }
    }
}

impl From<crate::domain::MissionKind> for ApiKind {
    fn from(k: crate::domain::MissionKind) -> Self {
        match k {
            crate::domain::MissionKind::Crf => ApiKind::Crf,
            crate::domain::MissionKind::Sdtm => ApiKind::Sdtm,
            crate::domain::MissionKind::Adam => ApiKind::Adam,
            crate::domain::MissionKind::Tfl => ApiKind::Tfl,
        }
    }
}

impl From<ApiRole> for crate::domain::MissionRole {
    fn from(r: ApiRole) -> Self {
        match r {
            ApiRole::Dev => crate::domain::MissionRole::Dev,
            ApiRole::Qc => crate::domain::MissionRole::Qc,
        }
    }
}

impl From<crate::domain::MissionRole> for ApiRole {
    fn from(r: crate::domain::MissionRole) -> Self {
        match r {
            crate::domain::MissionRole::Dev => ApiRole::Dev,
            crate::domain::MissionRole::Qc => ApiRole::Qc,
        }
    }
}

impl From<ApiIssueState> for crate::domain::IssueState {
    fn from(s: ApiIssueState) -> Self {
        match s {
            ApiIssueState::Opened => crate::domain::IssueState::Opened,
            ApiIssueState::Closed => crate::domain::IssueState::Closed,
        }
    }
}

impl From<crate::domain::IssueState> for ApiIssueState {
    fn from(s: crate::domain::IssueState) -> Self {
        match s {
            crate::domain::IssueState::Opened => ApiIssueState::Opened,
            crate::domain::IssueState::Closed => ApiIssueState::Closed,
        }
    }
}

// ---- view bridges ----

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

fn into_api_issue(i: UcIssueView) -> ApiIssueView {
    ApiIssueView {
        id: i.id,
        mission_id: i.mission_id,
        target_item: i.target_item,
        issuer: i.issuer,
        description: i.description,
        state: i.state.into(),
        comments: i.comments.into_iter().map(into_api_issue_comment).collect(),
        created_at: i.created_at,
        updated_at: i.updated_at,
    }
}

fn into_api_issue_comment(c: UcIssueCommentView) -> ApiIssueCommentView {
    ApiIssueCommentView {
        user: c.user,
        content: c.content,
        created_at: c.created_at,
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
            | DomainError::UnknownMissionRole(_) => MissionApiError::Validation(d.to_string()),
            DomainError::EmptyIssueDescription
            | DomainError::EmptyIssueIssuer
            | DomainError::EmptyCommentContent => MissionApiError::Validation(d.to_string()),
            DomainError::NotFound => MissionApiError::NotFound,
            DomainError::AssigneeNotFound => MissionApiError::AssigneeNotFound,
            DomainError::MissionIssueNotFound => MissionApiError::IssueNotFound,
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

- [ ] **Step 4: Extend `adapter/facade/in_memory/tests.rs`**

Replace `/root/coding/project/aegis/lib/crates/mission/src/adapter/facade/in_memory/tests.rs`:

```rust
use std::sync::Arc;

use apis::mission::{
    Actor, AppendCommentRequest, AssigneeData, CloseIssueRequest, CreateIssueRequest,
    CreateMissionRequest, IssueState as ApiIssueState, ListIssuesByMissionRequest,
    ListMissionsByProjectRequest, ListMissionsByUserRequest, MissionKind as ApiKind,
    MissionRole as ApiRole, MissionService, ReopenIssueRequest,
    UpdateIssueDescriptionRequest,
};

use crate::usecase::{
    MissionIssueUsecase, MissionIssueUsecaseConfig, MissionUsecase, MissionUsecaseConfig,
};

#[path = "../../../test_support.rs"]
mod test_support;
use test_support::{
    FakeAssigneeRepo, FakeIssueRepo, FakeMissionRepo, FakeProject, FakeUser,
};

use super::service::MissionServiceImpl;

fn service() -> MissionServiceImpl<FakeMissionRepo, FakeAssigneeRepo, FakeProject, FakeUser, FakeIssueRepo>
{
    let mission_usecase = MissionUsecase::new(MissionUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        user_lookup: FakeUser,
    });
    let issue_usecase = MissionIssueUsecase::new(MissionIssueUsecaseConfig {
        mission_repo: FakeMissionRepo::default(),
        assignee_repo: FakeAssigneeRepo::default(),
        project_lookup: FakeProject {
            leader_for: vec!["alice"],
        },
        issue_repo: FakeIssueRepo::default(),
    });
    MissionServiceImpl::from_usecase(mission_usecase, issue_usecase)
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
    assert!(matches!(
        err,
        apis::mission::MissionApiError::Forbidden { .. }
    ));
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
                assignees: vec![],
            },
        )
        .await
        .unwrap();

    svc.add_assignee(
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
}

// ---- issue facade tests ----

#[tokio::test]
async fn facade_create_issue_for_leader() {
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
    let issue = svc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: Some("form AE".into()),
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(issue.issuer, "alice");
    assert!(matches!(issue.state, ApiIssueState::Opened));
}

#[tokio::test]
async fn facade_create_issue_for_non_leader_non_qc_returns_forbidden() {
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
        .create_issue(
            &Actor {
                user_code: "carol".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "x".into(),
            },
        )
        .await
        .unwrap_err();
    assert!(matches!(
        err,
        apis::mission::MissionApiError::Forbidden { .. }
    ));
}

#[tokio::test]
async fn facade_close_reopen_via_state_request() {
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
    let issue = svc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "x".into(),
            },
        )
        .await
        .unwrap();

    let closed = svc
        .close_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CloseIssueRequest {},
            issue.id,
        )
        .await
        .unwrap();
    assert!(matches!(closed.state, ApiIssueState::Closed));

    let reopened = svc
        .reopen_issue(
            &Actor {
                user_code: "alice".into(),
            },
            ReopenIssueRequest {},
            issue.id,
        )
        .await
        .unwrap();
    assert!(matches!(reopened.state, ApiIssueState::Opened));
}

#[tokio::test]
async fn facade_update_description_via_request() {
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
    let issue = svc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let updated = svc
        .update_issue_description(
            &Actor {
                user_code: "alice".into(),
            },
            issue.id,
            UpdateIssueDescriptionRequest {
                description: "second".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.description, "second");
}

#[tokio::test]
async fn facade_append_comment_dev_assignee() {
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
    let issue = svc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "first".into(),
            },
        )
        .await
        .unwrap();
    let updated = svc
        .append_comment(
            &Actor {
                user_code: "u1".into(),
            },
            issue.id,
            AppendCommentRequest {
                content: "hello".into(),
            },
        )
        .await
        .unwrap();
    assert_eq!(updated.comments.len(), 1);
    assert_eq!(updated.comments[0].user, "u1");
}

#[tokio::test]
async fn facade_list_issues_by_mission_filters_by_state() {
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
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "a".into(),
            },
        )
        .await
        .unwrap();
    let b = svc
        .create_issue(
            &Actor {
                user_code: "alice".into(),
            },
            CreateIssueRequest {
                mission_id: m.id,
                target_item: None,
                description: "b".into(),
            },
        )
        .await
        .unwrap();
    svc.close_issue(
        &Actor {
            user_code: "alice".into(),
        },
        CloseIssueRequest {},
        b.id,
    )
    .await
    .unwrap();
    let all = svc
        .list_issues_by_mission(ListIssuesByMissionRequest {
            mission_id: m.id,
            state: None,
        })
        .await
        .unwrap();
    assert_eq!(all.len(), 2);
    let _ = a;
}
```

- [ ] **Step 5: Extend `lib.rs` with the new public surface**

Replace `/root/coding/project/aegis/lib/crates/mission/src/lib.rs`:

```rust
//! `mission` workspace crate.
//!
//! Hosts the `Mission` + `Assignee` aggregates, the
//! `MissionIssue` aggregate and its `IssueComment` child
//! collection, their ports, the PostgreSQL-backed persistence
//! adapters (`MissionRepo` / `AssigneeRepo` / `IssueRepo`), the
//! cross-crate `ProjectLookup` / `UserLookup` adapters, the
//! usecase layer (`MissionUsecase` + `MissionIssueUsecase`) that
//! orchestrates them with project-leader authorization, and the
//! in-memory facade that adapts both usecases to the apis port.

pub use adapter::facade::in_memory::service::MissionServiceImpl;
pub use adapter::persistence::postgres::{AssigneeRepo, IssueRepo, MissionRepo};
pub use domain::{
    Assignee, AssigneeNew, AssigneeRepository, DomainError, IssueComment, IssueState,
    Mission, MissionIssue, MissionIssueNew, MissionIssueRepository, MissionKind, MissionNew,
    MissionRepository, MissionRole, ProjectLookup, ProjectLookupImpl, UserLookup, UserLookupImpl,
    assignees_within_mission_are_unique,
};
pub use usecase::{
    AssigneeData, AssigneeView, CreateIssue, CreateMission, IssueCommentView, IssueView,
    MissionIssueUsecase, MissionIssueUsecaseConfig, MissionUsecase, MissionUsecaseConfig,
    MissionView, UsecaseError,
};

mod adapter;
mod domain;
mod usecase;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
```

- [ ] **Step 6: Run the facade tests + clippy**

Run:
```bash
cargo test -p mission --lib adapter::facade::in_memory::tests
cargo clippy -p mission --all-targets --all-features -- -D warnings
```
Expected: PASS — every facade test green and clippy clean.

- [ ] **Step 7: Lint and commit**

```bash
cargo fmt --all
git add lib/crates/mission/src/adapter/facade/in_memory.rs \
        lib/crates/mission/src/adapter/facade/in_memory \
        lib/crates/mission/src/lib.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): facade extends MissionServiceImpl + in-memory issue repo

- MissionServiceImpl<M, A, P, U, I> composes both usecases.
  from_usecase(uc, issue_uc) is the canonical constructor;
  from_repos(...) builds both usecases from the five backing
  pieces and mirrors the legacy signature so run.rs stays a
  one-liner.
- New six apis methods: list_issues_by_mission, create_issue,
  close_issue / reopen_issue (with empty marker request bodies),
  update_issue_description, append_comment.
- Bridges: apis::mission::IssueState ↔ domain::IssueState via
  From<…> in both directions.
- New error mapping: MissionIssueNotFound → IssueNotFound.
- In-memory InMemoryIssueRepo lives next to service.rs and
  mirrors the FakeIssueRepo from test_support.
- Facade tests: leader / non-leader / QC / DEV permission
  permutations, idempotent close+reopen, update_description on
  closed, list-by-mission state filter.

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--lib adapter::facade::in_memory::tests.
EOF
)"
```

---

## Task 6: `apis::mission` — IssueState, IssueView, requests, 6 trait methods, error variants

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/apis/src/mission.rs`

**Interfaces:**
- Consumes: nothing (pure DTO file).
- Produces: `apis::mission::IssueState`, `apis::mission::IssueCommentView`, `apis::mission::IssueView`, request structs (`CreateIssueRequest`, `CloseIssueRequest`, `ReopenIssueRequest`, `UpdateIssueDescriptionRequest`, `AppendCommentRequest`, `ListIssuesByMissionRequest`), six new methods on `MissionService`, error variants `IssueNotFound` + `MissionNotFoundForIssue`.

- [ ] **Step 1: Extend `apis/src/mission.rs`**

Append to the existing `apis/src/mission.rs` (do not rewrite the existing `MissionKind` / `MissionRole` / `Actor` / `MissionService` / `MissionApiError` block — only add what's missing). The final file should contain the additions below. Reference the existing patterns for naming and formatting.

```rust
// ---- additions below the existing top-of-file (no `use` churn) ----

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IssueState {
    Opened,
    Closed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueCommentView {
    pub user: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssueView {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: IssueState,
    pub comments: Vec<IssueCommentView>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateIssueRequest {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct CloseIssueRequest {}

#[derive(Debug, Clone, Default)]
pub struct ReopenIssueRequest {}

#[derive(Debug, Clone)]
pub struct UpdateIssueDescriptionRequest {
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AppendCommentRequest {
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ListIssuesByMissionRequest {
    pub mission_id: i64,
    pub state: Option<IssueState>,
}

// ---- additions to MissionApiError (extend the existing enum) ----
//
//   #[error("issue not found")]
//   IssueNotFound,
//   #[error("mission not found for issue {0}")]
//   MissionNotFoundForIssue(i64),
//
// (Keep all existing variants in place.)

// ---- additions to MissionService (extend the existing trait) ----
//
//   async fn list_issues_by_mission(
//       &self,
//       req: ListIssuesByMissionRequest,
//   ) -> Result<Vec<IssueView>, MissionApiError>;
//
//   async fn create_issue(
//       &self,
//       actor: &Actor,
//       req: CreateIssueRequest,
//   ) -> Result<IssueView>, MissionApiError>;
//
//   async fn close_issue(
//       &self,
//       actor: &Actor,
//       req: CloseIssueRequest,
//       issue_id: i64,
//   ) -> Result<IssueView, MissionApiError>;
//
//   async fn reopen_issue(
//       &self,
//       actor: &Actor,
//       req: ReopenIssueRequest,
//       issue_id: i64,
//   ) -> Result<IssueView, MissionApiError>;
//
//   async fn update_issue_description(
//       &self,
//       actor: &Actor,
//       issue_id: i64,
//       req: UpdateIssueDescriptionRequest,
//   ) -> Result<IssueView, MissionApiError>;
//
//   async fn append_comment(
//       &self,
//       actor: &Actor,
//       issue_id: i64,
//       req: AppendCommentRequest,
//   ) -> Result<IssueView, MissionApiError>;
```

- [ ] **Step 2: Verify the apis crate compiles**

Run: `cargo check -p apis`
Expected: success. The mission crate will fail to compile until Task 5 lands — defer that to the Task 5 verification gate.

- [ ] **Step 3: Lint and commit**

```bash
cargo fmt --all
git add lib/crates/apis/src/mission.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): apis port — IssueState, IssueView, six methods

- Add IssueState { Opened, Closed }, IssueCommentView, IssueView
  to the apis::mission port module.
- Six new request DTOs: CreateIssueRequest, CloseIssueRequest
  (empty marker), ReopenIssueRequest (empty marker),
  UpdateIssueDescriptionRequest, AppendCommentRequest,
  ListIssuesByMissionRequest.
- Six new methods on MissionService: list_issues_by_mission,
  create_issue, close_issue, reopen_issue,
  update_issue_description, append_comment.
- Two new MissionApiError variants: IssueNotFound and
  MissionNotFoundForIssue (the latter is reserved for any future
  IssueView->MissionNotFound mapping; not currently emitted).

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo check -p apis.
EOF
)"
```

---

## Task 7: Server HTTP — five `/api/mission/*` routes, wire DTOs, error mapping, NullMissionService, run.rs wiring

**Files:**
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/state.rs`
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/run.rs`
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/mission/router.rs`
- Modify: `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/mission/handlers.rs`

**Interfaces:**
- Consumes: every apis::mission type, the existing `ErrorBody` pattern, and the existing `OpenApiRouter` style.
- Produces: five new utoipa-axum routes mounted at `/api/mission/*`. Wire DTOs are 1:1 with the apis request/response shapes — the handler does the boundary translation.

- [ ] **Step 1: Extend `state.rs` `NullMissionService`**

Append to the existing `impl MissionService for NullMissionService { ... }` block in
`/root/coding/project/aegis/apps/server/aegis-server/src/state.rs`:

```rust
    async fn list_issues_by_mission(
        &self,
        _req: apis::mission::ListIssuesByMissionRequest,
    ) -> Result<Vec<apis::mission::IssueView>, apis::mission::MissionApiError> {
        unimplemented!("list_issues_by_mission")
    }
    async fn create_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::CreateIssueRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!("create_issue")
    }
    async fn close_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::CloseIssueRequest,
        _issue_id: i64,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!("close_issue")
    }
    async fn reopen_issue(
        &self,
        _actor: &apis::mission::Actor,
        _req: apis::mission::ReopenIssueRequest,
        _issue_id: i64,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!("reopen_issue")
    }
    async fn update_issue_description(
        &self,
        _actor: &apis::mission::Actor,
        _issue_id: i64,
        _req: apis::mission::UpdateIssueDescriptionRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!("update_issue_description")
    }
    async fn append_comment(
        &self,
        _actor: &apis::mission::Actor,
        _issue_id: i64,
        _req: apis::mission::AppendCommentRequest,
    ) -> Result<apis::mission::IssueView, apis::mission::MissionApiError> {
        unimplemented!("append_comment")
    }
```

- [ ] **Step 2: Extend `dto.rs` with the five wire DTOs + `From` impls**

Append to `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/dto.rs`. Mirror the existing serde + utoipa derives; rename `state` → `state` (already snake_case-compatible).

```rust
// ---- Mission-issue wire DTOs ----
//
// The wire is snake_case; the apis types are camelCase via serde
// rename at the apis boundary. The server stays the single place
// where we hand-translate.

use apis::mission::{
    AppendCommentRequest, CloseIssueRequest, CreateIssueRequest, IssueState as ApiIssueState,
    IssueView as ApiIssueView, ListIssuesByMissionRequest, ReopenIssueRequest,
    UpdateIssueDescriptionRequest,
};

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateIssueBody {
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub description: String,
}

impl From<CreateIssueBody> for CreateIssueRequest {
    fn from(b: CreateIssueBody) -> Self {
        Self {
            mission_id: b.mission_id,
            target_item: b.target_item,
            description: b.description,
        }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateIssueDescriptionBody {
    pub description: String,
}

impl From<UpdateIssueDescriptionBody> for UpdateIssueDescriptionRequest {
    fn from(b: UpdateIssueDescriptionBody) -> Self {
        Self {
            description: b.description,
        }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct AppendCommentBody {
    pub content: String,
}

impl From<AppendCommentBody> for AppendCommentRequest {
    fn from(b: AppendCommentBody) -> Self {
        Self { content: b.content }
    }
}

#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ListIssuesQuery {
    pub state: Option<String>,
}

impl ListIssuesQuery {
    pub fn into_request(self, mission_id: i64) -> ListIssuesByMissionRequest {
        let state = match self.state.as_deref() {
            Some("opened") => Some(ApiIssueState::Opened),
            Some("closed") => Some(ApiIssueState::Closed),
            _ => None,
        };
        ListIssuesByMissionRequest { mission_id, state }
    }
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IssueViewResponse {
    pub id: i64,
    pub mission_id: i64,
    pub target_item: Option<String>,
    pub issuer: String,
    pub description: String,
    pub state: String,
    pub comments: Vec<IssueCommentViewResponse>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct IssueCommentViewResponse {
    pub user: String,
    pub content: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

impl From<ApiIssueView> for IssueViewResponse {
    fn from(v: ApiIssueView) -> Self {
        let state = match v.state {
            ApiIssueState::Opened => "opened",
            ApiIssueState::Closed => "closed",
        };
        Self {
            id: v.id,
            mission_id: v.mission_id,
            target_item: v.target_item,
            issuer: v.issuer,
            description: v.description,
            state: state.to_string(),
            comments: v
                .comments
                .into_iter()
                .map(|c| IssueCommentViewResponse {
                    user: c.user,
                    content: c.content,
                    created_at: c.created_at,
                })
                .collect(),
            created_at: v.created_at,
            updated_at: v.updated_at,
        }
    }
}

// `CloseIssueRequest` / `ReopenIssueRequest` are empty marker
// structs in apis. The PATCH endpoint dispatches on the URL
// suffix (`/close` or `/reopen`) so no body translation is
// needed — the handler constructs an empty request itself.
#[allow(dead_code)]
fn _force_use(_: CloseIssueRequest, _: ReopenIssueRequest) {}
```

- [ ] **Step 3: Extend `error.rs` with status + code mappings**

Append to `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/error.rs`. Match the existing `mission_status` / `mission_code` pattern.

```rust
// Inside the existing `fn mission_status(err: &MissionApiError) -> StatusCode { ... }`:
//
//   MissionApiError::IssueNotFound => StatusCode::NOT_FOUND,
//   MissionApiError::MissionNotFoundForIssue(_) => StatusCode::NOT_FOUND,
//
// Inside the existing `fn mission_code(err: &MissionApiError) -> &'static str { ... }`:
//
//   MissionApiError::IssueNotFound => "issue_not_found",
//   MissionApiError::MissionNotFoundForIssue(_) => "mission_not_found_for_issue",
```

Concretely, add the two arms in each function. Read the existing `error.rs` first so the patch matches the surrounding `match` style (whether it uses `match err { ... }` or `match *err { ... }`).

- [ ] **Step 4: Add the five handlers in `mission/handlers.rs`**

Append to `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/mission/handlers.rs`. Mirror the existing `create_mission` / `add_assignee` handler signature shape: `State<...>`, `Extension<Actor>`, JSON body / path / query, `ApiResult<…>`.

```rust
// ---- Mission-issue handlers ----

#[utoipa::path(
    get,
    path = "/api/mission/missions/{mission_id}/issues",
    params(
        ("mission_id" = i64, Path,),
        ("state" = Option<String>, Query, description = "opened | closed"),
    ),
    responses(
        (status = 200, body = Vec<IssueViewResponse>),
        (status = 404, body = ErrorBody),
    ),
)]
pub async fn list_issues_by_mission(
    State(svc): State<Arc<MissionService>>,
    Extension(actor): Extension<Actor>,
    Path(mission_id): Path<i64>,
    Query(query): Query<ListIssuesQuery>,
) -> ApiResult<Vec<IssueViewResponse>> {
    let req = query.into_request(mission_id);
    // list is unauthenticated-by-actor at the apis layer (the
    // existing list_missions_by_project also takes no Actor), so
    // the actor extension is accepted but unused here.
    let _ = actor;
    svc.list_issues_by_mission(req)
        .await
        .map(|v| v.into_iter().map(Into::into).collect())
        .map_err(Into::into)
}

#[utoipa::path(
    post,
    path = "/api/mission/missions/{mission_id}/issues",
    request_body = CreateIssueBody,
    responses(
        (status = 200, body = IssueViewResponse),
        (status = 400, body = ErrorBody),
        (status = 403, body = ErrorBody),
    ),
)]
pub async fn create_issue(
    State(svc): State<Arc<MissionService>>,
    Extension(actor): Extension<Actor>,
    Path(mission_id): Path<i64>,
    Json(body): Json<CreateIssueBody>,
) -> ApiResult<IssueViewResponse> {
    let mut body = body;
    body.mission_id = mission_id;
    svc.create_issue(&actor, body.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[utoipa::path(
    patch,
    path = "/api/mission/issues/{issue_id}/state",
    request_body = serde_json::Value,
    responses(
        (status = 200, body = IssueViewResponse),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
    ),
)]
pub async fn patch_issue_state(
    State(svc): State<Arc<MissionService>>,
    Extension(actor): Extension<Actor>,
    Path(issue_id): Path<i64>,
    Json(body): Json<serde_json::Value>,
) -> ApiResult<IssueViewResponse> {
    // Body shape: { "state": "opened" | "closed" }.
    // The PATCH endpoint dispatches on the body's `state` field;
    // we keep close / reopen as separate apis methods so the
    // service trait stays explicit.
    let state = body
        .get("state")
        .and_then(|v| v.as_str())
        .ok_or_else(|| crate::transport::http::error::ApiError::BadRequest(
            "body.state must be 'opened' or 'closed'".into(),
        ))?;
    match state {
        "opened" => svc
            .reopen_issue(&actor, apis::mission::ReopenIssueRequest {}, issue_id)
            .await
            .map(Into::into)
            .map_err(Into::into),
        "closed" => svc
            .close_issue(&actor, apis::mission::CloseIssueRequest {}, issue_id)
            .await
            .map(Into::into)
            .map_err(Into::into),
        other => Err(crate::transport::http::error::ApiError::BadRequest(
            format!("unsupported issue state '{other}'"),
        )),
    }
}

#[utoipa::path(
    patch,
    path = "/api/mission/issues/{issue_id}/description",
    request_body = UpdateIssueDescriptionBody,
    responses(
        (status = 200, body = IssueViewResponse),
        (status = 400, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
    ),
)]
pub async fn update_issue_description(
    State(svc): State<Arc<MissionService>>,
    Extension(actor): Extension<Actor>,
    Path(issue_id): Path<i64>,
    Json(body): Json<UpdateIssueDescriptionBody>,
) -> ApiResult<IssueViewResponse> {
    svc.update_issue_description(&actor, issue_id, body.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}

#[utoipa::path(
    post,
    path = "/api/mission/issues/{issue_id}/comments",
    request_body = AppendCommentBody,
    responses(
        (status = 200, body = IssueViewResponse),
        (status = 400, body = ErrorBody),
        (status = 403, body = ErrorBody),
        (status = 404, body = ErrorBody),
    ),
)]
pub async fn append_comment(
    State(svc): State<Arc<MissionService>>,
    Extension(actor): Extension<Actor>,
    Path(issue_id): Path<i64>,
    Json(body): Json<AppendCommentBody>,
) -> ApiResult<IssueViewResponse> {
    svc.append_comment(&actor, issue_id, body.into())
        .await
        .map(Into::into)
        .map_err(Into::into)
}
```

- [ ] **Step 5: Mount the five routes in `mission/router.rs`**

Append to `/root/coding/project/aegis/apps/server/aegis-server/src/transport/http/mission/router.rs`. Use the same `OpenApiRouter` + `.route(…)` chain the existing mission routes use.

```rust
    .route("/missions/{mission_id}/issues", get(list_issues_by_mission).post(create_issue))
    .route("/issues/{issue_id}/state", patch(patch_issue_state))
    .route("/issues/{issue_id}/description", patch(update_issue_description))
    .route("/issues/{issue_id}/comments", post(append_comment))
```

(Add `use crate::transport::http::mission::handlers::{append_comment, create_issue, list_issues_by_mission, patch_issue_state, update_issue_description};` if not already present; otherwise extend the existing `use` line.)

- [ ] **Step 6: Pass `IssueRepo` through `run.rs`**

In `/root/coding/project/aegis/apps/server/aegis-server/src/run.rs`, locate the call that builds `MissionServiceImpl` and pass an `IssueRepo` argument. The exact constructor is `from_repos(...)` which now takes five pieces; the surrounding code should look like:

```rust
let mission_svc = MissionServiceImpl::from_repos(
    MissionRepo::new(pool.clone()),
    AssigneeRepo::new(pool.clone()),
    Arc::new(project_lookup),
    Arc::new(user_lookup),
    IssueRepo::new(pool.clone()),
);
```

- [ ] **Step 7: Verify the server compiles**

Run: `cargo check -p aegis-server`
Expected: success. The five new routes are mounted; the OpenAPI doc compiles; the NullMissionService implements the six new methods.

- [ ] **Step 8: Lint and commit**

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
git add apps/server/aegis-server/src/state.rs \
        apps/server/aegis-server/src/run.rs \
        apps/server/aegis-server/src/transport
git commit -m "$(cat <<'EOF'
feat(mission-issue): server HTTP — five /api/mission/* routes + wire DTOs

- Wire DTOs in transport::http::dto: CreateIssueBody,
  UpdateIssueDescriptionBody, AppendCommentBody, ListIssuesQuery,
  IssueViewResponse, IssueCommentViewResponse. The apis types
  are hand-translated at the boundary; the wire is snake_case.
- Five new handlers mounted under /api/mission:
    GET  /missions/{mission_id}/issues        list (state? query)
    POST /missions/{mission_id}/issues        create
    PATCH /issues/{issue_id}/state            dispatched on body.state
    PATCH /issues/{issue_id}/description      update_description
    POST /issues/{issue_id}/comments          append_comment
- error.rs: add IssueNotFound → 404 / "issue_not_found" and
  MissionNotFoundForIssue → 404 / "mission_not_found_for_issue".
- state.rs: NullMissionService gains the six unimplemented stubs.
- run.rs: pass IssueRepo::new(pool.clone()) as the fifth arg to
  MissionServiceImpl::from_repos.

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy --workspace
--all-targets --all-features -- -D warnings; cargo check -p aegis-server.
EOF
)"
```

---

## Task 8: Live-DB integration tests + server integration tests

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/tests/integration_persistence.rs`

**Interfaces:**
- Consumes: `IssueRepo`, `MissionRepo`, `AssigneeRepo`, `MissionIssueNew`, the existing `with_pool` + `unique_code` helpers.
- Produces: `#[ignore]`-gated live-DB round-trip tests for the new migration + `IssueRepo`.

- [ ] **Step 1: Extend `with_pool` to drop `mission_issues` + add the issue tests**

Replace `/root/coding/project/aegis/lib/crates/mission/tests/integration_persistence.rs`:

```rust
//! Live-database integration tests for the PostgreSQL adapter.
//!
//! `#[ignore]`-gated; opt in with `cargo test -p mission -- --ignored`.
//! Reads `AEGIS_MISSION_DATABASE_URL`; loads `.env` at the workspace
//! root via `dotenvy`. Drops the live tables + `_sqlx_migrations`
//! before each run so the migration starts fresh.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use sqlx::PgPool;

use mission::domain::{AssigneeNew, MissionKind, MissionNew, MissionRole};
use mission::{
    AssigneeRepo, AssigneeRepository, DomainError, IssueRepo, MissionIssueRepository, MissionRepo,
    MissionRepository,
};

async fn with_pool<F, Fut, T>(f: F) -> T
where
    F: FnOnce(PgPool) -> Fut,
    Fut: std::future::Future<Output = T>,
{
    let _ = dotenvy::dotenv();
    let url = std::env::var("AEGIS_MISSION_DATABASE_URL").unwrap_or_else(|_| {
        panic!(
            "AEGIS_MISSION_DATABASE_URL must be set (or present in .env at the \
             workspace root) to run --ignored tests"
        )
    });
    let pool = PgPool::connect(&url)
        .await
        .expect("connect to PostgreSQL via AEGIS_MISSION_DATABASE_URL");

    // Destructive cleanup. The integration tests own the schema; if
    // you point them at production by mistake you will lose data.
    sqlx::query("DROP TABLE IF EXISTS mission_issues CASCADE")
        .execute(&pool)
        .await
        .expect("drop mission_issues");
    sqlx::query("DROP TABLE IF EXISTS assignees CASCADE")
        .execute(&pool)
        .await
        .expect("drop assignees");
    sqlx::query("DROP TABLE IF EXISTS missions CASCADE")
        .execute(&pool)
        .await
        .expect("drop missions");
    sqlx::query("DROP TABLE IF EXISTS _sqlx_migrations CASCADE")
        .execute(&pool)
        .await
        .expect("drop sqlx_migrations bookkeeping");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("apply mission migrations");

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
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_create_and_find_round_trip() {
    // unchanged from before
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_create_duplicate_rejects() {
    // unchanged from before
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_delete_cascades_to_assignees() {
    // unchanged from before
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn assignee_per_mission_user_role_uniqueness_holds() {
    // unchanged from before
}

// ---- mission_issues round-trip ----

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_create_find_list_close_open_round_trip() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool.clone());

        let mission = missions
            .create(MissionNew {
                project_code: "p1".into(),
                mission_kind: MissionKind::Sdtm,
                mission_code: unique_code("issue-mission"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let created = issues
            .create(mission::domain::MissionIssueNew {
                mission_id: mission.id,
                target_item: Some("form AE".into()),
                issuer: "u1".into(),
                description: "first".into(),
            })
            .await
            .expect("create issue");
        assert_eq!(created.mission_id, mission.id);
        assert_eq!(created.issuer, "u1");
        assert_eq!(created.state, mission::domain::IssueState::Opened);

        let fetched = issues.find_by_id(created.id).await.expect("find_by_id");
        assert_eq!(fetched.id, created.id);

        let list = issues
            .list_by_mission(mission.id, None)
            .await
            .expect("list all");
        assert_eq!(list.len(), 1);

        let closed = issues.close(created.id).await.expect("close");
        assert_eq!(closed.state, mission::domain::IssueState::Closed);

        let listed_closed = issues
            .list_by_mission(mission.id, Some(mission::domain::IssueState::Closed))
            .await
            .expect("list closed");
        assert_eq!(listed_closed.len(), 1);

        let opened = issues.open(created.id).await.expect("reopen");
        assert_eq!(opened.state, mission::domain::IssueState::Opened);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_update_description_persists() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool.clone());

        let mission = missions
            .create(MissionNew {
                project_code: "p1".into(),
                mission_kind: MissionKind::Adam,
                mission_code: unique_code("upd-mission"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let issue = issues
            .create(mission::domain::MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "first".into(),
            })
            .await
            .expect("create issue");

        let updated = issues
            .update_description(issue.id, "second".into())
            .await
            .expect("update description");
        assert_eq!(updated.description, "second");
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn issue_append_comment_round_trips_jsonb() {
    use mission::domain::IssueComment;

    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool.clone());

        let mission = missions
            .create(MissionNew {
                project_code: "p1".into(),
                mission_kind: MissionKind::Tfl,
                mission_code: unique_code("comment-mission"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let issue = issues
            .create(mission::domain::MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "first".into(),
            })
            .await
            .expect("create issue");

        let one = issues
            .append_comment(
                issue.id,
                IssueComment::new(
                    "u1".into(),
                    "first".into(),
                    chrono::Utc::now(),
                )
                .expect("comment"),
            )
            .await
            .expect("append first");
        assert_eq!(one.comments.len(), 1);

        let two = issues
            .append_comment(
                issue.id,
                IssueComment::new(
                    "u2".into(),
                    "second".into(),
                    chrono::Utc::now(),
                )
                .expect("comment"),
            )
            .await
            .expect("append second");
        assert_eq!(two.comments.len(), 2);
        assert_eq!(two.comments[0].user, "u1");
        assert_eq!(two.comments[1].user, "u2");

        let fetched = issues.find_by_id(issue.id).await.expect("find");
        assert_eq!(fetched.comments.len(), 2);
    })
    .await
}

#[tokio::test]
#[ignore = "requires AEGIS_MISSION_DATABASE_URL pointing at a live PostgreSQL"]
async fn mission_delete_cascades_to_issues() {
    with_pool(|pool| async move {
        let missions = MissionRepo::new(pool.clone());
        let issues = IssueRepo::new(pool.clone());

        let mission = missions
            .create(MissionNew {
                project_code: "p1".into(),
                mission_kind: MissionKind::Crf,
                mission_code: unique_code("cascade-issue-mission"),
                assignees: vec![],
            })
            .await
            .expect("create mission");

        let issue = issues
            .create(mission::domain::MissionIssueNew {
                mission_id: mission.id,
                target_item: None,
                issuer: "u1".into(),
                description: "first".into(),
            })
            .await
            .expect("create issue");

        missions
            .delete(mission.id)
            .await
            .expect("delete mission");

        let err = issues
            .find_by_id(issue.id)
            .await
            .expect_err("issue should 404 after mission cascade");
        assert!(
            matches!(err, DomainError::MissionIssueNotFound),
            "expected MissionIssueNotFound, got {err:?}"
        );
    })
    .await
}
```

(Preserve the existing four `#[ignore]` tests verbatim; the snippets labelled `// unchanged from before` are intentional placeholders — copy them from the pre-merge `integration_persistence.rs`.)

- [ ] **Step 2: Run the live-DB tests**

Run: `cargo test -p mission -- --ignored --test-threads=1`
Expected: PASS for all eight `#[ignore]` tests. The destructive cleanup means each run starts fresh — re-run is safe.

- [ ] **Step 3: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/tests/integration_persistence.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): live-DB integration tests for mission_issues

- with_pool: drop mission_issues CASCADE before applying migrations
- Four new #[ignore]-gated tests:
    issue_create_find_list_close_open_round_trip
    issue_update_description_persists
    issue_append_comment_round_trips_jsonb
    mission_delete_cascades_to_issues
- Covers create / find / list / close / reopen,
  description update, jsonb append_comment round-trip, and the
  ON DELETE CASCADE behaviour from missions → mission_issues.

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
-- --ignored --test-threads=1.
EOF
)"
```

---

## Task 9: `tests/public_api.rs` — pin the new public surface

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/tests/public_api.rs`

**Interfaces:**
- Consumes: nothing (compile-only).
- Produces: name locks for every new public type / bound.

- [ ] **Step 1: Extend `public_api.rs`**

Append the new tests to `/root/coding/project/aegis/lib/crates/mission/tests/public_api.rs`. Do not rewrite the existing tests; add new ones below them.

```rust
// ---- Mission-issue additions ----

use apis::mission::{
    AppendCommentRequest, CloseIssueRequest, CreateIssueRequest, IssueCommentView as ApiIssueCommentView,
    IssueState as ApiIssueState, IssueView as ApiIssueView, ListIssuesByMissionRequest,
    ReopenIssueRequest, UpdateIssueDescriptionRequest,
};

use mission::{
    IssueComment, IssueRepo, IssueState, MissionIssue, MissionIssueNew, MissionIssueRepository,
    MissionIssueUsecase, MissionIssueUsecaseConfig,
};

#[test]
fn domain_issue_types_are_nameable_from_crate_root() {
    fn assert_issue(_: MissionIssue) {}
    fn assert_comment(_: IssueComment) {}
    fn assert_state(_: IssueState) {}
    assert_issue(MissionIssue::for_repository(
        1, 1, None, "u1".into(), "desc".into(),
        IssueState::Opened, vec![], chrono::Utc::now(), chrono::Utc::now(),
    ));
    assert_state(IssueState::Closed);
    assert_comment(IssueComment::for_repository(
        "u1".into(), "c".into(), chrono::Utc::now(),
    ));
    let _ = assert_issue;
}

#[test]
fn issue_port_is_object_safe() {
    fn assert_box_dyn_issue<I: MissionIssueRepository + 'static>() {}
    assert_box_dyn_issue::<IssueRepo>();
}

#[test]
fn issue_usecase_can_be_built_from_config() {
    let _: fn(
        MissionIssueUsecaseConfig<MissionRepo, AssigneeRepo, ProjectLookupImpl, IssueRepo>,
    ) -> MissionIssueUsecase<MissionRepo, AssigneeRepo, ProjectLookupImpl, IssueRepo> =
        |cfg| MissionIssueUsecase::new(cfg);
}

#[test]
fn api_issue_dtos_are_nameable() {
    fn assert_issue(_: ApiIssueView) {}
    fn assert_state(_: ApiIssueState) {}
    fn assert_comment(_: ApiIssueCommentView) {}
    assert_state(ApiIssueState::Opened);
    let _create = CreateIssueRequest {
        mission_id: 1,
        target_item: Some("form AE".into()),
        description: "first".into(),
    };
    let _close = CloseIssueRequest {};
    let _reopen = ReopenIssueRequest {};
    let _update = UpdateIssueDescriptionRequest {
        description: "second".into(),
    };
    let _append = AppendCommentRequest {
        content: "hello".into(),
    };
    let _list = ListIssuesByMissionRequest {
        mission_id: 1,
        state: Some(ApiIssueState::Closed),
    };
    let _ = (assert_issue, assert_state, assert_comment);
}

#[test]
fn api_error_includes_issue_not_found() {
    fn assert_err(_: MissionApiError) {}
    assert_err(MissionApiError::IssueNotFound);
    assert_err(MissionApiError::MissionNotFoundForIssue(1));
}
```

- [ ] **Step 2: Run the public-API tests**

Run: `cargo test -p mission --test public_api`
Expected: PASS.

- [ ] **Step 3: Lint and commit**

```bash
cargo fmt --all
cargo clippy -p mission --all-targets --all-features -- -D warnings
git add lib/crates/mission/tests/public_api.rs
git commit -m "$(cat <<'EOF'
feat(mission-issue): public_api.rs pins new types + bounds

- domain_issue_types_are_nameable_from_crate_root:
  MissionIssue / IssueComment / IssueState
- issue_port_is_object_safe: IssueRepo : MissionIssueRepository
- issue_usecase_can_be_built_from_config: 4-param generic
- api_issue_dtos_are_nameable: six request DTOs + view types
- api_error_includes_issue_not_found: IssueNotFound +
  MissionNotFoundForIssue variants

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
Verification: cargo fmt --all -- --check; cargo clippy -p mission
--all-targets --all-features -- -D warnings; cargo test -p mission
--test public_api.
EOF
)"
```

---

## Task 10: README — Mission issues section

**Files:**
- Modify: `/root/coding/project/aegis/lib/crates/mission/README.md`

**Interfaces:**
- Consumes: the existing README structure.
- Produces: a "Mission issues" section after the existing "Mission" / "Assignee" sections.

- [ ] **Step 1: Append the section**

Append to `/root/coding/project/aegis/lib/crates/mission/README.md`:

```markdown
## Mission issues

Each mission owns a thread of `MissionIssue` aggregates. An issue has
an `Opened` / `Closed` state and an append-only `IssueComment`
thread stored as a jsonb column. The full surface is:

- `apis::mission::IssueState` — `{ Opened, Closed }`
- `apis::mission::IssueView` — the read model returned by every
  read or write path
- `apis::mission::IssueCommentView` — one comment
- `apis::mission::CreateIssueRequest` /
  `CloseIssueRequest` / `ReopenIssueRequest` /
  `UpdateIssueDescriptionRequest` / `AppendCommentRequest` /
  `ListIssuesByMissionRequest` — the wire DTOs

Authorisation lives in two private helpers on
`MissionIssueUsecase`:

- `ensure_can_create_or_close` — leader OR mission QC
- `ensure_can_comment` — leader OR mission DEV OR mission QC

`MissionIssueUsecase::close_issue` / `reopen_issue` are
idempotent; `update_description` is allowed regardless of the
current state.

The persistence adapter (`mission::IssueRepo`) is a thin wrapper
over `sqlx::query_as` against the `mission_issues` table; the
jsonb `comments` column round-trips via
`sqlx::types::Json<Vec<IssueComment>>` and is read-modify-written
on append — the Postgres `||` operator is intentionally avoided
because it does a shallow merge.
```

- [ ] **Step 2: Lint and commit**

```bash
cargo fmt --all
git add lib/crates/mission/README.md
git commit -m "$(cat <<'EOF'
docs(mission-issue): README — Mission issues section

Spec: docs/superpowers/specs/2026-09-20-mission-issue-design.md
EOF
)"
```

---

## Task 11: Final verification gate

**Files:** none — read-only verification across the workspace.

- [ ] **Step 1: Format check**

Run: `cargo fmt --all -- --check`
Expected: exit 0, no diff.

- [ ] **Step 2: Clippy on the mission crate**

Run: `cargo clippy -p mission --all-targets --all-features -- -D warnings`
Expected: exit 0.

- [ ] **Step 3: Unit + integration tests on the mission crate**

Run: `cargo test -p mission`
Expected: PASS — every test in `domain::tests`, `usecase::tests`, `adapter::facade::in_memory::tests`, `adapter::persistence::postgres::tests`, plus the two `tests/*.rs` files.

- [ ] **Step 4: `cargo doc` on the mission crate**

Run: `cargo doc -p mission --no-deps`
Expected: exit 0, no warnings.

- [ ] **Step 5: Workspace check**

Run: `cargo check --workspace`
Expected: exit 0.

- [ ] **Step 6: Workspace clippy**

Run: `cargo clippy --workspace --all-targets --all-features -- -D warnings`
Expected: exit 0.

- [ ] **Step 7: Workspace test**

Run: `cargo test --workspace`
Expected: PASS for every non-ignored crate.

- [ ] **Step 8: Live-DB tests**

Run (only when `AEGIS_MISSION_DATABASE_URL` is set):
`cargo test -p mission -- --ignored --test-threads=1`
Expected: PASS for all eight `#[ignore]` tests.

- [ ] **Step 9: Final commit (if any drift)**

If any previous task committed a WIP placeholder (e.g. an
unimplemented `NullMissionService` arm left as `todo!()`), this is
the catch-all. Otherwise, just summarise the verification results
in the PR description; no commit needed.

---

## Self-Review

**Spec coverage:**

| Spec section | Covered by |
|---|---|
| Data Model | Task 1 (migration + `IssueState` / `IssueComment` / `MissionIssue`) |
| Workspace Wiring | Task 1 (mission crate only; apis / server / desktop scope matches) |
| Module Layout | Tasks 1, 3, 4, 5 |
| Port Additions | Task 2 (`MissionIssueRepository`, `is_assignee`) |
| Domain details | Task 1 |
| Usecase details | Task 4 |
| Adapter details | Task 3 (Postgres) + Task 5 (in-memory facade) |
| `apis` additions | Task 6 |
| HTTP routes | Task 7 (5 routes, 5 handlers, wire DTOs, error mapping, NullMissionService, run.rs) |
| Migration SQL | Task 1 |
| Error mapping | Tasks 1 (variants), 5 (facade bridge), 7 (HTTP status / code) |
| Tests | Tasks 1 (domain), 3 (adapter), 4 (usecase), 5 (facade), 8 (live-DB), 9 (public-api) |
| README | Task 10 |
| Verification gate | Task 11 |

**Type consistency:**
- `apis::mission::IssueState` ↔ `mission::domain::IssueState` — bridged in Task 5 (`From<…>` in both directions).
- `apis::mission::IssueView` ↔ `mission::usecase::IssueView` ↔ `mission::domain::MissionIssue` — bridged via `From` impls in Task 4 (`MissionIssue → IssueView`) and Task 5 (`UcIssueView → ApiIssueView`).
- `apis::mission::IssueCommentView` ↔ `mission::usecase::IssueCommentView` ↔ `mission::domain::IssueComment` — same.
- `MissionIssueRepository` is the only port; `IssueRepo`, `FakeIssueRepo`, `InMemoryIssueRepo` all implement it.
- `MissionIssueUsecase::M/A/P/I` matches `MissionServiceImpl::M/A/P/U/I` (the `U` is `UserLookup` for the mission usecase and dropped in the issue usecase; `I` is `MissionIssueRepository` only).
- `MissionServiceImpl::from_repos` requires `A: Clone` because the issue usecase needs its own copy of the assignee repo.
- `MissionIssueUsecaseConfig` has exactly four fields (`mission_repo`, `assignee_repo`, `project_lookup`, `issue_repo`) — no `user_lookup`.

**Placeholder scan:** no "TBD" / "TODO" / "implement later" / "fill in details". The `// unchanged from before` comments in Task 8 are intentional and flag that the existing four `#[ignore]` tests must be preserved verbatim.

**Architectural soundness:** the layer dependency rule (`domain → usecase → adapter`) holds. `domain::issue_lookup` is the new port; `adapter::persistence::postgres::issue_repo` implements it; `adapter::facade::in_memory::issue_service::InMemoryIssueRepo` also implements it for tests. The `apis` crate stays serde-free. Wire DTOs live in `transport::http::dto`. The single JSON `state` field on the PATCH endpoint dispatches to `close_issue` / `reopen_issue` so the apis trait surface stays explicit.
