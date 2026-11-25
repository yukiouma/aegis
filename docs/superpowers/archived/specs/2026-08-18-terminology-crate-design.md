# Terminology Crate Design

## Goal

Add `lib/crates/terminology` as a reusable Rust library that owns CRUD over
the CDISC terminology aggregates: `TerminologyVersion`, `CodeList`, and
`CodeItem`.

The crate owns:

- The PostgreSQL schema for `terminology_versions`, `code_lists`, and
  `code_items` (three SQLx migrations).
- Validation rules (`TerminologyKind` enum parsing; non-empty `name`/`code`
  fields on every aggregate).
- Three PostgreSQL-backed repositories — `TerminologyVersionRepo`,
  `CodeListRepo`, `CodeItemRepo` — implementing the inbound ports declared
  in `domain`.
- A single `TerminologyUsecase<V, L, I>` (generic over all three repositories)
  that orchestrates every CRUD operation, projects domain aggregates into
  view DTOs, and exposes a Postgres `tsvector`/`GIN`-backed full-text
  search per level.
- No outbound `apis` port for now — `TerminologyService` is added to `apis`
  when the server crate needs it.

The crate does **not** own user / project / auth. It only persists the
terminology tree.

The data model comes from the user spec:

```rust
pub enum TerminologyKind { Sdtm, Adam }

pub struct TerminologyVersion {
    pub kind: TerminologyKind,
    /// `yyyy-mm-dd` suffix of the matched sheet name (e.g. `"2026-03-27"`).
    pub name: String,
    pub codelist: Vec<CodeList>,
}

pub struct CodeList {
    pub code: String,
    pub extensible: bool,
    pub name: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub code_list: Vec<CodeItem>,
}

pub struct CodeItem {
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}
```

The persisted shape is flatter than the in-memory shape — each level is
its own table with a foreign key. `CodeList::codelist` (`Vec<CodeItem>`)
becomes a child query against `code_items.codelist_id`; `TerminologyVersion::codelist`
becomes a child query against `code_lists.version_id`. The usecase
re-assembles the tree on read.

## Architecture

Ports-and-adapters DDD structure, mirroring
[`lib/crates/user/`](../../lib/crates/user/) and
[`lib/crates/project/`](../../lib/crates/project/). No facade adapter in
this iteration — a single `apis::terminology::TerminologyService` will be
added in a follow-up spec when the server crate needs it.

- `domain` — `TerminologyKind`, `TerminologyVersion`, `CodeList`,
  `CodeItem`, ports (`TerminologyVersionRepository`, `CodeListRepository`,
  `CodeItemRepository`), and `DomainError`. No I/O, no `sqlx`, no `tokio`.
  Two constructors per aggregate (validating `new` returning
  `Result<Self, DomainError>`; `pub(crate) for_repository` for the adapter
  layer). Hand-rolled `Debug` impls.
- `usecase` — `TerminologyUsecase<V, L, I>` generic over the three
  repositories; command DTOs (`CreateTerminologyVersion`,
  `UpdateTerminologyVersion`, `CreateCodeList`, …); view DTOs
  (`TerminologyVersionView`, `CodeListView`, `CodeItemView`,
  `CodeListSearchHit`, `CodeItemSearchHit`); `UsecaseError`; free-function
  `*_validate_*` pre-flight checks mirroring the `user` crate's
  shape.
- `adapter` — concrete implementations of the domain ports.
  - `adapter/persistence/postgres/` — one repo file per aggregate
    (`terminology_version_repo.rs`, `code_list_repo.rs`,
    `code_item_repo.rs`); each holds its own `map_db_error` that
    translates `sqlx::Error::Database` SQLSTATE codes into the typed
    `DomainError` variants. No `row.rs` umbrella module — each repo has a
    private `row` submodule local to it (so `CodeListRow` does not have
    to leak its columns onto the other repos' boundary).

Per [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md):
no `mod.rs`; each top-level module uses `src/<module>.rs` +
`src/<module>/`. Terminal leaf modules (`*_repo.rs`, `tsv.rs`) are leaf
files with no companion directory.

## Data Model

```rust
// domain/terminology_kind.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TerminologyKind { Sdtm, Adam }
impl TerminologyKind { pub fn as_str(&self) -> &'static str; /* "sdtm" | "adam" */ }
impl std::convert::TryFrom<&str> for TerminologyKind {
    type Error = DomainError;
}

// domain/terminology_version.rs
pub struct TerminologyVersion {
    pub id: i64,
    pub kind: TerminologyKind,
    pub name: String,                              // "yyyy-mm-dd" suffix; not parsed
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// domain/code_list.rs
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
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

// domain/code_item.rs
pub struct CodeItem {
    pub id: i64,
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}
```

`name` is stored as `String` (never parsed into a `NaiveDate`). It is
the `yyyy-mm-dd` workbook sheet suffix and may one day carry non-date
names (e.g. `2026-03-27-rc1`). The domain invariant is non-empty after
trim, nothing else.

All string fields on `CodeList` and `CodeItem` are first-class text
columns — they back the full-text search index described below.

### Two-constructor pattern

```rust
impl TerminologyVersion {
    /// Public validating ctor used by the domain layer (tests + any
    /// in-crate path that constructs from raw inputs). Rejects
    /// empty / whitespace `name`.
    pub fn new(kind: TerminologyKind, name: String) -> Result<Self, DomainError> { … }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64, kind: TerminologyKind, name: String,
        created_at: DateTime<Utc>, updated_at: DateTime<Utc>,
    ) -> Self { … }
}

// CodeList::new(version_id, code, extensible, name, submission_value, …) —
//   rejects empty `code` (and surfaces DomainError::EmptyCode)
// CodeList::for_repository(id, version_id, code, …) — pub(crate)

// CodeItem::new(codelist_id, code, submission_value, …) — rejects empty `code`
// CodeItem::for_repository(id, codelist_id, code, …) — pub(crate)
```

Hand-rolled `Debug` impls follow the structural pattern used by `User`
and `Project` — every field is currently safe to log.

### DomainError

```rust
#[derive(Debug, thiserror::Error)]
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
    DuplicateVersion { kind: TerminologyKind, name: String },

    #[error("code list already exists for version {version_id} / {code}")]
    DuplicateCodeList { version_id: i64, code: String },

    #[error("code item already exists for codelist {codelist_id} / {code}")]
    DuplicateCodeItem { codelist_id: i64, code: String },

    #[error("referenced terminology version not found: {0}")]
    FkVersionNotFound(i64),

    #[error("referenced code list not found: {0}")]
    FkCodeListNotFound(i64),

    #[error("repository error: {0}")]
    Repository(String),
}
```

`InvalidKind` carries the rejected string so tests and ops can see what
the caller passed. `Fk*NotFound` maps SQLSTATE `23503` (FK violation) —
friendlier than a raw `Repository(driver_message)` for an obvious
caller error.

## Database Schema

Three migrations under `migrations/`.

### `0001_create_terminology_versions.sql`

```sql
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

### `0002_create_code_lists.sql`

```sql
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
    NEW.updated_at = NOW(); RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER code_lists_set_updated_at
    BEFORE UPDATE ON code_lists
    FOR EACH ROW EXECUTE FUNCTION code_lists_set_updated_at();
```

### `0003_create_code_items.sql`

Mirrors `code_lists`: FK to `code_lists(id) ON DELETE CASCADE`, UNIQUE
`(codelist_id, code)`, `tsvector` generated from the same five text
columns with the same weights, GIN index, and the
`code_items_set_updated_at` trigger on `BEFORE UPDATE`.

`updated_at` is auto-managed by the per-table trigger on every UPDATE.
The `CASCADE` on `version_id` lets `delete_version` remove children in
one repository call. `code` is excluded from the `tsvector` because
users search by meaning, not by NCI C-code.

### Module-level comment on SQLx API choice

Per the established workspace convention (see `user::adapter::persistence::postgres`),
this crate uses the **runtime** SQLx API (`sqlx::query_as`, `QueryBuilder`)
rather than the compile-time-checked macros — those macros require either
a live `DATABASE_URL` or a checked-in `sqlx-data.json` offline cache,
neither of which the workspace build currently provides.

## Repository Ports

```rust
// domain/terminology_version.rs
#[async_trait]
pub trait TerminologyVersionRepository: Send + Sync {
    async fn create(&self, input: TerminologyVersionNew)
        -> Result<TerminologyVersion, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<TerminologyVersion, DomainError>;
    async fn find_by_kind_and_name(
        &self, kind: TerminologyKind, name: &str,
    ) -> Result<TerminologyVersion, DomainError>;
    async fn list(&self) -> Result<Vec<TerminologyVersion>, DomainError>;
    async fn update(&self, input: TerminologyVersionUpdate)
        -> Result<TerminologyVersion, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
}

pub struct TerminologyVersionNew { pub kind: TerminologyKind, pub name: String }
pub struct TerminologyVersionUpdate { pub id: i64, pub kind: Option<TerminologyKind>, pub name: Option<String> }

// domain/code_list.rs
#[async_trait]
pub trait CodeListRepository: Send + Sync {
    async fn create(&self, input: CodeListNew) -> Result<CodeList, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeList, DomainError>;
    async fn list_by_version(&self, version_id: i64) -> Result<Vec<CodeList>, DomainError>;
    async fn update(&self, input: CodeListUpdate) -> Result<CodeList, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(&self, query: CodeListSearchQuery)
        -> Result<Vec<CodeListSearchHit>, DomainError>;
}

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

pub struct CodeListSearchQuery {
    pub kind: TerminologyKind,
    pub version_name: String,
    pub text: String,
    pub limit: u32,                                // default 50, hard cap 500 (clamped, not rejected)
}
pub struct CodeListSearchHit { pub codelist: CodeList, pub score: f32 }

// domain/code_item.rs
#[async_trait]
pub trait CodeItemRepository: Send + Sync {
    async fn create(&self, input: CodeItemNew) -> Result<CodeItem, DomainError>;
    async fn find_by_id(&self, id: i64) -> Result<CodeItem, DomainError>;
    async fn list_by_codelist(&self, codelist_id: i64) -> Result<Vec<CodeItem>, DomainError>;
    async fn update(&self, input: CodeItemUpdate) -> Result<CodeItem, DomainError>;
    async fn delete(&self, id: i64) -> Result<(), DomainError>;
    async fn search(&self, query: CodeItemSearchQuery)
        -> Result<Vec<CodeItemSearchHit>, DomainError>;
}

pub struct CodeItemNew {
    pub codelist_id: i64,
    pub code: String,
    pub submission_value: String,
    pub synonym: String,
    pub definition: String,
    pub nci_preferred_term: String,
}
pub struct CodeItemUpdate {
    pub id: i64,
    pub code: Option<String>,
    pub submission_value: Option<String>,
    pub synonym: Option<String>,
    pub definition: Option<String>,
    pub nci_preferred_term: Option<String>,
}

pub struct CodeItemSearchQuery {
    pub kind: TerminologyKind,
    pub version_name: String,
    pub text: String,
    pub limit: u32,                                // default 50, hard cap 500 (clamped, not rejected)
}
pub struct CodeItemSearchHit {
    pub item: CodeItem,
    pub score: f32,
    pub codelist_id: i64,                          // for the caller to link back
}
```

`CodeListRepo::delete` and `CodeItemRepo::delete` cascade to their
children via `ON DELETE CASCADE`. `TerminologyVersionRepo::delete`
cascades to `code_lists` (and via it to `code_items`) the same way.

`map_db_error` (one per repo) translates:

- `sqlx::Error::RowNotFound` → `DomainError::NotFound`
- `sqlx::Error::Database`:
  - SQLSTATE `23503` on `version_id` → `DomainError::FkVersionNotFound(<id>)`
  - SQLSTATE `23503` on `codelist_id` → `DomainError::FkCodeListNotFound(<id>)`
  - SQLSTATE `23505` → `DomainError::DuplicateVersion` /
    `DuplicateCodeList` / `DuplicateCodeItem`
  - other → `DomainError::Repository(driver_message)`
- everything else → `DomainError::Repository(...)`

## Search Query (full-text)

The `search` method on each child repository joins back to
`terminology_versions` on `kind = $1 AND name = $2`, then runs:

```sql
SELECT id, version_id, code, extensible, name, submission_value, synonym,
       definition, nci_preferred_term, created_at, updated_at,
       ts_rank_cd(tsv, websearch_to_tsquery('english', $3)) AS score
FROM code_lists
JOIN terminology_versions v ON v.id = code_lists.version_id
WHERE v.kind = $1
  AND v.name = $2
  AND code_lists.tsv @@ websearch_to_tsquery('english', $3)
ORDER BY score DESC
LIMIT $4;
```

`websearch_to_tsquery` accepts the same syntax as a web search engine
(`"phrase"`, `-exclude`, `OR`). A query that reduces to all stopwords
returns NULL and matches nothing; the repository maps that case to
empty results, never to an error. The hard `limit` cap (500) prevents
accidental scans.

The same shape applies to `code_items` with `codelist_id` joined back
through `code_lists` to `terminology_versions`.

Cache adapter: **not added in this iteration** (YAGNI — the query is
per-version and bounded by `LIMIT`, so the 500ms hit on cached work is
not worth a separate adapter yet).

## Usecase Layer

`TerminologyUsecase<V, L, I>` is generic over the three repositories and
constructed via a `TerminologyUsecaseConfig` because three args crosses
the guideline's readability threshold:

```rust
pub struct TerminologyUsecaseConfig<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    pub version_repo: V,
    pub code_list_repo: L,
    pub code_item_repo: I,
}

pub struct TerminologyUsecase<
    V: TerminologyVersionRepository,
    L: CodeListRepository,
    I: CodeItemRepository,
> {
    version_repo: V,
    code_list_repo: L,
    code_item_repo: I,
}

impl<V, L, I> TerminologyUsecase<V, L, I> {
    pub fn new(cfg: TerminologyUsecaseConfig<V, L, I>) -> Self { /* store fields */ }

    // Versions
    pub async fn create_version(&self, cmd: CreateTerminologyVersion) -> Result<TerminologyVersionView, UsecaseError>;
    pub async fn get_version_by_id(&self, id: i64) -> Result<TerminologyVersionView, UsecaseError>;
    pub async fn get_version(&self, kind: TerminologyKind, name: &str) -> Result<TerminologyVersionView, UsecaseError>;
    pub async fn list_versions(&self) -> Result<Vec<TerminologyVersionView>, UsecaseError>;
    pub async fn update_version(&self, cmd: UpdateTerminologyVersion) -> Result<TerminologyVersionView, UsecaseError>;
    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError>;

    // Code lists
    pub async fn create_code_list(&self, cmd: CreateCodeList) -> Result<CodeListView, UsecaseError>;
    pub async fn list_code_lists(&self, version_id: i64) -> Result<Vec<CodeListView>, UsecaseError>;
    pub async fn update_code_list(&self, cmd: UpdateCodeList) -> Result<CodeListView, UsecaseError>;
    pub async fn delete_code_list(&self, id: i64) -> Result<(), UsecaseError>;
    pub async fn search_code_lists(&self, q: CodeListSearchQuery) -> Result<Vec<CodeListSearchHit>, UsecaseError>;

    // Code items
    pub async fn create_code_item(&self, cmd: CreateCodeItem) -> Result<CodeItemView, UsecaseError>;
    pub async fn list_code_items(&self, codelist_id: i64) -> Result<Vec<CodeItemView>, UsecaseError>;
    pub async fn update_code_item(&self, cmd: UpdateCodeItem) -> Result<CodeItemView, UsecaseError>;
    pub async fn delete_code_item(&self, id: i64) -> Result<(), UsecaseError>;
    pub async fn search_code_items(&self, q: CodeItemSearchQuery) -> Result<Vec<CodeItemSearchHit>, UsecaseError>;
}
```

The version view is the *flat* projection of the row (mirroring the
user/project crate shape). Callers who want the tree in the original
in-memory shape (`TerminologyVersion { kind, name, codelist: Vec<CodeList>… }`)
will compose it themselves from `get_version_by_id` +
`list_code_lists` + `list_code_items`. That composition is deliberately
left out of the usecase — it crosses the aggregate boundary and would
force the usecase to refetch three different lists in one call (which
needs a different port surface and is a separate spec).

Pre-flight validation: `name`/`code` non-empty after trim. `update_version`
/ `update_code_list` / `update_code_item` re-run the same checks on any
`Some` text field. The pre-flight functions live alongside the usecase
file (`fn validate_create_version(&CreateTerminologyVersion) -> Result<(), UsecaseError>`).

`From<TerminologyVersion/CodeList/CodeItem> for *View` projects internal
types into safe equivalents. No field is currently secret; the indirection
is preserved so a future redaction has a single seam.

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

**Not added in this iteration.** `TerminologyService` is added to the
`apis` crate when the server crate needs the terminology CRUD surface
over HTTP — same pattern as `apis::user::UserService` and
`apis::project::ProjectService`.

## Public API

The crate root (`src/lib.rs`) re-exports the public surface:

```rust
pub mod adapter;
pub mod domain;
pub mod usecase;

pub use adapter::{
    CodeItemRepo, CodeListRepo, TerminologyVersionRepo,
};
pub use domain::{
    CodeItem, CodeItemNew, CodeItemRepository, CodeItemSearchHit, CodeItemSearchQuery,
    CodeItemUpdate, CodeList, CodeListNew, CodeListRepository, CodeListSearchHit,
    CodeListSearchQuery, CodeListUpdate, DomainError, TerminologyKind,
    TerminologyVersion, TerminologyVersionNew, TerminologyVersionRepository,
    TerminologyVersionUpdate,
};
pub use usecase::{
    CodeItemView, CodeListView, CreateCodeItem, CreateCodeList,
    CreateTerminologyVersion, TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersionView, UpdateCodeItem, UpdateCodeList, UpdateTerminologyVersion,
    UsecaseError,
};
```

Consumers write `use terminology::*`; they never reach into the
sub-modules.

Constructor chain:

```rust
use terminology::{
    CodeItemRepo, CodeListRepo, TerminologyUsecase, TerminologyUsecaseConfig,
    TerminologyVersionRepo,
};

let v_repo = TerminologyVersionRepo::new(pool.clone());
let l_repo = CodeListRepo::new(pool.clone());
let i_repo = CodeItemRepo::new(pool.clone());
let usecase = TerminologyUsecase::new(TerminologyUsecaseConfig {
    version_repo: v_repo,
    code_list_repo: l_repo,
    code_item_repo: i_repo,
});
```

## Workspace Wiring

- Add `lib/crates/terminology` to the root `Cargo.toml`
  `[workspace].members` array.
- `terminology/Cargo.toml` inherits every dep via `{ workspace = true }`:
  `sqlx`, `tokio`, `async-trait`, `thiserror`, `chrono`. The `chrono`
  dep gets a one-line comment explaining it carries
  `created_at` / `updated_at`.
- No path-dep on `apis` (this crate does not implement an outbound port
  yet).
- `dev-dependencies` add `dotenvy`, `sqlx`, and `tokio` for the ignored
  integration tests; `tokio` for the `#[tokio::test]` async runtime.

## Tests

Following the guideline's tier order. The "facade unit tests" tier from
the guideline becomes the **usecase in-memory wire-up tests** for this
crate (since there is no facade yet).

1. **Domain unit tests** (`src/domain/tests.rs`)
   - `TerminologyKind::try_from` parses `"sdtm"` / `"adam"`; rejects
     others as `DomainError::InvalidKind`.
   - `TerminologyVersion::new` rejects empty `name`.
   - `CodeList::new` rejects empty `code`.
   - `CodeItem::new` rejects empty `code`.

2. **Adapter unit tests** (`src/adapter/persistence/postgres/<repo>/tests.rs`)
   - Read each migration via
     `std::fs::read_to_string(env!("CARGO_MANIFEST_DIR") + "/migrations/<file>.sql")`
     and assert:
     - columns + their types
     - UNIQUE constraints
     - CHECK constraints
     - `updated_at` trigger
     - `tsv` GENERATED column + GIN index for `code_lists` / `code_items`
     - CASCADE on the FK
   - `*Row → *Aggregate` `TryFrom` happy-path conversion.

3. **Usecase in-memory tests** (`src/usecase/tests.rs`)
   - Wire the usecase against in-memory `Arc<Mutex<…>>` fakes for each
     of the three repositories (mirroring the user crate's facade-tier
     shape).
   - Cases: create → get by id → get by kind+name → list → update → delete
     for `TerminologyVersion`; create / list / update / delete / search
     round-trip for `CodeList` and `CodeItem`; cascade delete behaviour
     (`delete_version` removes both `code_lists` and their `code_items`).
   - Search: returns empty hits when the query is all stopwords; `Some`
     hits when the synthetic fakes store matching rows.

4. **`tests/public_api.rs`** — compile-only.
   - Names every documented consumer import.
   - Pins the constructor chain:
     `fn(sqlx::PgPool) -> _` for each of the three Postgres repos,
     `fn(TerminologyUsecaseConfig<V, L, I>) -> _` for
     `TerminologyUsecase::new`.
   - Asserts each repo `Send + Sync` (so the usecase config can be moved
     into an async server).

5. **`tests/integration_persistence.rs`** — live PG round-trips.
   - `#[ignore]`-gated.
   - Loads `.env` via `dotenvy::dotenv()`, reads
     `AEGIS_TERMINOLOGY_DATABASE_URL` (panic with a clear message if
     missing).
   - Drops `code_items`, `code_lists`, `terminology_versions`,
     `_sqlx_migrations` before each run, then applies migrations via
     `sqlx::migrate!("./migrations")`.
   - Per-run unique `code` values via an atomic counter + wall-clock
     nanos so concurrent runs do not collide on the UNIQUE constraints.
   - Round-trips CRUD + a search query that returns hits ranked by
     `ts_rank_cd`.

## README

`lib/crates/terminology/README.md` covers:

- One-sentence purpose (terminology versions, code lists, code items —
  CRUD and full-text search).
- A `src/` tree matching the actual module shape.
- Database setup: `sqlx migrate run --source lib/crates/terminology/migrations`
  + `AEGIS_TERMINOLOGY_DATABASE_URL` env var + a small constructor snippet.
- How to run the ignored tests
  (`cargo test -p terminology -- --ignored`).
- A back-link to the guideline.

## Verification Gate

```bash
cargo fmt --all -- --check
cargo clippy -p terminology --all-targets --all-features -- -D warnings
cargo test -p terminology
cargo doc -p terminology --no-deps
cargo test -p terminology -- --ignored --test-threads=1   # when AEGIS_TERMINOLOGY_DATABASE_URL is set
```

Plus `cargo check --workspace` / `cargo clippy --workspace` /
`cargo test --workspace` since the only-added workspace member rule
kicks in.

## Commits

One commit per logical change (matching the guideline):

1. **scaffold** — register crate, basic `Cargo.toml`, empty `lib.rs`.
2. **domain** — `TerminologyKind`, `TerminologyVersion`, `CodeList`,
   `CodeItem`; ports; `DomainError`; domain tests.
3. **usecase** — `TerminologyUsecase`, command / view DTOs,
   `UsecaseError`, usecase in-memory wire-up tests.
4. **persistence** — three migrations; `TerminologyVersionRepo`,
   `CodeListRepo`, `CodeItemRepo`; per-repo `row` submodule; postgres
   unit tests + integration tests (`#[ignore]`-gated).
5. **public_api** — `tests/public_api.rs` compile-only test.
6. **readme** — `README.md` at the crate root.
7. **chore: lockfile** — `Cargo.lock` drift after new deps land.

Each commit message lists the spec coverage and the verification
commands at the bottom so reviewers can run the same gate locally.
