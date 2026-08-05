# Auth Crate Design

## Goal

Add `lib/crates/auth` as a reusable Rust library that implements the
`apis::auth::AuthService` port defined in
[`lib/crates/apis/src/auth.rs`](../../lib/crates/apis/src/auth.rs).

The crate owns:

- Password hashing policy (Argon2id via `argon2 = "0.5.3"`).
- Per-user JWT token versioning used to invalidate outstanding tokens.
- Mapping of Windows-domain AD identities (domain_name / hostname / sid) to a
  `user::code`.

The crate does **not** own the user lifecycle. Active state and `Role` are
read on demand from the `apis::user::UserService` port (a private field on the
auth usecase). The auth crate depends on the `apis` crate, not on the `user`
crate directly.

## Architecture

Ports-and-adapters DDD structure, exactly mirroring
[`lib/crates/user/`](../../lib/crates/user/):

- `domain` — `UserCredentials`, `DomainIdentity`, `Role`, validating
  constructors, ports (`UserCredentialsRepository`, `DomainIdentityRepository`),
  and `DomainError`. No I/O, no `sqlx`, no `tokio`, no `argon2`.
- `usecase` — `AuthUsecase<R, D>`, command DTOs (`LoginWithPassword`,
  `LoginWithDomainUserInfo`, `VerifyAccessToken`, `RefreshAccessToken`,
  `Logout`), view DTOs (`TokenPairView`, `AuthClaimsView`, `AccessTokenView`,
  `LogoutAck`), and `UsecaseError`. Holds `Arc<dyn UserService>` as a private
  field. Generic over two repository ports so tests inject in-memory fakes.
- `adapter` — concrete implementations of the domain ports.
  - `adapter/persistence/postgres/` — `UserCredentialsRepo` and
    `DomainIdentityRepo` backed by `sqlx::PgPool`.
  - `adapter/facade/in_memory/` — `AuthServiceImpl<R, D>` adapting
    `AuthUsecase<R, D>` to `apis::auth::AuthService`.

Per [`docs/guidelines/lib-crate-development.md`](../guidelines/lib-crate-development.md):
no `mod.rs`; each top-level module uses `src/<module>.rs` + `src/<module>/`.
The terminal leaf modules (`role.rs`, `auth_repo.rs`, `service.rs`, …) are leaf
files with no companion directory.

## Public API

Constructors match:

```rust
let credentials_repo = UserCredentialsRepo::new(pool.clone());
let identities_repo = DomainIdentityRepo::new(pool.clone());
let user_service: Arc<dyn UserService> = Arc::new(/* … wired UserServiceImpl or fake … */);
let signing_key = SigningKey::from_bytes(&hmac_secret_bytes);

let usecase = AuthUsecase::new(AuthUsecaseConfig {
    credentials: credentials_repo,
    identities: identities_repo,
    user_service,
    signing_key,
    access_ttl: Duration::from_secs(15 * 60),
    refresh_ttl: Duration::from_secs(7 * 24 * 60 * 60),
});

let auth_service: Arc<dyn AuthService> = Arc::new(AuthServiceImpl::new(usecase));
```

`AuthUsecaseConfig<R, D>` is a plain struct with `pub` fields — no builder
ceremony. Generic over the same two repository types as `AuthUsecase` so the
field types stay concrete:

```rust
pub struct AuthUsecaseConfig<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    pub credentials: R,
    pub identities: D,
    pub user_service: Arc<dyn UserService>,
    pub signing_key: SigningKey,
    pub access_ttl: Duration,
    pub refresh_ttl: Duration,
}
```

The crate root re-exports the domain types, the two Postgres repos, the
in-memory facade, the usecase, the config struct, and the command / view DTOs
so consumers can `use auth::*;` without reaching into sub-modules. Specifically:

```rust
pub use domain::{
    DomainError, DomainIdentity, Role, UserCredentials,
    UserCredentialsRepository, DomainIdentityRepository,
};
pub use adapter::persistence::postgres::{UserCredentialsRepo, DomainIdentityRepo};
pub use adapter::facade::in_memory::AuthServiceImpl;
pub use usecase::{
    AuthUsecase, AuthUsecaseConfig, UsecaseError,
    LoginWithPassword, LoginWithDomainUserInfo, VerifyAccessToken,
    RefreshAccessToken, Logout, AuthClaimsView, TokenPairView,
    AccessTokenView, LogoutAck,
};
```

## Domain rules

`Role` has `Root`, `Admin`, `General` variants — mirror of
`apis::user::Role`. Conversion to / from `apis::user::Role` happens at the
facade boundary, exhaustive match. Validation via `Role::try_from(&str)`; the
database stores the lowercase string.

`UserCredentials` carries `{ code, password_hash, token_version, created_at,
updated_at }`. Validating constructor (`UserCredentials::new`) rejects empty
`code` or empty `password_hash`. The repository-bound constructor
(`UserCredentials::for_repository`) is `pub(crate)` and skips validation.

`DomainIdentity` carries `{ user_code, domain_name, hostname, sid }`. Same
two-constructor pattern.

`DomainError` variants:

- `EmptyCode`, `EmptyPasswordHash`
- `InvalidRole(String)`
- `NotFound`
- `DuplicateCode(String)` — produced by the unique-violation map
- `Inactive` — produced when the `UserService` reports `active = false`
- `InvalidCredentials` — produced when `argon2::verify_password` mismatches
- `Repository(String)` — driver / `UserService` message

## Persistence

Two SQLx migration files. Each file is its own migration so additive schema
changes remain purely additive.

### `migrations/0001_create_auth_user_credentials.sql`

```sql
CREATE TABLE auth_user_credentials (
    code TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    token_version INTEGER NOT NULL DEFAULT 1,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_user_credentials_password_hash_check CHECK (length(password_hash) > 0)
);

-- auth_user_credentials_set_updated_at trigger + function copied verbatim
-- from lib/crates/user/migrations/0001_create_users.sql.
```

`code` is the primary key because every auth code path looks up by `code`
only — no numeric id is needed. `password_hash` length check is
belt-and-braces against a missing hash.

### `migrations/0002_create_auth_user_domain_identities.sql`

```sql
CREATE TABLE auth_user_domain_identities (
    id INTEGER GENERATED BY DEFAULT AS IDENTITY PRIMARY KEY,
    user_code TEXT NOT NULL,
    domain_name TEXT NOT NULL,
    hostname TEXT NOT NULL,
    sid TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT auth_user_domain_identities_unique UNIQUE (user_code, domain_name, hostname, sid)
);
```

No FK to the user crate's `users` table — the auth schema must deploy
independently. The application layer is responsible for keeping
`user_code` in sync with the user crate's `users.code`.

## JWTs

Use `jsonwebtoken = "11"` from `[workspace.dependencies]`. HS256 only.

Two distinct claim structs, defined privately in the usecase module (no
re-export at the crate root — they are an internal mint/verify detail):

```rust
#[derive(Serialize, Deserialize)]
struct AccessClaims {
    sub: String,           // user code
    role: String,          // auth::Role::as_str()
    ver: u32,              // token_version at mint time
    exp: i64,              // unix timestamp
    iat: i64,
}

#[derive(Serialize, Deserialize)]
struct RefreshClaims {
    sub: String,           // user code
    ver: u32,              // token_version at mint time
    exp: i64,              // unix timestamp
    iat: i64,
}
```

- Refresh tokens carry only `sub`, `ver`, `exp`, `iat` — no role. The
  refresh path re-fetches the user from `user_service.get_by_code` so it
  always reads the current `Role` rather than trusting a stale value baked
  in at refresh-mint time.
- Token-type rejection is structural: `verify` calls
  `jsonwebtoken::decode::<AccessClaims>(token, …)`. If a refresh token is
  presented, serde deserialization fails because `role` is required on
  `AccessClaims` but absent from the token. The same logic works in reverse
  for `refresh`. No `typ` discriminator field needed.
- `ver` is the `token_version` baked in at mint time. `verify` and
  `refresh` both consult the in-memory cache described below; on cache miss
  they fall back to the DB and populate the cache. One
  `UPDATE auth_user_credentials SET token_version = token_version + 1`
  (exposed by `bump_token_version`) invalidates every outstanding token for
  that user, and the same call updates the cache so subsequent
  `verify`/`refresh` calls in this process reject the old `ver`.

## In-memory token-version cache

The cache lives inside the `UserCredentialsRepository` implementation
(`adapter/persistence/postgres::UserCredentialsRepo`), not in the
usecase. `AuthUsecase` reaches it through a new port method:

```rust
#[async_trait]
pub trait UserCredentialsRepository: Send + Sync {
    async fn find_by_code(&self, code: &str) -> Result<UserCredentials, DomainError>;
    async fn create(&self, credentials: UserCredentials) -> Result<UserCredentials, DomainError>;
    async fn bump_token_version(&self, code: &str) -> Result<u32, DomainError>;

    /// Returns the user's current `token_version`. Implementations
    /// may cache the result to avoid repeated database reads on the
    /// `verify` / `refresh` hot path. `bump_token_version` updates
    /// the cache atomically with the database write.
    async fn current_token_version(&self, code: &str) -> Result<u32, DomainError>;
}
```

The Postgres impl owns a private `Arc<std::sync::RwLock<HashMap<String,
u32>>>` keyed by user code. `current_token_version` reads cache → on
miss, runs a `SELECT token_version FROM auth_user_credentials WHERE code
= $1` and writes the result into the cache. `bump_token_version` runs
the existing `UPDATE ... RETURNING` and writes the returned new version
into the cache. `find_by_code` does NOT touch the cache — it returns
the full row for callers that need the password hash. Login reads
`row.token_version` from `find_by_code` directly; the cache populates
lazily on the first `verify` / `refresh` after login.

Concurrency:

- `std::sync::RwLock` (not `tokio::sync::RwLock`): the lock is held only
  during the in-memory map read / write, never across an `.await`.
  Multiple concurrent `current_token_version` calls share a read lock;
  `bump_token_version` takes a write lock for the brief duration of the
  single map entry update.
- `Arc` is required because `UserCredentialsRepo` is shared state and
  the cache must live alongside the pool.

Multi-process limitation (documented, not solved here):

- In a multi-process deployment, each process owns its own cache inside
  its own `UserCredentialsRepo`. If process A processes a `logout`,
  process B's cache for that code still holds the pre-bump version
  until B's next cold-miss DB read picks up the new value. This is
  acceptable for a single-process service. If cross-process revocation
  is needed later, the cache invalidation hook can be wired to a
  `LISTEN/NOTIFY` channel or a pub-sub backend; that is out of scope
  for this spec.

## Usecase

```rust
pub struct AuthUsecase<R: UserCredentialsRepository, D: DomainIdentityRepository> {
    credentials: R,
    identities: D,
    user_service: Arc<dyn UserService>,
    signing_key: SigningKey,
    access_ttl: Duration,
    refresh_ttl: Duration,
}

impl<R, D> AuthUsecase<R, D> {
    pub fn new(config: AuthUsecaseConfig<R, D>) -> Self {
        Self {
            credentials: config.credentials,
            identities: config.identities,
            user_service: config.user_service,
            signing_key: config.signing_key,
            access_ttl: config.access_ttl,
            refresh_ttl: config.refresh_ttl,
        }
    }
}
```

Why `Arc<dyn UserService>` (not generic): the apis `UserService` is an
object-safe trait; the auth facade already boxes `AuthService` the same way
(`Arc<dyn AuthService>`). This avoids threading a generic through the auth
facade's public surface and keeps `AuthServiceImpl` non-generic over the user
service.

The `token_versions` cache lives in the repository implementation (see
"In-memory token-version cache" above), not in the usecase.

## Usecase flow

| Command | Steps | Returns |
| --- | --- | --- |
| `LoginWithPassword { code, password }` | validate → `credentials.find_by_code` → `user_service.get_by_code` → check `active` → `argon2::verify_password` → mint access + refresh JWTs (uses `row.token_version` directly; the repo's cache populates lazily on the first verify/refresh) | `TokenPairView` |
| `LoginWithDomainUserInfo { code, domain_name, hostname, sid }` | validate → `identities.find` → `user_service.get_by_code` → check `active` → `credentials.find_by_code` → mint tokens (uses `row.token_version` directly) | `TokenPairView` |
| `VerifyAccessToken { access_token }` | decode JWT as `AccessClaims` → check signature + expiry → `credentials.current_token_version(sub)` (cache hit, or DB read + cache write on miss) → `user_service.get_by_code` → check `active` → compare `jwt.ver == current_version` → project to `AuthClaimsView` | `AuthClaimsView` |
| `RefreshAccessToken { refresh_token }` | decode JWT as `RefreshClaims` → check signature + expiry → `credentials.current_token_version(sub)` (cache hit, or DB read + cache write on miss) → `user_service.get_by_code` → check `active` → compare versions → mint new access token (fresh `AccessClaims`) | `AccessTokenView` |
| `Logout { code }` | validate → `credentials.bump_token_version(code)` (the repo atomically updates the DB and writes the new version into its cache) | `LogoutAck { code }` |

`logout` does not consult `user_service`; bumping `token_version` invalidates
every outstanding JWT regardless of current active state, and `Inactive` only
matters for new logins.

`UsecaseError`:

```rust
enum UsecaseError {
    Validation(#[source] DomainError),
    Repository(#[source] DomainError),
    Verification(String),   // JWT decode / signature / expiry / typ / version mismatch
}
```

No separate `Signing` variant — encode failures bubble up as
`Verification(String)` so the facade has a single decode-related variant.

## Facade

`AuthServiceImpl<R, D>` wraps `AuthUsecase<R, D>` and implements
`apis::auth::AuthService`. `UsecaseError` → `AuthApiError` mapping:

- `Validation(d)` → `Validation(d.to_string())`
- `Repository(DomainError::NotFound)` → `NotFound`
- `Repository(DomainError::Inactive)` → `Inactive`
- `Repository(DomainError::InvalidCredentials)` → `InvalidCredentials`
- `Repository(_)` → `Repository(msg)`
- `Verification(_)` → `Verification(_)`

`Role` ↔ `apis::user::Role` conversion is exhaustive on both sides.

The facade tests ship a `FakeUserService` (in
`adapter/facade/in_memory/fake_user_service.rs`) implementing
`apis::user::UserService` with canned `UserView`s so every `AuthService`
method is exercised without touching Postgres.

## Testing

Four kinds of tests, in the order listed in the lib-crate guideline:

1. **Domain unit tests** (`src/domain/tests.rs`):
   - `Role::try_from` / `Role::as_str` round trips and unknown-value rejection.
   - `UserCredentials::new` rejects empty `code` and empty `password_hash`.
   - `DomainIdentity::new` rejects empty fields.
2. **Adapter unit tests** (`src/adapter/persistence/postgres/tests.rs`):
   - Migration content tests: read each `migrations/*.sql` file as a string via
     `std::fs` + `env!("CARGO_MANIFEST_DIR")` and assert column / constraint /
     trigger set.
   - Row → domain `TryFrom` conversions for both `CredentialRow` and
     `DomainIdentityRow`.
3. **Usecase unit tests** (`src/usecase/tests.rs`): `MockUserCredentialsRepo`
   + `MockDomainIdentityRepo` + `FakeUserService`. Exercise every command,
   assert:
   - Empty input → `Validation`.
   - Inactive user → `Repository(Inactive)`.
   - Wrong password → `Repository(InvalidCredentials)`.
   - JWT round-trips through `verify` and `refresh`.
   - `token_version` mismatch → `Verification`.
   - `logout` bumps `token_version`; subsequent `verify` of the old access
     token fails.
   - `verify` and `refresh` call `MockUserCredentialsRepo::current_token_version`
     for the version check (assert the mock's `current_token_version_calls`
     counter increments).
4. **Persistence adapter cache tests** (`src/adapter/persistence/postgres/tests.rs`,
   beside the row conversion tests): a separate test module that wraps the
   Postgres impl behind a stub of the inner DB calls and asserts the
   cache-population + cache-invalidation behaviour directly. The current
   in-memory mock exposes `current_token_version` as a direct lookup, so
   the mock-level tests verify the contract; the live-DB `#[ignore]` tests
   cover the cache under real Postgres traffic.
4. **Facade unit tests** (`src/adapter/facade/in_memory/tests.rs`): wire
   `AuthServiceImpl` on top of the mocks + `FakeUserService`. Exercise every
   `AuthService` method; assert:
   - `AuthApiError` mapping for every `UsecaseError` branch.
   - Object safety: `Box<dyn AuthService>` and `Send + Sync`.
5. **`tests/` directory tests**:
   - `tests/public_api.rs` — compile-only type-naming test. Lock the
     `UserCredentialsRepo::new(pool)` / `DomainIdentityRepo::new(pool)`
     constructors as function pointers, and `AuthUsecase::new(cfg)` against
     a concrete `AuthUsecaseConfig` so the constructor chain is
     type-checked end-to-end without running anything.
   - `tests/integration_persistence.rs` — `#[ignore]`-gated; reads
     `AEGIS_AUTH_DATABASE_URL`; drops live tables before applying migrations.

## Workspace integration

- Root [`Cargo.toml`](../../Cargo.toml) `[workspace].members` already lists
  `lib/crates/auth` — keep that line as-is.
- Add `jsonwebtoken = "11"` to `[workspace.dependencies]` (already provides
  `sqlx`, `tokio`, `argon2`, `async-trait`, `thiserror`, `chrono`,
  `rand_core`).
- `Cargo.toml` dependencies: `sqlx`, `tokio`, `async-trait`, `thiserror`,
  `argon2`, `chrono`, `rand_core`, `jsonwebtoken`, `apis = { path =
  "../apis" }` with one-line comments where the role is non-obvious.
- `[dev-dependencies]`: `dotenvy`, `sqlx`.

## Verification gate

Per the lib-crate guideline:

```bash
cargo fmt --all -- --check
cargo clippy -p auth --all-targets --all-features -- -D warnings
cargo test -p auth
cargo doc -p auth --no-deps
cargo test -p auth -- --ignored --test-threads=1   # with AEGIS_AUTH_DATABASE_URL
```