# Library Crate Development Guideline

Opinionated, distilled from the `lib/crates/user` crate. Apply to every
new library crate that lands under `lib/crates/`.

## 1. Workspace membership and edition

- Register the crate in the root [`Cargo.toml`](../../Cargo.toml) `[workspace].members` array and inherit all dependencies from `[workspace.dependencies]` via `{ workspace = true }`. Centralises version selection and prevents drift between crates.
- Set `edition = "2024"` (and `resolver = "3"` at the workspace root). The 2024 edition requires `rust-version >= 1.85`; verify the toolchain pin in `rust-toolchain.toml` matches before bumping.
- Keep the crate's `Cargo.toml` minimal. If a dependency looks unused after `cargo build -p <crate>`, drop it; transient feature-flag requirements belong in a one-line comment so the next reviewer can see why it's still there (see the `rand_core` justification in `lib/crates/user/Cargo.toml`).

## 2. Module layout — no `mod.rs`

- Use `src/<module>.rs` plus a `src/<module>/` directory of child files. `mod.rs` is the deprecated style on the 2024 edition.
- Each top-level module exposes its public surface via `pub use …;` at the bottom of `<module>.rs`. Consumers write `use crate::Type` rather than reaching into nested module paths.
- Private implementation details (`row`, internal helpers) are `pub(crate)` or private. The crate root only re-exports what consumers are allowed to name.

## 3. Layered design

- Default to a ports-and-adapters split: `domain` (pure types, ports, errors), `usecase` / `application` (orchestration, command DTOs, side-effect-free business rules), `infrastructure` (adapters that implement the domain's ports).
- The domain layer must not depend on any external service crate (no `sqlx`, no `tokio` in pure types). Only the usecase/infrastructure layers do I/O.
- Validate inputs and enforce invariants in the domain layer; the usecase layer projects domain types into safe view types (e.g. `User` → `UserView` without the password hash) and the infrastructure layer translates driver errors into domain errors.

## 4. Public API surface

- Re-export everything consumers need from the crate root: the data types, the request/response DTOs, the error variants, and the constructors (`new`).
- Hide fields that carry sensitive data behind `pub(crate)` and redact them in `Debug` implementations. Manual `Debug` impls are required whenever a `derive(Hash)` / `Debug` would leak a secret.
- Keep the constructor signature obvious: `UserRepo::new(pool)`, `UserUsecase::new(repo)`. No builder ceremony, no async constructors.

## 5. Async and error handling

- Async trait methods get `#[async_trait]` from a workspace dep when the trait must be object-safe (e.g. for mock injection in tests). Plain `pub async fn` on concrete types is fine.
- Define a single `*Error` enum per layer with `#[derive(thiserror::Error)]` and `#[source]` on every inner-error variant so `Error::source()` keeps the chain. Map driver errors into the layer's enum at the boundary.
- Never panic on I/O failure. The repository's `map_db_error` is the canonical pattern: special-case known SQLSTATEs (e.g. `23505` → `DuplicateCode`), then wrap the rest as `Repository(message)`.

## 6. Database schema

- One migration file per schema change. Place migrations under `<crate>/migrations/` and consume them via `sqlx::migrate!` so the file inventory is the source of truth.
- Use SQLx runtime API (`sqlx::query_as`, `QueryBuilder`) when the workspace lacks a `sqlx-data.json` cache or a live `DATABASE_URL` at build time. Document the choice in a module-level comment so the next reviewer can switch to compile-time macros when the cache is wired in.
- For tables with auto-managed timestamps, prefer `DEFAULT NOW()` + a `BEFORE UPDATE` trigger over a per-caller `NOW()` write — the trigger covers every code path including direct SQL.

## 7. Tests

- Three layers of tests, in this order:
  1. Unit tests inside each module (`#[cfg(test)] mod tests;`) for type conversions, validation, and pure logic.
  2. Schema content tests that read migration files as strings and assert the column/constraint set, so the schema cannot regress silently.
  3. `tests/` directory integration tests for the public API and for ignored live-database round-trips.
- Live-database tests must be `#[ignore]`-gated so `cargo test -p <crate>` stays green without infrastructure. Load `.env` via `dotenvy` at test startup. Apply the migration via `sqlx::migrate!("./migrations").run(&pool).await` and drop any prior state in a fixture-reset step so each run starts clean.
- Public-API compile tests at `tests/public_api.rs` assert that the documented consumer imports type-check without performing I/O.

## 8. Verification gate

Before any PR:

```bash
cargo fmt --all -- --check
cargo clippy -p <crate> --all-targets --all-features -- -D warnings
cargo test -p <crate>
cargo doc -p <crate> --no-deps
cargo test -p <crate> -- --ignored --test-threads=1   # when AEGIS_*_DATABASE_URL is set
```

If the crate is the only workspace member, also run `cargo check --workspace` / `cargo clippy --workspace` and `cargo test --workspace`. If other workspace members have unrelated build issues (e.g. system-library failures), document them rather than working around them.

## 9. README and discoverability

Every library crate gets a `README.md` at its root covering:

- One-sentence purpose.
- `src/` layout tree.
- Database setup if the crate owns a schema (the migration command and the env var).
- How to run the ignored tests.

Link the crate's README from any workspace-level documentation so newcomers can find it.

## 10. Commits and review

- One commit per logical change (scaffolding, domain, usecase, infrastructure, public-API integration, follow-up fix). Lockfile drift gets its own `chore:` commit.
- Each commit message lists the spec coverage and the verification commands at the bottom so reviewers can run the same gate locally.