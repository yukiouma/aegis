# User-Service Facade — Design

**Date:** 2026-08-05
**Status:** Approved (pending spec review)
**Scope:** `lib/crates/user` — adds a new module `adapter::facade::in_memory` and
re-exports `UserServiceImpl` from the crate root. No changes to other crates.

## Goal

Provide an implementation of the outbound port `apis::user::UserService` that
adapts the `user` crate's `UserUsecase` to the API contract. Today no such
implementation exists: `apis::user::UserService` is declared but unused inside
the workspace, so any server (axum handler, tarpc service, etc.) that wants to
depend on the API contract cannot be wired against a concrete backend.

## Architecture

```
            ┌──────────────────────────┐
            │   apis::user::UserService│  (trait, in apis crate)
            └──────────────┬───────────┘
                           │ impl
            ┌──────────────▼───────────┐
            │   UserServiceImpl<R>     │  (user::adapter::facade::in_memory)
            │   holds UserUsecase<R>   │
            └──────────────┬───────────┘
                           │ uses
            ┌──────────────▼───────────┐
            │   UserUsecase<R>         │  (user::usecase)
            │   + UserRepository port  │
            └──────────────────────────┘
```

The facade sits in the adapter layer because it adapts the *usecase* to an
*outbound port*. Construction follows the same pattern as `UserRepo`:

```rust
let repo = UserRepo::new(pool);
let usecase = UserUsecase::new(repo);
let service: Arc<dyn UserService> = Arc::new(UserServiceImpl::new(usecase));
```

## Module layout & visibility

Files (new):

- `lib/crates/user/src/adapter/facade.rs` — declares `mod in_memory;
  pub use in_memory::UserServiceImpl;`
- `lib/crates/user/src/adapter/facade/in_memory.rs` — hosts
  `UserServiceImpl<R>`, the `UserService` impl, and the `From<UsecaseError>
  for UserApiError` conversion.
- `lib/crates/user/src/adapter/facade/in_memory/tests.rs` — unit tests with
  an in-memory fake `UserRepository`.

Files (edited):

- `lib/crates/user/src/adapter.rs` — add `mod facade; pub use
  facade::UserServiceImpl;`
- `lib/crates/user/src/lib.rs` — add `pub use adapter::UserServiceImpl;`
  alongside the existing `UserRepo` / `UserUsecase` re-exports.
- `lib/crates/user/Cargo.toml` — add `apis = { workspace = true }` to
  `[dependencies]`.

Visibility:

- `in_memory` is `pub` inside `facade` (so the parent's `pub use` is
  well-formed) but never crosses the `facade` module boundary directly.
- `UserServiceImpl` is reachable only via the crate root or
  `user::adapter::UserServiceImpl`. This mirrors how `postgres` is structured
  today: the child module is `pub`, but only `UserRepo` reaches the outside
  world.

## Type translation

`UserServiceImpl<R>` holds a `UserUsecase<R>` and translates per-call:

| `apis::user::*`            | direction          | `user::usecase::*`         |
|----------------------------|--------------------|----------------------------|
| `CreateUserRequest`        | input →            | `CreateUser`               |
| `UpdateUserRequest`        | input →            | `UpdateUser`               |
| `Role`                     | ↔ (manual match)   | `user::domain::Role`       |
| `UserView`                 | ← (struct literal) | `user::usecase::UserView`  |
| `UserApiError`             | ← (`From` impl)    | `UsecaseError`             |

Conversion details:

- Request DTOs (`CreateUserRequest`, `UpdateUserRequest`) have the same
  field shapes as their usecase counterparts, so they map field-for-field
  inside the method body.
- `apis::user::Role` and `user::domain::Role` are distinct types with the
  same three variants; conversion is a single `match`.
- `user::usecase::UserView` and `apis::user::UserView` have identical field
  shapes, so the conversion is a one-line struct literal at the call site.
  No cross-crate `From` impl (would hit the orphan rule anyway).
- Error translation lives in a single `impl From<UsecaseError> for
  UserApiError`. The `?` operator then handles every call site:

  | `UsecaseError`                                         | `UserApiError`                       |
  |--------------------------------------------------------|--------------------------------------|
  | `Validation(DomainError::EmptyCode)`                   | `Validation("user code must not be empty".into())` |
  | `Validation(DomainError::EmptyName)`                   | `Validation("user name must not be empty".into())` |
  | `Validation(DomainError::InvalidRole(s))`              | `Validation(format!("invalid role: {s}"))` |
  | `Repository(DomainError::NotFound)`                    | `NotFound`                           |
  | `Repository(DomainError::DuplicateCode(c))`            | `DuplicateCode(c)`                   |
  | `Repository(DomainError::Repository(s))`               | `Repository(s)`                      |

- `UserApiError::Hashing(_)` is unreachable from this implementation: the
  `apis::user::CreateUserRequest` DTO has no `password` field, so no
  password hashing occurs at this layer. The variant is part of the trait's
  contract for future adapters that do handle passwords; here it is never
  produced.

## Object safety and concurrency

`UserServiceImpl<R>` is generic over `R: UserRepository`. Because the trait
implementation does not mention `R` in any return position (it returns
`apis::user::UserView` and `apis::user::UserApiError`, both concrete), the
generic parameter is on the *implementor*, not the *trait method*, and the
result is still object-safe. A `Box<dyn UserService>` continues to be
valid; only the implementor carries the generic.

`UserServiceImpl<R>` itself is `Send + Sync` whenever `R` and
`UserUsecase<R>` are. Both are under our control, and both are `Send + Sync`
for the production `UserRepo`. Tests will assert the bound.

## Tests

`tests.rs` defines a minimal in-memory `UserRepository` (a
`Mutex<Vec<User>>` plus a monotonically-increasing `next_id`) and covers:

- `create` happy path → returns `UserView` with the assigned id.
- `create` rejects empty `code` / `name` with `Validation`.
- `create` rejects duplicate `code` with `DuplicateCode`.
- `get_by_id` happy path / `NotFound`.
- `get_by_code` happy path / `NotFound` / `Validation` (empty code).
- `list` returns the seeded users in insertion order.
- `update` happy path / `NotFound` / `DuplicateCode`.
- `Box<dyn UserService>` is constructible (object-safety) and
  `Send + Sync`.

No live PostgreSQL connection is required for any of these tests. The
existing `lib/crates/user/tests/integration_persistence.rs` continues to
own live-DB coverage for `UserRepo` and `UserUsecase`; the facade is
covered entirely by in-memory unit tests.

## What this design deliberately does *not* do

- Does not add a password / hashing path. The `apis::user` trait surface
  intentionally omits `password`, and the `Hashing` variant of
  `UserApiError` is preserved for future adapters without being exercised
  here.
- Does not add a second adapter layer entry point (e.g. a `facade::grpc`).
  If a second outbound port is added later, it gets its own module under
  `facade/`.
- Does not change the existing `UserUsecase` or `UserRepo` surface.
- Does not change the `apis` crate.

## Acceptance criteria

- `cargo build -p user` succeeds with the new module wired in.
- `cargo test -p user` passes, including the new in-memory tests.
- `cargo test -p apis` still passes (no change to the trait contract).
- `UserServiceImpl` is reachable via `user::UserServiceImpl` and
  `user::adapter::UserServiceImpl`.