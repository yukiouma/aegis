# User Crate Design

## Goal

Add `lib/crates/user` as a reusable Rust library that provides domain-driven user management backed by PostgreSQL through SQLx. Consumers should be able to construct `UserRepo` from a `sqlx::PgPool`, inject it into `UserUsecase`, and perform asynchronous user operations.

## Architecture

Use a ports-and-adapters DDD structure:

- `domain`: `User`, `Role`, validation, domain errors, and the repository port.
- `usecase`: `UserUsecase`, command/query DTOs, password hashing orchestration, and application errors.
- `infrastructure`: SQLx `UserRepo`, database row conversion, and migrations.

The domain layer must not depend on SQLx. The infrastructure repository implements the domain repository port. The crate root re-exports the public model, usecase, repository, and relevant DTO/error types.

## Public API

Expose constructors matching:

```rust
let user_repo = UserRepo::new(pool);
let user_usecase = UserUsecase::new(user_repo);
```

Usecase methods are asynchronous and return `Result` values. Supported operations are create, fetch by ID, fetch by code, list, update, and deactivate. There is no hard-delete method; deactivation updates `active` to `false` and retains the row.

Create and update inputs contain the user code, name, role, and password as appropriate. Update supports partial changes and hashes a replacement password before persistence. Query outputs do not expose the stored password hash.

## Domain rules

`Role` has `Root`, `Admin`, and `General` variants. PostgreSQL stores roles as lowercase strings `root`, `admin`, and `general`, with conversion validation for unknown values. User identifiers are `i32`; `code` is required to be unique. Basic validation rejects invalid empty values and malformed update inputs through typed errors.

## Persistence

Add a SQLx migration defining a `users` table with:

- `id` integer primary key
- `code` unique non-null text
- `name` non-null text
- `role` non-null text
- `active` non-null boolean
- `password` non-null text containing an Argon2 password hash

The repository uses `sqlx::PgPool` and parameterized queries. Row mapping converts persisted role strings into the domain enum and returns typed errors for invalid data or database failures.

## Password security

The usecase layer hashes passwords with Argon2 and a cryptographically random salt during create and password-changing updates. Plaintext passwords are never written to PostgreSQL or returned from query results. Hashing failures are represented in the crate's error type.

## Testing

Provide unit tests for role serialization/deserialization, validation, password hashing behavior, and usecase orchestration using a mock/in-memory repository. Include compile-level coverage for SQLx row mappings and migration shape where possible without requiring a live PostgreSQL server. Integration tests requiring PostgreSQL should be designed to run when a database URL is supplied.

## Workspace integration

Register the crate in the root Cargo workspace and add the required SQLx, Tokio, Argon2, async-trait, and error-handling dependencies with compatible features. Keep the crate independently consumable by other workspace members.
