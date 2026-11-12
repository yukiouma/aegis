# sdtm-project

Owns the **per-project SDTM model tree** — the domains and variables a project
declares on top of the CDISC SDTM standard — behind three PostgreSQL tables and
an `apis::sdtm_project::SdtmProjectService` port.

The crate deliberately shares no types with `domain-model`: the CDISC standard
and a project's overlay are allowed to drift independently, so the value objects
(`DomainCategory`, `SdtmVariableType`, `SdtmVariableCore`, `SdtmRole`,
`SdtmVariableControlled` and the description DTOs) are **copied** rather than
imported.

## Layout

```
src/
├── lib.rs                        # three layer modules + crate-root re-exports
├── domain.rs                     # declares + re-exports the domain children
├── domain/
│   ├── domain_category.rs        # copied from domain-model
│   ├── variable_type.rs          # copied from domain-model
│   ├── sdtm_domain_description.rs
│   ├── sdtm_variable_description.rs
│   ├── sdtm_project_version.rs
│   ├── sdtm_project_domain.rs
│   ├── sdtm_project_variable.rs
│   ├── conversion_rule.rs        # VariableSource / FromCrf / ConversionRule
│   ├── project_lookup.rs         # cross-crate port onto the `project` crate
│   ├── repository.rs             # the three repository ports
│   ├── error.rs                  # DomainError
│   └── tests.rs
├── usecase.rs
├── usecase/
│   ├── commands.rs               # six Create/Update command DTOs
│   ├── error.rs                  # UsecaseError
│   ├── views.rs                  # three view DTOs + From impls
│   ├── sdtm_project_usecase.rs   # the usecase + its config
│   └── tests.rs
└── adapter.rs
└── adapter/
    ├── persistence.rs
    ├── persistence/postgres.rs
    ├── persistence/postgres/     # one *RepoPg per port
    ├── service.rs
    ├── service/project/          # ProjectLookupImpl -> apis::project::ProjectService
    ├── facade.rs
    └── facade/internal/          # SdtmProjectServiceImpl -> the apis port
```

## The data model

| Aggregate | Table | Uniqueness |
| --- | --- | --- |
| `SdtmProjectVersion` | `sdtm_project_versions` | `(project_code, version_name)` |
| `SdtmProjectDomain` | `sdtm_project_domains` | `(sdtm_project_version_id, domain_code)` |
| `SdtmProjectVariable` | `sdtm_project_variables` | `(sdtm_project_domain_id, variable_code)` |

A project domain and a project variable are **complete copies** of the
corresponding `domain-model` SDTM domain and variable: a domain carries its
own `category` and `descriptions`, a variable its own `variable_type`,
`variable_core`, `variable_role`, `variable_controlled` and `descriptions`.
There is no override row and no reference-to-the-standard design.

`project_code` is a **soft reference** to `projects.code` in the `project`
crate. It is validated at the application level through a
`domain::ProjectLookup` port, not by a database foreign key, so the two
crates' schemas stay independent and either can be migrated first.

`key_variables` is an **ordered** list: the order the caller supplies is
preserved through the JSONB column and out to the wire. Duplicates collapse to
their first occurrence; nothing is ever re-sorted.

Every code in `key_variables` must name a variable of the **same** domain. A
code that exists in a sibling domain is still missing here. The usecase
enforces this on every write: a `key_variables` value naming no variable is
rejected with `KeyVariableNotFound` (HTTP 400), and renaming or deleting a
variable a domain names as a key variable is rejected with
`VariableReferencedByKeyVariables` (HTTP 409) rather than silently rewriting
the study's key.

Because a brand-new domain has no variables yet, **a domain is created with
an empty `key_variables`**. The authoring order is *create domain → create
variables → set `key_variables`*.

### Conversion rules

A variable carries `conversion_rules`: where its value comes from and how it
is derived.

```json
{ "source": "CRF", "from": { "form_code": "AE" }, "rule": "AESEV" }
```

`VariableSource` is adjacently tagged (`source` / `from`) and a
`ConversionRule` **flattens** its source, so the source's own fields sit
beside the rule text rather than nesting under a second `source` key. The
variant spellings are pinned per-variant — `Protocol`, `Derived`, `Assigned`,
`CRF`, `eDT` — because no blanket `rename_all` produces both `CRF` and
`eDT`. A unit variant omits `from` entirely; a payload variant carries it
(as `null` when the form is absent).

`rule` is opaque text: the crate neither parses nor evaluates it. A blank
`rule` is rejected. `form_code` names a CRF form but is **not** validated
against the `crf` crate.

## Database setup

The server does not auto-migrate. To create the three `sdtm_project_*` tables
in the server's database:

```bash
export AEGIS_DATABASE_URL=postgres://user:pass@localhost/aegis
sqlx migrate run --source lib/crates/sdtm-project/migrations
```

Adding a migration: let the sqlx CLI create the file, then write the
DDL into the file it names. Never hand-write the file name — the CLI's
timestamp prefix is the migration's version.

```bash
sqlx migrate add --source lib/crates/sdtm-project/migrations --timestamp <description>
```

### Running the live tests

```bash
export AEGIS_TEST_DATABASE_URL=postgres://user:pass@localhost/aegis_test
cargo test -p sdtm-project -- --ignored --test-threads=1
```

> **The URL must point at a DISPOSABLE test database.**
>
> Every crate's live-DB test reads the same `AEGIS_TEST_DATABASE_URL`, so it
> must not be the server's database. The live test drops the
> `_sqlx_migrations` bookkeeping table so each run starts clean. SQLx keeps
> exactly one `_sqlx_migrations` table *per database*, shared by every crate
> that migrates it — so pointing this at the server's database would erase the
> applied-migration record for auth, user, project, terminology, mission and
> crf alike, and the next `sqlx migrate run` would try to re-apply every one
> of them.
>
> The test is destructive against whatever database it is pointed at. That is
> intentional.

The test panics with a clear message when the variable is unset — it never
silently skips.

## Constructing the service

```rust
use std::sync::Arc;

use sdtm_project::{
    ProjectLookupImpl, SdtmProjectDomainRepoPg, SdtmProjectServiceImpl, SdtmProjectUsecase,
    SdtmProjectUsecaseConfig, SdtmProjectVariableRepoPg, SdtmProjectVersionRepoPg,
};

let usecase = SdtmProjectUsecase::new(SdtmProjectUsecaseConfig {
    version_repo: SdtmProjectVersionRepoPg::new(pool.clone()),
    domain_repo: SdtmProjectDomainRepoPg::new(pool.clone()),
    variable_repo: SdtmProjectVariableRepoPg::new(pool),
    projects: Arc::new(ProjectLookupImpl::new(project_service)),
});
let service = SdtmProjectServiceImpl::new(usecase);
```

`project_service` is an `Arc<dyn apis::project::ProjectService>` — the same
service the server already builds — so the project-existence check reuses the
canonical `projects` table.

## Conventions

This crate follows
[`docs/guidelines/lib-crate-development.md`](../../docs/guidelines/lib-crate-development.md):
DDD layering (`domain` → `usecase` → `adapter`), ports-and-adapters with every
I/O behind a `domain` port, one error enum per layer, and a `README.md` per
crate.
