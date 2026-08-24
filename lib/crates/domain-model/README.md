# domain-model

CRUD over the CDISC SDTM domain model aggregates
(`SdtmVersion`, `SdtmDomain`, `SdtmVariable`), backed by
PostgreSQL.

## Layered architecture

```
domain-model crate
└── adapter
    ├── facade             (in-memory, generic over V/D/Va)
    └── persistence        (postgres, sqlx runtime API)
usecase
└── DomainModelUsecase<V, D, Va>
    └── commands / views / UsecaseError
domain
└── DomainCategory, SdtmVariableType, SdtmVariableCore, SdtmRole
    └── SdtmVersion, SdtmDomain, SdtmVariable
    └── SdtmVersionRepository, SdtmDomainRepository,
        SdtmVariableRepository
    └── DomainError
```

`adapter::persistence::postgres::*RepoPg` implements the three
ports. `adapter::facade::in_memory::service::DomainModelServiceImpl`
adapts `DomainModelUsecase` to the
`apis::domain_model::DomainModelService` outbound port.

## Data model

| Aggregate      | Fields                                                                |
| -------------- | --------------------------------------------------------------------- |
| `SdtmVersion`  | `id`, `name` (unique), `created_at`, `updated_at`                     |
| `SdtmDomain`   | `id`, `version_id` (FK CASCADE), `name`, `category`, `descriptions` (JSONB), `created_at`, `updated_at` |
| `SdtmVariable` | `id`, `domain_id` (FK CASCADE), `name`, `variable_controlled`, `variable_type`, `variable_core`, `variable_role`, `variable_sequence`, `descriptions` (JSONB), `created_at`, `updated_at` |

`descriptions` carries `Vec<SdtmDomainDescription>` /
`Vec<SdtmVariableDescription>` as a single JSONB column
(`NOT NULL DEFAULT '[]'::jsonb`).

## HTTP surface

Mounted under `/api/domain-model/*` in `aegis-server`. Every
write route (`POST`, `PUT`, `DELETE`) calls
`require_admin_or_root(&claims)?;` first. Reads require only
authenticated claims.

```
GET    /api/domain-model/versions
POST   /api/domain-model/versions           (admin/root)
PUT    /api/domain-model/versions/:id       (admin/root)
DELETE /api/domain-model/versions/:id       (admin/root)

POST   /api/domain-model/domains            (admin/root)
GET    /api/domain-model/domains/:id
GET    /api/domain-model/versions/:version_id/domains
PUT    /api/domain-model/domains/:id        (admin/root)
DELETE /api/domain-model/domains/:id        (admin/root)

POST   /api/domain-model/variables          (admin/root)
GET    /api/domain-model/variables/:id
GET    /api/domain-model/domains/:domain_id/variables
PUT    /api/domain-model/variables/:id      (admin/root)
DELETE /api/domain-model/variables/:id      (admin/root)
```

## Verification

```bash
cargo fmt --all -- --check
cargo clippy -p domain-model --all-targets --all-features -- -D warnings
cargo test -p domain-model
cargo doc -p domain-model --no-deps
```

Live-DB integration tests (gated with `#[ignore]`) require
`AEGIS_DOMAIN_MODEL_DATABASE_URL`:

```bash
cargo test -p domain-model -- --ignored --test-threads=1
```

Spec: `docs/superpowers/specs/2026-08-24-domain-model-crate-design.md`.
Conventions: `docs/guidelines/lib-crate-development.md`.