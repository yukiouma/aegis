# Auth `UserService` Domain Port Design

## Goal

Decouple the `domain` and `usecase` layers of the `auth` crate from
`apis::user::UserService`. Today both layers import the apis trait,
its `UserView`, `Role`, and `UserApiError` — so an unrelated change
to the apis surface ripples into auth internals. The auth usecase
only consumes `user.active` and `user.role` (for `get_by_code`),
so a minimal domain port is enough.

After this refactor:

- `domain` defines a new `UserService` trait + `UserSummary` struct
  using only `crate::domain::{Role, DomainError}`.
- `usecase::auth_usecase` depends on `Arc<dyn crate::domain::UserService>`.
  All `apis::user::*` imports are gone from `domain` and `usecase`.
- `adapter/service/user::UserServiceImpl` adapts the new domain trait
  onto an inner `Arc<dyn apis::user::UserService>` (typically the
  `user` crate's `UserServiceImpl`), translating types at the
  boundary.
- `apis::user::UserService` is untouched. It remains the canonical
  outbound port for HTTP handlers and the `user` crate's facade.

## Architecture

Add one new port to `auth::domain` and one new adapter under
`auth::adapter::service`. No other layers change.

```
auth/src/
├── domain/
│   ├── credentials.rs          (unchanged)
│   ├── domain_identity.rs      (unchanged)
│   ├── error.rs                (unchanged)
│   ├── repository.rs           (unchanged)
│   ├── role.rs                 (unchanged)
│   ├── service.rs              (NEW — UserService trait + UserSummary)
│   ├── tests.rs                (unchanged)
│   └── token_version_cache.rs  (unchanged)
├── usecase/
│   ├── auth_usecase.rs         (uses crate::domain::UserService; drops apis::user imports)
│   ├── commands.rs             (unchanged)
│   ├── error.rs                (unchanged)
│   └── tests.rs                (FakeUserService implements the domain trait)
└── adapter/
    ├── cache.rs                (unchanged)
    ├── facade.rs               (unchanged)
    ├── persistence.rs          (unchanged)
    └── service.rs              (NEW — pub(crate) mod user;)
    └── service/
        └── user.rs             (NEW — UserServiceImpl adapting domain::UserService over apis::user::UserService)
```

## New Domain Port — `domain/service.rs`

Defines the minimal projection the auth usecase needs and the trait
that exposes it.

```rust
//! Outbound port: look up a user's `active` state and `role` by code.
//!
//! `domain` defines this trait so the `usecase` layer never has to
//! reach into `apis::user::UserService` for these two facts. The
//! concrete adapter lives in
//! `crate::adapter::service::user::UserServiceImpl` and delegates
//! to `apis::user::UserService`.

use async_trait::async_trait;

use super::{DomainError, Role};

/// Minimal projection of a user — just the fields the auth usecase
/// needs to decide whether to mint tokens for `code`. The full user
/// record (id, name, timestamps, etc.) stays on the apis side.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserSummary {
    pub code: String,
    pub active: bool,
    pub role: Role,
}

#[async_trait]
pub trait UserService: Send + Sync {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError>;
}
```

`domain.rs` adds `mod service;` and `pub use service::{UserService, UserSummary};`.
The trait and struct become reachable as `auth::domain::UserService`
and `auth::domain::UserSummary`.

## New Adapter — `adapter/service/user.rs`

Implements `domain::UserService` on top of the apis trait. Holds the
apis implementation behind an `Arc<dyn apis::user::UserService>` and
translates apis types into domain types on each call.

```rust
use std::sync::Arc;

use async_trait::async_trait;

use apis::user::{Role as ApiRole, UserApiError, UserService as ApiUserService};
use crate::domain::{DomainError, Role, UserService, UserSummary};

/// Adapter that implements the domain `UserService` port on top of
/// the apis `UserService`. Delegates `get_by_code` to the inner apis
/// implementation and translates the apis types into the domain
/// equivalents.
pub struct UserServiceImpl {
    inner: Arc<dyn ApiUserService>,
}

impl UserServiceImpl {
    pub fn new(inner: Arc<dyn ApiUserService>) -> Self {
        Self { inner }
    }
}

fn map_role(r: ApiRole) -> Role {
    match r {
        ApiRole::Root => Role::Root,
        ApiRole::Admin => Role::Admin,
        ApiRole::General => Role::General,
    }
}

fn map_error(err: UserApiError) -> DomainError {
    match err {
        UserApiError::NotFound => DomainError::NotFound,
        other => DomainError::Repository(other.to_string()),
    }
}

#[async_trait]
impl UserService for UserServiceImpl {
    async fn get_by_code(&self, code: &str) -> Result<UserSummary, DomainError> {
        let view = self.inner.get_by_code(code).await.map_err(map_error)?;
        Ok(UserSummary {
            code: view.code,
            active: view.active,
            role: map_role(view.role),
        })
    }
}
```

`adapter.rs` adds `pub(crate) mod service;`. `UserServiceImpl` is
**not** re-exported at the auth crate root — it is a wiring detail;
end-users hold the trait object.

## Usecase Changes — `usecase/auth_usecase.rs`

- Drop `use apis::user::{UserApiError, UserService};`.
- Extend the existing `use crate::domain::{…}` line to also
  include `UserService`. (Do not add a new `use` line — the
  existing block already pulls the rest of the domain names.)
- `AuthUsecaseConfig.user_service` and `AuthUsecase.user_service`
  field types change from `Arc<dyn apis::user::UserService>` to
  `Arc<dyn crate::domain::UserService>`.
- Delete the file-local helpers `map_user_service_error` and
  `role_from_api` (no longer needed — the domain trait returns
  `DomainError` directly and `summary.role` is already
  `crate::domain::Role`).
- At each of the four call sites (`login_with_password`,
  `login_with_domain_user_info`, `verify`, `refresh`) replace the
  binding `let user = self.user_service.get_by_code(...)?;` with
  `let summary = self.user_service.get_by_code(...)?;`. Use
  `summary.active` and `summary.role` directly — no more
  `role_from_api` call.

The error mapping for `current_token_version` and the JWT paths is
unchanged.

## Test Wiring Updates

### `src/usecase/tests.rs` — `FakeUserService`

Currently implements `apis::user::UserService` and seeds apis
`UserView`s. Rewrite it to implement `crate::domain::UserService`
and seed `UserSummary`s.

- Drop the imports of `CreateUserRequest`, `UpdateUserRequest`,
  `UserApiError`, `UserView`.
- Drop the alias `Role as ApiRole`; use `crate::domain::Role`
  directly in the `seed` helper signature.
- `FakeUserService::seed` now stores `UserSummary` instead of
  `UserView`; remove the unused fields `id`, `name`, `created_at`,
  `updated_at` from the seeded record.
- The trait impl collapses to a single method (`get_by_code`); the
  `create` / `get_by_id` / `list` / `update` stubs go away.
- The integration persistence test that also defined a
  `FakeUserService` (see next item) imports this one if visible;
  otherwise it gets its own local copy. Visibility: the type is
  declared `pub struct FakeUserService` already, and the module is
  `pub(crate) mod tests;` — so `auth::usecase::tests::FakeUserService`
  is reachable only inside the crate. The integration test lives in
  `tests/` (an external crate), so it cannot import the in-crate
  fake; keep a local copy in that file (see below).

### `tests/integration_persistence.rs` — local `FakeUserService`

Rewrite the local `FakeUserService` to implement
`crate::domain::UserService` instead of `apis::user::UserService`.
Drop the apis types from imports. The smoke test wiring stays the
same — `Arc::new(FakeUserService::new())` still slots into
`AuthUsecaseConfig.user_service`.

### `tests/public_api.rs`

Two references to `apis::user::UserService` change to
`auth::domain::UserService`:

- `use apis::user::UserService;` → `use auth::domain::UserService;`
- `Arc<dyn UserService>` → `Arc<dyn auth::domain::UserService>`
  (or just `Arc<dyn UserService>` after the `use`).
- `Box<dyn UserService>` → `Box<dyn auth::domain::UserService>`.

### `src/adapter/facade/in_memory/tests.rs`

No structural changes. It consumes `FakeUserService` from
`crate::usecase::tests`; that type now implements the domain
trait, which is what `AuthUsecaseConfig` expects, so construction
keeps compiling.

## Public API

After this refactor, `auth::*` adds two names:

- `auth::domain::UserService` (trait).
- `auth::domain::UserSummary` (struct).

No other public surface changes. `UserServiceImpl` is reachable as
`auth::adapter::service::user::UserServiceImpl` for wiring code but
is not re-exported at the crate root.

## Testing

- `cargo test -p auth` — unit tests in `src/usecase/tests.rs` and
  `src/adapter/facade/in_memory/tests.rs` continue to pass with
  the rewired `FakeUserService`.
- `cargo test -p auth --test public_api` — the public API compile
  test continues to pass with the new trait import.
- `cargo test -p auth -- --ignored` (requires `AEGIS_DATABASE_URL`)
  — the integration smoke test continues to pass.
- `cargo test -p apis` — the apis crate is not touched by this
  refactor; its tests stay green unchanged.
- `cargo test -p user` — the user crate is not touched; its tests
  stay green unchanged.

## What Stays Untouched

- `apis::user::UserService`, `UserView`, `Role`, `UserApiError`.
- `adapter/facade/in_memory/service.rs` (the `AuthServiceImpl`
  body is identical — only the field type of the usecase's
  `user_service` changes, and that change is transparent at this
  layer).
- `domain::role.rs`, `domain::error.rs`, `domain::repository.rs`,
  `domain::credentials.rs`, `domain::domain_identity.rs`,
  `domain::token_version_cache.rs`.
- `usecase::commands.rs`, `usecase::error.rs`.
- The `user` crate entirely.

## Out of Scope

- Adding `create`, `update`, `list`, `get_by_id` to the new
  domain trait — they are not used by the auth usecase today
  (YAGNI).
- A new dedicated `UserServiceError` enum — `DomainError`
  already has `NotFound` and `Repository(String)` which cover
  every variant of `UserApiError`.
- Re-exporting `UserServiceImpl` at the auth crate root.