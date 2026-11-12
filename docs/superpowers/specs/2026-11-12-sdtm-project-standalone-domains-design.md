# SDTM Project — Standalone Domains & Variables Design

## Intent

The `sdtm-project` crate was built around an *override* model: a project
domain was a thin pointer to a `domain-model` SDTM domain (a `domain_code` and
a list of `key_variables`), and the project's own metadata lived in a 1:1
`sdtm_project_domain_overrides` row. A project variable was a
`(domain_id, variable_code)` pair plus a 1:1
`sdtm_project_variable_overrides` row carrying type / core / role / descriptions.

That model does not fit a real project. A study's SDTM domain is not "the CDISC
domain plus a patch" — it *is* the domain the study uses, with the study's own
categories, descriptions, variable metadata and the rules that convert CRF
data into it. Splitting "identity" from "override" forced every read to join
two tables, forced two writes to keep them in step, and made the override
tables the de-facto home of the data while the parent tables stayed hollow.

This refactor makes each project domain and each project variable a **complete,
self-contained copy** of the corresponding `domain-model` SDTM domain /
variable, and adds the piece the model was missing: **conversion rules** — how
a variable's value is derived from a protocol, from a derived variable, from an
assigned value, from a CRF form, or from an eDiT.

## Goals

1. Drop the override design. `sdtm_project_domains` and
   `sdtm_project_variables` each carry the full metadata of their row. The
   two override tables, their repository ports, their `apis` methods and their
   HTTP endpoints are removed — nothing replaces them.
2. Add `conversion_rules` to a project variable, with the `VariableSource`
   taxonomy below.
3. Keep `sdtm_project_domains.key_variables` referentially sound: every code
   it names must be a variable that exists in the same domain, and that must
   stay true as variables are created, renamed and deleted.
4. Rewrite the crate's Postgres DDL in place rather than layering `ALTER
   TABLE` migrations on top.

## Non-goals

- **`aegis-desktop` is not touched.** No TS types, no HTTP client, no feature
  modules. The desktop app does not currently reference the sdtm-project
  endpoints at all, so the wire changes are safe to make now.
- **No sharing of types with `domain-model`.** The crate keeps its deliberate
  copy convention (see "Relationship to `domain-model`" below), even though the
  copied shapes are now field-for-field identical to `domain-model`'s.
- **`FromCrf.form_code` is not validated.** A conversion rule may name a CRF
  form code that does not exist; the `crf` crate is not consulted.
- **No new `apis` service or new top-level endpoint.** The sdtm-project
  namespace keeps its current route shape minus the override routes.

## Anti-goals

- Do **not** keep the override tables "for history" — a half-populated parent
  row plus a populated override row is exactly the split this refactor exists
  to remove.
- Do **not** silently repair `key_variables` on the caller's behalf. If a
  caller renames a key variable, the rename is rejected; the caller removes it
  from `key_variables` first. Quietly rewriting a study's key is not a
  decision an adapter should make.

## Constraints

- `edition = "2024"`, `resolver = "3"`, workspace-inherited dependencies.
- The crate follows `docs/guidelines/lib-crate-development.md`: DDD layering
  (domain → usecase → adapter), one repository port per aggregate, one error
  type per boundary, live-DB tests behind `#[ignore]`.
- The `apis` crate holds only ports and DTOs — no `domain` / `usecase` /
  `adapter` split.
- Wire DTOs are hand-written in both halves: `apis/src/sdtm_project.rs` and
  `apps/server/aegis-server/src/transport/http/dto.rs`. Both change in the
  same commit.

## Relationship to `domain-model`

`domain-model` describes the **CDISC SDTM standard** (one row per published IG
version, one row per standard domain, one row per standard variable).
`sdtm-project` describes **a project's own model tree**.

Before this refactor the relationship was "reference + patch": the project
pointed at a standard domain and stored an override. After this refactor the
relationship is "an independent, complete copy": the project domain carries
the same fields a standard domain carries, with the project's own values.

`domain-model`'s aggregates are the **field reference** for the shape:

| `domain-model` | `sdtm-project` (before) | `sdtm-project` (after) |
| --- | --- | --- |
| `SdtmDomain { id, version_id, name, category, descriptions, … }` | `SdtmProjectDomain { id, version_id, domain_code, key_variables, domain_override }` | `SdtmProjectDomain { id, sdtm_project_version_id, domain_code, category, descriptions, key_variables, … }` |
| `SdtmVariable { id, domain_id, name, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, … }` | `SdtmProjectVariable { id, domain_id, variable_code, variable_sequence, variable_override }` | `SdtmProjectVariable { id, sdtm_project_domain_id, variable_code, variable_controlled, variable_type, variable_core, variable_role, variable_sequence, descriptions, conversion_rules, … }` |

The crate **copies** the value objects rather than importing them
(`DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`,
`SdtmVariableControlled`, and the description DTOs). The rationale is
unchanged: the standard and a project's model are allowed to drift
independently, and a shared type would silently couple them. Two new copies
are added to `sdtm-project`'s `domain/` layer: `SdtmVariableControlled` and
`SdtmDomainDescription` / `SdtmVariableDescription` (the latter two already
exist there).

### Naming decisions

- The FK fields are renamed per the task: `version_id` →
  `sdtm_project_version_id`, `domain_id` → `sdtm_project_domain_id`. The
  rename makes the target explicit now that a "domain id" no longer implies
  the standard domain.
- The code fields keep their current names — `domain_code` and
  `variable_code` — rather than becoming `name` as in `domain-model`. The
  sdtm-project aggregates are never ambiguous about what a code identifies, and
  `key_variables` holds *variable codes*, so the vocabulary stays consistent
  across the crate.
- The rename carries all the way to the wire: the HTTP request/response
  fields become `sdtmProjectVersionId` and `sdtmProjectDomainId`.

## Data Model

### `VariableSource`

Where a variable's value comes from.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "source", content = "from")]
pub enum VariableSource {
    /// The value is specified by the protocol.
    #[serde(rename = "Protocol")]
    Protocol,
    /// The value is computed from other variables.
    #[serde(rename = "Derived")]
    Derived,
    /// The value is assigned manually, optionally from a CRF form.
    #[serde(rename = "Assigned")]
    Assigned(Option<FromCrf>),
    /// The value is collected on a CRF form, optionally a specific one.
    #[serde(rename = "CRF")]
    Crf(Option<FromCrf>),
    /// The value comes from an eDiT.
    #[serde(rename = "eDT")]
    Edt,
}

/// The CRF form a source draws from.
///
/// `form_code` is the code field of the CRF form. It is **not** validated
/// against the `crf` crate in this iteration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FromCrf {
    pub form_code: String,
}
```

`VariableSource` is **adjacently tagged** — `#[serde(tag = "source", content =
"from")]` — so the unit variants stay flat strings while the two payload
variants carry an optional `from` object:

```json
{ "source": "Protocol" }
{ "source": "Derived" }
{ "source": "eDT" }
{ "source": "CRF", "from": { "form_code": "AE" } }
{ "source": "Assigned", "from": null }
```

The Rust variant names stay `Crf` and `Edt`; the wire spellings are `CRF` and
`eDT`, set with per-variant `#[serde(rename = ...)]` rather than a blanket
`rename_all` (a blanket rule cannot produce both `CRF` and `eDT`).

The tags above are the **domain / `apis`** spelling, which is snake_case. The
HTTP layer keeps the repo's camelCase convention, so the same value reaches a
browser as `{"source": "CRF", "from": {"formCode": "AE"}}`.

### `ConversionRule`

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversionRule {
    #[serde(flatten)]
    pub source: VariableSource,
    pub rule: String,
}
```

`rule` is the expression that turns the source into the variable's value. It
is opaque text to this crate — no parsing, no evaluation. A rule with a blank
`rule` string is rejected at validation time (the crate already rejects blank
codes and blank names; a blank rule carries no information).

The source is **flattened** into the rule, so a rule serialises as:

```json
{ "source": "CRF", "from": { "form_code": "AE" }, "rule": "AESEV" }
```

The source's own fields sit beside the rule text. Without the flatten the
source would nest under a second `source` key
(`{"source":{"source":"CRF",…},"rule":"…"}`), which is not the agreed shape.
Serde's `flatten` handles an adjacently-tagged enum correctly in both
directions — every variant round-trips, and a unit variant still omits the
`from` key entirely.

The HTTP layer keeps the repo's camelCase convention, so a browser sees
`{"source": "CRF", "from": {"formCode": "AE"}, "rule": "AESEV"}`.

### `SdtmProjectVariable`

```rust
pub struct SdtmProjectVariable {
    pub id: i64,
    pub sdtm_project_domain_id: i64,
    pub variable_code: String,
    pub variable_controlled: Option<SdtmVariableControlled>,
    pub variable_type: SdtmVariableType,
    pub variable_core: SdtmVariableCore,
    pub variable_role: Option<SdtmRole>,
    pub variable_sequence: i64,
    pub descriptions: Vec<SdtmVariableDescription>,
    pub conversion_rules: Vec<ConversionRule>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`sdtm_project_domain_id` is immutable — a variable always belongs to the
domain it was created under. `variable_controlled` and `variable_role` are
nullable columns; `variable_type` and `variable_core` are required.

### `SdtmProjectDomain`

```rust
pub struct SdtmProjectDomain {
    pub id: i64,
    pub sdtm_project_version_id: i64,
    pub domain_code: String,
    pub category: DomainCategory,
    pub descriptions: Vec<SdtmDomainDescription>,
    pub key_variables: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`sdtm_project_version_id` is immutable. `key_variables` keeps its existing
semantics: an ordered list, trimmed, de-duplicated to first occurrence, never
re-sorted — plus the new referential constraint below.

## The `key_variables` constraint

`key_variables` names variable codes **within the same domain**. Three rules:

1. **On write** (`create_domain` / `update_domain` with a `key_variables`
   value): every code must name a variable that exists in that domain.
   Otherwise → `KeyVariableNotFound { variable_code }`.
2. **On variable rename** (`update_variable` changing `variable_code`): if the
   variable's **current** code is listed in its domain's `key_variables`, the
   rename is rejected →
   `VariableReferencedByKeyVariables { variable_code }`. The caller must first
   remove the code from `key_variables`, then rename, then re-add it.
3. **On variable delete** (`delete_variable`): if the variable's code is listed
   in its domain's `key_variables`, the delete is rejected →
   `VariableReferencedByKeyVariables { variable_code }`.

The consequence worth stating plainly: **a domain is created with empty
`key_variables`**, because at creation time it has no variables yet. The
natural authoring order is therefore *create domain → create variables → set
`key_variables`*. A `create_domain` request carrying a non-empty
`key_variables` is rejected with `KeyVariableNotFound`, not silently accepted.

Rules 2 and 3 are the "reject" policy: the aggregate never enters a state
where `key_variables` names a code that does not exist. The database does not
enforce this (the codes live in a JSONB array, and the domain / variable live
in different tables), so the usecase is the enforcement point.

## Error surface

New `DomainError` variants in `sdtm-project`:

```rust
#[error("key variable not found in domain: {variable_code}")]
KeyVariableNotFound { variable_code: String },

#[error("variable is referenced by key variables: {variable_code}")]
VariableReferencedByKeyVariables { variable_code: String },
```

The domain is not named in `KeyVariableNotFound`: the request that carries the
key-variable list already names the domain, and the create path has no domain
id yet — a new domain has no variables, so every code it lists is missing.

Matching `apis::sdtm_project::SdtmProjectApiError` variants, and the HTTP
mapping in `aegis-server`:

| Error | HTTP | `ErrorBody.code` |
| --- | --- | --- |
| `KeyVariableNotFound` | 400 Bad Request | `key_variable_not_found` |
| `VariableReferencedByKeyVariables` | 409 Conflict | `variable_referenced_by_key_variables` |

`KeyVariableNotFound` is a 400: the request body named something that does not
exist. `VariableReferencedByKeyVariables` is a 409: the request is
well-formed but conflicts with the current state of the domain.

All the override-related error variants are removed:
`SdtmProjectDomainOverrideNotFound`, `SdtmProjectVariableOverrideNotFound`,
`DuplicateSdtmProjectDomainOverride`, `DuplicateSdtmProjectVariableOverride`.

## Postgres DDL

The crate's migrations are **rewritten in place**, not amended. The crate is
unreleased, so the five migration files are edited in place and the two
override files are deleted outright — there is no `ALTER TABLE` anywhere in
this change.

| File | Action |
| --- | --- |
| `20261103135411_create_sdtm_project_versions.sql` | unchanged |
| `20261103135412_create_sdtm_project_domains.sql` | **rewritten** |
| `20261103135413_create_sdtm_project_domain_overrides.sql` | **deleted** |
| `20261103135414_create_sdtm_project_variables.sql` | **rewritten** |
| `20261103135416_create_sdtm_project_variable_overrides.sql` | **deleted** |

> **Migration-history note.** Rewriting in place means the checksum of an
> already-applied migration changes. Any database that has already run these
> five migrations must have its `sdtm_project_*` tables and the matching
> `_sqlx_migrations` rows dropped before the rewritten migrations will apply.
> The crate's live-DB test harness already drops both on every run. This is
> the same destructive posture the rest of the workspace takes with
> `AEGIS_TEST_DATABASE_URL`.

### `sdtm_project_domains`

```sql
CREATE TABLE IF NOT EXISTS sdtm_project_domains (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_version_id  BIGINT      NOT NULL REFERENCES sdtm_project_versions(id) ON DELETE CASCADE,
    domain_code              TEXT        NOT NULL,
    category                 TEXT        NOT NULL,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    key_variables            JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_domains_version_code_unique UNIQUE (sdtm_project_version_id, domain_code),
    CONSTRAINT sdtm_project_domains_category_check CHECK (
        category IN (
            'Special Purpose', 'Interventions', 'Events', 'Findings',
            'Trial Design', 'Relationships', 'Study Reference'
        )
    ),
    CONSTRAINT sdtm_project_domains_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_domains_key_variables_is_array CHECK (jsonb_typeof(key_variables) = 'array')
);
```

The `updated_at` trigger function and trigger are carried over unchanged
(patterned after the existing file).

### `sdtm_project_variables`

```sql
CREATE TABLE IF NOT EXISTS sdtm_project_variables (
    id                       BIGSERIAL PRIMARY KEY,
    sdtm_project_domain_id  BIGINT      NOT NULL REFERENCES sdtm_project_domains(id) ON DELETE CASCADE,
    variable_code            TEXT        NOT NULL,
    variable_sequence        BIGINT      NOT NULL,
    variable_controlled      JSONB,
    variable_type            TEXT        NOT NULL,
    variable_core            TEXT        NOT NULL,
    variable_role            TEXT,
    descriptions             JSONB       NOT NULL DEFAULT '[]'::jsonb,
    conversion_rules         JSONB       NOT NULL DEFAULT '[]'::jsonb,
    created_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at               TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT sdtm_project_variables_domain_code_unique UNIQUE (sdtm_project_domain_id, variable_code),
    CONSTRAINT sdtm_project_variables_type_check CHECK (variable_type IN ('Numeric', 'Character')),
    CONSTRAINT sdtm_project_variables_core_check CHECK (variable_core IN ('Req', 'Exp', 'Perm', 'Supp')),
    CONSTRAINT sdtm_project_variables_role_check CHECK (
        variable_role IS NULL OR variable_role IN (
            'Identifier', 'Topic', 'Timing', 'Record Qualifier',
            'Synonym Qualifier', 'Variable Qualifier', 'Grouping Qualifier', 'Rule'
        )
    ),
    CONSTRAINT sdtm_project_variables_descriptions_is_array CHECK (jsonb_typeof(descriptions) = 'array'),
    CONSTRAINT sdtm_project_variables_conversion_rules_is_array CHECK (jsonb_typeof(conversion_rules) = 'array')
);
```

`variable_controlled` is nullable JSONB — the same shape `domain-model` uses
for `sdtm_variables.variable_controlled` (`{"code": [...], "format": [...]}`,
both members optional and omitted when unset).

## Implementation

### `lib/crates/sdtm-project`

**`domain/`**

- **new `conversion_rule.rs`** — `VariableSource`, `FromCrf`, `ConversionRule`.
- **`variable_type.rs`** — add the `SdtmVariableControlled` copy (from
  `domain-model/src/domain/variable_type.rs`).
- **`sdtm_project_domain.rs`** — rewrite: drop `domain_override`, add
  `category` + `descriptions`, rename `version_id` →
  `sdtm_project_version_id`. `normalize_key_variables` keeps its current
  behaviour.
- **`sdtm_project_variable.rs`** — rewrite: drop `variable_override`, add
  `variable_controlled`, `variable_type`, `variable_core`, `variable_role`,
  `descriptions`, `conversion_rules`; rename `domain_id` →
  `sdtm_project_domain_id`.
- **delete `sdtm_project_domain_override.rs`** and
  **`sdtm_project_variable_override.rs`**.
- **`repository.rs`** — delete `SdtmProjectDomainOverrideRepository` and
  `SdtmProjectVariableOverrideRepository`. The domain and variable ports lose
  their "reads hydrate the override" note. No new port method is needed: the
  key-variable check reads the domain's variables through the existing
  `SdtmProjectVariableRepository::list_by_domain`.
- **`error.rs`** — delete the four override variants; add
  `KeyVariableNotFound` and `VariableReferencedByKeyVariables`.
- **`domain.rs`** — update the module list and the re-exports.

**`usecase/`**

- **`commands.rs`** — `CreateSdtmProjectDomain` / `UpdateSdtmProjectDomain`
  and `CreateSdtmProjectVariable` / `UpdateSdtmProjectVariable` gain the new
  fields; the four override command structs are deleted.
- **`views.rs`** — the domain and variable views gain the new fields; the two
  override views are deleted.
- **`sdtm_project_usecase.rs`** —
  - The usecase becomes generic over **three** repositories plus the project
    lookup: `SdtmProjectUsecase<V, D, Va, P>`. `SdtmProjectUsecaseConfig`
    shrinks to match. All four override methods are deleted.
  - `create_domain` — normalises `key_variables`, then checks each code
    against the domain's variables (empty at creation → any code fails).
  - `update_domain` — checks each supplied `key_variables` code against the
    domain's variables.
  - `update_variable` — when `variable_code` changes, rejects the rename if
    the old code is in the domain's `key_variables`.
  - `delete_variable` — rejects the delete if the variable's code is in the
    domain's `key_variables`.
  - Validation of a `ConversionRule`'s blank `rule` string lives here.

**`adapter/persistence/postgres/`**

- **delete** `sdtm_project_domain_override_repo.rs` and
  `sdtm_project_variable_override_repo.rs`.
- **rewrite** `sdtm_project_domain_repo.rs` — drop the `LEFT JOIN` on
  `sdtm_project_domain_overrides` and the five `override_*` row fields; the
  row struct now carries `category` and `descriptions` directly.
- **rewrite** `sdtm_project_variable_repo.rs` — drop the `LEFT JOIN` on
  `sdtm_project_variable_overrides`; the row struct carries the metadata
  columns plus `variable_controlled` and `conversion_rules` directly.
- The `map_*_err` helpers keep their shape; the unique-violation and
  foreign-key-violation mappings still work because the unique constraint
  names change but the SQLSTATE codes do not.

**`adapter/facade/internal/service.rs`**

- The eight override methods are deleted; the domain and variable methods map
  the new fields.
- The `From<UsecaseError> for SdtmProjectApiError` impl loses the override
  arms and gains the two new ones.

**`lib.rs`** — crate-root re-exports drop the override items and gain
`ConversionRule`, `VariableSource`, `FromCrf`, `SdtmVariableControlled`.

**`README.md`** — the layout tree, the data-model table and the aggregate
list are updated to three tables / three aggregates.

**Tests** — `domain/tests.rs`, `usecase/tests.rs`,
`adapter/facade/internal/tests.rs`, `tests/public_api.rs` and
`tests/integration_persistence.rs` are updated. The migration-content unit
tests inside the Postgres repos are rewritten to assert the new DDL.

### `lib/crates/apis`

**`src/sdtm_project.rs`**

- New DTOs mirroring the domain layer: `VariableSource` (adjacently tagged
  with `source` / `from`, variants spelled `Protocol`, `Derived`, `Assigned`,
  `CRF`, `eDT`), `FromCrf { form_code }`, `ConversionRule { source, rule }`,
  `SdtmVariableControlled`.
- `SdtmProjectDomainView` — `sdtm_project_version_id`, `domain_code`,
  `category`, `descriptions`, `key_variables`.
- `SdtmProjectVariableView` — `sdtm_project_domain_id`, `variable_code`,
  `variable_controlled`, `variable_type`, `variable_core`, `variable_role`,
  `variable_sequence`, `descriptions`, `conversion_rules`.
- Request DTOs gain the new fields; `CreateSdtmProjectDomainRequest` and
  `CreateSdtmProjectVariableRequest` keep their required/optional split:
  `category`, `variable_type` and `variable_core` are required on create;
  everything else is optional or defaults to empty.
- The four override request DTOs, the two override view DTOs and the eight
  override trait methods are deleted.
- `SdtmProjectApiError` gains `KeyVariableNotFound` and
  `VariableReferencedByKeyVariables` and loses the four override variants.

**`tests/public_api.rs`** — updated for the new surface.

### `apps/server/aegis-server`

- **`transport/http/dto.rs`** — the sdtm-project request/response DTOs are
  rewritten to match: new fields on the domain and variable create/update
  requests and on the two view responses; the override DTOs are deleted; new
  `SdtmProjectVariableControlled`, `SdtmProjectVariableSource` and
  `SdtmProjectConversionRule` DTOs with `ToSchema`. The wire keeps the
  camelCase rename, so `sdtm_project_domain_id` → `sdtmProjectDomainId` and
  `form_code` → `formCode`; the `source` / `from` tags keep their spelling.
- **`transport/http/sdtm_project/handlers.rs`** — the eight override handlers
  are deleted; the domain and variable handlers carry the new fields; the
  documented status codes gain 400 / 409 entries where the new errors can
  surface.
- **`transport/http/sdtm_project/router.rs`** — the eight override routes are
  deleted.
- **`transport/http/error.rs`** — `sdtm_project_status` / `sdtm_project_code`
  gain the two new variants.
- **`transport/http/openapi.rs`** — the sdtm-project schema registration list
  is updated (removed override schemas, added the new ones).
- **`state.rs`** — `NullSdtmProjectService` loses the eight override methods.
- **`run.rs`** — `build_sdtm_project_service` no longer constructs the two
  override repositories.
- **`tests/integration_auth.rs`** — the local `SdtmProjectService` stub loses
  the eight override methods.

## HTTP surface after the change

```
/api/sdtm-project/versions                          GET POST
/api/sdtm-project/versions/{id}                     GET PUT DELETE
/api/sdtm-project/versions?projectCode=…            GET
/api/sdtm-project/domains                           POST
/api/sdtm-project/domains/{id}                      GET PUT DELETE
/api/sdtm-project/versions/{versionId}/domains      GET
/api/sdtm-project/variables                         POST
/api/sdtm-project/variables/{id}                    GET PUT DELETE
/api/sdtm-project/domains/{domainId}/variables      GET
```

The four `…/override` routes per resource are gone.

## Verification

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo fmt --all -- --check

# live-DB (destructive) — needs AEGIS_TEST_DATABASE_URL
cargo test -p sdtm-project -- --ignored --test-threads=1
cargo test -p aegis-server -- --ignored --test-threads=1
```

The desktop app is untouched by this change and is not part of the
verification loop, but `pnpm --filter aegis-desktop typecheck` is a cheap
regression check that nothing in the shared API surface moved.

## Resolved design questions

Decisions the spec left open, settled during planning. Each is binding.

1. **The update contract for the promoted fields.** `category`,
   `variable_type` and `variable_core` are flat `Option<T>` on the update
   request — `None` means "leave unchanged", `Some(value)` means "replace".
   `variable_role` and `variable_controlled` are the only nullable columns, so
   they use the three-state `Option<Option<T>>`: absent = leave unchanged,
   `null` = clear to SQL `NULL`, value = replace. `descriptions` and
   `conversion_rules` replace the whole list; an explicit empty list clears
   it. This is the same split `domain-model` and the old override DTOs use,
   and it is why the Postgres UPDATE uses a `CASE WHEN` for the two nullable
   columns and plain `COALESCE` for the rest.
2. **How the two new errors reach the wire.** They ride inside
   `UsecaseError::Repository(DomainError::…)` and the facade maps those two
   `DomainError` variants explicitly to the new `SdtmProjectApiError`
   variants. `UsecaseError` does not gain a third arm: the crate already
   routes reference errors that way (`FkSdtmProjectDomainNotFound`), so this
   follows the existing idiom.
3. **`create_domain` with a non-empty `key_variables`.** The field stays on
   the create request and a non-empty value is rejected with
   `KeyVariableNotFound { variable_code }`. The domain does not exist yet, so
   there is no `domain_id` to name in the error, and the field is not
   silently dropped.
4. **Re-sending the same `variable_code` is not a rename.** The key-variable
   guard fires only when the new code differs from the variable's current
   code, so ordinary metadata edits on a key variable stay possible.
5. **The blank-rule error.** `DomainError::EmptyConversionRule`, raised by the
   usecase's validation and surfacing as `SdtmProjectApiError::Validation` →
   400 `validation_failed`. A blank `FromCrf.form_code` is **not** rejected —
   `form_code` is unvalidated by design.
6. **The `ConversionRule` shape.** Flattened, as shown above.

## Out of scope, deliberately

- Copying an existing `domain-model` domain into a project domain as a
  one-shot ("import from standard") action. The new model makes that
  possible; the action itself is a separate feature.
- Validating `FromCrf.form_code` against the `crf` crate.
- Parsing or evaluating `ConversionRule.rule`.
- Any `aegis-desktop` surface for conversion rules.
