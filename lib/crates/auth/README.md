# auth crate

Workspace library implementing the `apis::auth::AuthService` port. Mints
HS256 JWTs, validates them against an in-memory `token_version` cache
backed by Postgres, and adapts the usecase layer into the `AuthService`
contract through an in-memory facade.

> See [docs/guidelines/lib-crate-development.md](../../docs/guidelines/lib-crate-development.md)
> for the cross-cutting conventions this crate follows (workspace deps,
> no `mod.rs`, ports-and-adapters layering, ignored live-DB tests, etc.).

## Layout

```text
src/
  domain/                       # UserCredentials, DomainIdentity, Role, ports
  usecase/                      # AuthUsecase, command DTOs, errors, JWT mint/verify
  adapter/
    persistence/
      postgres/                 # SQLx-backed UserCredentialsRepo, DomainIdentityRepo
    facade/
      in_memory/                # AuthServiceImpl wiring usecase -> apis::auth::AuthService
migrations/                     # SQLx migrations applied to the database
```

The crate root re-exports the public surface (`UserCredentials`,
`DomainIdentity`, `Role`, `DomainError`, `UserCredentialsRepository`,
`DomainIdentityRepository`, `UserCredentialsRepo`, `DomainIdentityRepo`,
`AuthUsecase`, `AuthUsecaseConfig`, `AuthServiceImpl`, the command DTOs
`LoginWithPassword` / `LoginWithDomainUserInfo` / `VerifyAccessToken` /
`RefreshAccessToken` / `Logout`, and the view DTOs `TokenPairView` /
`AuthClaimsView` / `AccessTokenView` / `LogoutAck`) so consumers can
`use auth::*;` without reaching into the sub-modules.

## Database setup

The crate ships two SQLx migrations that define `auth_user_credentials`
and `auth_user_domain_identities`. Apply them before pointing the
repositories at the database:

```bash
sqlx migrate run --source lib/crates/auth/migrations
```

Once the migrations are applied, construct the repositories and usecase
from a `sqlx::PgPool`:

```rust
use std::sync::Arc;
use std::time::Duration;

use auth::{
    AuthServiceImpl, AuthUsecase, AuthUsecaseConfig, DomainIdentityRepo,
    UserCredentialsRepo,
};
use apis::user::UserService; // production wiring uses the user crate's facade

let credentials_repo = UserCredentialsRepo::new(pool.clone());
let identities_repo = DomainIdentityRepo::new(pool);
let user_service: Arc<dyn UserService> = Arc::new(/* … */);
let signing_key = b"<32 random bytes>".to_vec();

let usecase = AuthUsecase::new(AuthUsecaseConfig {
    credentials: credentials_repo,
    identities: identities_repo,
    user_service,
    signing_key,
    access_ttl: Duration::from_secs(15 * 60),
    refresh_ttl: Duration::from_secs(7 * 24 * 60 * 60),
});

let auth_service: Arc<dyn apis::auth::AuthService> =
    Arc::new(AuthServiceImpl::new(usecase));
```

The `auth` crate does not run migrations at runtime; a deployment step
is required.

## Integration tests

The crate ships live-database integration tests at
[`tests/integration_persistence.rs`](tests/integration_persistence.rs).
They connect to PostgreSQL, apply both migrations, and exercise the
full repository surface plus a smoke test of `AuthUsecase::new`. They
are `#[ignore]`-gated so the default `cargo test -p auth` run stays
green without a database.

Run them against a local PostgreSQL with:

```bash
# .env at the workspace root is sourced automatically by the tests
# via dotenvy; AEGIS_AUTH_DATABASE_URL must point at a reachable server.
cargo test -p auth -- --ignored
```

## Token-version cache

`AuthUsecase` keeps an in-memory `Arc<RwLock<HashMap<String, u32>>>`
mapping `code -> token_version`. The cache is populated lazily on the
first `verify` / `refresh` for a code, refreshed on every successful
login, and replaced on every `logout` (which calls
`credentials.bump_token_version` and writes the returned new version
back). The DB is the source of truth; the cache is only invalidated
in-process. In a multi-process deployment each process owns its own
cache and cross-process revocation relies on the next cold-miss DB
read — see the spec for the full discussion.