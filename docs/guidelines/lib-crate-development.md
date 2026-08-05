# Library Crate Development Guideline

Opinionated, distilled from the `lib/crates/user` crate. Apply to every
new library crate that lands under `lib/crates/`.

## 1. Workspace membership and edition

- Register the crate in the root [`Cargo.toml`](../../Cargo.toml) `[workspace].members` array and inherit all dependencies from `[workspace.dependencies]` via `{ workspace = true }`. Centralises version selection and prevents drift between crates.
- Set `edition = "2024"` and `resolver = "3"` at the workspace root. The 2024 edition requires `rust-version >= 1.85`; pin the toolchain via `rust-toolchain.toml` if a specific version is needed by the build, otherwise rely on whatever the developer / CI image ships.
- Keep the crate's `Cargo.toml` minimal. If a dependency looks unused after `cargo build -p <crate>`, drop it; non-obvious dependencies get a one-line comment explaining why they are still there (see the `chrono` and `apis` justifications in `lib/crates/user/Cargo.toml`).
- Path-dependencies between sibling workspace crates (`apis = { path = "../apis" }`) are allowed when the two crates share the workspace, and the comment should say so explicitly so reviewers do not try to "promote" the dep to a crates.io version.

## 2. Module layout — no `mod.rs`

- Use `src/<module>.rs` plus a `src/<module>/` directory of child files. `mod.rs` is the deprecated style on the 2024 edition.
- Each top-level module (`domain`, `usecase`, `adapter`) exposes its public surface via `pub use …;` at the bottom of `<module>.rs`. Consumers write `use crate::Type` rather than reaching into nested module paths.
- Private implementation details (`row`, internal helpers) are `pub(crate)` or private. The crate root only re-exports what consumers are allowed to name. A layer boundary like `adapter::persistence` may be `pub(crate)` if callers reach concrete implementations via a re-export at the layer above (e.g. `adapter::UserRepo`), and `pub` on the `postgres` child so the re-export is well-formed.

## 3. Layered design

- Default to a ports-and-adapters split with three top-level modules:
  - `domain` — pure types, value objects, ports (traits), and domain errors. No I/O, no `sqlx`, no `tokio`.
  - `usecase` — orchestration, command DTOs (`CreateUser`, `UpdateUser`), the `UserView` projection, and a `UsecaseError` enum that wraps `DomainError`. Generic over the repository port so tests can inject in-memory fakes.
  - `adapter` — concrete implementations of the domain ports. Sub-organised by direction:
    - `adapter::persistence::<backend>` for storage adapters (`UserRepo`).
    - `adapter::facade::<backend>` for outbound-port adapters that adapt the usecase to API-facing traits defined in *other* workspace crates (e.g. `apis::user::UserService`).
- The domain layer must not depend on any external service crate. Only the usecase and adapter layers touch `sqlx`, `tokio`, `chrono` (for the `DateTime<Utc>` column type), etc.
- Validate inputs and enforce invariants in the domain layer (e.g. `User::new` rejects empty `code` / `name`). The usecase layer projects domain types into safe view types (`User` → `UserView` via `From`) and re-validates command inputs before reaching the repository. The adapter layer translates driver errors into domain errors.
- A second storage backend (e.g. `sqlite`) or a second facade backend (e.g. a future gRPC adapter) must be additive: a new `adapter/persistence/<backend>/` or `adapter/facade/<backend>/` directory that implements the port, plus a re-export at the `adapter` boundary. No edits to the existing backends.

## 4. Public API surface

- Re-export everything consumers need from the crate root: the data types (`User`, `Role`), the request/response DTOs (`UserNew`, `UserUpdate`, `CreateUser`, `UpdateUser`, `UserView`), the error variants (`DomainError`, `UsecaseError`), the trait (`UserRepository`), and the constructors (`UserRepo`, `UserUsecase`, `UserServiceImpl`).
- Hide fields that carry sensitive data behind `pub(crate)` and redact them in `Debug` implementations. Hand-roll `Debug` whenever a `derive(Debug)` would leak a secret; today every field on `User` is safe to log, so the manual impl is structural rather than redact-and-reveal.
- Keep two domain constructors for every aggregate:
  - `User::new(...)` — the public-facing validating constructor that the rest of the crate uses. Returns `Result<Self, DomainError>` and rejects empty / whitespace-only fields.
  - `User::for_repository(...)` — a `pub(crate)` constructor reserved for the adapter layer. Skips domain validation because the data is assumed to have been validated on the way in. The `FromRow` row bridge calls this and the doc-comment names the rule so future code does not call it from outside `adapter::persistence`.
- Keep the constructor signature obvious: `UserRepo::new(pool)`, `UserUsecase::new(repo)`, `UserServiceImpl::new(usecase)`. No builder ceremony, no async constructors.

## 5. Async and error handling

- Async trait methods get `#[async_trait]` from a workspace dep when the trait must be object-safe (e.g. for `Box<dyn UserRepository>` injection in tests or `Box<dyn UserService>` for the facade). Plain `pub async fn` on concrete types is fine.
- Define a single `*Error` enum per layer with `#[derive(thiserror::Error)]` and `#[source]` on every inner-error variant so `Error::source()` keeps the chain (`UsecaseError::Validation(#[source] DomainError)`, `UsecaseError::Repository(#[source] DomainError)`). Map driver errors into the layer's enum at the boundary.
- Implement `From<DomainError> for UsecaseError` so the usecase can `?` straight through repository calls. The `From` impl must choose the right variant (`Repository`, not `Validation`); a domain validation error that surfaces from the repository is treated as a `Repository` error.
- Never panic on I/O failure. The repository's `map_db_error` is the canonical pattern:
  - `sqlx::Error::RowNotFound` → `DomainError::NotFound`.
  - `sqlx::Error::Database` whose SQLSTATE is `23505` (unique violation) → `DomainError::DuplicateCode(constraint_name)`. SQLx does not surface the offending bound value, so the payload is the constraint name (e.g. `(constraint users_code_unique)`); the usecase surfaces the original `code` alongside the error if the caller needs it.
  - Every other error → `DomainError::Repository(driver_message)`.

## 6. Database schema

- One migration file per schema change. Place migrations under `<crate>/migrations/` and consume them via `sqlx::migrate!("./migrations")` so the file inventory is the source of truth.
- Use SQLx runtime API (`sqlx::query_as`, `QueryBuilder`) when the workspace lacks a `sqlx-data.json` cache or a live `DATABASE_URL` at build time. Document the choice in a module-level comment (see the header on `adapter::persistence::postgres`) so the next reviewer can switch to compile-time macros when the cache is wired in.
- For tables with auto-managed timestamps, prefer `DEFAULT NOW()` on `created_at` / `updated_at` plus a `BEFORE UPDATE` trigger that refreshes `updated_at`. The trigger covers every code path including direct SQL and removes the obligation to remember to bind `NOW()` from each caller.
- Add CHECK constraints for every enum-shaped column. The application-level `Role::try_from` is the single source of truth for the allowed values; the CHECK constraint is belt-and-braces so an out-of-band insert cannot smuggle an unknown value past the type system.

## 7. Tests

- Four kinds of tests, in this order:
  1. **Domain unit tests** inside `src/domain/tests.rs` (and one per layer that has its own logic). Cover value-object conversions (`Role::try_from`), invariant enforcement (`User::new` rejects empty inputs), and any pure logic.
  2. **Adapter unit tests** inside `src/adapter/<backend>/tests.rs`. Cover the row-to-domain `TryFrom` impl, and — when the adapter owns a schema — **schema content tests** that read the migration file as a string (via `std::fs` + `env!("CARGO_MANIFEST_DIR")`) and assert the column / constraint / trigger set so the schema cannot regress silently.
  3. **Facade unit tests** inside `src/adapter/facade/<backend>/tests.rs` that wire the adapter on top of an in-memory `UserRepository` (`Mutex<Vec<User>>` + an `AtomicI32` for ids). They exercise the public-facing behaviour of the API port without touching PostgreSQL and lock in object-safety / `Send + Sync` bounds.
  4. **`tests/` directory tests**:
     - `tests/public_api.rs` — a compile-only test that names every documented consumer import (`use user::Foo`), pins the constructor dependency chain (`fn(PgPool) -> _` / `fn(R) -> _` as function pointers), and asserts the trait bounds the usecase relies on (`UserRepo: UserRepository`).
     - `tests/integration_persistence.rs` — live-database round-trips, `#[ignore]`-gated.
- Live-database tests must be `#[ignore]`-gated so `cargo test -p <crate>` stays green without infrastructure. Load `.env` via `dotenvy::dotenv()` at test startup; read the connection URL from `AEGIS_<CRATE>_DATABASE_URL` (panic with a clear message if it is missing). Apply the migration via `sqlx::migrate!("./migrations").run(&pool).await`. Before applying, **drop the live table and the `_sqlx_migrations` bookkeeping table** so each run starts clean — this is destructive against a real production database, which is intentional so the failure is loud rather than silent.
- Generate a per-run unique value (atomic counter + wall-clock nanoseconds) for any column that has a UNIQUE constraint, so concurrent runs do not collide on the unique index.
- `tests/public_api.rs` is the safety net for refactors that touch the documented public surface: a removed re-export or a tightened trait bound shows up there before reviewers see it.

## 8. Verification gate

Before any PR:

```bash
cargo fmt --all -- --check
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo test -p <crate>
cargo doc -p <crate> --no-deps
cargo test -p <crate> -- --ignored --test-threads=1   # when AEGIS_<CRATE>_DATABASE_URL is set
```

If the crate is the only workspace member, also run `cargo check --workspace` / `cargo clippy --workspace` and `cargo test --workspace`. If other workspace members have unrelated build issues (e.g. system-library failures), document them rather than working around them.

## 9. README and discoverability

Every library crate gets a `README.md` at its root covering:

- One-sentence purpose.
- `src/` layout tree (matching the actual module shape — e.g. `domain/`, `usecase/`, `adapter/persistence/postgres/`, `adapter/facade/in_memory/`).
- Database setup if the crate owns a schema (the migration command and the env var, e.g. `sqlx migrate run --source lib/crates/<crate>/migrations` and `AEGIS_<CRATE>_DATABASE_URL`).
- How to run the ignored tests (`cargo test -p <crate> -- --ignored`) and what env var they require.
- A back-link to this guideline so newcomers find the cross-cutting conventions.

Link the crate's README from any workspace-level documentation so newcomers can find it.

## 10. Commits and review

- One commit per logical change (scaffolding, domain, usecase, infrastructure, public-API integration, follow-up fix). Lockfile drift gets its own `chore:` commit.
- Each commit message lists the spec coverage and the verification commands at the bottom so reviewers can run the same gate locally.