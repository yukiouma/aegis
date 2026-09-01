# 2026-09-01 — `mission` crate design

> Status: design approved, pending implementation plan.
>
> Scope: create the `lib/crates/mission` business-lib crate, add the
> `apis::mission::MissionService` outbound port, and wire the HTTP
> router in `apps/server/aegis-server`. Desktop tauri commands and
> referential cascades (user/project deletion) are explicitly out of
> scope this round.

## 1. Goal and non-goals

**Goal.** Add CRUD over a `Mission` aggregate and its `Assignee` child
collection, behind a new `apis::mission::MissionService` port, exposed
via `/api/mission/*` HTTP routes. Authenticate via the existing JWT
infrastructure; authorize via the project-leader relationship on
`apis::project::ProjectView.members.leaders`.

**Non-goals (this round).**

- Desktop tauri commands and TS client surface for mission data.
- Cascade behavior on `user` or `project` removal: deleting a user
  with active assignments, or a project with active missions, leaves
  the corresponding mission / assignee rows dangling. Deferred.
- Bulk create / update of missions.
- Soft delete or audit log of mission lifecycle events.
- Updating `mission_code`, `mission_kind`, or `project_code` after
  creation. (Update endpoint is skipped this round.)

## 2. Data model

### Aggregates

```rust
pub struct Mission {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<Assignee>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct Assignee {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

### Enums

```rust
pub enum MissionKind { Crf, Sdtm, Adam, Tfl }
pub enum MissionRole { Dev, Qc }
```

`MissionKind` and `MissionRole` are the source of truth; the DB
`CHECK` constraints and the apis mirrors stay in sync via the
`try_from` / `as_str` round-trip.

### Invariants

- `(project_code, mission_kind, mission_code)` is unique.
- `mission_code` is non-empty / non-whitespace (rejected by
  `Mission::new`).
- `user_code` on an assignee is non-empty / non-whitespace (rejected
  by `Assignee::new`).
- `(mission_id, user_code, role)` is unique **per mission**. A user
  can be `Dev` on mission A and `Dev` on mission B, and can hold
  both `Dev` and `Qc` on the same mission.
- Deleting a mission cascades to its assignees via
  `ON DELETE CASCADE` on `assignees.mission_id`.

## 3. Workspace wiring

- New crate `lib/crates/mission` registered in the root
  `Cargo.toml` `[workspace].members`.
- Path-dep on `apis` (port + inbound service ports),
  `sqlx`, `tokio`, `async-trait`, `thiserror`, `chrono`.
  Same `edition = "2024"`, `resolver = "3"` as the rest of the
  workspace.
- New env var `AEGIS_MISSION_DATABASE_URL` for the `#[ignore]`-gated
  live-DB tests.

## 4. Module layout

```
mission crate
├── src/
│   ├── lib.rs            (pub mod domain; pub mod usecase; pub mod adapter;  + pub use)
│   ├── domain.rs         → error.rs, mission.rs, assignee.rs,
│   │                       mission_kind.rs, mission_role.rs,
│   │                       mission_lookup.rs, project_lookup.rs,
│   │                       user_lookup.rs, tests.rs
│   ├── usecase.rs        → commands.rs, error.rs, views.rs,
│   │                       mission_usecase.rs, tests.rs
│   └── adapter/
│       ├── adapter.rs    (re-exports)
│       ├── persistence/postgres/
│       │     mission_repo.rs, assignee_repo.rs,
│       │     row.rs, tests.rs
│       ├── facade/in_memory/
│       │     service.rs, tests.rs
│       └── service/
│             project.rs (ProjectLookupImpl on apis::project::ProjectService)
│             user.rs    (UserLookupImpl on apis::user::UserService)
├── migrations/
│   ├── 0001_create_missions.sql
│   └── 0002_create_assignees.sql
├── tests/
│   ├── public_api.rs        (compile-only)
│   └── integration_persistence.rs  (#[ignore], destructive live-DB)
└── README.md
```

**Dependency direction.** Domain → no imports of `apis` / `sqlx` /
`tokio`. Usecase → only `domain`. Adapter → all three. The in-memory
facade adapts `MissionUsecase` to `apis::mission::MissionService`.

## 5. Domain layer

### 5.1 Ports

```rust
/// Persistence-input DTO for `MissionRepository::create`. Includes
/// the initial assignee list so the repo can insert both the mission
/// row and its assignee rows inside one transaction (the DB CHECK +
/// UNIQUE on `assignees` is the safety net for the per-mission
/// uniqueness invariant the usecase enforces up front).
pub struct MissionNew {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeNew>,
}

/// Persistence-input DTO for `AssigneeRepository::add` (single-row
/// insert used by the standalone `add_assignee` flow, after a
/// mission already exists).
pub struct AssigneeNew {
    pub user_code: String,
    pub role: MissionRole,
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
    async fn delete(&self, id: i64) -> Result<(), DomainError>; // cascades to assignees
}

#[async_trait]
pub trait AssigneeRepository: Send + Sync {
    async fn add(&self, mission_id: i64, input: AssigneeNew) -> Result<Assignee, DomainError>;
    async fn remove(&self, mission_id: i64, assignee_id: i64) -> Result<(), DomainError>;
}

// Cross-crate lookups. Same shape as `crf::domain::ProjectLookup`;
// `is_leader` is the auth primitive.
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError>;
}

#[async_trait]
pub trait UserLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
```

### 5.2 Two-constructor pattern

Every aggregate has `new(...)` (validating, returns `Result`) and
`pub(crate) fn for_repository(...)` (skips validation, used by the
row bridge).

`MissionNew` carries the initial assignee list so the repo can
insert mission + assignees in one transaction. The per-mission
uniqueness check (`(user_code, role)` unique within one mission)
lives in `MissionUsecase::create_mission` — the usecase rejects
duplicate pairs in `input.assignees` up front, and the DB UNIQUE
constraint is the safety net.

### 5.3 Errors

```rust
#[derive(Debug, thiserror::Error)]
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

## 6. Usecase layer

`MissionUsecase<M, A, P, U>` is generic over the four ports.

```rust
pub struct MissionUsecaseConfig<M, A, P, U> {
    pub mission_repo: M,
    pub assignee_repo: A,
    pub project_lookup: P,
    pub user_lookup: U,
}

pub struct MissionUsecase<M, A, P, U> { /* fields from config */ }
```

Surface:

```rust
impl<M, A, P, U> MissionUsecase<M, A, P, U>
where
    M: MissionRepository,
    A: AssigneeRepository,
    P: ProjectLookup,
    U: UserLookup,
{
    pub async fn create_mission(
        &self, actor: &Actor, input: CreateMission,
    ) -> Result<MissionView, UsecaseError>;

    pub async fn get_mission_by_id(
        &self, id: i64,
    ) -> Result<MissionView, UsecaseError>;

    pub async fn list_missions_by_project(
        &self, project_code: &str, kind: Option<MissionKind>,
    ) -> Result<Vec<MissionView>, UsecaseError>;

    pub async fn list_missions_by_user(
        &self, user_code: &str,
    ) -> Result<Vec<MissionView>, UsecaseError>;

    pub async fn delete_mission(
        &self, actor: &Actor, id: i64,
    ) -> Result<(), UsecaseError>;

    pub async fn add_assignee(
        &self, actor: &Actor, mission_id: i64, data: AssigneeData,
    ) -> Result<AssigneeView, UsecaseError>;

    pub async fn remove_assignee(
        &self, actor: &Actor, mission_id: i64, assignee_id: i64,
    ) -> Result<(), UsecaseError>;
}
```

**Auth model.** Every write method calls
`project_lookup.is_leader(project_code, actor.user_code)`. On
`false` → `UsecaseError::Forbidden`. Strict leader-only — even admin
and root users must be in `project.members.leaders`. Reads do not
check leadership.

**Command DTOs** (`usecase/commands.rs`):

```rust
pub struct CreateMission {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}
```

**View DTOs** (`usecase/views.rs`): `MissionView`, `AssigneeView`,
produced via `From` impls from the domain aggregates. The usecase
never returns raw domain types to the facade.

**Error:**

```rust
#[derive(Debug, thiserror::Error)]
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

## 7. Persistence layer

### 7.1 Schema

`migrations/0001_create_missions.sql`:

```sql
CREATE TABLE missions (
    id              BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    project_code    TEXT NOT NULL,
    mission_kind    TEXT NOT NULL,
    mission_code    TEXT NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT missions_natural_key UNIQUE (project_code, mission_kind, mission_code),
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

`migrations/0002_create_assignees.sql`:

```sql
CREATE TABLE assignees (
    id           BIGINT GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    mission_id   BIGINT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
    user_code    TEXT NOT NULL,
    role         TEXT NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT assignees_per_mission_unique UNIQUE (mission_id, user_code, role),
    CONSTRAINT assignees_role_check CHECK (role IN ('dev', 'qc'))
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

The `BEFORE UPDATE` triggers cover every code path, including direct
SQL. The CHECK constraints belt-and-brace the Rust enum conversions.
`ON DELETE CASCADE` on `assignees.mission_id` makes mission deletion
a single DELETE.

### 7.2 Repos

`MissionRepoPg` and `AssigneeRepoPg` are structs around `PgPool` (no
generics), implementing their respective domain ports. SQLx runtime
API (`sqlx::query_as`, `sqlx::QueryBuilder`) is used until the
workspace ships a `sqlx-data.json` cache.

`MissionRepoPg::create` opens a transaction: INSERT mission → INSERT
each assignee (`SQLSTATE 23505` → `DuplicateAssignee` /
`DuplicateMission`) → COMMIT → reload via `find_by_id` so the
returned `Mission` carries the hydrated assignee list.

`find_by_id` runs two queries (mission + assignees). `list_by_project`
and `list_by_user` use a single SELECT for the missions plus a
follow-up SELECT for assignees grouped by `mission_id`.

### 7.3 Service adapters

`adapter/service/project.rs`:

```rust
pub struct ProjectLookupImpl {
    projects: Arc<dyn apis::project::ProjectService>,
}

impl ProjectLookupImpl {
    pub fn new(projects: Arc<dyn apis::project::ProjectService>) -> Self;
}

#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> { /* NotFound → ProjectNotFound */ }
    async fn is_leader(&self, project_code: &str, user_code: &str) -> Result<bool, DomainError> {
        // get_project_by_code → view.members.leaders.iter().any(|u| u.code == user_code)
    }
}
```

`adapter/service/user.rs`: same shape, one method (`get_by_code`).

## 8. Facade

`adapter/facade/in_memory/service.rs`:

```rust
pub struct MissionServiceImpl<M, A, P, U> {
    usecase: MissionUsecase<M, A, P, U>,
}
```

Two constructors (mirror `CrfServiceImpl`):

```rust
impl<M, A, P, U> MissionServiceImpl<M, A, P, U>
where
    M: MissionRepository, A: AssigneeRepository,
    P: ProjectLookup,    U: UserLookup,
{
    pub fn from_usecase(usecase: MissionUsecase<M, A, P, U>) -> Self;
    pub fn from_repos(
        mission_repo: M, assignee_repo: A,
        projects: Arc<P>, users: Arc<U>,
    ) -> Self;
}
```

Every `MissionService` method does `usecase.x(...)` then
`.map(Into::into).map_err(map_usecase_to_api_error)`. The
`map_usecase_to_api_error` function translates every `UsecaseError`
variant to its `MissionApiError` counterpart:

| `UsecaseError`                                       | `MissionApiError`                                         |
| ---------------------------------------------------- | --------------------------------------------------------- |
| `Forbidden { user_code, project_code }`              | `Forbidden { user_code, project_code }`                   |
| `Domain(EmptyMissionCode)` / `EmptyUserCode` / `UnknownMissionKind(_)` / `UnknownMissionRole(_)` | `Validation(msg)` |
| `Domain(NotFound)`                                   | `NotFound`                                                |
| `Domain(AssigneeNotFound)`                           | `AssigneeNotFound`                                        |
| `Domain(ProjectNotFound(c))`                         | `ProjectNotFound(c)`                                      |
| `Domain(UserNotFound(c))`                            | `UserNotFound(c)`                                         |
| `Domain(DuplicateMission { … })`     | `DuplicateMission { … }`                  |
| `Domain(DuplicateAssignee { … })`   | `DuplicateAssignee { … }`                |
| `Domain(Repository(s))`                             | `Repository(s)`                                           |

## 9. Apis port (`apis::mission`)

```rust
pub enum MissionKind { Crf, Sdtm, Adam, Tfl }
pub enum MissionRole { Dev, Qc }

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
    Forbidden { user_code: String, project_code: String },
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

pub struct AssigneeView {
    pub id: i64,
    pub user_code: String,
    pub role: MissionRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct MissionView {
    pub id: i64,
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeView>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct CreateMissionRequest {
    pub project_code: String,
    pub mission_kind: MissionKind,
    pub mission_code: String,
    pub assignees: Vec<AssigneeData>,
}

pub struct AssigneeData {
    pub user_code: String,
    pub role: MissionRole,
}

pub struct ListMissionsByProjectRequest {
    pub project_code: String,
    pub kind: Option<MissionKind>,
}

pub struct ListMissionsByUserRequest {
    pub user_code: String,
}

/// Shared actor type for any port that authorizes on behalf of an
/// authenticated user. Built by the transport layer from the JWT
/// subject; passed to every write method.
pub struct Actor {
    pub user_code: String,
}

#[async_trait]
pub trait MissionService: Send + Sync {
    async fn create_mission(
        &self, actor: &Actor, req: CreateMissionRequest,
    ) -> Result<MissionView, MissionApiError>;
    async fn get_mission_by_id(
        &self, id: i64,
    ) -> Result<MissionView, MissionApiError>;
    async fn list_missions_by_project(
        &self, req: ListMissionsByProjectRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;
    async fn list_missions_by_user(
        &self, req: ListMissionsByUserRequest,
    ) -> Result<Vec<MissionView>, MissionApiError>;
    async fn delete_mission(
        &self, actor: &Actor, id: i64,
    ) -> Result<(), MissionApiError>;
    async fn add_assignee(
        &self, actor: &Actor, mission_id: i64, data: AssigneeData,
    ) -> Result<AssigneeView, MissionApiError>;
    async fn remove_assignee(
        &self, actor: &Actor, mission_id: i64, assignee_id: i64,
    ) -> Result<(), MissionApiError>;
}
```

`Actor` is the minimal authorization context: `user_code` is the
JWT subject. Future ports can add fields (role, token version, etc.)
without breaking existing call sites.

## 10. HTTP transport

`apps/server/aegis-server/src/transport/http/mission/{router.rs, handlers.rs}`.

### Router

```rust
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

Mounted at `/api/mission` by `transport::http::router::router()`.

### Routes

| Method | Path | Body | Authz |
| --- | --- | --- | --- |
| `POST` | `/api/mission` | `CreateMissionRequest` | leader of `req.project_code` |
| `GET` | `/api/mission/{id}` | — | any authed user |
| `GET` | `/api/mission/by-project/{project_code}?kind={kind}` | — | any authed user |
| `GET` | `/api/mission/by-user/{user_code}` | — | any authed user |
| `DELETE` | `/api/mission/{id}` | — | leader of `mission.project_code` |
| `POST` | `/api/mission/{id}/assignee` | `AssigneeDataRequest` | leader of `mission.project_code` |
| `DELETE` | `/api/mission/{id}/assignee/{assignee_id}` | — | leader of `mission.project_code` |

`kind` is optional. When absent, the by-project listing returns
every mission kind for the project.

For the three "mission-id-keyed" writes the handler must call
`state.mission.get_mission_by_id(id)` first to discover
`project_code`, then forward to the write method. The usecase
re-runs the leadership check; that inner check is the security
boundary.

### Wire DTOs

`apps/server/aegis-server/src/transport/http/dto.rs` gains
`CreateMissionRequest`, `AssigneeDataRequest`, `MissionViewResponse`,
`AssigneeViewResponse`, `MissionListResponse`, `PathId`,
`PathProjectCode`, `PathUserCode`, `PathMissionId`, `PathAssigneeId`,
plus wire enums `MissionKind` and `MissionRole` (with `From` /
`Into` for the apis types).

All wire types derive `Serialize + Deserialize + ToSchema`. JSON
field names use `camelCase` (`#[serde(rename_all = "camelCase")]`).

### Error mapping

`apps/server/aegis-server/src/transport/http/error.rs` gains:

```rust
#[error("{0}")]
Mission(#[from] apis::mission::MissionApiError),
```

`status()` / `code()` tables get the mission arm:

| Variant | Status | Code |
| --- | --- | --- |
| `Mission(Validation(_))` | 400 | `mission_validation_failed` |
| `Mission(NotFound)` | 404 | `mission_not_found` |
| `Mission(AssigneeNotFound)` | 404 | `assignee_not_found` |
| `Mission(ProjectNotFound)` | 404 | `project_not_found` |
| `Mission(UserNotFound)` | 404 | `user_not_found` |
| `Mission(Forbidden { .. })` | 403 | `mission_forbidden` |
| `Mission(DuplicateMission { .. })` | 409 | `mission_duplicate` |
| `Mission(DuplicateAssignee { .. })` | 409 | `assignee_duplicate` |
| `Mission(Repository(_))` | 500 | `mission_repository_error` |

### State

`AppState` gains `pub mission: Arc<dyn apis::mission::MissionService>`.
`state::test_support` gains a `NullMissionService` whose every method
`unimplemented!()`s.

### Wiring (`run.rs`)

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
state.mission = Arc::new(mission::MissionServiceImpl::from_usecase(mission_usecase));
```

### OpenAPI

`transport::http::openapi::ApiDoc` gains every new wire DTO in its
`components(schemas(...))` registry. Each handler is decorated with
`#[utoipa::path(...)]`.

## 11. Tests (per `lib-crate-development.md` §9)

In order:

1. **Domain unit tests** (`src/domain/tests.rs`):
   - `MissionKind::try_from` / `as_str` round-trip for every variant.
   - `MissionRole::try_from` / `as_str` round-trip.
   - `Mission::new` rejects empty `mission_code`.
   - `Assignee::new` rejects empty `user_code`.

2. **Adapter unit tests**:
   - `adapter/persistence/postgres/tests.rs`: row-bridge
     `TryFrom<MissionRow> for Mission` and
     `TryFrom<AssigneeRow> for Assignee`. Schema assertion via
     `std::fs::read_to_string(env!("CARGO_MANIFEST_DIR") + "/migrations/0001_create_missions.sql")`
     asserting presence of `missions_natural_key`,
     `missions_kind_check`, and `missions_set_updated_at`; same for
     `assignees`.
   - `adapter/service/project/tests.rs`:
     `ProjectLookupImpl::is_leader` returns `true` iff the test
     fixture has the user in `members.leaders`.
   - `adapter/service/user/tests.rs`:
     `UserLookupImpl::get_by_code` returns `Ok(())` for an existing
     user, `UserNotFound` otherwise.

3. **Facade unit tests** (`adapter/facade/in_memory/tests.rs`):
   - In-memory fakes for the four ports
     (`Arc<Mutex<Vec<Mission>>>`, `Arc<Mutex<Vec<Assignee>>>`,
     atomic counters for ids).
   - Exercise every `MissionService` method.
   - Assert object-safety (`Arc<dyn MissionService>` compiles),
     `Send + Sync`.
   - Leadership enforcement: a write by a non-leader yields
     `MissionApiError::Forbidden`.
   - Per-mission `(user_code, role)` uniqueness: adding the same
     pair twice yields `DuplicateAssignee`.
   - Cascade delete: deleting a mission wipes its assignees.

4. **`tests/public_api.rs`** (compile-only):
   - Every `pub use` path used in `run.rs` and `state.rs`.
   - Constructor-chain signature:
     `fn(PgPool) -> MissionRepo`,
     `fn(PgPool) -> AssigneeRepo`,
     `fn(Arc<dyn ProjectService>) -> ProjectLookupImpl`,
     `fn(Arc<dyn UserService>) -> UserLookupImpl`,
     `fn(MissionUsecaseConfig) -> MissionUsecase`,
     `fn(MissionUsecase<…>) -> MissionServiceImpl<…>`.
   - Trait bounds: `Arc<dyn apis::mission::MissionService>` compiles.

5. **`tests/integration_persistence.rs`** (`#[ignore]`, destructive):
   - `dotenvy::dotenv()`; panic on missing
     `AEGIS_MISSION_DATABASE_URL`.
   - Apply migrations; drop `assignees` + `missions` +
     `_sqlx_migrations`.
   - Atomic-counter + nanosecond timestamp for unique `mission_code`
     per run.
   - Round-trip: create mission with assignees → `find_by_id` →
     `list_by_project` → `list_by_user` → `delete` → list is empty.

## 12. Verification gate

Per `lib-crate-development.md` §11:

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

## 13. Commits

Per `lib-crate-development.md` §12 — one logical change per commit:

1. `feat(mission): scaffold crate + Cargo workspace wiring`
2. `feat(mission): domain layer (Mission, Assignee, ports, errors, tests)`
3. `feat(mission): persistence + service adapters + migrations + tests`
4. `feat(mission): usecase + facade + apis::mission port`
5. `feat(aegis-server): mission HTTP router + state + run + integration test`
6. `docs(mission): README + verification follow-up`

Lockfile drift gets its own `chore:` commit. Each commit message
lists the spec coverage and the verification commands at the
bottom.

## 14. Cross-references

- Conventions: [`docs/guidelines/lib-crate-development.md`](../../guidelines/lib-crate-development.md)
- Precedent: [`lib/crates/crf`](../../../lib/crates/crf) (multi-aggregate
  business lib crate with cross-crate `ProjectLookup`)
- Precedent: [`lib/crates/project`](../../../lib/crates/project) (auth
  via `ProjectView.members.leaders` and `UserService` adapter)