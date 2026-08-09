# Project Crate Design

## Goal

Add `lib/crates/project` as a reusable Rust library that implements the new
`apis::project::ProjectService` port. The crate owns CRUD over the
`Product` and `Project` aggregates and the membership lists that hang off
`Project`.

The crate owns:

- The schema for `products`, `projects`, and `project_members` (two SQLx
  migrations).
- Validation rules for `Product` (non-empty `code` / `name`) and `Project`
  (non-empty `code`, must reference an existing `product_id`, member
  invariants below).
- A PostgreSQL-backed `ProductRepo` and `ProjectRepo`.
- A narrow `domain::UserService` port (just `get_by_code` and `list`) that
  the usecase uses to hydrate membership codes into user summaries. The
  concrete adapter delegates to `apis::user::UserService`.
- A single `ProjectUsecase<P, R, U>` (generic over both repositories and the
  user service) that orchestrates every CRUD operation and projects the
  domain aggregates into view DTOs.
- A `ProjectServiceImpl` facade that adapts `ProjectUsecase` to the apis
  `ProjectService` port.

The crate does **not** own user lifecycle. Users live in the `user` crate
behind `apis::user::UserService`; this crate depends on the `apis` crate, not
on the `user` crate directly.

## Architecture

Ports-and-adapters DDD structure, exactly mirroring
[`lib/crates/user/`](../../lib/crates/user/) and
[`lib/crates/auth/`](../../lib/crates/auth/):

- `domain` — `Product`, `Project`, `ProjectMember`, `TeamType`, `RoleType`,
  ports (`ProductRepository`, `ProjectRepository`, `UserService`), and
  `DomainError`. No I/O, no `sqlx`, no `tokio`. The narrow `UserService`
  port lives here so the domain never reaches the `apis` crate.
- `usecase` — `ProjectUsecase<P, R, U>`, command DTOs
  (`CreateProduct`, `UpdateProduct`, `CreateProject`, `UpdateProject`),
  view DTOs (`ProductView`, `ProjectView`, `ProjectMemberView`,
  `UserSummaryView`), `ProjectUsecaseConfig<P, R, U>`, and
  `UsecaseError`. Holds the user service as a private field. Generic over
  both repository ports and the user service so tests inject in-memory
  fakes.
- `adapter` — concrete implementations of the domain ports.
  - `adapter/persistence/postgres/` — `ProductRepo` and `ProjectRepo`
    backed by `sqlx::PgPool`. `ProjectRepo::create` / `update` write the
    `projects` row and the `project_members` rows in a single
    transaction; member updates are delete-then-insert of that project's
    rows in the same transaction.
  - `adapter/service/user/` — `UserServiceImpl` adapts
    `apis::user::UserService` to the domain `UserService` (mirrors
    [`auth::adapter::service::user`](../../lib/crates/auth/src/adapter/service/user.rs)).
  - `adapter/facade/in_memory/` — `ProjectServiceImpl<P, R, U>` adapts
    `ProjectUsecase<P, R, U>` to `apis::project::ProjectService`.

Per [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md):
no `mod.rs`; each top-level module uses `src/<module>.rs` + `src/<module>/`.
The terminal leaf modules (`role.rs`, `product_repo.rs`, `service.rs`, …)
are leaf files with no companion directory.

## Data Model

```rust
// domain/product.rs
pub struct Product {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub description: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/project.rs
pub struct Project {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/project_member.rs
pub struct ProjectMember {
    pub leaders: Vec<String>,   // user codes
    pub workers: Vec<String>,   // user codes
}

pub enum TeamType { Members, UnblindMembers }
pub enum RoleType { Leader, Worker }

// domain/user.rs — narrow port (this crate never reaches apis::user directly)
#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
    async fn list(&self) -> Result<Vec<UserSummary>, DomainError>;
}
pub struct UserSummary { pub code: String, pub name: String }
```

`Product` and `Project` both carry `active bool` (soft-delete), parallel to
the `user` crate's posture — no hard `DELETE` from the application.

`TeamType` / `RoleType` are stored as strings (`"members"` /
`"unblind_members"` / `"leader"` / `"worker"`), with `TryFrom<&str>` as the
single source of truth and a `CHECK` constraint at the database.

## ProjectMember invariants

- `ProjectMember::new` rejects duplicate user codes within `leaders`.
- `ProjectMember::new` rejects duplicate user codes within `workers`.
- `ProjectMember::new` allows a code to appear in **both** `leaders` and
  `workers` of the same team (a leader can also do worker work).
- A user **may** appear in both `members` and `unblind_members` of the
  same project (the two teams are independent).
- Across the four sets of a project there is no uniqueness check beyond
  the above; the application does not enforce "user must be a member
  before becoming unblinded".

These rules keep the join-table PK `(project_id, team_type, role_type,
user_code)` honest: the same `user_code` may legitimately appear in
multiple rows.

## Database Schema

Two migrations under `migrations/`:

### `0001_create_products.sql`

```sql
CREATE TABLE products (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT products_code_unique UNIQUE (code)
);

CREATE OR REPLACE FUNCTION products_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER products_set_updated_at
    BEFORE UPDATE ON products
    FOR EACH ROW
    EXECUTE FUNCTION products_set_updated_at();
```

### `0002_create_projects.sql`

```sql
CREATE TABLE projects (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    code TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    product_id INTEGER NOT NULL,
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT projects_code_unique UNIQUE (code),
    CONSTRAINT projects_product_fk FOREIGN KEY (product_id)
        REFERENCES products(id)
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

`updated_at` is auto-managed by the trigger on every UPDATE. The
`project_members` PK is the composite — it implicitly enforces "no
duplicate row in the same set"; the `CHECK` constraints match the Rust
enums.

## Repository Ports

```rust
// domain/product.rs
#[async_trait]
pub trait ProductRepository: Send + Sync {
    async fn create(&self, input: ProductNew) -> Result<Product, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Product, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Product, DomainError>;
    async fn list(&self) -> Result<Vec<Product>, DomainError>;
    async fn update(&self, input: ProductUpdate) -> Result<Product, DomainError>;
}

pub struct ProductNew  { pub code: String, pub name: String, pub description: String }
pub struct ProductUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
    pub active: Option<bool>,
}

// domain/project.rs
#[async_trait]
pub trait ProjectRepository: Send + Sync {
    async fn create(&self, input: ProjectNew) -> Result<Project, DomainError>;
    async fn find_by_id(&self, id: i32) -> Result<Project, DomainError>;
    async fn find_by_code(&self, code: &str) -> Result<Project, DomainError>;
    async fn list(&self) -> Result<Vec<Project>, DomainError>;
    async fn update(&self, input: ProjectUpdate) -> Result<Project, DomainError>;
}

pub struct ProjectNew {
    pub code: String,
    pub description: String,
    pub product_id: i32,
    pub members: ProjectMember,
    pub unblind_members: ProjectMember,
}
pub struct ProjectUpdate {
    pub id: i32,
    pub code: Option<String>,
    pub description: Option<String>,
    pub product_id: Option<i32>,
    pub active: Option<bool>,
    /// `None` = leave that team unchanged; `Some(vec)` = replace that team's
    /// membership rows atomically.
    pub members: Option<ProjectMember>,
    pub unblind_members: Option<ProjectMember>,
}
```

`ProjectRepo::create` opens a transaction, inserts the `projects` row,
inserts the `project_members` rows (four small `INSERT` loops keyed by
the four `(team_type, role_type)` combinations), and commits. `update`
does the same shape: `UPDATE` the `projects` row, then for each `Some`
team wipe-and-rewrite that team's rows in the same transaction.

`map_db_error` translates SQLSTATE `23503` on `product_id` into
`DomainError::ProductNotFound(product_id)`, SQLSTATE `23505` into
`DomainError::DuplicateCode(constraint_name)` (mirrors the user crate).

## Usecase Layer

```rust
// usecase/project_usecase.rs
pub struct ProjectUsecaseConfig<P: ProductRepository, R: ProjectRepository, U: UserService> {
    pub product_repo: P,
    pub project_repo: R,
    pub users: U,
}

pub struct ProjectUsecase<P: ProductRepository, R: ProjectRepository, U: UserService> {
    product_repo: P,
    project_repo: R,
    users: U,
}

impl<P, R, U> ProjectUsecase<P, R, U> {
    pub fn new(cfg: ProjectUsecaseConfig<P, R, U>) -> Self { /* store fields */ }

    // Products
    pub async fn create_product(&self, cmd: CreateProduct) -> Result<ProductView, UsecaseError>
    pub async fn get_product_by_id(&self, id: i32) -> Result<ProductView, UsecaseError>
    pub async fn get_product_by_code(&self, code: &str) -> Result<ProductView, UsecaseError>
    pub async fn list_products(&self) -> Result<Vec<ProductView>, UsecaseError>
    pub async fn update_product(&self, cmd: UpdateProduct) -> Result<ProductView, UsecaseError>

    // Projects
    pub async fn create_project(&self, cmd: CreateProject) -> Result<ProjectView, UsecaseError>
    pub async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, UsecaseError>
    pub async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, UsecaseError>
    pub async fn list_projects(&self) -> Result<Vec<ProjectView>, UsecaseError>
    pub async fn update_project(&self, cmd: UpdateProject) -> Result<ProjectView, UsecaseError>
}
```

`ProjectUsecase::new(cfg)` uses the `*Config` rule from the guideline
because the constructor takes three arguments.

`ProjectUsecase::create_product` validates `code` and `name` are
non-empty, then forwards to `ProductRepo::create`. `update_product`
validates any `Some` fields the same way.

`ProjectUsecase::create_project` validates `code` is non-empty, looks up
the referenced product via `ProductRepo::find_by_id` to surface
`ProductNotFound` early (the FK would catch it later, but failing early
gives a clearer error path). Membership validation in the domain
(`ProjectMember::new`) runs before any repository call.

`ProjectUsecase::get_project_*` resolves the view in three steps:
read project row → `ProductRepo::find_by_id` to fill `ProjectView::product`
→ `UserService::list` (single batch) and bucket the returned
`UserSummary` codes into the four membership sets. Any referenced code
that the user service did not return becomes
`UsecaseError::Repository(DomainError::UserNotFound(code))`. `list_projects`
performs the same hydration per project — the user service is called once
per `list_projects` invocation (not per project), the product lookup is
batched via the in-memory fake in tests and one row at a time in
PostgreSQL (acceptable for the initial scope; a batched product fetch
can be added later without breaking the trait).

`update_project` validates inputs, applies the metadata update, and (for
each `Some` membership field) replaces that team's rows via the
`ProjectRepo::update` transaction. Returns the hydrated view.

### UsecaseError

```rust
#[derive(Debug, thiserror::Error)]
pub enum UsecaseError {
    #[error("validation failed: {0}")]
    Validation(#[source] DomainError),

    #[error("repository error: {0}")]
    Repository(#[source] DomainError),
}

impl From<DomainError> for UsecaseError {
    fn from(err: DomainError) -> Self {
        // Validation errors that originated upstream of the repository
        // already came through `UsecaseError::Validation`; everything
        // else surfaces as `Repository`.
        UsecaseError::Repository(err)
    }
}
```

## apis Port

New file `lib/crates/apis/src/project.rs`:

```rust
#[derive(Debug, Clone, thiserror::Error)]
pub enum ProjectApiError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("product not found: {0}")]
    ProductNotFound(String),
    #[error("user not found: {0}")]
    UserNotFound(String),
    #[error("code already exists: {0}")]
    DuplicateCode(String),
    #[error("repository error: {0}")]
    Repository(String),
}

pub struct ProductView { /* id, code, name, description, active, created_at, updated_at */ }

pub struct ProjectView {
    pub id: i32,
    pub code: String,
    pub description: String,
    pub product: ProductView,
    pub members: ProjectMemberView,
    pub unblind_members: ProjectMemberView,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub struct ProjectMemberView {
    pub leaders: Vec<UserSummaryView>,
    pub workers: Vec<UserSummaryView>,
}

pub struct UserSummaryView { pub code: String, pub name: String }

// Request DTOs mirror the usecase command DTOs.
pub struct CreateProductRequest { pub code, pub name, pub description }
pub struct UpdateProductRequest { pub id, pub code?, pub name?, pub description?, pub active? }
pub struct CreateProjectRequest { pub code, pub description, pub product_id, pub members, pub unblind_members }
pub struct ProjectMemberData { pub leaders: Vec<String>, pub workers: Vec<String> }
pub struct UpdateProjectRequest {
    pub id,
    pub code?, pub description?, pub product_id?, pub active?,
    pub members: Option<ProjectMemberData>,
    pub unblind_members: Option<ProjectMemberData>,
}

#[async_trait]
pub trait ProjectService: Send + Sync {
    // Products
    async fn create_product(&self, CreateProductRequest) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_id(&self, id: i32) -> Result<ProductView, ProjectApiError>;
    async fn get_product_by_code(&self, code: &str) -> Result<ProductView, ProjectApiError>;
    async fn list_products(&self) -> Result<Vec<ProductView>, ProjectApiError>;
    async fn update_product(&self, UpdateProductRequest) -> Result<ProductView, ProjectApiError>;

    // Projects
    async fn create_project(&self, CreateProjectRequest) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_id(&self, id: i32) -> Result<ProjectView, ProjectApiError>;
    async fn get_project_by_code(&self, code: &str) -> Result<ProjectView, ProjectApiError>;
    async fn list_projects(&self) -> Result<Vec<ProjectView>, ProjectApiError>;
    async fn update_project(&self, UpdateProjectRequest) -> Result<ProjectView, ProjectApiError>;
}
```

The `apis` crate's `lib.rs` adds `pub mod project;`. The `ProjectServiceImpl`
in this crate translates `UsecaseError → ProjectApiError` at the facade
boundary.

## Public API

Constructors match:

```rust
use std::sync::Arc;
use project::{
    ProjectRepo, ProductRepo, ProjectServiceImpl, ProjectUsecase, ProjectUsecaseConfig,
};
use auth::UserServiceImpl; // or the in-memory UserServiceImpl from project::adapter::service::user

let product_repo = ProductRepo::new(pool.clone());
let project_repo = ProjectRepo::new(pool.clone());
let users: Arc<dyn project::UserService> = Arc::new(
    project::adapter::service::user::UserServiceImpl::new(arc_user_service_from_user_crate),
);

let usecase = ProjectUsecase::new(ProjectUsecaseConfig {
    product_repo,
    project_repo,
    users,
});

let project_service: Arc<dyn apis::project::ProjectService> =
    Arc::new(ProjectServiceImpl::new(usecase));
```

`ProjectUsecaseConfig<P, R, U>` is a plain struct with `pub` fields — no
builder ceremony. Generic over both repository types and the user service
so the field types stay concrete.

The crate root (`lib.rs`) re-exports the documented public surface:
`Product`, `Project`, `ProjectMember`, `TeamType`, `RoleType`,
`ProductRepository`, `ProjectRepository`, `UserService`, `UserSummary`,
`DomainError`, `ProductNew`, `ProductUpdate`, `ProjectNew`,
`ProjectUpdate`, `CreateProduct`, `UpdateProduct`, `CreateProject`,
`UpdateProject`, `ProductView`, `ProjectView`, `ProjectMemberView`,
`UserSummaryView`, `ProjectUsecase`, `ProjectUsecaseConfig`,
`UsecaseError`, `ProductRepo`, `ProjectRepo`, `UserServiceImpl`,
`ProjectServiceImpl`. Consumers write `use project::*`; they never reach
into the sub-modules.

## Workspace Wiring

- Add `lib/crates/project` to root `Cargo.toml` `[workspace].members`.
- `project/Cargo.toml` inherits every dep via `{ workspace = true }`:
  `sqlx`, `tokio`, `async-trait`, `thiserror`, `chrono`, plus a
  path-dep on `apis = { path = "../apis" }`. The `chrono` and `apis`
  deps get one-line comments explaining why they exist.
- `dev-dependencies` add `dotenvy`, `sqlx`, and `tokio` (the last two
  because the integration tests construct their own `PgPool` and run
  `#[tokio::test]`).
- `[dev-dependencies]` does **not** include the live-DB integration
  test crate-only; the integration test lives under `tests/`.

## Tests

Following the guideline's tier order:

1. **Domain unit tests** (`src/domain/tests.rs`)
   - `Product::new` rejects empty `code`, empty `name`.
   - `Project::new` rejects empty `code` and `product_id == 0`.
   - `ProjectMember::new` rejects duplicates within `leaders` and within
     `workers`; allows a code across both sets and across teams.
   - `TeamType::try_from` and `RoleType::try_from` parse the four known
     string values; reject unknowns.

2. **Adapter unit tests** (`src/adapter/persistence/postgres/tests.rs`)
   - Load both migration files via
     `std::fs::read_to_string(env!("CARGO_MANIFEST_DIR"))` and assert:
     - `products` columns (`id`, `code`, `name`, `description`, `active`,
       `created_at`, `updated_at`), UNIQUE on `code`, trigger
       `products_set_updated_at`.
     - `projects` columns, FK to `products(id)`, UNIQUE on `code`,
       trigger `projects_set_updated_at`.
     - `project_members` PK `(project_id, team_type, role_type,
       user_code)`, the two CHECK constraints.
   - `ProductRow → Product` and `ProjectRow → Project` `TryFrom` tests.

3. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`)
   - In-memory `ProductRepo` (Arc<Mutex<Vec<Product>>> + AtomicI32).
   - In-memory `ProjectRepo` (products + membership maps behind a
     `Mutex`).
   - In-memory `UserService` returning a fixed `Vec<UserSummary>`.
   - Wire `ProjectServiceImpl::new(ProjectUsecase::new(cfg))` on top of
     the three fakes.
   - Cases: create/get/list/update product; create/get/list/update
     project with full membership hydration; `UserNotFound` when a
     member code is unknown; `ProductNotFound` when `product_id` FK
     misses; membership replacement on update; full-list membership
     delivery on read.
   - `Box<dyn ProjectService>` compiles; `Send + Sync` is asserted in a
     compile-only test.

4. **`tests/public_api.rs`** — compile-only.
   - Names every documented consumer import.
   - Pins the constructor chain:
     `fn(PgPool) -> _` for both `ProductRepo::new` and `ProjectRepo::new`,
     `fn(ProjectUsecaseConfig<P, R, U>) -> _` for `ProjectUsecase::new`,
     `fn(ProjectUsecase<P, R, U>) -> _` for `ProjectServiceImpl::new`.

5. **`tests/integration_persistence.rs`** — live PG round-trips.
   - `#[ignore]`-gated.
   - Loads `.env` via `dotenvy::dotenv()`, reads
     `AEGIS_PROJECT_DATABASE_URL` (panic with a clear message if
     missing).
   - Drops `products`, `projects`, `project_members`,
     `_sqlx_migrations` before each run, then applies migrations via
     `sqlx::migrate!("./migrations")`.
   - Per-run unique codes via an atomic counter + wall-clock nanos so
     concurrent runs do not collide on the UNIQUE constraint.

## README

`lib/crates/project/README.md` covers:

- One-sentence purpose (product + project + project members CRUD).
- A `src/` tree matching the actual module shape.
- Database setup: `sqlx migrate run --source lib/crates/project/migrations`
  + `AEGIS_PROJECT_DATABASE_URL` env var + a small constructor snippet
  showing `ProjectServiceImpl::new(ProjectUsecase::new(cfg))`.
- How to run the ignored tests
  (`cargo test -p project -- --ignored`).
- A back-link to the guideline.

## Verification Gate

```bash
cargo fmt --all -- --check
cargo clippy -p project --all-targets --all-features -- -D warnings
cargo test -p project
cargo doc -p project --no-deps
cargo test -p project -- --ignored --test-threads=1   # when AEGIS_PROJECT_DATABASE_URL is set
```

Plus `cargo check --workspace` / `cargo clippy --workspace` since the
`apis` crate gets a new file. If unrelated workspace members fail because
of system libraries, document that rather than working around it.

## Commits

One commit per logical change (matching the guideline):

1. **scaffold** — register crate, basic `Cargo.toml`, empty `lib.rs`.
2. **domain** — aggregates, value objects, ports, `DomainError`, domain
   tests.
3. **apis** — `apis::project` port, types, error; `apis::lib.rs` re-export.
4. **usecase** — `ProjectUsecase`, command / view DTOs, `UsecaseError`,
   usecase tests with in-memory repository fakes.
5. **persistence** — migrations, `ProductRepo`, `ProjectRepo`,
   `*Row` bridges, postgres unit tests + integration tests
   (`#[ignore]`-gated).
6. **service** — `adapter::service::user::UserServiceImpl` adapting
   `apis::user::UserService` to the domain `UserService`.
7. **facade** — `ProjectServiceImpl` adapting `ProjectUsecase` to the
   apis `ProjectService`, with in-memory facade tests.
8. **public_api** — `tests/public_api.rs` compile-only test.
9. **readme** — `README.md` at the crate root.
10. **chore: lockfile** — `Cargo.lock` drift after new deps land.

Each commit message lists the spec coverage and the verification commands
at the bottom so reviewers can run the same gate locally.