# crf

CRUD over the Case Report Form aggregates (`CrfVersion`,
`CrfForm`, `CrfItem`, `CrfOption`, `CrfUnit`, `DomainAnnotation`,
`Annotation`) and version-scoped ILIKE search, backed by
PostgreSQL.

## Layered architecture

```
crf crate
└── adapter
    ├── facade                  (in-memory, generic over V/F/I/O/U/Da/A/P)
    ├── persistence             (postgres, sqlx runtime API)
    └── service                 (project::ProjectLookupImpl bridge)
usecase
└── CrfUsecase<V, F, I, O, U, Da, A, P>
    └── commands / views / UsecaseError
domain
└── CrfItemKind, AnnotationOwner
    └── CrfVersion, CrfForm, CrfItem, CrfOption, CrfUnit,
        DomainAnnotation, Annotation
    └── CrfVersionRepository, CrfFormRepository, CrfItemRepository,
        CrfOptionRepository, CrfUnitRepository,
        DomainAnnotationRepository, AnnotationRepository
    └── ProjectLookup
    └── DomainError
```

`adapter::persistence::postgres::*RepoPg` implements the seven
ports. `adapter::service::project::ProjectLookupImpl` adapts
`apis::project::ProjectService` to the domain `ProjectLookup`.
`adapter::facade::in_memory::CrfServiceImpl` adapts
`CrfUsecase` to `apis::crf::CrfService`.

## Data model

| Aggregate           | Fields                                                                                          |
| ------------------- | ----------------------------------------------------------------------------------------------- |
| `CrfVersion`        | `id`, `project_code`, `name`, `created_at`, `updated_at`                                        |
| `CrfForm`           | `id`, `version_id` (FK CASCADE), `code`, `name`, `order`, `not_submitted`, `created_at`, `updated_at` |
| `CrfItem`           | `id`, `form_id` (FK CASCADE), `code`, `name`, `kind`, `order`, `not_submitted`, `created_at`, `updated_at` |
| `CrfOption`         | `id`, `item_id` (FK CASCADE), `value`, `not_submitted`, `created_at`, `updated_at`               |
| `CrfUnit`           | `id`, `item_id` (FK CASCADE), `value`, `not_submitted`, `created_at`, `updated_at`               |
| `DomainAnnotation`  | `id`, `form_id` (FK CASCADE), `name`, `description`, `created_at`, `updated_at`                  |
| `Annotation`        | `id`, `domain_annotation_id` (FK RESTRICT), `content`, `assign`, polymorphic `owner`, `created_at`, `updated_at` |

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p crf --all-targets --all-features -- -D warnings
cargo test -p crf
cargo doc -p crf --no-deps
```

Live-DB integration tests (gated with `#[ignore]`) require
`AEGIS_CRF_DATABASE_URL`:

```bash
cargo test -p crf -- --ignored --test-threads=1
```

Spec: `docs/superpowers/specs/2026-08-27-crf-crate-design.md`.
Conventions: `docs/guidelines/lib-crate-development.md`.
