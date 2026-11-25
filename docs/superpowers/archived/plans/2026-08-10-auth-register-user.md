# Auth User Registration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add allowlisted, administrator-only user registration across the auth usecase, APIs crate, and Aegis HTTP server.

**Architecture:** Extend the existing auth-domain ports rather than bypassing the usecase. `AuthUsecase` owns allowlist normalization and idempotent registration orchestration; adapters translate to the existing user API and PostgreSQL repositories; `AuthServiceImpl` translates usecase DTOs to API DTOs; the HTTP handler authenticates with the existing bearer extractor and authorizes root/admin roles.

**Tech Stack:** Rust, Tokio, async-trait, Axum, Utoipa, SQLx PostgreSQL, Argon2, existing Aegis ports-and-adapters crates.

## Global Constraints

- Normalize configured and requested domains by trimming whitespace and lowercasing them before comparison.
- An empty `allow_domains` list rejects every registration.
- Registration always creates `Role::General` users with `active = false` when the user is missing.
- Reuse existing user, credential, and exact domain-identity records instead of overwriting them.
- Hash the raw password with the existing Argon2 helper before persistence; never return or log plaintext/password hashes.
- Only bearer-authenticated `Root` and `Admin` callers may invoke `POST /api/auth/user-credential`; `General` receives HTTP 403.
- Preserve the existing `PATCH /api/auth/user-credential` behavior.
- Do not introduce a cross-repository transaction in this feature.

---

### Task 1: Extend user and API creation contracts for inactive registration users

**Files:**
- Modify: `lib/crates/apis/src/user.rs`
- Modify: `lib/crates/user/src/usecase/commands.rs`
- Modify: `lib/crates/user/src/usecase/user_usecase.rs`
- Modify: `lib/crates/user/src/adapter/facade/in_memory/service.rs`
- Test: `lib/crates/user/src/usecase/tests.rs`
- Test: `lib/crates/apis/tests/public_api.rs`

**Interfaces:**
- `apis::user::CreateUserRequest` gains `pub active: bool`.
- `user::usecase::CreateUser` gains `pub active: bool`.
- `UserUsecase::create` passes the command's `active` to `UserNew` instead of hardcoding `true`.
- The user facade passes `req.active` through unchanged.

- [ ] **Step 1: Update user tests and all compile-time request literals**

Change every `CreateUserRequest { code, name, role }` and `CreateUser { code, name, role }` literal in user tests and API public-surface tests to include `active: true`, then add a test that a command with `active: false` reaches the repository as `UserNew { active: false }`.

- [ ] **Step 2: Run the focused tests and verify the new test fails for the hardcoded behavior**

Run:

```bash
cargo test -p user create --lib
```

Expected: the new inactive-create test fails while `UserUsecase::create` still hardcodes `active: true`.

- [ ] **Step 3: Thread `active` through the user implementation**

In `lib/crates/user/src/usecase/user_usecase.rs`, construct:

```rust
let input = UserNew {
    code: cmd.code,
    name: cmd.name,
    role: cmd.role,
    active: cmd.active,
};
```

Add the field to `CreateUser` and map it in the facade:

```rust
CreateUser {
    code: req.code,
    name: req.name,
    role: map_role(req.role),
    active: req.active,
}
```

- [ ] **Step 4: Run user and API tests**

Run:

```bash
cargo test -p user
cargo test -p apis
```

Expected: PASS.

- [ ] **Step 5: Commit the contract change**

```bash
git add lib/crates/apis/src/user.rs lib/crates/apis/tests/public_api.rs lib/crates/user/src/usecase/commands.rs lib/crates/user/src/usecase/user_usecase.rs lib/crates/user/src/adapter/facade/in_memory/service.rs lib/crates/user/src/usecase/tests.rs
git commit -m "feat(user): allow inactive users at creation"
```

---

### Task 2: Extend auth domain ports, errors, commands, and PostgreSQL identity creation

**Files:**
- Modify: `lib/crates/auth/src/domain/error.rs`
- Modify: `lib/crates/auth/src/domain/service.rs`
- Modify: `lib/crates/auth/src/domain/repository.rs`
- Modify: `lib/crates/auth/src/domain/tests.rs`
- Modify: `lib/crates/auth/src/adapter/service/user.rs`
- Modify: `lib/crates/auth/src/adapter/persistence/postgres/auth_repo.rs`
- Modify: `lib/crates/auth/src/usecase/commands.rs`
- Modify: `lib/crates/auth/src/lib.rs`
- Test: `lib/crates/auth/src/adapter/persistence/postgres/tests.rs`

**Interfaces:**
- `DomainError` gains `DomainNotAllowed(String)`.
- Auth-domain `UserService` gains `create(code: &str, name: &str) -> Result<UserSummary, DomainError>`; the adapter calls the API user service with `Role::General` and `active: false`.
- `DomainIdentityRepository` gains `create(identity: DomainIdentity) -> Result<DomainIdentity, DomainError>`.
- Add `RegisterUser` and `RegisteredUserView` usecase DTOs, and re-export them from `auth`.

- [ ] **Step 1: Add failing port/adapter tests**

Extend the existing fake API user service and repository mocks so tests can assert the create request and identity insert. Add a domain test asserting `DomainError::DomainNotAllowed("EXAMPLE.COM")` formats as a useful validation error.

- [ ] **Step 2: Run focused auth tests to capture compile failures**

```bash
cargo test -p auth --lib
```

Expected: FAIL until all `UserService` and `DomainIdentityRepository` implementations satisfy the new methods.

- [ ] **Step 3: Implement the port and adapter changes**

In `UserServiceImpl::create`, call:

```rust
self.inner.create(apis::user::CreateUserRequest {
    code: code.to_owned(),
    name: name.to_owned(),
    role: ApiRole::General,
    active: false,
})
```

Translate the returned `UserView` to `UserSummary`, preserving `active` and mapping the role. In `DomainIdentityRepo::create`, insert and return the four identity columns:

```sql
INSERT INTO auth_user_domain_identities (user_code, domain_name, hostname, sid)
VALUES ($1, $2, $3, $4)
RETURNING user_code, domain_name, hostname, sid
```

Use the existing `QueryBuilder`, row conversion, and `map_db_error`.

- [ ] **Step 4: Add command/view types and re-exports**

Define the exact fields:

```rust
pub struct RegisterUser {
    pub user_code: String,
    pub user_name: String,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
    pub password: String,
}

pub struct RegisteredUserView {
    pub user_code: String,
    pub user_name: String,
    pub role: Role,
    pub active: bool,
    pub domain_name: String,
    pub hostname: String,
    pub sid: String,
}
```

- [ ] **Step 5: Run the auth unit and persistence tests**

```bash
cargo test -p auth --lib
```

Expected: PASS. Live PostgreSQL tests remain `#[ignore]`; run them only when a database is configured.

- [ ] **Step 6: Commit the auth port and persistence foundation**

```bash
git add lib/crates/auth/src/domain lib/crates/auth/src/adapter/service/user.rs lib/crates/auth/src/adapter/persistence/postgres/auth_repo.rs lib/crates/auth/src/adapter/persistence/postgres/tests.rs lib/crates/auth/src/usecase/commands.rs lib/crates/auth/src/lib.rs
git commit -m "feat(auth): add registration persistence ports"
```

---

### Task 3: Implement allowlisted idempotent registration in `AuthUsecase`

**Files:**
- Modify: `lib/crates/auth/src/usecase/auth_usecase.rs`
- Modify: `lib/crates/auth/src/usecase/tests.rs`
- Modify: `lib/crates/auth/src/usecase/error.rs` only if validation mapping needs a new wrapper
- Modify: `lib/crates/auth/tests/public_api.rs`
- Modify: `lib/crates/auth/tests/integration_persistence.rs`

**Interfaces:**
- `AuthUsecaseConfig` gains `pub allow_domains: Vec<String>`.
- `AuthUsecase` stores normalized domains, preferably `HashSet<String>` for direct membership checks.
- `AuthUsecase::register_user(RegisterUser) -> Result<RegisteredUserView, UsecaseError>` performs the six-field flow defined below.

- [ ] **Step 1: Add failing registration tests**

Extend `make_usecase` and every `AuthUsecaseConfig` literal with an allowlist. Add tests for:

1. `vec![]` rejects before `get_by_code`, credential lookup, or identity lookup.
2. `vec![" EXAMPLE.com "]` accepts request domain `" example.COM "`.
3. A disallowed domain returns `UsecaseError::Validation(DomainError::DomainNotAllowed(_))`.
4. A missing user invokes create with the supplied code/name and yields `General` + inactive.
5. Missing credentials invokes `create` with a PHC Argon2 hash and token version `0`.
6. Missing identity invokes `create` with the normalized domain and supplied hostname/SID.
7. Existing user, credential, and identity are reused without create calls.
8. Repository errors from each lookup/create propagate.

- [ ] **Step 2: Run only the new tests and verify they fail**

```bash
cargo test -p auth register_user --lib
```

Expected: FAIL because configuration and the method do not yet exist.

- [ ] **Step 3: Normalize and store the allowlist**

Add a helper equivalent to:

```rust
fn normalize_domain(value: &str) -> String {
    value.trim().to_ascii_lowercase()
}
```

In `new`, collect non-empty normalized configured domains into a `HashSet<String>`. In registration, reject empty required fields using existing domain errors, normalize the requested domain for comparison/storage, and return:

```rust
Err(UsecaseError::Validation(DomainError::DomainNotAllowed(
    cmd.domain_name,
)))
```

when the set does not contain it.

- [ ] **Step 4: Implement ordered idempotent orchestration**

Use `get_by_code`; on `Ok(user)` reuse it, on `Err(DomainError::NotFound)` call `user_service.create(&cmd.user_code, &cmd.user_name)`, and propagate all other errors. Then use the same `find_by_code`/`create` pattern for credentials, hashing `cmd.password` with `Self::hash_password` and constructing `UserCredentials::for_repository(code, hash, 0, now, now)`. Finally use identity `find` with exact user code, normalized domain, hostname, and SID; on not found call `identities.create(DomainIdentity::for_repository(...))`.

Return `RegisteredUserView` using the effective user and identity values. Never include the credential object in the view.

- [ ] **Step 5: Run the auth test suite**

```bash
cargo test -p auth
```

Expected: PASS.

- [ ] **Step 6: Commit the usecase implementation**

```bash
git add lib/crates/auth/src/usecase/auth_usecase.rs lib/crates/auth/src/usecase/tests.rs lib/crates/auth/src/usecase/error.rs lib/crates/auth/tests/public_api.rs lib/crates/auth/tests/integration_persistence.rs
git commit -m "feat(auth): register users by allowed domain"
```

---

### Task 4: Add the `apis::auth::AuthService` registration contract and facade implementation

**Files:**
- Modify: `lib/crates/apis/src/auth.rs`
- Modify: `lib/crates/apis/tests/public_api.rs`
- Modify: `lib/crates/auth/src/adapter/facade/in_memory/service.rs`
- Modify: `lib/crates/auth/src/adapter/facade/in_memory/tests.rs`

**Interfaces:**
- Add `RegisterUserRequest` and `RegisterUserResponse` with the exact fields in the design.
- Add `async fn register_user(&self, req: RegisterUserRequest) -> Result<RegisterUserResponse, AuthApiError>` to `AuthService`.

- [ ] **Step 1: Add API contract tests/literals**

Update every `AuthService` fake implementation with a `register_user` method. Add a facade test that captures all six request fields, returns a view with `General` and `active: false`, and asserts exact API response conversion.

- [ ] **Step 2: Run API tests to verify the new contract is incomplete**

```bash
cargo test -p apis
```

Expected: FAIL at existing fake implementations until the required method is added.

- [ ] **Step 3: Add DTOs and facade translation**

In `AuthServiceImpl`, translate:

```rust
RegisterUser {
    user_code: req.user_code,
    user_name: req.user_name,
    domain_name: req.domain_name,
    hostname: req.hostname,
    sid: req.sid,
    password: req.password,
}
```

Map `RegisteredUserView` fields and `Role` to `RegisterUserResponse`. Keep `map_error` mapping `UsecaseError::Validation` to `AuthApiError::Validation`, which covers `DomainNotAllowed`.

- [ ] **Step 4: Run facade and API tests**

```bash
cargo test -p apis
cargo test -p auth adapter::facade --lib
```

Expected: PASS.

- [ ] **Step 5: Commit the API/facade change**

```bash
git add lib/crates/apis/src/auth.rs lib/crates/apis/tests/public_api.rs lib/crates/auth/src/adapter/facade/in_memory/service.rs lib/crates/auth/src/adapter/facade/in_memory/tests.rs
git commit -m "feat(auth): expose user registration service"
```

---

### Task 5: Add configuration wiring and HTTP registration DTOs

**Files:**
- Modify: `apps/server/aegis-server/src/config.rs`
- Modify: `apps/server/aegis-server/src/run.rs`
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`
- Test: `apps/server/aegis-server/src/config.rs` tests if present

**Interfaces:**
- `Config` gains `allow_domains: Vec<String>`.
- Environment variable: `AEGIS_AUTH_ALLOW_DOMAINS`, parsed as a comma-separated list, defaulting to `Vec::new()`.
- `build_auth_service` forwards `config.allow_domains.clone()` into `AuthUsecaseConfig`.
- Wire DTOs gain `RegisterUserRequest` and `RegisterUserResponse` plus `From<apis::auth::RegisterUserResponse>`.

- [ ] **Step 1: Add config parsing tests**

Assert `AEGIS_AUTH_ALLOW_DOMAINS=" EXAMPLE.com,corp.example "` becomes `vec!["EXAMPLE.com", "corp.example"]` (the usecase performs canonical normalization) and an unset variable becomes an empty vector.

- [ ] **Step 2: Run config tests and verify the new env field is absent**

```bash
cargo test -p aegis-server config
```

Expected: FAIL until the field and parser are implemented.

- [ ] **Step 3: Implement config/run/DTO/OpenAPI changes**

Add the six request fields and seven response fields using the existing `Serialize`, `Deserialize`, and `ToSchema` conventions. Register both schemas in `openapi.rs`. Forward the config field exactly in the existing `AuthUsecaseConfig` literal.

- [ ] **Step 4: Run server compile/tests**

```bash
cargo test -p aegis-server --lib config
cargo check -p aegis-server
```

Expected: PASS or only expected failures from the not-yet-implemented handler/mock method, which Task 6 resolves.

- [ ] **Step 5: Commit wiring and DTOs**

```bash
git add apps/server/aegis-server/src/config.rs apps/server/aegis-server/src/run.rs apps/server/aegis-server/src/transport/http/dto.rs apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(server): configure registration domains"
```

---

### Task 6: Expose the administrator-only HTTP POST route

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/auth/user_credential/handlers.rs`
- Modify: `apps/server/aegis-server/src/transport/http/auth/user_credential/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs`
- Modify: all `AuthService` mock implementations under `apps/server/aegis-server/src/transport/http/`

**Interfaces:**
- Handler: `POST /api/auth/user-credential` returns `Result<(StatusCode, Json<dto::RegisterUserResponse>), ApiError>`.
- It extracts `AuthClaims`, accepts `Json<dto::RegisterUserRequest>`, rejects roles other than `Root`/`Admin` with `ApiError::Forbidden`, calls `state.auth.register_user`, and returns `StatusCode::CREATED`.

- [ ] **Step 1: Write HTTP tests before the handler**

Add tests for:

- root bearer + valid JSON → `201` and response fields
- admin bearer + valid JSON → `201`
- general bearer → `403` without calling `register_user`
- no bearer → existing `401`
- auth service validation/repository errors → existing mapped status/body
- all six fields forwarded unchanged, including password only into the API request

Update every local `MockAuth` implementation to include the new trait method, initially returning `unimplemented!()` except the user-credential handler mock, which records and returns configured registration data.

- [ ] **Step 2: Run the focused HTTP tests and verify failure**

```bash
cargo test -p aegis-server user_credential --lib
```

Expected: FAIL because the POST route/handler is not registered.

- [ ] **Step 3: Implement authorization and handler translation**

Use the existing project authorization pattern:

```rust
fn require_admin_or_root(claims: &AuthClaims) -> Result<(), ApiError> {
    match claims.0.role {
        apis::user::Role::Root | apis::user::Role::Admin => Ok(()),
        apis::user::Role::General => Err(ApiError::Forbidden),
    }
}
```

Translate the body into `apis::auth::RegisterUserRequest`, call `state.auth.register_user`, convert the response, and return `(StatusCode::CREATED, Json(response))`. Add a `#[utoipa::path(post, ... security(("BearerAuth" = [])))]` annotation documenting 201, 400, 401, 403, and 500 responses.

- [ ] **Step 4: Register POST beside the existing PATCH route**

Add the registration handler to `user_credential/router.rs` with `routes!(handlers::register_user)` while retaining `routes!(handlers::update)`, so both methods resolve at `/api/auth/user-credential`.

- [ ] **Step 5: Update route/OpenAPI assertions**

Replace the old assertion that POST is unregistered. Assert the OpenAPI document contains `POST /api/auth/user-credential` with bearer security and the documented response codes, while preserving the PATCH assertions.

- [ ] **Step 6: Run all server unit tests**

```bash
cargo test -p aegis-server
```

Expected: PASS.

- [ ] **Step 7: Commit the HTTP route**

```bash
git add apps/server/aegis-server/src/transport/http
git commit -m "feat(server): expose admin user registration route"
```

---

### Task 7: Update integration fixtures, docs, and run the complete verification suite

**Files:**
- Modify: `lib/crates/auth/README.md`
- Modify: `lib/crates/auth/tests/integration_persistence.rs`
- Modify: `apps/server/aegis-server/tests/integration_auth.rs`
- Modify: any remaining `AuthUsecaseConfig` or `AuthService` literals reported by the compiler

- [ ] **Step 1: Update README construction example**

Add `allow_domains: vec!["example.com".into()]` to the documented `AuthUsecaseConfig` construction and document that an empty list denies registration.

- [ ] **Step 2: Update ignored persistence/integration fixtures**

Add allowlist values to all config literals. Add an ignored auth persistence test that creates a missing domain identity and verifies it can subsequently be found. Add server integration coverage for an admin registration success and a general-user 403, using the existing database setup and token helpers.

- [ ] **Step 3: Search for stale interfaces**

```bash
grep -R "AuthUsecaseConfig {\|impl AuthService for\|CreateUserRequest {\|CreateUser {" -n lib apps --include='*.rs'
```

Update every remaining literal/implementation so the workspace has no stale trait or struct shapes.

- [ ] **Step 4: Format and run the full non-database suite**

```bash
cargo fmt --all
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

Expected: all non-ignored tests pass, formatting is clean, and clippy reports no warnings.

- [ ] **Step 5: Run ignored tests when PostgreSQL is available**

```bash
cargo test -p auth -- --ignored
cargo test -p aegis-server --test integration_auth -- --ignored
```

Expected: PASS with a configured `AEGIS_DATABASE_URL`; if no database is available, report these tests as skipped rather than claiming completion.

- [ ] **Step 6: Review the final diff and commit documentation/fixtures**

```bash
git diff HEAD~1 --check
git status --short
git add lib/crates/auth/README.md lib/crates/auth/tests/integration_persistence.rs apps/server/aegis-server/tests/integration_auth.rs
git commit -m "test(auth): cover registered user integration flow"
```

Expected: only intended files remain changed, with no plaintext credentials or secrets in the diff.
