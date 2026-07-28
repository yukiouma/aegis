# user crate

Workspace library providing a SQLx/PostgreSQL-backed DDD user
repository and an async `UserUsecase`.

## Layout

```text
src/
  domain/        # User, Role, validation, errors, repository port
  usecase/       # UserUsecase, command DTOs, password hashing, errors
  infrastructure/# SQLx-backed UserRepo, UserRow, migrations
migrations/      # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`User`, `Role`,
`UserRepo`, `UserUsecase`, `CreateUser`, `UpdateUser`, `UserView`,
`UserNew`, `UserUpdate`, `DomainError`, `UsecaseError`) so consumers
can `use user::...` without reaching into the sub-modules.

## Database setup

The crate ships a single SQLx migration that defines the `users`
table. Apply it before pointing `UserRepo` (or any other consumer) at
the database:

```bash
sqlx migrate run --source lib/crates/user/migrations
```

Once the migration is applied, construct the repository and usecase
from a `sqlx::PgPool`:

```rust
use user::{UserRepo, UserUsecase};

let user_repo = UserRepo::new(pool);
let user_usecase = UserUsecase::new(user_repo);
```

The `user` crate does not run migrations at runtime; a deployment
step is required.
