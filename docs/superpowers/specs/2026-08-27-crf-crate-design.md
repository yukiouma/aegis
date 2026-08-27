# CRF Crate Design

## Goal

Add `lib/crates/crf` as a reusable Rust library that implements the new
`apis::crf::CrfService` outbound port. The crate owns CRUD + version-scoped
search over the Case Report Form aggregates (`CrfVersion`, `CrfForm`,
`CrfItem`, `CrfOption`, `CrfUnit`, `DomainAnnotation`, `Annotation`).

The crate owns:

- The schema for seven `crf_*` tables (eight migrations including the
  polymorphic-annotation shape).
- Validation rules for each aggregate (non-empty fields, `CrfItemKind` CHECK).
- A PostgreSQL-backed `*RepoPg` per aggregate.
- A narrow `domain::ProjectLookup` port (just `get_by_code`) that the usecase
  uses to validate `CrfVersion.project_code` against the `project` crate. The
  concrete adapter delegates to `apis::project::ProjectService`.
- A transactional `domain::CrfBulkFormRepository` port (plus the
  `CrfBulkFormRepoPg` adapter) that atomically inserts a form, every
  item, and each item's options + units in a single
  `pool.begin()` transaction. The port stays free of `sqlx` types.
- A single `CrfUsecase<V, F, I, O, U, Da, A, P, B>` generic over all nine
  ports (seven repos + project lookup + bulk form) that orchestrates
  every CRUD operation, the version-scoped search, the bulk form
  creation, and projects domain aggregates into view DTOs.
- A `CrfServiceImpl` in-memory facade that adapts `CrfUsecase` to the apis
  `CrfService` port.

The crate does **not** own project lifecycle. Projects live in the `project`
crate behind `apis::project::ProjectService`; this crate depends on the `apis`
crate, not on `project` directly.

## Architecture

Ports-and-adapters DDD structure, mirroring
[`lib/crates/domain-model/`](../../lib/crates/domain-model/) and
[`lib/crates/project/`](../../lib/crates/project/):

- `domain` — value objects (`CrfItemKind`), aggregates (one per table +
  `*New` / `*Update` DTOs), ports (`CrfVersionRepository`,
  `CrfFormRepository`, `CrfItemRepository`, `CrfOptionRepository`,
  `CrfUnitRepository`, `DomainAnnotationRepository`,
  `AnnotationRepository`), cross-crate port (`ProjectLookup`), and
  `DomainError`. No I/O, no `sqlx`, no `tokio`. The narrow `ProjectLookup`
  port lives here so the domain never reaches the `apis` crate.
- `usecase` — `CrfUsecase<V, F, I, O, U, Da, A, P, B>`, command DTOs (one
  `Create*` / `Update*` per aggregate plus `CreateAnnotation`),
  view DTOs (`CrfVersionView`, `CrfFormView`, `CrfItemView`,
  `CrfOptionView`, `CrfUnitView`, `DomainAnnotationView`,
  `AnnotationView`, `AnnotationOwner`), `CrfUsecaseConfig`, and
  `UsecaseError`. Holds the project lookup as a private field. Generic
  over all ports so tests inject in-memory fakes.
- `adapter` — concrete implementations of the domain ports.
  - `adapter/persistence/postgres/` — one `*RepoPg` per repository port
    backed by `sqlx::PgPool`. SQLx runtime API throughout (no compile-time
    macros — the workspace ships no `sqlx-data.json` cache).
  - `adapter/service/project/` — `ProjectLookupImpl` adapts
    `apis::project::ProjectService` to the domain `ProjectLookup`.
  - `adapter/facade/in_memory/` — `CrfServiceImpl` adapts
    `CrfUsecase` to `apis::crf::CrfService`.

Per [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md):
**no `mod.rs`** — each top-level module uses `src/<module>.rs` +
`src/<module>/`. The terminal leaf modules (`crf_version_repo.rs`,
`crf_form_repo.rs`, …) are leaf files with no companion directory.

### Module tree

```
crf/
├── Cargo.toml
├── README.md
├── migrations/                          (one .sql per table)
├── src/
│   ├── lib.rs                           # three layer mods + crate re-exports
│   ├── domain.rs                        # ports, aggregates, value objects, errors
│   ├── domain/
│   │   ├── error.rs
│   │   ├── project_lookup.rs            # narrow port wrapping ProjectService
│   │   ├── crf_item_kind.rs
│   │   ├── crf_version.rs               # aggregate + New/Update + port
│   │   ├── crf_form.rs
│   │   ├── crf_item.rs
│   │   ├── crf_option.rs
│   │   ├── crf_unit.rs
│   │   ├── domain_annotation.rs
│   │   ├── annotation.rs                # aggregate + polymorphic constructors
│   │   ├── crf_bulk_form.rs             # bulk port + CrfBulkCreateForm input + validate_bulk_create
│   │   └── tests.rs
│   ├── usecase.rs
│   ├── usecase/
│   │   ├── error.rs
│   │   ├── commands.rs                  # Create*/Update* DTOs + CreateCrfBulkForm/CreateCrfBulkItem
│   │   ├── views.rs                     # View DTOs + From impls + CrfBulkFormResult
│   │   ├── crf_usecase.rs               # the orchestrator (incl. create_bulk_form)
│   │   └── tests.rs                     # in-memory fakes + InMemoryBulkForms + bulk_create_form tests
│   ├── adapter.rs
│   └── adapter/
│       ├── persistence.rs               # pub mod postgres
│       ├── persistence/postgres.rs      # pub mod {crf_version_repo, ..., crf_bulk_form_repo}
│       ├── persistence/postgres/crf_version_repo.rs
│       ├── persistence/postgres/crf_form_repo.rs
│       ├── persistence/postgres/crf_item_repo.rs
│       ├── persistence/postgres/crf_option_repo.rs
│       ├── persistence/postgres/crf_unit_repo.rs
│       ├── persistence/postgres/domain_annotation_repo.rs
│       ├── persistence/postgres/annotation_repo.rs
│       ├── persistence/postgres/crf_bulk_form_repo.rs  # transactional bulk insert
│       ├── service.rs                   # pub mod project
│       ├── service/project.rs           # pub mod project_lookup_impl
│       ├── service/project/project_lookup_impl.rs
│       ├── facade.rs                    # pub mod in_memory
│       ├── facade/in_memory.rs          # mod service; #[cfg(test)] mod tests;
│       ├── facade/in_memory/service.rs  # CrfServiceImpl (incl. bulk_create_form)
│       └── facade/in_memory/tests.rs    # facade_bulk_create_form_* tests
└── tests/
    ├── public_api.rs
    └── integration_persistence.rs
```

## Data Model

```rust
// domain/crf_version.rs
pub struct CrfVersion {
    pub id: i64,
    pub project_code: String,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/crf_form.rs
pub struct CrfForm {
    pub id: i64,
    pub version_id: i64,
    pub code: String,
    pub name: String,
    pub order: i64,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/crf_item.rs
pub struct CrfItem {
    pub id: i64,
    pub form_id: i64,
    pub code: String,
    pub name: String,
    pub kind: CrfItemKind,
    pub order: i64,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/crf_option.rs
pub struct CrfOption {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/crf_unit.rs
pub struct CrfUnit {
    pub id: i64,
    pub item_id: i64,
    pub value: String,
    pub not_submitted: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/domain_annotation.rs
pub struct DomainAnnotation {
    pub id: i64,
    pub form_id: i64,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// domain/annotation.rs
pub struct Annotation {
    pub id: i64,
    pub domain_annotation_id: i64,
    pub content: String,
    pub assign: bool,
    pub owner: AnnotationOwner,    // {Form(i64), Item(i64), Option(i64), Unit(i64)}
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub enum CrfItemKind { Text, Selection, Checkbox, Datetime, Label }
```

All aggregates carry `created_at` / `updated_at` per the workspace convention.

## Schema

```
crf_versions
  id            BIGSERIAL PRIMARY KEY
  project_code  TEXT NOT NULL -- app-level validated against projects.code
  name          TEXT NOT NULL
  created_at, updated_at                       -- auto + BEFORE UPDATE trigger
  UNIQUE (project_code, name)

crf_forms
  id             BIGSERIAL PRIMARY KEY
  version_id     BIGINT NOT NULL REFERENCES crf_versions(id) ON DELETE CASCADE
  code           TEXT NOT NULL
  name           TEXT NOT NULL
  order          INT NOT NULL DEFAULT 0
  not_submitted  BOOLEAN NOT NULL DEFAULT FALSE
  created_at, updated_at
  UNIQUE (version_id, code)

crf_items
  id             BIGSERIAL PRIMARY KEY
  form_id        BIGINT NOT NULL REFERENCES crf_forms(id) ON DELETE CASCADE
  code           TEXT NOT NULL
  name           TEXT NOT NULL
  kind           TEXT NOT NULL
  order          INT NOT NULL DEFAULT 0
  not_submitted  BOOLEAN NOT NULL DEFAULT FALSE
  created_at, updated_at
  UNIQUE (form_id, code)
  CHECK (kind IN ('Text','Selection','Checkbox','Datetime','Label'))

crf_options
  id             BIGSERIAL PRIMARY KEY
  item_id        BIGINT NOT NULL REFERENCES crf_items(id) ON DELETE CASCADE
  value          TEXT NOT NULL
  not_submitted  BOOLEAN NOT NULL DEFAULT FALSE
  created_at, updated_at

crf_units
  id             BIGSERIAL PRIMARY KEY
  item_id        BIGINT NOT NULL REFERENCES crf_items(id) ON DELETE CASCADE
  value          TEXT NOT NULL
  not_submitted  BOOLEAN NOT NULL DEFAULT FALSE
  created_at, updated_at

crf_domain_annotations
  id             BIGSERIAL PRIMARY KEY
  form_id        BIGINT NOT NULL REFERENCES crf_forms(id) ON DELETE CASCADE
  name           TEXT NOT NULL
  description    TEXT NOT NULL DEFAULT ''
  created_at, updated_at
  UNIQUE (form_id, name)

crf_annotations                               -- polymorphic: owned by exactly one of form/item/option/unit
  id                    BIGSERIAL PRIMARY KEY
  form_id               BIGINT REFERENCES crf_forms(id)    ON DELETE CASCADE
  item_id               BIGINT REFERENCES crf_items(id)    ON DELETE CASCADE
  option_id             BIGINT REFERENCES crf_options(id)  ON DELETE CASCADE
  unit_id               BIGINT REFERENCES crf_units(id)    ON DELETE CASCADE
  domain_annotation_id  BIGINT NOT NULL REFERENCES crf_domain_annotations(id) ON DELETE RESTRICT
  content               TEXT NOT NULL
  assign                BOOLEAN NOT NULL DEFAULT FALSE
  created_at, updated_at
  CHECK ((form_id IS NOT NULL)::int + (item_id IS NOT NULL)::int
       + (option_id IS NOT NULL)::int + (unit_id IS NOT NULL)::int = 1)
```

One `BEFORE UPDATE` trigger per table refreshes `updated_at` to `NOW()` so every
code path (direct SQL, ad-hoc psql, future pgBouncer proxies) is covered.

`DomainAnnotation` belongs to exactly one form (`form_id` FK). It
deliberately carries no `version_id`: a form is the unique parent, and
version_id is reachable through `crf_forms.version_id` whenever a
version-scoped query needs to join. `UNIQUE (form_id, name)` enforces
"label names are unique within a form" — the natural key for a form-scoped
label pool.

## List strategy

| Method | Scope | Nesting |
| --- | --- | --- |
| `list_versions_by_project` | all versions of a project | none |
| `list_forms_by_version` | all forms of a version | none |
| `list_items_by_form` | all items of a form | **full tree** (item → options / units / annotations / domain-annotation label) |
| `list_options_by_item` / `list_units_by_item` | all options / units of an item | none |
| `list_domain_annotations_by_form` | all annotations attached to a form | none |
| `list_annotations_by_form / by_item / by_option / by_unit` | annotations on one owner | none |

`list_items_by_form` hydrates the full tree with **four** round-trips
(`crf_items`, `crf_options`, `crf_units`, `crf_annotations` for the
collected item IDs) and assembles the tree in memory. This is much cheaper
than a JOIN-per-row and keeps the per-entity queries cacheable.

## Search strategy (version-scoped, ILIKE)

`fragment` is required, non-empty (`fragment.trim().is_empty()` →
`UsecaseError::Validation(DomainError::EmptySearchFragment)`).

| Method | Fields | Scoping |
| --- | --- | --- |
| `search_forms_by_version` | `code, name` | `WHERE version_id = $1` |
| `search_items_by_version` | `code, name` | `WHERE form_id IN (SELECT id FROM crf_forms WHERE version_id = $1)` |
| `search_options_by_version` | `value` | `WHERE item_id IN (SELECT id FROM crf_items WHERE form_id IN (SELECT id FROM crf_forms WHERE version_id = $1))` |
| `search_units_by_version` | `value` | same path as options |
| `search_domain_annotations_by_version` | `name, description` | `JOIN crf_forms ON crf_domain_annotations.form_id = crf_forms.id WHERE crf_forms.version_id = $1` |
| `search_annotations_by_version` | `content` | UNION of all four owner types where the chain reaches the version |

Search uses `ILIKE '%' || $2 || '%'` (case-insensitive). No FTS — keeps the
spec aligned with the data model and avoids GIN index management. A future
migration to `tsvector` is additive.

## Cross-crate: project validation

`CrfVersion::new` and the `CrfUsecase::create_version` orchestration
both rely on `ProjectLookup::get_by_code`:

```rust
// domain/project_lookup.rs
#[async_trait]
pub trait ProjectLookup: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError>;
}
```

```rust
// adapter/service/project/project_lookup_impl.rs
pub struct ProjectLookupImpl {
    projects: Arc<dyn apis::project::ProjectService>,
}
#[async_trait]
impl ProjectLookup for ProjectLookupImpl {
    async fn get_by_code(&self, code: &str) -> Result<(), DomainError> {
        match self.projects.get_project_by_code(code).await {
            Ok(_) => Ok(()),
            Err(apis::project::ProjectApiError::NotFound) =>
                Err(DomainError::ProjectNotFound(code.to_string())),
            Err(e) => Err(DomainError::Repository(e.to_string())),
        }
    }
}
```

Only `create_version` validates; `update_version` does not (it does not
touch `project_code`). Mirrors the pattern used by
`project::adapter::service::user::UserServiceImpl`.

`get_by_code` returns `Result<(), DomainError>` (not the full project data)
because the usecase only needs an existence check; carrying the project
view across the port would couple the crf crate to the project crate's
DTOs. Mirrors the same minimal-surface decision behind
`project::domain::UserService::get_by_code` returning `UserSummary`
rather than a full user record.

## Kind-shape validation

`CrfUsecase::create_item` (and `update_item` when `code` or `kind`
change) enforces:

- `Selection | Checkbox` ⇒ at least one option must exist on the item
  **before** the insert returns success. The usecase inserts the item,
  then counts `crf_options` where `item_id = $1`; if 0, deletes the item
  row and returns `UsecaseError::Repository(DomainError::KindShapeViolation
  { kind, field: "options" })`.
- `Text | Datetime | Label` ⇒ no option may exist on the item; this is
  enforced on `update_item` (no create path attaches options at create
  time). Returns `KindShapeViolation { kind, field: "options" }` if any
  option exists.
- `Datetime` ⇒ no `unit`-kind restriction (items can carry multiple
  units via `crf_units`, one row per unit, so no DB-level uniqueness).

This is the boundary check that catches malformed CRUD requests before
they reach the wire.

## Public API surface at the crate root

```rust
pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::facade::in_memory::CrfServiceImpl;
pub use adapter::persistence::postgres::{
    AnnotationRepoPg, CrfBulkFormRepoPg, CrfFormRepoPg, CrfItemRepoPg,
    CrfOptionRepoPg, CrfUnitRepoPg, CrfVersionRepoPg, DomainAnnotationRepoPg,
};
pub use adapter::service::project::ProjectLookupImpl;
pub use domain::{
    Annotation, AnnotationOwner, CrfApiError as _, /* never — error stays in apis */
    CrfBulkCreateForm, CrfBulkCreateFormResult, CrfBulkCreateItem,
    CrfBulkFormRepository,
    CrfForm, CrfFormNew, CrfFormRepository, CrfFormUpdate,
    CrfItem, CrfItemKind, CrfItemNew, CrfItemRepository, CrfItemUpdate,
    CrfOption, CrfOptionNew, CrfOptionRepository, CrfOptionUpdate,
    CrfUnit, CrfUnitNew, CrfUnitRepository, CrfUnitUpdate,
    CrfVersion, CrfVersionNew, CrfVersionRepository, CrfVersionUpdate,
    DomainAnnotation, DomainAnnotationNew, DomainAnnotationRepository, DomainAnnotationUpdate,
    DomainError, ProjectLookup,
};
pub use usecase::{
    AnnotationView, CrfBulkFormResult, CrfFormView, CrfItemView, CrfOptionView, CrfUsecase,
    CrfUsecaseConfig, CrfUnitView, CrfVersionView, CreateAnnotation,
    CreateCrfBulkForm, CreateCrfBulkItem,
    CreateCrfForm, CreateCrfItem, CreateCrfOption, CreateCrfUnit,
    CreateCrfVersion, CreateDomainAnnotation, DomainAnnotationView,
    SearchCrfFormsByVersion, SearchCrfItemsByVersion,
    SearchCrfOptionsByVersion, SearchCrfUnitsByVersion,
    SearchDomainAnnotationsByVersion, SearchAnnotationsByVersion,
    UpdateAnnotation, UpdateCrfForm, UpdateCrfItem, UpdateCrfOption,
    UpdateCrfUnit, UpdateCrfVersion, UpdateDomainAnnotation, UsecaseError,
};
```

`UsecaseError` mirrors `domain_model::UsecaseError` — two variants
(`Validation(DomainError)`, `Repository(DomainError)`) — so adapters can
exhaustively match. The `From<DomainError> for UsecaseError` impl maps
domain validation variants into `Repository`, per the convention that
"contract broken upstream" is `Repository`.

## `DomainError` variants

Exhaustive variant list, used by both `UsecaseError` (as the inner of
`Validation` / `Repository`) and as the source of every per-aggregate
`*NotFound` mapping in `CrfApiError`:

```rust
#[derive(Debug, Clone, Error)]
pub enum DomainError {
    // ---- validation (constructor-time) ----
    #[error("empty project code")]
    EmptyProjectCode,
    #[error("empty name")]
    EmptyName,
    #[error("empty code")]
    EmptyCode,
    #[error("empty value")]
    EmptyValue,
    #[error("empty content")]
    EmptyContent,
    #[error("invalid crf item kind: {0}")]
    InvalidCrfItemKind(String),
    #[error("kind-shape violation: {kind:?} cannot carry {field}")]
    KindShapeViolation { kind: CrfItemKind, field: String },

    // ---- existence / FK / duplicate (runtime) ----
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("crf version not found: {0}")] CrfVersionNotFound(i64),
    #[error("crf form not found: {0}")] CrfFormNotFound(i64),
    #[error("crf item not found: {0}")] CrfItemNotFound(i64),
    #[error("crf option not found: {0}")] CrfOptionNotFound(i64),
    #[error("crf unit not found: {0}")] CrfUnitNotFound(i64),
    #[error("domain annotation not found: {0}")]
    DomainAnnotationNotFound(i64),
    #[error("annotation not found: {0}")]
    AnnotationNotFound(i64),

    #[error("crf version already exists: {project_code} / {name}")]
    DuplicateCrfVersion { project_code: String, name: String },
    #[error("crf form already exists: version {version_id} / {code}")]
    DuplicateCrfForm { version_id: i64, code: String },
    #[error("crf item already exists: form {form_id} / {code}")]
    DuplicateCrfItem { form_id: i64, code: String },
    #[error("domain annotation already exists: form {form_id} / {name}")]
    DuplicateDomainAnnotation { form_id: i64, name: String },

    #[error("referenced crf version not found: {0}")]
    FkCrfVersionNotFound(i64),
    #[error("referenced crf form not found: {0}")]
    FkCrfFormNotFound(i64),
    #[error("referenced crf item not found: {0}")]
    FkCrfItemNotFound(i64),
    #[error("referenced crf option not found: {0}")]
    FkCrfOptionNotFound(i64),
    #[error("referenced crf unit not found: {0}")]
    FkCrfUnitNotFound(i64),
    #[error("referenced domain annotation not found: {0}")]
    FkDomainAnnotationNotFound(i64),

    #[error("not found")]
    NotFound,
    #[error("repository error: {0}")]
    Repository(String),
}
```

## `apis::crf::CrfService` (the public-facing port)

New file `lib/crates/apis/src/crf.rs`, exporting `pub mod crf;` from
`apis/src/lib.rs`. Mirrors `apis/src/domain_model.rs`.

```rust
pub enum CrfItemKind { Text, Selection, Checkbox, Datetime, Label }
pub enum AnnotationOwner { Form(i64), Item(i64), Option(i64), Unit(i64) }

pub struct CrfVersionView       { pub id: i64, pub project_code: String, pub name: String,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct CrfFormView          { pub id: i64, pub version_id: i64, pub code: String, pub name: String,
                                   pub order: i64, pub not_submitted: bool,
                                   pub domain_annotations: Vec<DomainAnnotationView>,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct CrfItemView          { pub id: i64, pub form_id: i64, pub code: String, pub name: String,
                                   pub kind: CrfItemKind, pub order: i64, pub not_submitted: bool,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct CrfOptionView        { pub id: i64, pub item_id: i64, pub value: String,
                                   pub not_submitted: bool,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct CrfUnitView          { pub id: i64, pub item_id: i64, pub value: String,
                                   pub not_submitted: bool,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct DomainAnnotationView { pub id: i64, pub form_id: i64,
                                   pub name: String, pub description: String,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }
pub struct AnnotationView       { pub id: i64, pub domain_annotation_id: i64,
                                   pub content: String, pub assign: bool,
                                   pub owner: AnnotationOwner,
                                   pub created_at: DateTime<Utc>, pub updated_at: DateTime<Utc> }

pub struct CreateCrfVersionRequest     { pub project_code: String, pub name: String }
pub struct UpdateCrfVersionRequest     { pub id: i64, pub name: Option<String> }
pub struct CreateCrfFormRequest        { pub version_id: i64, pub code: String, pub name: String,
                                          pub order: int, pub not_submitted: bool }
pub struct UpdateCrfFormRequest        { pub id: i64, /* all optional except id */ }
pub struct BulkCreateCrfFormRequest    { pub form: CreateCrfFormRequest,
                                          pub items: Vec<BulkCreateCrfItemInput> }
pub struct BulkCreateCrfItemInput      { pub item: CreateCrfItemRequest,
                                          pub options: Vec<CreateCrfOptionRequest>,
                                          pub units: Vec<CreateCrfUnitRequest> }
pub struct BulkCreateCrfFormResult     { pub form: CrfFormView, pub items: Vec<CrfItemView> }
pub struct CreateCrfItemRequest        { pub form_id: i64, pub code: String, pub name: String,
                                          pub kind: CrfItemKind, pub order: int, pub not_submitted: bool }
pub struct UpdateCrfItemRequest        { ... }
pub struct CreateCrfOptionRequest      { pub item_id: i64, pub value: String, pub not_submitted: bool }
pub struct UpdateCrfOptionRequest      { ... }
pub struct CreateCrfUnitRequest        { pub item_id: i64, pub value: String, pub not_submitted: bool }
pub struct UpdateCrfUnitRequest        { ... }
pub struct CreateDomainAnnotationRequest { pub form_id: i64,
                                            pub name: String, pub description: String }
pub struct UpdateDomainAnnotationRequest { pub id: i64, /* name?, description? */ }
pub struct CreateAnnotationRequest     { pub owner: AnnotationOwner, pub domain_annotation_id: i64,
                                          pub content: String, pub assign: bool }
pub struct UpdateAnnotationRequest     { pub id: i64, pub content: Option<String>,
                                          pub assign: Option<bool> }

pub struct SearchCrfFormsByVersionRequest           { pub version_id: i64, pub fragment: String }
pub struct SearchCrfItemsByVersionRequest           { pub version_id: i64, pub fragment: String }
pub struct SearchCrfOptionsByVersionRequest         { pub version_id: i64, pub fragment: String }
pub struct SearchCrfUnitsByVersionRequest           { pub version_id: i64, pub fragment: String }
pub struct SearchDomainAnnotationsByVersionRequest  { pub version_id: i64, pub fragment: String }
pub struct SearchAnnotationsByVersionRequest        { pub version_id: i64, pub fragment: String }

#[derive(Debug, Clone, Error)]
pub enum CrfApiError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("project not found: {0}")]
    ProjectNotFound(String),
    #[error("crf version not found: {0}")] CrfVersionNotFound(i64),
    #[error("crf form not found: {0}")] CrfFormNotFound(i64),
    #[error("crf item not found: {0}")] CrfItemNotFound(i64),
    #[error("crf option not found: {0}")] CrfOptionNotFound(i64),
    #[error("crf unit not found: {0}")] CrfUnitNotFound(i64),
    #[error("domain annotation not found: {0}")] DomainAnnotationNotFound(i64),
    #[error("annotation not found: {0}")] AnnotationNotFound(i64),
    #[error("crf version already exists: {project_code} / {name}")]
    DuplicateCrfVersion { project_code: String, name: String },
    #[error("crf form already exists: version {version_id} / {code}")]
    DuplicateCrfForm { version_id: i64, code: String },
    #[error("crf item already exists: form {form_id} / {code}")]
    DuplicateCrfItem { form_id: i64, code: String },
    #[error("domain annotation already exists: form {form_id} / {name}")]
    DuplicateDomainAnnotation { form_id: i64, name: String },
    #[error("referenced crf version not found: {0}")] FkCrfVersionNotFound(i64),
    #[error("referenced crf form not found: {0}")] FkCrfFormNotFound(i64),
    #[error("referenced crf item not found: {0}")] FkCrfItemNotFound(i64),
    #[error("referenced crf option not found: {0}")] FkCrfOptionNotFound(i64),
    #[error("referenced crf unit not found: {0}")] FkCrfUnitNotFound(i64),
    #[error("referenced domain annotation not found: {0}")]
    FkDomainAnnotationNotFound(i64),
    #[error("search fragment cannot be empty")]
    EmptySearchFragment,
    #[error("kind-shape violation: {kind:?} cannot carry {field}")]
    KindShapeViolation { kind: CrfItemKind, field: String },
    #[error("repository error: {0}")]
    Repository(String),
}

#[async_trait]
pub trait CrfService: Send + Sync {
    // ---- CrfVersion ----
    async fn create_version(&self, req: CreateCrfVersionRequest) -> Result<CrfVersionView, CrfApiError>;
    async fn get_version_by_id(&self, id: i64) -> Result<CrfVersionView, CrfApiError>;
    async fn list_versions_by_project(&self, project_code: &str) -> Result<Vec<CrfVersionView>, CrfApiError>;
    async fn update_version(&self, req: UpdateCrfVersionRequest) -> Result<CrfVersionView, CrfApiError>;
    async fn delete_version(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- CrfForm ----
    async fn create_form(&self, req: CreateCrfFormRequest) -> Result<CrfFormView, CrfApiError>;
    /// Atomically create a form + every item + each item's options +
    /// units. The owning `version_id` comes from the path segment
    /// on the wire, so the body carries only the form's own
    /// scalar fields plus the items subtree. Item / option / unit
    /// inputs use the same `Create*` shapes as the single-row
    /// endpoints; the bulk port stamps the surrogate `form_id` /
    /// `item_id` onto each row at insert time. Returns the new
    /// form view plus every item view in input order.
    async fn bulk_create_form(&self, req: BulkCreateCrfFormRequest)
        -> Result<BulkCreateCrfFormResult, CrfApiError>;
    async fn get_form_by_id(&self, id: i64) -> Result<CrfFormView, CrfApiError>;
    async fn list_forms_by_version(&self, version_id: i64) -> Result<Vec<CrfFormView>, CrfApiError>;
    async fn update_form(&self, req: UpdateCrfFormRequest) -> Result<CrfFormView, CrfApiError>;
    async fn delete_form(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- CrfItem ----
    async fn create_item(&self, req: CreateCrfItemRequest) -> Result<CrfItemView, CrfApiError>;
    async fn get_item_by_id(&self, id: i64) -> Result<CrfItemView, CrfApiError>;
    /// Every item attached to the form, each one carrying the full
    /// nested tree (options / units / annotations).
    async fn list_items_by_form(&self, form_id: i64) -> Result<Vec<CrfItemView>, CrfApiError>;
    async fn update_item(&self, req: UpdateCrfItemRequest) -> Result<CrfItemView, CrfApiError>;
    async fn delete_item(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- CrfOption / CrfUnit ----
    async fn create_option(&self, req: CreateCrfOptionRequest) -> Result<CrfOptionView, CrfApiError>;
    async fn get_option_by_id(&self, id: i64) -> Result<CrfOptionView, CrfApiError>;
    async fn list_options_by_item(&self, item_id: i64) -> Result<Vec<CrfOptionView>, CrfApiError>;
    async fn update_option(&self, req: UpdateCrfOptionRequest) -> Result<CrfOptionView, CrfApiError>;
    async fn delete_option(&self, id: i64) -> Result<(), CrfApiError>;
    async fn create_unit(&self, req: CreateCrfUnitRequest) -> Result<CrfUnitView, CrfApiError>;
    async fn get_unit_by_id(&self, id: i64) -> Result<CrfUnitView, CrfApiError>;
    async fn list_units_by_item(&self, item_id: i64) -> Result<Vec<CrfUnitView>, CrfApiError>;
    async fn update_unit(&self, req: UpdateCrfUnitRequest) -> Result<CrfUnitView, CrfApiError>;
    async fn delete_unit(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- DomainAnnotation ----
    async fn create_domain_annotation(&self, req: CreateDomainAnnotationRequest) -> Result<DomainAnnotationView, CrfApiError>;
    async fn get_domain_annotation_by_id(&self, id: i64) -> Result<DomainAnnotationView, CrfApiError>;
    async fn list_domain_annotations_by_form(&self, form_id: i64) -> Result<Vec<DomainAnnotationView>, CrfApiError>;
    async fn update_domain_annotation(&self, req: UpdateDomainAnnotationRequest) -> Result<DomainAnnotationView, CrfApiError>;
    async fn delete_domain_annotation(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- Annotation ----
    async fn create_annotation(&self, req: CreateAnnotationRequest) -> Result<AnnotationView, CrfApiError>;
    async fn get_annotation_by_id(&self, id: i64) -> Result<AnnotationView, CrfApiError>;
    async fn list_annotations_by_form(&self, form_id: i64) -> Result<Vec<AnnotationView>, CrfApiError>;
    async fn list_annotations_by_item(&self, item_id: i64) -> Result<Vec<AnnotationView>, CrfApiError>;
    async fn list_annotations_by_option(&self, option_id: i64) -> Result<Vec<AnnotationView>, CrfApiError>;
    async fn list_annotations_by_unit(&self, unit_id: i64) -> Result<Vec<AnnotationView>, CrfApiError>;
    async fn update_annotation(&self, req: UpdateAnnotationRequest) -> Result<AnnotationView, CrfApiError>;
    async fn delete_annotation(&self, id: i64) -> Result<(), CrfApiError>;

    // ---- Search ----
    async fn search_forms_by_version(&self, req: SearchCrfFormsByVersionRequest) -> Result<Vec<CrfFormView>, CrfApiError>;
    async fn search_items_by_version(&self, req: SearchCrfItemsByVersionRequest) -> Result<Vec<CrfItemView>, CrfApiError>;
    async fn search_options_by_version(&self, req: SearchCrfOptionsByVersionRequest) -> Result<Vec<CrfOptionView>, CrfApiError>;
    async fn search_units_by_version(&self, req: SearchCrfUnitsByVersionRequest) -> Result<Vec<CrfUnitView>, CrfApiError>;
    async fn search_domain_annotations_by_version(&self, req: SearchDomainAnnotationsByVersionRequest) -> Result<Vec<DomainAnnotationView>, CrfApiError>;
    async fn search_annotations_by_version(&self, req: SearchAnnotationsByVersionRequest) -> Result<Vec<AnnotationView>, CrfApiError>;
}
```

## Persistence (sqlx, runtime API)

Each `*_repo.rs` follows the shape of
`domain-model/src/adapter/persistence/postgres/sdtm_version_repo.rs`:

- `*Row` `FromRow` struct (private).
- `From<*Row> for Aggregate` → calls `Aggregate::for_repository(...)`.
- `*RepoPg { pool: PgPool }` newtype.
- `#[async_trait] impl Port` — `create`, `find_by_id`, list, `update`, `delete`.
- `map_db_err` translating:
  - `sqlx::Error::RowNotFound` → `DomainError::NotFound`
  - `sqlx::Error::Database` with SQLSTATE `23505` →
    `DomainError::Duplicate*` (using the bind order to pick the variant)
  - otherwise → `DomainError::Repository(err.to_string())`
  - Update returning no rows → `DomainError::*NotFound(id)`
- `migration_file_is_present_and_idempotent` test (loads the .sql via
  `include_str!` + `env!("CARGO_MANIFEST_DIR")` and asserts the column /
  FK / CHECK / trigger text).

`AnnotationRepoPg` additionally exposes `find_by_form / find_by_item /
find_by_option / find_by_unit` plus a `search_by_version(fragment)` that
UNIONs the four owner chains.

`CrfItemRepoPg::list_full_by_form(form_id)` runs the four round-trips
described in "List strategy" and assembles the tree in memory before
returning a `Vec<CrfItem>` with populated `options / units / annotations`
slices (the in-memory representation of which is delegated to the
domain: the domain `CrfItem` aggregate carries just the scalar fields,
and the usecase stitches the per-row children in via separate lookups).

### Bulk form repo (`crf_bulk_form_repo.rs`)

`CrfBulkFormRepoPg` is the one persistence adapter that owns a
transaction end-to-end. The full path:

1. `pool.begin()` → `tx`.
2. `INSERT INTO crf_forms (...) RETURNING id, ...` — the freshly
   stamped `id` becomes the `form_id` for the items subtree.
3. For each item: `INSERT INTO crf_items (...) RETURNING id, ...`,
   then `INSERT INTO crf_options / crf_units` for the child's
   children.
4. `tx.commit()`.

If any `INSERT` returns `Err`, the `Transaction` is dropped without
`commit()` — sqlx issues a `ROLLBACK` from `Drop`, so every prior
insert in the same call is reversed. No partial state can survive
an `Err`.

`map_db_err` for the bulk port maps:
- SQLSTATE `23505` against `crf_forms_version_code_unique` →
  `DuplicateCrfForm { version_id: 0, code: "(unknown)" }`
- SQLSTATE `23505` against `crf_items_form_code_unique` →
  `DuplicateCrfItem { form_id: 0, code: "(unknown)" }`
- FK violation against `crf_forms_version_id_fkey` →
  `FkCrfVersionNotFound(0)`
- FK violation against `crf_items_form_id_fkey` →
  `FkCrfFormNotFound(0)`
- FK violation against `crf_options_item_id_fkey` /
  `crf_units_item_id_fkey` → `FkCrfItemNotFound(0)`
- otherwise → `Repository(err.to_string())`

The `(unknown)` placeholders are intentional — the bulk port
treats every constraint violation as a single `Err` and the caller
gets the same shape as the single-row endpoints; the usecase's
`validate_bulk_create` runs first so a constraint failure is
always an unexpected race, not a normal flow.

## Bulk port — `domain::crf_bulk_form`

The bulk port is intentionally separate from the per-aggregate
`CrfFormRepository` / `CrfItemRepository` / etc. — it doesn't
belong on any one aggregate, and the `pool.begin()` transaction
belongs in the adapter, not in the domain. The port stays free
of `sqlx` types:

```rust
#[derive(Debug, Clone)]
pub struct CrfBulkCreateForm {
    pub form: CrfFormNew,           // form fields, no form_id
    pub items: Vec<CrfBulkCreateItem>,
}
pub struct CrfBulkCreateItem {
    pub item: CrfItemNew,           // item fields, form_id must be 0
    pub options: Vec<CrfOptionNew>, // option_id must be 0
    pub units: Vec<CrfUnitNew>,     // unit_id must be 0
}
pub struct CrfBulkCreateFormResult { pub form: CrfForm, pub items: Vec<CrfItem> }

#[async_trait]
pub trait CrfBulkFormRepository: Send + Sync {
    async fn bulk_create(
        &self,
        input: CrfBulkCreateForm,
    ) -> Result<CrfBulkCreateFormResult, DomainError>;
}
```

`validate_bulk_create(&input)` runs up-front at the usecase (BEFORE
the port call) so a `KindShapeViolation` or empty-field violation
never leaves partial state. The port stamps the surrogate ids as
it walks the input — the caller passes placeholder `0`s in the
`*New` DTOs and the port fills them in.

## Facade — `adapter/facade/in_memory/service.rs`

`CrfServiceImpl<V, F, I, O, U, Da, A, P, B>` wraps `CrfUsecase` and
implements `apis::crf::CrfService`. Two constructors:
`from_usecase(usecase)` and `from_repos(version_repo, form_repo, item_repo,
option_repo, unit_repo, domain_annotation_repo, annotation_repo,
project_lookup, bulk_form_repo)` — note the 9-argument `from_repos`,
with `bulk_form_repo: Arc<B>` appended.

The body is a per-method projection: convert `apis::crf::Request` →
`usecase::Command`, call usecase, project the internal view to
`apis::crf::View`, and map `UsecaseError → CrfApiError` via a single
`From<UsecaseError> for CrfApiError` impl next to the other mappers.

`CrfApiError` mapping:
- `UsecaseError::Validation(d)` → `CrfApiError::Validation(d.to_string())`
- `UsecaseError::Repository(d)` → match on every `DomainError` variant
  (covers every `*NotFound`, `Duplicate*`, `ProjectNotFound`,
  `EmptySearchFragment`, `KindShapeViolation`, `Repository(msg)`,
  and the `Empty*` validation variants — which the `unreachable!` arm
  documents as validation-only paths).

## Crate root re-exports

`src/lib.rs` re-exports every type the consumer is allowed to name, in
the same shape as `domain-model/src/lib.rs`. No builders, no async
constructors; the public constructors are `*RepoPg::new(PgPool)`,
`ProjectLookupImpl::new(Arc<dyn ProjectService>)`,
`CrfUsecase::new(CrfUsecaseConfig)`,
`CrfServiceImpl::from_repos(...) / from_usecase(...)`.

## Tests

In this order, per `lib-crate-development.md` §9:

1. **Domain unit tests** (`src/domain/tests.rs`):
   - `CrfItemKind::as_str` / `try_from_str` round-trip
   - `CrfVersion::new` / `CrfForm::new` / `CrfItem::new` /
     `CrfOption::new` / `CrfUnit::new` / `DomainAnnotation::new` /
     `Annotation::for_*` reject empty / whitespace inputs
   - `Annotation::new` rejects empty `content` and `domain_annotation_id <= 0`

2. **Adapter unit tests** (`src/adapter/persistence/postgres/*_repo.rs`):
   - `From<*Row>` for the aggregate
   - `migration_file_is_present_and_idempotent` per repo

3. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`):
   - Wires `Arc<Mutex<Vec<*>>>` + `AtomicI32` per repo plus a mock
     `ProjectLookup` into the usecase. Exercises:
     - create_version rejects missing project (`ProjectNotFound`)
     - create_item rejects `Selection` with zero options
       (`KindShapeViolation`)
     - search respects version fragment scope
     - the FK error mapping round-trips through `CrfApiError`

4. **`tests/public_api.rs`** (compile-only):
   - Names every documented consumer import
   - Pins the constructor chain (`fn(PgPool) -> _`, `fn(*Config) -> _`)
   - Asserts `Send + Sync` for `Box<dyn CrfService>`

5. **`tests/integration_persistence.rs`** (`#[ignore]`-gated, live DB):
   - Reads `AEGIS_CRF_DATABASE_URL`; panics with a clear message if missing
   - Loads `.env` via `dotenvy::dotenv()`
   - Drops `crf_*` tables + `_sqlx_migrations` before each run
   - Generates unique `(project_code, name)` / `(version_id, code)` via
     `AtomicI32::fetch_add` + wall-clock nanoseconds for any `UNIQUE`
     column
   - Round-trips: full lifecycle create → read → list → update → delete
     on every aggregate; polymorphic FK CHECK rejects annotations with
     two owners; FK CASCADE removes children when parent is deleted;
     search returns version-scoped rows only
   - `bulk_create_form` happy path on a fresh version: form + items +
     options + units appear together after a single call; duplicate
     item code surfaces as `DomainError::DuplicateCrfItem` and the
     transaction rolls back (no form row visible afterward)

**Bulk-port unit tests** (`src/usecase/tests.rs` + facade tests):

- `bulk_create_form_inserts_form_items_options_units` — happy path
  through `InMemoryBulkForms`, two items each with options / units,
  every item stamped with the freshly-allocated form_id
- `bulk_create_form_returns_results_in_input_order` — five items
  round-trip back in input order
- `bulk_create_form_rejects_empty_form_code` — `Validation`
  surface before the port is called
- `bulk_create_form_rejects_text_kind_with_options` —
  `KindShapeViolation { kind: Text, field: "options" }`
- `bulk_create_form_rejects_selection_without_options` —
  `KindShapeViolation { kind: Selection, field: "options" }`
- `bulk_create_form_rejects_empty_item_code` — `Validation`
- `bulk_create_form_validation_rejects_empty_code_with_existing_version`
  — confirms validation runs after the version lookup
- `bulk_create_form_rejects_missing_parent_version` —
  `CrfVersionNotFound(id)` from the usecase's pre-port check
- `facade_bulk_create_form_round_trip` — same shape through the
  `apis::crf::CrfService` facade
- `facade_bulk_create_form_rejects_text_with_options` — wire-shape
  `KindShapeViolation` end-to-end

## Dependencies (`Cargo.toml`)

```toml
[dependencies]
sqlx = { workspace = true }
tokio = { workspace = true }
async-trait = { workspace = true }
thiserror = { workspace = true }
chrono = { workspace = true }   # created_at / updated_at
serde = { workspace = true }
serde_json = { workspace = true }
apis = { path = "../apis" }     # outbound ProjectService port

[dev-dependencies]
dotenvy = { workspace = true }
sqlx = { workspace = true }
tokio = { workspace = true }
```

No new workspace deps.

## Workspace wiring

Root `Cargo.toml`:

```toml
[workspace]
members = [
    "apps/desktop/aegis-desktop/src-tauri",
    "apps/server/aegis-server",
    "lib/crates/apis",
    "lib/crates/auth",
    "lib/crates/crf",                                                # <-- new
    "lib/crates/domain-model",
    "lib/crates/project",
    "lib/crates/terminology",
    "lib/crates/user",
    "lib/crates/windows-utils",
]
```

`lib/crates/apis/src/lib.rs` adds `pub mod crf;`.

## `README.md`

Mirrors `domain-model/README.md`:

- One-sentence purpose.
- `src/` tree (the one in this spec).
- Data-model table.
- Verification block (the §11 gate).
- Live-DB env var (`AEGIS_CRF_DATABASE_URL`).
- Back-link to `docs/guidelines/lib-crate-development.md`.
- Spec pointer: this file.

Out-of-scope call-outs: HTTP routes / utoipa annotations; desktop-side TS
types; Tauri commands. The crate is reusable; any future server wires its
own router on top of `CrfService`.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p crf --all-targets --all-features -- -D warnings
cargo test -p crf
cargo doc -p crf --no-deps
cargo test -p crf -- --ignored --test-threads=1   # needs AEGIS_CRF_DATABASE_URL
cargo check --workspace
```

## Out of scope

- HTTP layer (a future `aegis-server` route module mounts `CrfServiceImpl`
  and translates each trait call into an `axum` handler with `utoipa`
  annotations). The bulk endpoint is `POST
  /api/crf/versions/{version_id}/forms/bulk`; the request body
  mirrors `BulkCreateCrfFormRequest` and the response is
  `BulkCreateCrfFormResponse` (form + items in input order).
- Desktop-side TS types and feature modules (a future spec wires the
  Tauri shell).
- Full-text search via `tsvector` (the ILIKE-based search is sufficient
  for the data-model's stated needs; FTS is an additive upgrade).
- Cross-version analytics / rollups.