# mission

Lifecycle for project-scoped `Mission` aggregates and their
`Assignee` join rows. Each mission belongs to exactly one
project, carries a `MissionKind` (`crf` | `sdtm` | `adam` | `tfl`),
and names a human-readable `mission_code` (e.g. `CRF-V1`).
Assignees attach users to a mission under a `MissionRole`
(`dev` | `qc`); the `(user_code, role)` pair is unique within a
mission.

Write operations (create mission, delete mission, add assignee,
remove assignee) are gated on the caller being a *leader* of the
owning project — checked via the `project` business crate's
`ProjectLookup::is_leader`. There is no separate role gate; the
authorisation is purely project-membership-based.

This crate is a business lib crate; see
`docs/guidelines/lib-crate-development.md` for the cross-cutting
conventions (workspace wiring, DDD layout, error chain, the
five-tier test rule) and
`docs/superpowers/specs/2026-09-01-mission-crate-design.md` for
the data model + port surface.

## Source layout

    src/
    ├── lib.rs                                  # pub mod + re-exports
    ├── domain.rs                               # children, pub use
    ├── domain/
    │   ├── mission_kind.rs                     # Crf | Sdtm | Adam | Tfl
    │   ├── mission_role.rs                     # Dev | Qc
    │   ├── mission.rs                          # aggregate + DTOs
    │   ├── assignee.rs                         # aggregate + DTOs + uniqueness
    │   ├── project_lookup.rs                   # is_leader lookup port
    │   ├── user_lookup.rs                      # get_by_code lookup port
    │   ├── mission_lookup.rs                   # MissionRepository port
    │   ├── assignee_lookup.rs                  # AssigneeRepository port
    │   ├── error.rs                            # DomainError
    │   └── tests.rs                            # domain unit tests
    ├── usecase.rs
    ├── usecase/
    │   ├── commands.rs                         # Create*/AssigneeData DTOs
    │   ├── views.rs                            # *View DTOs
    │   ├── error.rs                            # UsecaseError + From<DomainError>
    │   ├── mission_usecase.rs                  # MissionUsecase<R, A, P, U>
    │   └── tests.rs                            # in-memory wire-up tests
    ├── adapter.rs
    ├── adapter/
    │   ├── facade/
    │   │   ├── facade.rs                       # MissionService facade module
    │   │   └── in_memory/
    │   │       ├── in_memory.rs                # module index
    │   │       └── service.rs                  # MissionServiceImpl backed by in-memory state
    │   ├── persistence.rs
    │   └── persistence/postgres/
    │       ├── postgres.rs                     # module index, re-exports
    │       ├── mission_repo.rs                 # MissionRepository impl
    │       ├── assignee_repo.rs                # AssigneeRepository impl
    │       └── row.rs                          # MissionRow + AssigneeRow
    ├── service/
    │   ├── project.rs                          # MissionProjectLookupImpl
    │   └── user.rs                             # MissionUserLookupImpl
    └── test_support.rs                         # in-memory fixtures for tests

## Database setup

Migrations live under `migrations/` and are applied via
`sqlx migrate run --source lib/crates/mission/migrations`.

The live-DB URL comes from the
`AEGIS_MISSION_DATABASE_URL` environment variable (or `.env` at
the workspace root).

```rust
use sqlx::postgres::PgPoolOptions;
use mission::{
    AssigneeRepo, MissionRepo, MissionServiceImpl, MissionUsecase, MissionUsecaseConfig,
};
use apis::project::{ProjectService, ProjectLookup};
use apis::user::UserService;
use std::sync::Arc;

let pool = PgPoolOptions::new()
    .connect(&std::env::var("AEGIS_MISSION_DATABASE_URL")?)
    .await?;

let mission_repo = MissionRepo::new(pool.clone());
let assignee_repo = AssigneeRepo::new(pool);

// ProjectLookup + UserLookup are thin adapters over the
// already-wired project + user services — see
// `mission::service::{project, user}` for the impls.
let project_lookup: Arc<dyn ProjectLookup> = Arc::new(project_service);
let user_lookup: Arc<dyn UserLookup> = Arc::new(user_service);

let usecase = MissionUsecase::new(MissionUsecaseConfig {
    mission_repo,
    assignee_repo,
    project_lookup,
    user_lookup,
});

let service: Arc<dyn apis::mission::MissionService> =
    Arc::new(MissionServiceImpl::from_usecase(usecase));
```

## Tests

```bash
cargo test -p mission                                  # unit + public_api
cargo test -p mission -- --ignored --test-threads=1    # when AEGIS_MISSION_DATABASE_URL is set
```

The live-DB integration tests are destructive on purpose: they
drop the live `missions`, `assignees`, and `_sqlx_migrations`
tables before each run.

## HTTP surface (aegis-server)

The mission crate ships only the business lib; HTTP routes live
in `apps/server/aegis-server/src/transport/http/mission/` and
are mounted at `/api/mission`. See that module for the URL map.

## Guideline

See `docs/guidelines/lib-crate-development.md` for the
cross-cutting conventions every lib crate in this workspace
follows.