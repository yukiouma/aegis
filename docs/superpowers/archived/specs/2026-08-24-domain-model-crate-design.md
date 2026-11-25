# Domain Model Crate Design

## Goal

Add `lib/crates/domain-model` as a reusable Rust library that owns CRUD over
the CDISC SDTM domain model aggregates: `SdtmVersion`, `SdtmDomain`, and
`SdtmVariable`.

The crate owns:

- The PostgreSQL schema for `sdtm_versions`, `sdtm_domains`, and
  `sdtm_variables` (three SQLx migrations).
- The typed enums (`DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`,
  `SdtmRole`) with `as_str` / `TryFrom<&str>` round-trip.
- The three PostgreSQL-backed repositories — `SdtmVersionRepo`,
  `SdtmDomainRepo`, `SdtmVariableRepo` — implementing the inbound ports
  declared in `domain`.
- A single `DomainModelUsecase<V, D, Va>` (generic over all three repositories)
  that orchestrates every CRUD operation and projects domain aggregates into
  view DTOs.
- The outbound `apis::domain_model::DomainModelService` port plus an
  in-memory facade adapter (`DomainModelServiceImpl`) so HTTP handler tests
  can run without Postgres.

The crate does **not** own user / project / auth / terminology. It only
persists the SDTM domain model tree. `aegis-desktop` is intentionally
untouched in this iteration.

The data model comes from the user spec:

```rust
struct SdtmVersion {
    id: i64,
    name: String,
}

struct SdtmDomain {
    id: i64,
    version_id: i64,
    name: String,
    category: DomainCategory,
    descriptions: Vec<SdtmDomainDescription>, // stored as JSONB
}

struct SdtmVariable {
    id: i64,
    domain_id: i64,
    name: String,
    variable_controlled: Option<String>,
    variable_type: SdtmVariableType,
    variable_core: SdtmVariableCore,
    variable_role: Option<SdtmRole>,
    variable_sequence: i64,
    descriptions: Vec<SdtmVariableDescription>, // stored as JSONB
}
```

ID type is `i64` (BIGSERIAL / BIGINT) to match the workspace convention used
by `user`, `project`, and `terminology` — the user spec's `i32` is upgraded
to `i64` per their decision.

## Architecture

Ports-and-adapters DDD structure, mirroring
[`lib/crates/terminology/`](../../lib/crates/terminology/) per
[`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md).

- `domain` — enums, three aggregates, ports (`SdtmVersionRepository`,
  `SdtmDomainRepository`, `SdtmVariableRepository`), and `DomainError`. No
  I/O, no `sqlx`, no `tokio`. Two constructors per aggregate (validating
  `new` returning `Result<Self, DomainError>`; `pub(crate) for_repository`
  for the adapter layer). Hand-rolled `Debug` impls.
- `usecase` — `DomainModelUsecase<V, D, Va>` generic over the three
  repositories; command DTOs (`CreateSdtmVersion`, `UpdateSdtmDomain`, …);
  view DTOs (`SdtmVersionView`, `SdtmDomainView`, `SdtmVariableView`);
  `UsecaseError`; free-function `*_validate_*` pre-flight checks mirroring
  the `terminology` crate's shape.
- `adapter`
  - `adapter/persistence/postgres/` — one repo file per aggregate
    (`sdtm_version_repo.rs`, `sdtm_domain_repo.rs`,
    `sdtm_variable_repo.rs`); each holds its own `map_db_error` that
    translates `sqlx::Error::Database` SQLSTATE codes into the typed
    `DomainError` variants.
  - `adapter/facade/in_memory/` — `DomainModelServiceImpl` adapts
    `DomainModelUsecase` to `apis::domain_model::DomainModelService`,
    translating `UsecaseError` → `DomainModelApiError`.

Per the guideline: no `mod.rs`; each top-level module uses `src/<module>.rs`
+ `src/<module>/`. Terminal leaf modules (`*_repo.rs`, `row` submodules) are
leaf files with no companion directory.

## Repository Surface

After the user's clarifications:

- `SdtmVersionRepository` — `create`, `list`, `update`, `delete`.
  No `find_by_id`, no `find_by_name` (the apis port likewise drops
  `get_version_by_id` / `get_version_by_name`).
- `SdtmDomainRepository` — `create`, `find_by_id`, `list_by_version`,
  `update`, `delete`. No bare `list()` — the only list path is scoped to a
  version.
- `SdtmVariableRepository` — `create`, `find_by_id`, `list_by_domain`,
  `update`, `delete`. No bare `list()` — the only list path is scoped to a
  domain.

`update` returns the updated aggregate via `UPDATE … RETURNING *`;
`delete` runs `DELETE FROM … WHERE id = $1`. Neither needs a separate
fetch on the port surface.

## Data Model

### Enums

```rust
// src/domain/domain_category.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DomainCategory {
    #[serde(rename = "Special Purpose")] SpecialPurpose,
    #[serde(rename = "Interventions")]   Interventions,
    #[serde(rename = "Events")]          Events,
    #[serde(rename = "Findings")]        Findings,
    #[serde(rename = "Trial Design")]    TrialDesign,
    #[serde(rename = "Relationships")]   Relationships,
    #[serde(rename = "Study Reference")] StudyReference,
}
impl DomainCategory { pub fn as_str(&self) -> &'static str; /* all 7 cases */ }
impl std::convert::TryFrom<&str> for DomainCategory {
    type Error = DomainError;
}

// src/domain/variable_type.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableType { Numeric, Character }
impl SdtmVariableType { pub fn as_str(&self) -> &'static str; /* "Numeric" | "Character" */ }
impl std::convert::TryFrom<&str> for SdtmVariableType {
    type Error = DomainError;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum SdtmVariableCore { Req, Exp, Perm, Supp }
impl SdtmVariableCore { pub fn as_str(&self) -> &'static str; /* "Req" | "Exp" | "Perm" | "Supp" */ }
impl std::convert::TryFrom<&str> for SdtmVariableCore {
    type Error = DomainError;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SdtmRole {
    Identifier,
    #[serde(rename = "Topic")]              Topic,
    #[serde(rename = "Timing")]             Timing,
    #[serde(rename = "Record Qualifier")]   RecordQualifier,
    #[serde(rename = "Synonym Qualifier")]  SynonymQualifier,
    #[serde(rename = "Variable Qualifier")] VariableQualifier,
    #[serde(rename = "Grouping Qualifier")] GroupingQualifier,
    Rule,
}
impl SdtmRole { pub fn as_str(&self) -> &'static str; /* all 8 cases */ }
impl std::convert::TryFrom<&str> for SdtmRole {
    type Error = DomainError;
}
```

Each enum has its own `as_str` and `TryFrom` so the DB ↔ domain boundary
maps strings losslessly. `serde` derives exist so the same enum round-trips
through the JSONB column and through the apis port DTOs.

### Aggregates

```rust
// src/domain/sdtm_version.rs
pub struct SdtmVersion {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/domain/sdtm_domain.rs
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

pub struct SdtmDomain {
    pub id: i64,
    pub version_id: i64,
    pub name: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// src/domain/sdtm_variable.rs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescription {
    pub lang: String,
    pub details: SdtmVariableDescriptionDetail,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SdtmVariableDescriptionDetail {
    pub label: String,
}

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
```

The two `*Description` and `*DescriptionDetail` types derive
`Serialize + Deserialize` so the `Vec<_>` stored in the JSONB column
round-trips losslessly through `serde_json::to_value` / `from_value` at the
adapter boundary.

### Two-constructor pattern

```rust
impl SdtmVersion {
    /// Public validating ctor used by the domain layer. Rejects
    /// empty / whitespace `name`.
    pub fn new(name: String) -> Result<Self, DomainError> { … }

    /// Bypasses validation. Reserved for the adapter layer when
    /// materialising rows from persistence.
    pub(crate) fn for_repository(
        id: i64, name: String,
        created_at: DateTime<Utc>, updated_at: DateTime<Utc>,
    ) -> Self { … }
}

impl SdtmDomain {
    pub fn new(
        version_id: i64, name: String, category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
    ) -> Result<Self, DomainError> { … }                       // rejects empty `name`
    pub(crate) fn for_repository(
        id: i64, version_id: i64, name: String, category: DomainCategory,
        descriptions: Vec<SdtmDomainDescription>,
        created_at: DateTime<Utc>, updated_at: DateTime<Utc>,
    ) -> Self { … }
}

impl SdtmVariable {
    pub fn new(
        domain_id: i64, name: String,
        variable_controlled: Option<String>,
        variable_type: SdtmVariableType, variable_core: SdtmVariableCore,
        variable_role: Option<SdtmRole>, variable_sequence: i64,
        descriptions: Vec<SdtmVariableDescription>,
    ) -> Result<Self, DomainError> { … }                       // rejects empty `name`
    pub(crate) fn for_repository(/* full field set */) -> Self { … }
}
```

Hand-rolled `Debug` impls follow the structural pattern used by
`TerminologyVersion` and `Project` — every field is currently safe to log;
the pattern is preserved so a future redaction has a single seam.

### DomainError

```rust
#[derive(Debug, thiserror::Error)]
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

The `Invalid*` variants carry the rejected string so tests and ops can see
what the caller passed. `Fk*NotFound` maps SQLSTATE `23503` (FK violation) —
friendlier than a raw `Repository(driver_message)` for an obvious caller
error.

## Database Schema

Three migrations under `migrations/`.

### `0001_create_sdtm_versions.sql`

```sql
CREATE TABLE sdtm_versions (
    id BIGSERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_versions_name_unique UNIQUE (name)
);

CREATE OR REPLACE FUNCTION sdtm_versions_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sdtm_versions_set_updated_at
    BEFORE UPDATE ON sdtm_versions
    FOR EACH ROW EXECUTE FUNCTION sdtm_versions_set_updated_at();
```

### `0002_create_sdtm_domains.sql`

```sql
CREATE TABLE sdtm_domains (
    id BIGSERIAL PRIMARY KEY,
    version_id BIGINT NOT NULL REFERENCES sdtm_versions(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    category TEXT NOT NULL,
    descriptions JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_domains_category_check
        CHECK (category IN ('Special Purpose','Interventions','Events',
                            'Findings','Trial Design','Relationships','Study Reference')),
    CONSTRAINT sdtm_domains_version_name_unique UNIQUE (version_id, name)
);
CREATE INDEX sdtm_domains_version_id_idx ON sdtm_domains (version_id);

CREATE OR REPLACE FUNCTION sdtm_domains_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sdtm_domains_set_updated_at
    BEFORE UPDATE ON sdtm_domains
    FOR EACH ROW EXECUTE FUNCTION sdtm_domains_set_updated_at();
```

### `0003_create_sdtm_variables.sql`

```sql
CREATE TABLE sdtm_variables (
    id BIGSERIAL PRIMARY KEY,
    domain_id BIGINT NOT NULL REFERENCES sdtm_domains(id) ON DELETE CASCADE,
    name TEXT NOT NULL,
    variable_controlled TEXT NULL,
    variable_type TEXT NOT NULL,
    variable_core  TEXT NOT NULL,
    variable_role  TEXT NULL,
    variable_sequence BIGINT NOT NULL,
    descriptions JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_variables_type_check
        CHECK (variable_type IN ('Numeric','Character')),
    CONSTRAINT sdtm_variables_core_check
        CHECK (variable_core IN ('Req','Exp','Perm','Supp')),
    CONSTRAINT sdtm_variables_role_check
        CHECK (variable_role IS NULL OR variable_role IN
            ('Identifier','Topic','Timing','Record Qualifier',
             'Synonym Qualifier','Variable Qualifier','Grouping Qualifier','Rule')),
    CONSTRAINT sdtm_variables_domain_name_unique UNIQUE (domain_id, name)
);
CREATE INDEX sdtm_variables_domain_id_idx ON sdtm_variables (domain_id);

CREATE OR REPLACE FUNCTION sdtm_variables_set_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER sdtm_variables_set_updated_at
    BEFORE UPDATE ON sdtm_variables
    FOR EACH ROW EXECUTE FUNCTION sdtm_variables_set_updated_at();
```

`updated_at` is auto-managed by the per-table trigger on every UPDATE. The
`ON DELETE CASCADE` on `sdtm_domains.version_id` and
`sdtm_variables.domain_id` lets `delete_version` and `delete_domain` remove
children in one repository call. JSONB is the on-disk format for
`descriptions`; the Rust types round-trip via
`serde_json::to_value` / `from_value` at the adapter boundary. CHECK
constraints mirror the Rust enum's allowed values for belt-and-braces.

### Module-level comment on SQLx API choice

Per the established workspace convention (see
`terminology::adapter::persistence::postgres`), this crate uses the
**runtime** SQLx API (`sqlx::query_as`, `QueryBuilder`) rather than the
compile-time-checked macros — those macros require either a live
`DATABASE_URL` or a checked-in `sqlx-data.json` offline cache, neither of
which the workspace build currently provides.

## Usecase Layer

`DomainModelUsecase<V, D, Va>` is generic over the three repositories and
constructed via a `DomainModelUsecaseConfig` because three args crosses the
guideline's readability threshold:

```rust
pub struct DomainModelUsecaseConfig<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    pub version_repo: V,
    pub domain_repo: D,
    pub variable_repo: Va,
}

pub struct DomainModelUsecase<
    V: SdtmVersionRepository,
    D: SdtmDomainRepository,
    Va: SdtmVariableRepository,
> {
    version_repo: V,
    domain_repo: D,
    variable_repo: Va,
}

impl<V, D, Va> DomainModelUsecase<V, D, Va> where … {
    pub fn new(cfg: DomainModelUsecaseConfig<V, D, Va>) -> Self { /* store fields */ }

    // SdtmVersion
    pub async fn create_version(&self, cmd: CreateSdtmVersion)
        -> Result<SdtmVersionView, UsecaseError>;
    pub async fn list_versions(&self)
        -> Result<Vec<SdtmVersionView>, UsecaseError>;
    pub async fn update_version(&self, cmd: UpdateSdtmVersion)
        -> Result<SdtmVersionView, UsecaseError>;
    pub async fn delete_version(&self, id: i64) -> Result<(), UsecaseError>;

    // SdtmDomain
    pub async fn create_domain(&self, cmd: CreateSdtmDomain)
        -> Result<SdtmDomainView, UsecaseError>;
    pub async fn get_domain_by_id(&self, id: i64)
        -> Result<SdtmDomainView, UsecaseError>;
    pub async fn list_domains_by_version(&self, version_id: i64)
        -> Result<Vec<SdtmDomainView>, UsecaseError>;
    pub async fn update_domain(&self, cmd: UpdateSdtmDomain)
        -> Result<SdtmDomainView, UsecaseError>;
    pub async fn delete_domain(&self, id: i64) -> Result<(), UsecaseError>;

    // SdtmVariable
    pub async fn create_variable(&self, cmd: CreateSdtmVariable)
        -> Result<SdtmVariableView, UsecaseError>;
    pub async fn get_variable_by_id(&self, id: i64)
        -> Result<SdtmVariableView, UsecaseError>;
    pub async fn list_variables_by_domain(&self, domain_id: i64)
        -> Result<Vec<SdtmVariableView>, UsecaseError>;
    pub async fn update_variable(&self, cmd: UpdateSdtmVariable)
        -> Result<SdtmVariableView, UsecaseError>;
    pub async fn delete_variable(&self, id: i64) -> Result<(), UsecaseError>;
}
```

The view is the *flat* projection of the row (mirroring the user / project /
terminology crate shape). Callers who want the full tree compose it
themselves from `list_versions` + `list_domains_by_version` +
`list_variables_by_domain`.

Pre-flight validation: `name` non-empty after trim on every create / update
(only re-checking `Some` fields on updates). The pre-flight functions live
alongside the usecase file
(`fn validate_create_version(&CreateSdtmVersion) -> Result<(), UsecaseError>`).

`From<SdtmVersion/SdtmDomain/SdtmVariable> for *View` projects internal
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

## `apis::domain_model` Outbound Port

`lib/crates/apis/src/domain_model.rs` is a new file that mirrors
`apis::terminology::TerminologyService` and `apis::project::ProjectService`.
The apis crate remains a leaf (no path-dep on `domain-model`).

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DomainCategory { /* 7 cases, matches domain-model */ }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmVariableType { Numeric, Character }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmVariableCore { Req, Exp, Perm, Supp }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SdtmRole { /* 8 cases, matches domain-model */ }

#[derive(Debug, Clone, Error)]
pub enum DomainModelApiError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("not found")]
    NotFound,
    #[error("sdtm version already exists: {name}")]
    DuplicateSdtmVersion { name: String },
    #[error("sdtm domain already exists for version {version_id} / {name}")]
    DuplicateSdtmDomain { version_id: i64, name: String },
    #[error("sdtm variable already exists for domain {domain_id} / {name}")]
    DuplicateSdtmVariable { domain_id: i64, name: String },
    #[error("repository error: {0}")]
    Repository(String),
}

// View projections
pub struct SdtmVersionView { pub id, name, created_at, updated_at }
pub struct SdtmDomainView { pub id, version_id, name, category, descriptions, created_at, updated_at }
pub struct SdtmVariableView { pub id, domain_id, name, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, created_at, updated_at }

// Request DTOs
pub struct CreateSdtmVersionRequest { pub name: String }
pub struct UpdateSdtmVersionRequest { pub id: i64, pub name: Option<String> }
pub struct CreateSdtmDomainRequest { pub version_id: i64, pub name: String, pub category: DomainCategory, pub descriptions: Vec<SdtmDomainDescription> }
pub struct UpdateSdtmDomainRequest { pub id: i64, pub name: Option<String>, pub category: Option<DomainCategory>, pub descriptions: Option<Vec<SdtmDomainDescription>> }
pub struct CreateSdtmVariableRequest { pub domain_id: i64, pub name: String, pub variable_controlled: Option<String>, pub variable_type: SdtmVariableType, pub variable_core: SdtmVariableCore, pub variable_role: Option<SdtmRole>, pub variable_sequence: i64, pub descriptions: Vec<SdtmVariableDescription> }
pub struct UpdateSdtmVariableRequest { pub id: i64, pub name: Option<String>, pub variable_controlled: Option<Option<String>>, pub variable_type: Option<SdtmVariableType>, pub variable_core: Option<SdtmVariableCore>, pub variable_role: Option<Option<SdtmRole>>, pub variable_sequence: Option<i64>, pub descriptions: Option<Vec<SdtmVariableDescription>> }

#[async_trait]
pub trait DomainModelService: Send + Sync {
    // SdtmVersion — 4 methods
    async fn create_version(&self, req: CreateSdtmVersionRequest)
        -> Result<SdtmVersionView, DomainModelApiError>;
    async fn list_versions(&self) -> Result<Vec<SdtmVersionView>, DomainModelApiError>;
    async fn update_version(&self, req: UpdateSdtmVersionRequest)
        -> Result<SdtmVersionView, DomainModelApiError>;
    async fn delete_version(&self, id: i64) -> Result<(), DomainModelApiError>;

    // SdtmDomain — 5 methods
    async fn create_domain(&self, req: CreateSdtmDomainRequest)
        -> Result<SdtmDomainView, DomainModelApiError>;
    async fn get_domain_by_id(&self, id: i64)
        -> Result<SdtmDomainView, DomainModelApiError>;
    async fn list_domains_by_version(&self, version_id: i64)
        -> Result<Vec<SdtmDomainView>, DomainModelApiError>;
    async fn update_domain(&self, req: UpdateSdtmDomainRequest)
        -> Result<SdtmDomainView, DomainModelApiError>;
    async fn delete_domain(&self, id: i64) -> Result<(), DomainModelApiError>;

    // SdtmVariable — 5 methods
    async fn create_variable(&self, req: CreateSdtmVariableRequest)
        -> Result<SdtmVariableView, DomainModelApiError>;
    async fn get_variable_by_id(&self, id: i64)
        -> Result<SdtmVariableView, DomainModelApiError>;
    async fn list_variables_by_domain(&self, domain_id: i64)
        -> Result<Vec<SdtmVariableView>, DomainModelApiError>;
    async fn update_variable(&self, req: UpdateSdtmVariableRequest)
        -> Result<SdtmVariableView, DomainModelApiError>;
    async fn delete_variable(&self, id: i64) -> Result<(), DomainModelApiError>;
}
```

`UpdateSdtmVariableRequest.variable_controlled` and
`UpdateSdtmVariableRequest.variable_role` use `Option<Option<T>>` so the
caller can distinguish "don't change" (outer `None`) from "clear the
field" (outer `Some(None)`). The other `Option<…>` fields
(`descriptions`, `variable_sequence`, …) are flat `Option<T>` — `None`
means "don't change", `Some(value)` means "replace with this value"
(empty `Vec` is the explicit clear for `descriptions`).

## In-Memory Facade

`src/adapter/facade/in_memory/service.rs`:

```rust
pub struct DomainModelServiceImpl<U: DomainModelUsecaseMethods> { usecase: U }
impl<U> DomainModelServiceImpl<U> where U: /* usecase-erased bounds */ {
    pub fn new(usecase: U) -> Self { Self { usecase } }
}
#[async_trait]
impl<U> DomainModelService for DomainModelServiceImpl<U> where … {
    // 14 methods: forward to usecase, translate UsecaseError -> DomainModelApiError
}
```

Error mapping: `UsecaseError::Validation(EmptyName)` →
`DomainModelApiError::Validation("name must not be empty")`;
`UsecaseError::Repository(SdtmVersionNotFound(id))` →
`DomainModelApiError::NotFound`; everything else →
`DomainModelApiError::Repository(msg)`. Same translation table across all
three aggregates — a free function `fn map_domain_model_error(e:
UsecaseError) -> DomainModelApiError` lives at the top of the file.

## HTTP Routes in `aegis-server`

### Workspace wiring

- `apps/server/aegis-server/Cargo.toml` — add
  `domain-model = { path = "../../../lib/crates/domain-model" }`.
- `apps/server/aegis-server/src/state.rs` — add
  `pub domain_model: Arc<dyn apis::domain_model::DomainModelService>` to
  `AppState`.
- `apps/server/aegis-server/src/run.rs` — build via
  `build_domain_model_service(pool)`. The pool is shared with
  auth / user / project / terminology — no new pool or env var.

```rust
fn build_domain_model_service(pool: PgPool)
    -> Arc<dyn apis::domain_model::DomainModelService>
{
    let version_repo  = domain_model::SdtmVersionRepo::new(pool.clone());
    let domain_repo   = domain_model::SdtmDomainRepo::new(pool.clone());
    let variable_repo = domain_model::SdtmVariableRepo::new(pool);
    let usecase = domain_model::DomainModelUsecase::new(
        domain_model::DomainModelUsecaseConfig {
            version_repo, domain_repo, variable_repo,
        },
    );
    Arc::new(domain_model::DomainModelServiceImpl::new(usecase))
}
```

### Routes under `/api/domain-model/*`

| Verb   | Path                                          | Auth       |
|--------|-----------------------------------------------|------------|
| POST   | `/api/domain-model/versions`                  | admin/root |
| GET    | `/api/domain-model/versions`                  | any auth   |
| PATCH  | `/api/domain-model/versions/{id}`             | admin/root |
| DELETE | `/api/domain-model/versions/{id}`             | admin/root |
| POST   | `/api/domain-model/domains`                   | admin/root |
| GET    | `/api/domain-model/domains/{id}`              | any auth   |
| GET    | `/api/domain-model/versions/{id}/domains`     | any auth   |
| PATCH  | `/api/domain-model/domains/{id}`              | admin/root |
| DELETE | `/api/domain-model/domains/{id}`              | admin/root |
| POST   | `/api/domain-model/variables`                 | admin/root |
| GET    | `/api/domain-model/variables/{id}`            | any auth   |
| GET    | `/api/domain-model/domains/{id}/variables`    | any auth   |
| PATCH  | `/api/domain-model/variables/{id}`            | admin/root |
| DELETE | `/api/domain-model/variables/{id}`            | admin/root |

Every write handler calls `require_admin_or_root(&claims)?;` (existing
helper in `transport::http::auth::middleware`) before dispatching to the
usecase. Reads require authentication (`AuthClaims` extractor without the
role check) so the bearer-token path is exercised end-to-end.

Module shape:

```
apps/server/aegis-server/src/transport/http/
├── domain_model.rs           # new: pub mod handlers; pub mod router;
└── domain_model/
    ├── handlers.rs           # new
    └── router.rs             # new
```

`router.rs` uses `OpenApiRouter::new().routes(routes!(...))` per HTTP verb
(matching the `terminology/router.rs` pattern).

`handlers.rs` follows the terminology handlers shape: utoipa-axum
`#[utoipa::path(...)]` annotation per handler with `security(("BearerAuth" =
[]))`, the existing `ErrorBody` for 400/401/403/404/409/500 response codes,
and a 201 `Created` for the `POST` handlers.

### Wire DTOs

`apps/server/aegis-server/src/transport/http/dto.rs` is extended with:

- `CreateSdtmVersionRequest`, `UpdateSdtmVersionRequest`,
  `SdtmVersionViewResponse`, `SdtmVersionListResponse`
- `CreateSdtmDomainRequest`, `UpdateSdtmDomainRequest`,
  `SdtmDomainViewResponse`, `SdtmDomainListResponse`,
  `SdtmDomainDescription`, `SdtmDomainDescriptionDetail`
- `CreateSdtmVariableRequest`, `UpdateSdtmVariableRequest`,
  `SdtmVariableViewResponse`, `SdtmVariableListResponse`,
  `SdtmVariableDescription`, `SdtmVariableDescriptionDetail`
- Wire-side re-declarations of the four enums (`DomainCategory`,
  `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`) with
  `utoipa::ToSchema` and serde renames matching the apis / domain
  variants.

Each wire DTO has `From<apis::domain_model::*>` and `Into<…>` for the
request direction.

### Error mapping

`apps/server/aegis-server/src/transport/http/error.rs` gains one new
`From<DomainModelApiError> for ApiError`:

- `Validation(_)` → `ApiError::Validation(...)` (400)
- `NotFound` → `ApiError::NotFound` (404)
- `DuplicateSdtmVersion` / `DuplicateSdtmDomain` / `DuplicateSdtmVariable`
  → `ApiError::Conflict(...)` (409)
- `Repository(_)` → `ApiError::Internal(...)` (500)

### OpenAPI registration

`apps/server/aegis-server/src/transport/http/openapi.rs` is extended with
the new `DomainModelViewResponse`, `DomainModelListResponse`, request
DTOs, and the new path listing.

## Workspace Wiring

- `Cargo.toml` `[workspace].members` gains `"lib/crates/domain-model"`.
- `lib/crates/domain-model/Cargo.toml` inherits every dep via
  `{ workspace = true }`: `sqlx`, `tokio`, `async-trait`, `thiserror`,
  `chrono`. The `chrono` dep gets a one-line comment explaining it
  carries `created_at` / `updated_at`.
- `serde` is added to the workspace-dep inherited set (the four enums +
  the description DTOs derive `Serialize + Deserialize`).
- `serde_json` is added to the workspace-dep inherited set (JSONB
  round-trip at the adapter boundary).
- Path-dep on `apis` (`apis = { path = "../apis" }`) for the
  `DomainModelService` trait the facade implements.
- `dev-dependencies` add `dotenvy`, `sqlx`, and `tokio` for the ignored
  integration tests.

## Tests

Following the guideline's tier order.

1. **Domain unit tests** (`src/domain/tests.rs`)
   - Each enum's `TryFrom<&str>` parses every legal value; rejects
     others as the right `DomainError` variant.
   - `SdtmVersion::new`, `SdtmDomain::new`, `SdtmVariable::new` reject
     empty / whitespace `name`.
   - Each enum's `as_str()` round-trips through `TryFrom`.

2. **Adapter unit tests** (`src/adapter/persistence/postgres/tests.rs`)
   - Read each migration via
     `std::fs::read_to_string(env!("CARGO_MANIFEST_DIR") + "/migrations/<file>")`
     and assert: columns + their types, `UNIQUE` constraints, `CHECK`
     constraints, `updated_at` trigger, JSONB default `'[]'::jsonb` on
     `descriptions`, `ON DELETE CASCADE` on the FKs.
   - `*Row → *Aggregate` `TryFrom` happy-path conversion for each of the
     three repos.

3. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`)
   - `DomainModelServiceImpl` wired on top of in-memory
     `Arc<Mutex<Vec<…>>>` + `AtomicI64` fakes for the three
     repositories (mirrors
     `terminology::adapter::facade::in_memory::tests`).
   - Round-trip every public method (4 + 5 + 5 = 14 methods).
   - Cascade: `delete_version` clears domains and variables;
     `delete_domain` clears variables.

4. **`tests/public_api.rs`** — compile-only
   - Names every documented consumer import.
   - Pins the constructor chain: `fn(sqlx::PgPool) -> _` for each of
     the three Postgres repos, `fn(DomainModelUsecaseConfig<V, D, Va>) -> _`
     for `DomainModelUsecase::new`, `fn(DomainModelUsecase<...>) -> _`
     for `DomainModelServiceImpl::new`.
   - Asserts each repo `Send + Sync` and `DomainModelServiceImpl` is
     `Send + Sync`.

5. **`tests/integration_persistence.rs`** — live PG round-trips.
   - `#[ignore]`-gated.
   - Loads `.env` via `dotenvy::dotenv()`, reads `AEGIS_DATABASE_URL`
     (panic with a clear message if missing).
   - Drops `sdtm_variables`, `sdtm_domains`, `sdtm_versions`,
     `_sqlx_migrations` before each run, then applies migrations via
     `sqlx::migrate!("./migrations")`.
   - Per-run unique `name` values via an atomic counter + wall-clock
     nanos so concurrent runs do not collide on the UNIQUE constraints.
   - Round-trips CRUD + the scoped-list queries + cascade delete.

6. **HTTP integration tests**
   (`apps/server/aegis-server/tests/integration_domain_model.rs`,
   matches the existing `integration_auth.rs` shape):
   - `dotenvy::dotenv()`; reads `AEGIS_DATABASE_URL`; drops the four
     `domain_model` tables + `_sqlx_migrations`; applies migrations.
   - Wire tests against the real axum router using
     `tower::ServiceExt::oneshot` + the same `MockAuth` pattern from
     `transport::http::auth::middleware::tests`.
   - Cases: read routes return 200 for any authenticated user (token
     `Bearer good:u1:general:0`); write routes return 403 for the
     `general` token, 200 for `admin` (`Bearer good:u1:admin:0`), 200 for
     `root` (`Bearer good:u1:root:0`), 401 for missing token.

## README

`lib/crates/domain-model/README.md` covers:

- One-sentence purpose (SDTM domain model — versions, domains,
  variables — CRUD).
- A `src/` tree matching the actual module shape.
- Database setup: `sqlx migrate run --source lib/crates/domain-model/migrations`
  + `AEGIS_DATABASE_URL` env var + a small constructor snippet.
- How to run the ignored tests
  (`cargo test -p domain-model -- --ignored`).
- A back-link to the guideline.

## Verification Gate

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
cargo test -p domain-model -- --ignored --test-threads=1   # when AEGIS_DATABASE_URL is set

# After the aegis-server work lands:
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test -p aegis-server

# Full workspace sanity:
cargo check --workspace
cargo clippy --workspace
cargo test --workspace
```

## Commits

One commit per logical change (matching the guideline):

1. **scaffold** — register crate, basic `Cargo.toml`, empty `lib.rs`.
2. **domain** — enums, three aggregates, ports, `DomainError`, domain
   tests.
3. **usecase** — `DomainModelUsecase`, command / view DTOs,
   `UsecaseError`, usecase in-memory wire-up tests.
4. **persistence** — three migrations; `SdtmVersionRepo`,
   `SdtmDomainRepo`, `SdtmVariableRepo`; per-repo `row` submodule;
   postgres unit tests + integration tests (`#[ignore]`-gated).
5. **apis port** — `apis::domain_model` new file
   (`DomainModelService` trait + DTOs + error).
6. **facade** — `DomainModelServiceImpl` in
   `adapter/facade/in_memory/service.rs`; facade tests.
7. **public_api** — `tests/public_api.rs` compile-only test.
8. **readme** — `README.md` at the crate root.
9. **chore: lockfile** — `Cargo.lock` drift after new deps land.
10. **aegis-server wiring** — Cargo.toml dep, `state.rs` field,
    `run.rs::build_domain_model_service`.
11. **aegis-server handlers + router** — `transport/http/domain_model/`,
    DTO extensions, error mapping, OpenAPI registration, HTTP
    integration tests.
12. **chore: lockfile** (server-side drift).

Each commit message lists the spec coverage and the verification commands
at the bottom so reviewers can run the same gate locally.
