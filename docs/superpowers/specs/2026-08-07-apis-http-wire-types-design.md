# `apis` HTTP Wire Types Design

## Goal

Make the DTOs in `lib/crates/apis` usable directly as axum request /
response bodies and as utoipa OpenAPI schemas, so a future
`aegis-server` HTTP router can route to `apis::user::UserService` and
`apis::auth::AuthService` without redefining every type.

Today `apis` depends only on `async-trait`, `chrono`, and `thiserror`.
No type in the workspace derives `Serialize`, `Deserialize`, or
`ToSchema`; `apps/server/aegis-server` is a `println!("Hello, world!")`
stub, and `axum` / `utoipa` / `utoipa-axum` / `utoipa-swagger-ui` are
pinned in `[workspace.dependencies]` but absent from every crate's
`[dependencies]` and from `Cargo.lock`.

After this change:

- Every DTO in `apis::user` and `apis::auth` derives `Serialize` +
  `Deserialize` unconditionally, with a camelCase wire contract.
- `Role` serializes as `"root"` / `"admin"` / `"general"`, matching
  `Role::as_str()` in both domain crates and the Postgres CHECK
  constraint.
- An optional `openapi` feature adds `utoipa::ToSchema` to every DTO
  and to both error enums.
- `apis` gains **no** `axum` dependency. Status-code mapping,
  `IntoResponse`, `#[utoipa::path]`, routers, and the error response
  body all remain the server's responsibility.
- Both trait surfaces (`UserService`, `AuthService`) are byte-for-byte
  unchanged. This change is additive attributes plus a manifest.

## Scope

`lib/crates/apis` only, plus one line in the root `Cargo.toml`
(`serde_json` as a workspace dev-dependency). `apps/server/aegis-server`
stays a stub; wiring axum is a separate task with its own spec.

## Decisions

Each of these was chosen deliberately over a named alternative.

### 1. `apis` gets serde + utoipa, never axum

The alternatives were a separate `http-api` crate holding wire DTOs
with `From<apis::…>` conversions, or making `apis` a full HTTP crate
with `IntoResponse` and `OpenApiRouter` factories.

A separate crate would honour `docs/guidelines/lib-crate-development.md`
§3 most literally (transport lives in `adapter::facade::<backend>`) but
would add a fourth near-identical DTO set to a workspace that already
carries three parallel `Role` enums and three `UserView`-shaped types.
Pulling axum into `apis` would go the other way and make every future
consumer — a gRPC facade, the Tauri app — depend on a web framework.

Serde and utoipa are both serialization-shaped concerns, not transport
concerns; a gRPC or Tauri consumer can use them harmlessly. axum cannot
make that claim. That is the line this design draws.

### 2. serde is unconditional; only `utoipa` is feature-gated

An earlier draft gated serde behind a `serde` feature and made
`openapi` imply it. That implication was load-bearing rather than
stylistic: the camelCase wire names live in `#[serde(rename_all)]`
attributes, and utoipa's derive reads those attributes at
macro-expansion time to name schema properties. Under `cfg_attr`
gating, an `openapi`-without-`serde` build would emit a schema
advertising `user_code` while the wire emitted `userCode` — a silent,
type-checked-clean contract divergence.

Making serde unconditional deletes that failure mode outright. There is
no feature combination in which the `rename_all` attributes are absent
while `ToSchema` is derived. It also removes ~19 `cfg_attr` lines.

The cost: `user` and `auth` now depend on serde transitively with no
opt-out. `auth` already depends on serde directly for its JWT claim
structs, so only `user` gains anything, and only at compile time.

### 3. Uniform derives, including the credential types

`UserCredentialView`, `CreateUserCredentialRequest`, and
`UpdateUserCredentialRequest` all carry a `password_hash`. Deriving
`Serialize` on them means `Json(view)` compiles and ships an Argon2
hash to the client; deriving `Deserialize` on the request types implies
a client that computes hashes, which is not how the `auth` crate works
(hashing policy lives server-side).

The alternative — omitting derives on those three so the compiler
rejects `Json(view)` — was considered and rejected in favour of a
uniform, predictable rule for all 17 DTOs. The hazard is handled by
documentation instead: see "Security documentation" below.

### 4. `derive(Debug)` is retained on the credential types

`docs/guidelines/lib-crate-development.md` §4 says to hand-roll `Debug`
whenever a `derive(Debug)` would leak a secret, and
`auth::domain::UserCredentials` already redacts exactly this field.
Redaction was considered and explicitly declined: the three `apis`
types keep `#[derive(Debug)]`.

Consequence to accept knowingly: any `{:?}` or `tracing` call on these
types prints the full Argon2 hash, including from inside a server error
log. The `# Security` doc-comments are the only mitigation.

### 5. camelCase fields, lowercase `Role`

The wire is consumed by JavaScript/TypeScript clients (the Tauri
desktop frontend is the near-term one), so structs get
`#[serde(rename_all = "camelCase")]`.

`Role` gets `#[serde(rename_all = "lowercase")]` rather than camelCase,
because variant casing is a different question from field casing. Rust
defaults would emit `"General"`, which disagrees with `Role::as_str()`
in both `auth::domain` and `user::domain` and with the Postgres CHECK
constraint that stores `'root' | 'admin' | 'general'`. Lowercase makes
one spelling of a role true across HTTP, application code, and storage.

### 6. Error enums get `ToSchema` but no serde

`UserApiError` and `AuthApiError` describe failure modes in the OpenAPI
document, but `apis` decides nothing about HTTP status codes and does
not define an error response body — the server owns both.

An earlier draft proposed an inherent `fn status_code(&self) -> u16`
(expressible without axum) plus a shared `ErrorBody` schema. That was
considered and rejected: status semantics are a transport decision, and
`apis` is staying out of transport.

Known trade-off: because the enums are not `Serialize`, their schemas
describe a shape `apis` itself cannot produce. They are a documented
catalogue of failure modes that the server may reference from
`responses(...)`, not a body the server is obliged to emit.

## Manifest

`lib/crates/apis/Cargo.toml`:

```toml
[dependencies]
async-trait = { workspace = true }
# `features = ["serde"]` on top of the workspace pin: the workspace
# sets `default-features = false`, and DateTime<Utc> needs chrono's
# serde impls to round-trip as RFC 3339 on the wire.
chrono      = { workspace = true, features = ["serde"] }
serde       = { workspace = true }
thiserror   = { workspace = true }
# `features = ["chrono"]` is a utoipa-gen codegen flag (it declares no
# dependency of its own). Without it DateTime<Utc> renders as an empty
# object instead of `string` / `date-time`.
utoipa      = { workspace = true, optional = true, features = ["chrono"] }

[dev-dependencies]
serde_json  = { workspace = true }

[features]
default = []
openapi = ["dep:utoipa"]
```

Root `Cargo.toml` gains one line under `[workspace.dependencies]`:

```toml
serde_json = "1"
```

### Verified feature mechanics

Checked against the registry index rather than assumed, because two of
these were counter-intuitive:

- **`utoipa`'s `axum_extras` does not depend on axum.** The workspace
  pins `utoipa = { version = "5.5.0", features = ["axum_extras", "debug"] }`,
  and `apis` inherits that pin. In `utoipa-gen` 5.5.0 the feature
  expands to `["regex", "syn/extra-traits"]` — a proc-macro codegen
  flag with no axum dependency. So inheriting the workspace pin does
  not leak axum into `apis`, and no root-manifest feature change is
  needed.
- **`utoipa`'s `chrono` feature is `[]` in `utoipa-gen`** — also a pure
  codegen flag, but required for correct `DateTime<Utc>` schemas.
- **`chrono` has no explicit `serde` feature entry**; `serde` is an
  optional dependency, which gives it an implicit feature of the same
  name. `features = ["serde"]` is therefore valid.
- `utoipa` depends on `serde` and `serde_json` non-optionally, so the
  `openapi` feature adds no serde-shaped weight beyond what is already
  present.

## Derive pattern

Every DTO takes the same three-line shape. Fully-qualified derive paths
mean no `use serde::…` or `use utoipa::…` imports are added to either
module.

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct UserView {
    pub id: i32,
    pub code: String,
    pub name: String,
    pub role: Role,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

`Role` differs only in the `rename_all` value:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum Role { Root, Admin, General }
```

Error enums take `ToSchema` alone:

```rust
#[derive(Debug, thiserror::Error)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum UserApiError { /* variants unchanged */ }
```

### Types affected

`apis::user` — `Role`, `UserView`, `CreateUserRequest`,
`UpdateUserRequest` (DTO pattern); `UserApiError` (`ToSchema` only).

`apis::auth` — `TokenPair`, `AuthClaims`, `LoginWithPasswordRequest`,
`LoginWithDomainUserInfoRequest`, `LogoutRequest`, `VerifyRequest`,
`RefreshRequest`, `CreateUserCredentialRequest`,
`UpdateUserCredentialRequest`, `LogoutResponse`, `RefreshResponse`,
`UserCredentialView`, `RemoveUserCredentialResponse` (DTO pattern);
`AuthApiError` (`ToSchema` only).

That is 17 DTOs and 2 error enums.

`LogoutResponse` and `RemoveUserCredentialResponse` are field-less and
serialize to `{}`; `rename_all` is a harmless no-op on both, kept for
uniformity so a later field addition inherits the right convention.

### Incidental fixes

`CreateUserRequest` and `UpdateUserRequest` are the only DTOs in the
crate without `Debug` / `Clone`. Both gain `#[derive(Debug, Clone)]` so
all 17 DTOs carry the same baseline. `UpdateUserRequest` keeps its
existing `Default`.

## Wire contract

`rename_all = "camelCase"` changes exactly eight distinct field names.
Every other field is single-word and unaffected (`id`, `code`, `name`,
`role`, `active`, `password`, `sid`, `hostname`).

| Rust field      | JSON key       | Types                                                             |
| --------------- | -------------- | ----------------------------------------------------------------- |
| `user_code`     | `userCode`     | `UserCredentialView`, `Create`/`UpdateUserCredentialRequest`      |
| `password_hash` | `passwordHash` | `UserCredentialView`, `Create`/`UpdateUserCredentialRequest`      |
| `token_version` | `tokenVersion` | `AuthClaims`, `UserCredentialView`                                |
| `access_token`  | `accessToken`  | `TokenPair`, `VerifyRequest`, `RefreshResponse`                   |
| `refresh_token` | `refreshToken` | `TokenPair`, `RefreshRequest`, `LogoutRequest`                    |
| `domain_name`   | `domainName`   | `LoginWithDomainUserInfoRequest`                                  |
| `created_at`    | `createdAt`    | `UserView`                                                        |
| `updated_at`    | `updatedAt`    | `UserView`                                                        |

`Role` emits `"root"`, `"admin"`, `"general"`.

`DateTime<Utc>` emits RFC 3339 (`"1970-01-01T00:00:00Z"`) via chrono's
serde impl, and is documented as `string` / `date-time` in OpenAPI.

### Partial updates

`UpdateUserRequest` and `UpdateUserCredentialRequest` hold `Option<T>`
fields. Serde treats a missing field of type `Option<T>` as `None`
without needing `#[serde(default)]`, so both types already behave
correctly as PATCH-style bodies: `{"id": 1}` deserializes to an
`UpdateUserRequest` with every optional field `None`. utoipa marks the
same fields not-required. No extra attributes needed; a test pins it.

### Unknown fields

Request DTOs do **not** get `#[serde(deny_unknown_fields)]`. The
workspace has precedent for it (`AccessClaims` / `RefreshClaims` in
`auth::usecase::auth_usecase`), but those are internal JWT payloads
where strictness is a security property. For a public HTTP API,
tolerating unknown fields keeps older servers compatible with newer
clients. Flagged here as a deliberate default rather than an oversight.

## Security documentation

Because decisions 3 and 4 leave the credential types both serializable
and `Debug`-printable with their hash intact, documentation is the only
guard. Three additions:

1. A `# Security` doc section on `UserCredentialView`,
   `CreateUserCredentialRequest`, and `UpdateUserCredentialRequest`
   stating that these are admin-plane types, that `passwordHash` must
   never be serialized to a client, and that `Debug` prints the hash
   unredacted.
2. A warning in the `apis::auth` module header naming the three types.
3. A note in the crate-root docs (`lib.rs`) that DTOs are serializable
   by default and that not every DTO is safe to route.

## Tests

Per `docs/guidelines/lib-crate-development.md` §7, `tests/`:

- **`tests/public_api.rs`** — existing, unchanged. Compile-only surface
  lock; still passes with `--no-default-features`.
- **`tests/wire_format.rs`** — new, always compiled. The real contract
  lock. For each DTO: serialize a fully-populated value and assert the
  exact JSON object; assert camelCase keys are present *and* their
  snake_case spellings are absent; assert `Role::General` → `"general"`
  for all three variants; assert `DateTime<Utc>` → RFC 3339; assert a
  deserialize → serialize round-trip is identity. Plus the partial-update
  case: `{"id": 1}` → all-`None` `UpdateUserRequest`. A stray
  `rename_all` deletion fails loudly here.
- **`tests/openapi_schema.rs`** — new, `#![cfg(feature = "openapi")]`.
  Generate each schema via `ToSchema` and assert: property names are
  camelCase; `Role`'s enum values are the three lowercase strings;
  `createdAt` is `string` with format `date-time` (this is the test that
  catches a missing `utoipa/chrono` feature); both error enums produce a
  schema.

### Known risk

`UserApiError` and `AuthApiError` mix unit variants (`NotFound`) with
single-unnamed-field variants (`Validation(String)`). utoipa 5 supports
mixed enums and ignores the `#[error(...)]` attributes it does not
recognise, but this is the one derive in the change that could fail to
compile. `tests/openapi_schema.rs` surfaces it immediately. Fallback if
it does not compile: annotate the enums with
`#[cfg_attr(feature = "openapi", schema(value_type = String))]` and
document them as their `Display` output.

## Verification gate

```bash
cargo fmt --all -- --check
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo test  -p apis                  # serde always on; openapi off
cargo test  -p apis --all-features   # + openapi
cargo check -p user -p auth          # apis dep graph changed; prove both still build
cargo doc   -p apis --no-deps --all-features
```

`cargo test -p apis --no-default-features` is equivalent to the default
run (`default = []`) and is not listed separately.

## README

`apis` has no `README.md`; guideline §9 requires one per library crate.
Add one covering: one-sentence purpose, the `src/` layout, the `openapi`
feature and what it adds, the camelCase / lowercase-`Role` wire rules,
the credential-type security warning, how to run the tests, and a
back-link to the guideline.

## Out of scope

- Any change to `apps/server/aegis-server`. No axum router, no
  `IntoResponse`, no `#[utoipa::path]`, no Swagger UI, no
  `OpenApiRouter`, no `#[derive(OpenApi)]` document root.
- Any change to `UserService` / `AuthService` method signatures.
- Error-to-status-code mapping and the error response body shape.
- The `password` gap: `apis::user::CreateUserRequest` has no password
  field, so an HTTP "create user" endpoint cannot set one through this
  port today. Real, but a trait-surface question, not a wire-format one.
