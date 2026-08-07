# `apis` HTTP Wire Types Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make every DTO in `lib/crates/apis` serialize as a camelCase JSON wire type and, behind an optional `openapi` feature, produce a utoipa schema — so a future axum router can use them as request/response bodies without redefining them.

**Architecture:** Purely additive derive attributes plus a manifest change. `serde` becomes an unconditional dependency (every DTO in this crate is a wire type); `utoipa` is optional behind `openapi = ["dep:utoipa"]`. No trait signature changes, and `apis` gains **no** axum dependency — status mapping, `IntoResponse`, `#[utoipa::path]`, and routers stay in the server.

**Tech Stack:** Rust 2024 edition (rustc 1.97.1), `serde` 1.0.229 (`derive`), `chrono` 0.4 (`serde`), `utoipa` 5.5.0 (`chrono`), `serde_json` 1 (dev only).

**Spec:** `docs/superpowers/specs/2026-08-07-apis-http-wire-types-design.md`

## Global Constraints

- **`apis` must never depend on `axum`.** Verified safe: `utoipa`'s `axum_extras` feature (inherited from the workspace pin) expands to `["regex", "syn/extra-traits"]` in `utoipa-gen` and declares no axum dependency.
- **Struct fields use `#[serde(rename_all = "camelCase")]`.** `Role` uses `#[serde(rename_all = "lowercase")]` — variant casing is a separate decision from field casing.
- **`Role` must serialize as `"root"` / `"admin"` / `"general"`** to match `Role::as_str()` in `auth::domain` / `user::domain` and the Postgres CHECK constraint.
- **Error enums (`UserApiError`, `AuthApiError`) get `ToSchema` only — never `Serialize` / `Deserialize`.** `apis` decides nothing about HTTP status codes or the error response body.
- **`derive(Debug)` is retained on the three `password_hash`-bearing types.** Explicitly declined redaction; doc-comments are the only guard.
- **Request DTOs do NOT get `#[serde(deny_unknown_fields)]`** — forward compatibility over strictness for a public HTTP API.
- **Derive paths are fully qualified** (`serde::Serialize`, `utoipa::ToSchema`) so no `use` statements are added to either module.
- **No trait signature changes.** `UserService` and `AuthService` are byte-for-byte unchanged.
- **`default = []`.** Only `openapi` is optional; serde is always on.
- Follow `docs/guidelines/lib-crate-development.md`: inherit deps via `{ workspace = true }`, one-line comment on every non-obvious dep, no `mod.rs`.

---

## File Structure

| File | Responsibility | Task |
|---|---|---|
| `Cargo.toml` (root) | Add `serde_json` to `[workspace.dependencies]` | 1 |
| `lib/crates/apis/Cargo.toml` | serde/chrono-serde deps, optional utoipa, `openapi` feature | 1 |
| `lib/crates/apis/src/user.rs` | Derives on `Role`, `UserView`, `CreateUserRequest`, `UpdateUserRequest`, `UserApiError` | 1, 3 |
| `lib/crates/apis/src/auth.rs` | Derives on 13 DTOs + `AuthApiError`; `# Security` docs | 2, 3 |
| `lib/crates/apis/src/lib.rs` | Crate-root note that DTOs are serializable and not all are safe to route | 4 |
| `lib/crates/apis/tests/wire_format.rs` | **NEW** — the wire-contract lock (exact JSON per DTO) | 1, 2 |
| `lib/crates/apis/tests/openapi_schema.rs` | **NEW** — `openapi`-gated schema assertions | 3 |
| `lib/crates/apis/README.md` | **NEW** — guideline §9 requires one per lib crate | 4 |
| `lib/crates/apis/tests/public_api.rs` | **UNCHANGED** — existing compile-only surface lock | — |

---

### Task 1: Manifest + `apis::user` wire types

The manifest is folded in here because no wire-format test can compile without `serde` and `serde_json` present.

**Files:**
- Modify: `Cargo.toml` (root, after line 13)
- Modify: `lib/crates/apis/Cargo.toml` (whole file)
- Modify: `lib/crates/apis/src/user.rs:18-23` (`Role`), `:52-61` (`UserView`), `:69-73` (`CreateUserRequest`), `:80-87` (`UpdateUserRequest`)
- Create: `lib/crates/apis/tests/wire_format.rs`

**Interfaces:**
- Consumes: nothing (first task).
- Produces: the `openapi` cargo feature (declared but unused until Task 3); `assert_round_trip::<T>(serde_json::Value)` test helper in `tests/wire_format.rs`, reused by Task 2; the camelCase/lowercase derive pattern that Tasks 2 and 3 copy verbatim.

- [ ] **Step 1: Add `serde_json` to the workspace dependencies**

In the root `Cargo.toml`, insert immediately after the `serde` line (line 13):

```toml
serde_json = "1"
```

The `[workspace.dependencies]` block is not alphabetized; placing it next to `serde` keeps the two together.

- [ ] **Step 2: Rewrite `lib/crates/apis/Cargo.toml`**

Replace the entire file with:

```toml
[package]
name = "apis"
version = "0.1.0"
edition = "2024"

[dependencies]
async-trait = { workspace = true }
# `chrono` provides the `DateTime<Utc>` carried by `UserView`. The
# `serde` feature is added on top of the workspace pin (which sets
# `default-features = false`) so timestamps round-trip as RFC 3339.
chrono = { workspace = true, features = ["serde"] }
# `serde` is unconditional: every DTO in this crate is a wire type.
# Gating it behind a feature would allow an `openapi`-only build to
# derive `ToSchema` without the `#[serde(rename_all)]` attributes in
# scope, emitting a schema that disagrees with the wire.
serde = { workspace = true }
thiserror = { workspace = true }
# `utoipa` is optional because only an OpenAPI-producing consumer (the
# axum server) needs it. `features = ["chrono"]` is a utoipa-gen
# codegen flag — it declares no dependency of its own — and is required
# for `DateTime<Utc>` to render as `string` / `date-time`.
utoipa = { workspace = true, optional = true, features = ["chrono"] }

[dev-dependencies]
# Used by `tests/wire_format.rs` and `tests/openapi_schema.rs` to assert
# the exact JSON shape of every DTO and every generated schema.
serde_json = { workspace = true }

[features]
default = []
openapi = ["dep:utoipa"]
```

- [ ] **Step 3: Populate the lockfile and confirm no axum leaked in**

Run:

```bash
cargo check -p apis --all-features 2>&1 | tail -5
cargo tree -p apis --all-features -e normal | grep -c axum
```

Expected: the check succeeds, and `grep -c axum` prints `0`. If it prints anything else, **stop** — the Global Constraint is violated and the `utoipa` feature set needs revisiting before continuing.

- [ ] **Step 4: Write the failing wire-format test for `apis::user`**

Create `lib/crates/apis/tests/wire_format.rs`:

```rust
//! Wire-contract lock for every DTO in `apis`.
//!
//! Asserts the exact JSON each type produces: camelCase keys, absent
//! snake_case spellings, lowercase `Role`, RFC 3339 timestamps, and
//! JSON -> T -> JSON round-trip stability. Deleting a
//! `#[serde(rename_all)]` attribute fails here rather than surfacing as
//! a broken client at integration time.

use apis::user::{CreateUserRequest, Role, UpdateUserRequest, UserView};
use serde_json::json;

/// Parse an RFC 3339 literal into the `DateTime<Utc>` the DTOs carry.
fn ts(s: &str) -> chrono::DateTime<chrono::Utc> {
    s.parse().expect("test timestamp must be valid RFC 3339")
}

/// JSON -> T -> JSON must be the identity.
///
/// The comparison is on the JSON rather than on the value because
/// several DTOs deliberately do not derive `PartialEq`; this keeps one
/// round-trip helper usable for all of them.
fn assert_round_trip<T>(value: serde_json::Value)
where
    T: serde::Serialize + serde::de::DeserializeOwned,
{
    let parsed: T = serde_json::from_value(value.clone()).unwrap_or_else(|e| {
        panic!("deserialize failed for {}: {e}", std::any::type_name::<T>())
    });
    assert_eq!(
        serde_json::to_value(&parsed).unwrap(),
        value,
        "round-trip changed the JSON for {}",
        std::any::type_name::<T>()
    );
}

fn sample_user_view() -> UserView {
    UserView {
        id: 1,
        code: "u1".into(),
        name: "Alice".into(),
        role: Role::General,
        active: true,
        created_at: ts("2026-08-07T12:00:00Z"),
        updated_at: ts("2026-08-07T12:30:00Z"),
    }
}

fn sample_user_view_json() -> serde_json::Value {
    json!({
        "id": 1,
        "code": "u1",
        "name": "Alice",
        "role": "general",
        "active": true,
        "createdAt": "2026-08-07T12:00:00Z",
        "updatedAt": "2026-08-07T12:30:00Z"
    })
}

#[test]
fn user_view_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(sample_user_view()).unwrap(),
        sample_user_view_json()
    );
}

#[test]
fn user_view_omits_snake_case_keys() {
    let s = serde_json::to_string(&sample_user_view()).unwrap();
    for leaked in ["created_at", "updated_at"] {
        assert!(!s.contains(leaked), "snake_case key `{leaked}` leaked: {s}");
    }
}

#[test]
fn user_view_round_trips() {
    assert_round_trip::<UserView>(sample_user_view_json());
}

#[test]
fn role_serializes_lowercase() {
    assert_eq!(serde_json::to_value(Role::Root).unwrap(), json!("root"));
    assert_eq!(serde_json::to_value(Role::Admin).unwrap(), json!("admin"));
    assert_eq!(serde_json::to_value(Role::General).unwrap(), json!("general"));
}

#[test]
fn role_deserializes_lowercase() {
    assert_eq!(
        serde_json::from_value::<Role>(json!("root")).unwrap(),
        Role::Root
    );
    assert_eq!(
        serde_json::from_value::<Role>(json!("admin")).unwrap(),
        Role::Admin
    );
    assert_eq!(
        serde_json::from_value::<Role>(json!("general")).unwrap(),
        Role::General
    );
}

#[test]
fn role_rejects_rust_variant_spelling() {
    // Guards the `rename_all = "lowercase"` attribute: without it the
    // wire would speak "General" and disagree with `Role::as_str()`
    // and the Postgres CHECK constraint.
    assert!(serde_json::from_value::<Role>(json!("General")).is_err());
}

#[test]
fn create_user_request_round_trips() {
    assert_round_trip::<CreateUserRequest>(json!({
        "code": "u1",
        "name": "Alice",
        "role": "general"
    }));
}

#[test]
fn update_user_request_round_trips() {
    assert_round_trip::<UpdateUserRequest>(json!({
        "id": 1,
        "code": "u2",
        "name": "Bob",
        "role": "admin",
        "active": false
    }));
}

#[test]
fn update_user_request_treats_missing_optionals_as_none() {
    // Serde defaults a missing `Option<T>` field to `None` without
    // `#[serde(default)]`, so this type already behaves as a PATCH body.
    let req: UpdateUserRequest = serde_json::from_value(json!({ "id": 1 })).unwrap();
    assert_eq!(req.id, 1);
    assert!(req.code.is_none());
    assert!(req.name.is_none());
    assert!(req.role.is_none());
    assert!(req.active.is_none());
}
```

- [ ] **Step 5: Run the test to verify it fails**

Run: `cargo test -p apis --test wire_format 2>&1 | tail -20`

Expected: FAIL at compile time with errors like ``the trait bound `UserView: Serialize` is not satisfied`` / ``the trait bound `Role: Deserialize<'_>` is not satisfied``. This confirms the test is actually exercising the derives.

- [ ] **Step 6: Add the derives to `apis::user`**

In `lib/crates/apis/src/user.rs`, add attributes to four types. Leave every field, doc-comment, and the `UserService` trait untouched.

`Role` (currently line 18) — replace the single `#[derive(...)]` line with:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
```

`UserView` (currently line 52):

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserView {
```

`CreateUserRequest` (currently line 69) has **no** derive line today — add both lines above `pub struct`:

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
```

`UpdateUserRequest` (currently line 80) — replace `#[derive(Default)]` with:

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
```

`UserApiError` is **not** touched in this task — it gets `ToSchema` only, in Task 3.

- [ ] **Step 7: Run the tests to verify they pass**

Run: `cargo test -p apis 2>&1 | tail -20`

Expected: PASS — 10 tests in `wire_format`, plus the 4 existing `public_api` tests, all green.

- [ ] **Step 8: Verify `user` and `auth` still build**

Run: `cargo check -p user -p auth 2>&1 | tail -5`

Expected: success. `apis`'s dependency graph changed, and both crates depend on it; this proves the change is non-breaking for them.

- [ ] **Step 9: Commit**

```bash
git add Cargo.toml Cargo.lock lib/crates/apis/Cargo.toml \
        lib/crates/apis/src/user.rs lib/crates/apis/tests/wire_format.rs
git commit -m "feat(apis): make user DTOs camelCase JSON wire types

serde becomes an unconditional dependency and utoipa is declared
optional behind an \`openapi\` feature (unused until a later commit).
Role serializes lowercase to match Role::as_str() and the Postgres
CHECK constraint. CreateUserRequest / UpdateUserRequest gain Debug +
Clone so all DTOs share one baseline.

tests/wire_format.rs locks the exact JSON so a deleted rename_all
fails loudly."
```

---

### Task 2: `apis::auth` wire types + security documentation

**Files:**
- Modify: `lib/crates/apis/src/auth.rs` — 13 DTOs at lines `13`, `20`, `28`, `35`, `44`, `50`, `56`, `65`, `76`, `88`, `92`, `104`, `117`; plus the module header at `:1-6`
- Modify: `lib/crates/apis/tests/wire_format.rs` (append)

**Interfaces:**
- Consumes: `assert_round_trip::<T>` and the derive pattern from Task 1.
- Produces: all 13 `apis::auth` DTOs serializable with the camelCase contract; `# Security` doc sections on the three `password_hash`-bearing types.

- [ ] **Step 1: Write the failing wire-format tests for `apis::auth`**

Append to `lib/crates/apis/tests/wire_format.rs`:

```rust
// -- apis::auth ---------------------------------------------------------

use apis::auth::{
    AuthClaims, CreateUserCredentialRequest, LoginWithDomainUserInfoRequest,
    LoginWithPasswordRequest, LogoutRequest, LogoutResponse, RefreshRequest, RefreshResponse,
    RemoveUserCredentialResponse, TokenPair, UpdateUserCredentialRequest, UserCredentialView,
    VerifyRequest,
};

#[test]
fn token_pair_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(TokenPair {
            access_token: "a".into(),
            refresh_token: "r".into(),
        })
        .unwrap(),
        json!({ "accessToken": "a", "refreshToken": "r" })
    );
}

#[test]
fn auth_claims_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(AuthClaims {
            code: "u1".into(),
            role: Role::Admin,
            token_version: 3,
        })
        .unwrap(),
        json!({ "code": "u1", "role": "admin", "tokenVersion": 3 })
    );
}

#[test]
fn user_credential_view_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(UserCredentialView {
            user_code: "u1".into(),
            password_hash: "h".into(),
            token_version: 0,
        })
        .unwrap(),
        json!({ "userCode": "u1", "passwordHash": "h", "tokenVersion": 0 })
    );
}

#[test]
fn login_with_domain_user_info_serializes_to_camel_case() {
    assert_eq!(
        serde_json::to_value(LoginWithDomainUserInfoRequest {
            code: "u1".into(),
            domain_name: "d".into(),
            hostname: "h".into(),
            sid: "s".into(),
        })
        .unwrap(),
        json!({ "code": "u1", "domainName": "d", "hostname": "h", "sid": "s" })
    );
}

#[test]
fn auth_dtos_omit_snake_case_keys() {
    let payloads = [
        serde_json::to_string(&TokenPair {
            access_token: "a".into(),
            refresh_token: "r".into(),
        })
        .unwrap(),
        serde_json::to_string(&AuthClaims {
            code: "u1".into(),
            role: Role::Admin,
            token_version: 3,
        })
        .unwrap(),
        serde_json::to_string(&UserCredentialView {
            user_code: "u1".into(),
            password_hash: "h".into(),
            token_version: 0,
        })
        .unwrap(),
        serde_json::to_string(&LoginWithDomainUserInfoRequest {
            code: "u1".into(),
            domain_name: "d".into(),
            hostname: "h".into(),
            sid: "s".into(),
        })
        .unwrap(),
    ];
    for s in payloads {
        for leaked in [
            "access_token",
            "refresh_token",
            "token_version",
            "user_code",
            "password_hash",
            "domain_name",
        ] {
            assert!(!s.contains(leaked), "snake_case key `{leaked}` leaked: {s}");
        }
    }
}

#[test]
fn auth_request_dtos_round_trip() {
    assert_round_trip::<LoginWithPasswordRequest>(json!({ "code": "u1", "password": "p" }));
    assert_round_trip::<LoginWithDomainUserInfoRequest>(json!({
        "code": "u1", "domainName": "d", "hostname": "h", "sid": "s"
    }));
    assert_round_trip::<LogoutRequest>(json!({ "refreshToken": "r" }));
    assert_round_trip::<VerifyRequest>(json!({ "accessToken": "a" }));
    assert_round_trip::<RefreshRequest>(json!({ "refreshToken": "r" }));
    assert_round_trip::<CreateUserCredentialRequest>(json!({
        "userCode": "u1", "passwordHash": "h"
    }));
    assert_round_trip::<UpdateUserCredentialRequest>(json!({
        "userCode": "u1", "passwordHash": "h"
    }));
}

#[test]
fn auth_response_dtos_round_trip() {
    assert_round_trip::<TokenPair>(json!({ "accessToken": "a", "refreshToken": "r" }));
    assert_round_trip::<AuthClaims>(json!({
        "code": "u1", "role": "general", "tokenVersion": 0
    }));
    assert_round_trip::<RefreshResponse>(json!({ "accessToken": "a" }));
    assert_round_trip::<UserCredentialView>(json!({
        "userCode": "u1", "passwordHash": "h", "tokenVersion": 0
    }));
}

#[test]
fn empty_response_dtos_serialize_to_empty_objects() {
    assert_eq!(serde_json::to_value(LogoutResponse {}).unwrap(), json!({}));
    assert_eq!(
        serde_json::to_value(RemoveUserCredentialResponse {}).unwrap(),
        json!({})
    );
}

#[test]
fn update_user_credential_request_treats_missing_hash_as_none() {
    let req: UpdateUserCredentialRequest =
        serde_json::from_value(json!({ "userCode": "u1" })).unwrap();
    assert_eq!(req.user_code, "u1");
    assert!(req.password_hash.is_none());
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p apis --test wire_format 2>&1 | tail -20`

Expected: FAIL at compile time with ``the trait bound `TokenPair: Serialize` is not satisfied`` and similar for the other auth types.

- [ ] **Step 3: Add the derives to all 13 `apis::auth` DTOs**

In `lib/crates/apis/src/auth.rs`, every DTO gets the same two-line treatment. Leave all fields and doc-comments intact. `AuthApiError` is **not** touched here.

Types that currently have `#[derive(Debug, Clone, PartialEq, Eq)]` — `TokenPair` (line 13), `AuthClaims` (line 20), `RefreshResponse` (line 92), `UserCredentialView` (line 104):

```rust
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
```

Types that currently have `#[derive(Debug, Clone)]` — `LoginWithPasswordRequest` (line 28), `LoginWithDomainUserInfoRequest` (line 35), `LogoutRequest` (line 44), `VerifyRequest` (line 50), `RefreshRequest` (line 56), `CreateUserCredentialRequest` (line 65):

```rust
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
```

Types that currently have `#[derive(Debug, Clone, Default)]` — `UpdateUserCredentialRequest` (line 76):

```rust
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
```

Types that currently have `#[derive(Debug, Clone, PartialEq, Eq, Default)]` — `LogoutResponse` (line 88), `RemoveUserCredentialResponse` (line 117):

```rust
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
```

`rename_all` on the two field-less response types is a deliberate no-op, kept so a later field addition inherits the right convention automatically.

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p apis 2>&1 | tail -20`

Expected: PASS — all `wire_format` and `public_api` tests green.

- [ ] **Step 5: Add the `# Security` documentation**

Uniform derives leave three types both serializable and `Debug`-printable with an Argon2 hash intact. Documentation is the only guard, so it must be explicit.

Append this `# Security` section to the existing doc-comment on **each** of `UserCredentialView` (line 104), `CreateUserCredentialRequest` (line 65), and `UpdateUserCredentialRequest` (line 76), immediately above its `#[derive(...)]` block:

```rust
/// # Security
///
/// Admin-plane type. `passwordHash` is a credential secret: never
/// return this type from a client-facing HTTP handler, and never log
/// it. `Debug` is derived and prints the hash **unredacted** — prefer
/// naming individual safe fields over `{:?}` on the whole value.
```

Then replace the module header at `lib/crates/apis/src/auth.rs:1-6` with:

```rust
//! Outbound port for authentication.
//!
//! See [`AuthService`] for the trait surface. All supporting types
//! (`TokenPair`, `AuthClaims`, the request / view / response DTOs,
//! and `AuthApiError`) are defined alongside the trait so a single
//! `use apis::auth::*;` brings the whole contract into scope.
//!
//! # Security
//!
//! Every DTO here derives `Serialize` / `Deserialize`, but not every
//! DTO is safe to route. [`UserCredentialView`],
//! [`CreateUserCredentialRequest`], and [`UpdateUserCredentialRequest`]
//! carry a `password_hash` and are admin-plane only; serializing one
//! to a client leaks a credential secret.
```

- [ ] **Step 6: Verify the docs build and render the links**

Run: `cargo doc -p apis --no-deps 2>&1 | tail -10`

Expected: success with no broken-intra-doc-link warnings.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis/src/auth.rs lib/crates/apis/tests/wire_format.rs
git commit -m "feat(apis): make auth DTOs camelCase JSON wire types

All 13 apis::auth DTOs derive Serialize/Deserialize with the
camelCase contract; AuthApiError is deliberately excluded.

Uniform derives leave UserCredentialView and the two credential
request DTOs serializable with a live password hash, and Debug is
retained on them by design, so both carry an explicit # Security
section and the module header names all three."
```

---

### Task 3: `openapi` feature — `ToSchema` derives

**Files:**
- Modify: `lib/crates/apis/src/user.rs` (5 types: 4 DTOs + `UserApiError` at line 31)
- Modify: `lib/crates/apis/src/auth.rs` (14 types: 13 DTOs + `AuthApiError` at line 126)
- Create: `lib/crates/apis/tests/openapi_schema.rs`

**Interfaces:**
- Consumes: the `openapi` feature declared in Task 1; the derive blocks from Tasks 1 and 2 (the `#[cfg_attr]` line goes directly beneath each `#[serde(rename_all = ...)]`).
- Produces: `utoipa::ToSchema` on all 17 DTOs and both error enums. Schemas are reachable via `utoipa::PartialSchema::schema()`.

- [ ] **Step 1: Write the failing schema test**

Create `lib/crates/apis/tests/openapi_schema.rs`:

```rust
//! OpenAPI schema lock for the `openapi` feature.
//!
//! Assertions are made against the serialized schema string rather than
//! a hand-built nested structure, so they survive utoipa's internal
//! layout changes while still pinning the two things that matter: the
//! property names, and that `chrono` timestamps are documented as
//! `date-time`.
#![cfg(feature = "openapi")]

use apis::auth::{AuthApiError, AuthClaims, TokenPair, UserCredentialView};
use apis::user::{Role, UserApiError, UserView};
use utoipa::PartialSchema;

/// Serialize a type's schema to a JSON string for substring assertions.
fn schema_of<T: PartialSchema>() -> String {
    serde_json::to_string(&T::schema()).expect("schema must serialize")
}

#[test]
fn user_view_schema_uses_camel_case_properties() {
    let s = schema_of::<UserView>();
    for expected in ["createdAt", "updatedAt"] {
        assert!(s.contains(expected), "schema missing `{expected}`: {s}");
    }
    for leaked in ["created_at", "updated_at"] {
        assert!(!s.contains(leaked), "schema leaked `{leaked}`: {s}");
    }
}

#[test]
fn user_view_schema_documents_timestamps_as_date_time() {
    // This is the test that catches a missing `utoipa/chrono` feature:
    // without it `DateTime<Utc>` renders as an empty object.
    let s = schema_of::<UserView>();
    assert!(
        s.contains("date-time"),
        "DateTime<Utc> not documented as date-time — is the `chrono` \
         feature missing from the utoipa dependency? {s}"
    );
}

#[test]
fn auth_schemas_use_camel_case_properties() {
    let token_pair = schema_of::<TokenPair>();
    assert!(token_pair.contains("accessToken"), "{token_pair}");
    assert!(token_pair.contains("refreshToken"), "{token_pair}");
    assert!(!token_pair.contains("access_token"), "{token_pair}");

    let claims = schema_of::<AuthClaims>();
    assert!(claims.contains("tokenVersion"), "{claims}");
    assert!(!claims.contains("token_version"), "{claims}");

    let cred = schema_of::<UserCredentialView>();
    assert!(cred.contains("userCode"), "{cred}");
    assert!(cred.contains("passwordHash"), "{cred}");
    assert!(!cred.contains("user_code"), "{cred}");
}

#[test]
fn role_schema_lists_lowercase_values() {
    let s = schema_of::<Role>();
    for expected in ["\"root\"", "\"admin\"", "\"general\""] {
        assert!(s.contains(expected), "schema missing {expected}: {s}");
    }
    assert!(!s.contains("\"Root\""), "schema leaked Rust spelling: {s}");
}

#[test]
fn error_enums_produce_schemas() {
    // The primary value of this test is that the derives compile; the
    // assertions guard against an empty or degenerate schema.
    let user = schema_of::<UserApiError>();
    assert!(user.contains("NotFound"), "{user}");

    let auth = schema_of::<AuthApiError>();
    assert!(auth.contains("InvalidCredentials"), "{auth}");
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p apis --all-features --test openapi_schema 2>&1 | tail -20`

Expected: FAIL at compile time with ``no function or associated item named `schema` found`` / ``the trait bound `UserView: PartialSchema` is not satisfied``.

Note: without `--all-features` the file compiles to zero tests because of the `#![cfg(feature = "openapi")]` guard — that is correct behavior, not a passing test.

- [ ] **Step 3: Add `ToSchema` to `apis::user`**

In `lib/crates/apis/src/user.rs`, add this line directly beneath the `#[serde(rename_all = ...)]` line of each of `Role`, `UserView`, `CreateUserRequest`, `UpdateUserRequest`:

```rust
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
```

`UserApiError` (line 31) has no serde attributes and gets `ToSchema` alone — it must never be `Serialize`. Its derive block becomes:

```rust
#[derive(Debug, Error)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum UserApiError {
```

- [ ] **Step 4: Add `ToSchema` to `apis::auth`**

In `lib/crates/apis/src/auth.rs`, add the same line beneath the `#[serde(rename_all = "camelCase")]` of all 13 DTOs:

```rust
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
```

And on `AuthApiError` (line 126), which likewise gets `ToSchema` alone:

```rust
#[derive(Debug, Error)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub enum AuthApiError {
```

- [ ] **Step 5: Run the tests to verify they pass**

Run: `cargo test -p apis --all-features 2>&1 | tail -20`

Expected: PASS — `openapi_schema` (6 tests), `wire_format`, and `public_api` all green.

**If the error-enum derives fail to compile:** `UserApiError` and `AuthApiError` mix unit variants (`NotFound`) with single-unnamed-field variants (`Validation(String)`). This is the one derive in the change with real risk. The spec's documented fallback is to treat them as opaque strings — add to each enum:

```rust
#[derive(Debug, Error)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
#[cfg_attr(feature = "openapi", schema(value_type = String))]
pub enum UserApiError {
```

then change `error_enums_produce_schemas` to assert `s.contains("string")` instead of the variant names, and note the fallback in the commit message.

- [ ] **Step 6: Confirm the feature is genuinely optional**

Run:

```bash
cargo test -p apis 2>&1 | tail -5
cargo tree -p apis -e normal | grep -c utoipa
```

Expected: the default-feature tests pass, and `grep -c utoipa` prints `0` — proving `utoipa` is absent from the default build.

- [ ] **Step 7: Commit**

```bash
git add lib/crates/apis/src/user.rs lib/crates/apis/src/auth.rs \
        lib/crates/apis/tests/openapi_schema.rs
git commit -m "feat(apis): add optional openapi feature with ToSchema derives

All 17 DTOs plus UserApiError and AuthApiError derive utoipa::ToSchema
behind the openapi feature. The error enums get ToSchema only — apis
decides nothing about status codes or the error body shape.

tests/openapi_schema.rs pins camelCase property names, lowercase Role
values, and that DateTime<Utc> is documented as date-time, which is
what catches a missing utoipa \`chrono\` feature."
```

---

### Task 4: README, crate docs, and the full verification gate

**Files:**
- Create: `lib/crates/apis/README.md`
- Modify: `lib/crates/apis/src/lib.rs:1-10`

**Interfaces:**
- Consumes: the finished feature set and wire contract from Tasks 1–3.
- Produces: nothing consumed by later tasks — this is the closing task.

- [ ] **Step 1: Update the crate-root documentation**

Replace the contents of `lib/crates/apis/src/lib.rs` with:

```rust
//! `apis` workspace crate.
//!
//! Hosts outbound port traits that adapters (HTTP/gRPC handlers,
//! other backends) consume. Each trait is a self-contained
//! contract: this crate does not depend on any other workspace
//! crate, so any backend can implement the traits by adapting its
//! own types to the ones defined here.
//!
//! # Wire format
//!
//! Every DTO derives `serde::Serialize` + `serde::Deserialize` and
//! serializes with camelCase field names, so the types can be used
//! directly as HTTP request / response bodies. [`user::Role`]
//! serializes lowercase (`"root"` / `"admin"` / `"general"`) to match
//! `Role::as_str()` in the `auth` and `user` crates and the Postgres
//! CHECK constraint.
//!
//! Enabling the `openapi` feature additionally derives
//! `utoipa::ToSchema` on every DTO and on both error enums. The error
//! enums get `ToSchema` only: this crate deliberately decides nothing
//! about HTTP status codes, the error response body, or routing, and
//! never depends on a web framework.
//!
//! # Security
//!
//! DTOs are serializable by default, but not every DTO is safe to
//! route. See the `# Security` note on [`auth`] for the three
//! credential types that carry a password hash.

pub mod auth;
pub mod user;
```

- [ ] **Step 2: Create the crate README**

Create `lib/crates/apis/README.md`:

````markdown
# `apis`

Outbound port traits — `user::UserService` and `auth::AuthService` —
plus the DTOs that cross them. Backends implement these traits by
adapting their own types; `apis` depends on no other workspace crate.

## Layout

```
src/
├── lib.rs    crate docs + module declarations
├── auth.rs   AuthService, 13 DTOs, AuthApiError
└── user.rs   UserService, Role, 3 DTOs, UserApiError
```

## Features

| Feature   | Default | Adds                                                     |
| --------- | ------- | -------------------------------------------------------- |
| `openapi` | no      | `utoipa::ToSchema` on every DTO and both error enums      |

`serde` is **not** a feature — it is unconditional, because every DTO
here is a wire type. Gating it would allow an `openapi`-only build to
generate a schema without the `#[serde(rename_all)]` attributes in
scope, producing a document that disagrees with the wire.

This crate never depends on `axum`. Status-code mapping,
`IntoResponse`, `#[utoipa::path]`, and routers belong to the server.

## Wire contract

- Struct fields are **camelCase**: `userCode`, `passwordHash`,
  `tokenVersion`, `accessToken`, `refreshToken`, `domainName`,
  `createdAt`, `updatedAt`.
- `Role` is **lowercase**: `"root"`, `"admin"`, `"general"` — matching
  `Role::as_str()` in the `auth` / `user` crates and the Postgres CHECK
  constraint.
- `DateTime<Utc>` is RFC 3339 (`"2026-08-07T12:00:00Z"`), documented in
  OpenAPI as `string` / `date-time`.
- Request DTOs do not set `deny_unknown_fields`; unknown fields are
  ignored so older servers stay compatible with newer clients.
- `Option<T>` fields may be omitted — serde defaults them to `None`, so
  `UpdateUserRequest` and `UpdateUserCredentialRequest` work as
  PATCH-style bodies.

## Security

`auth::UserCredentialView`, `auth::CreateUserCredentialRequest`, and
`auth::UpdateUserCredentialRequest` carry a `password_hash`. They are
admin-plane types: never return them from a client-facing handler.
`Debug` is derived and prints the hash **unredacted**, so avoid `{:?}`
on the whole value in log statements.

## Tests

```bash
cargo test -p apis                  # wire format + public API surface
cargo test -p apis --all-features   # + generated OpenAPI schemas
```

- `tests/public_api.rs` — compile-only lock on the trait surface,
  object-safety, and `Send + Sync` bounds.
- `tests/wire_format.rs` — the JSON contract: exact keys, lowercase
  roles, RFC 3339 timestamps, round-trip stability.
- `tests/openapi_schema.rs` — `openapi`-gated schema assertions.

No test touches a database or the network.

## See also

[`docs/guidelines/lib-crate-development.md`](../../../docs/guidelines/lib-crate-development.md)
for the cross-cutting workspace conventions this crate follows.
````

- [ ] **Step 3: Run the full verification gate**

Run each command; all must pass:

```bash
cargo fmt --all -- --check
cargo clippy -p apis --all-targets --all-features -- -D warnings
cargo test  -p apis
cargo test  -p apis --all-features
cargo check -p user -p auth
cargo doc   -p apis --no-deps --all-features
```

Expected: no output from `fmt`, no warnings from `clippy`, all tests green, both dependent crates build, docs build clean.

`cargo test -p apis --no-default-features` is equivalent to the plain run (`default = []`) and is intentionally not listed.

- [ ] **Step 4: Confirm the trait surface never moved**

Run: `git diff <base>..HEAD -- lib/crates/apis/src/ | grep -E '^[-+].*(async fn|pub trait)'`

Replace `<base>` with the commit before Task 1 (`git log --oneline` to find it).

Expected: **no output**. Every `async fn` and `pub trait` line is unchanged — this change is additive attributes only. Any output here means a trait signature moved and the Global Constraint was violated.

- [ ] **Step 5: Commit**

```bash
git add lib/crates/apis/README.md lib/crates/apis/src/lib.rs
git commit -m "docs(apis): add crate README and wire-format crate docs

Guideline §9 requires a README per library crate; apis had none.
Documents the openapi feature, the camelCase / lowercase-Role wire
rules, the credential-type security warning, and the test layout."
```

---

## Self-Review

**Spec coverage** — every section maps to a task:

| Spec section | Task |
|---|---|
| Manifest (deps, features, `serde_json`) | 1 |
| Decision 1 (serde+utoipa, no axum) | 1 (Global Constraints + Step 3 `cargo tree` check) |
| Decision 2 (serde unconditional) | 1 |
| Decision 3 (uniform derives incl. credentials) | 2 |
| Decision 4 (`derive(Debug)` retained) | 2 (Step 5 docs it explicitly) |
| Decision 5 (camelCase / lowercase `Role`) | 1, 2 |
| Decision 6 (errors: `ToSchema`, no serde) | 3 |
| Derive pattern + types affected (17 DTOs, 2 errors) | 1, 2, 3 |
| Incidental fixes (`Debug`/`Clone` on 2 DTOs) | 1 Step 6 |
| Wire contract (8 renames, RFC 3339) | 1, 2 |
| Partial updates (`Option<T>` → `None`) | 1 Step 4, 2 Step 1 |
| Unknown fields (no `deny_unknown_fields`) | Global Constraints; README |
| Security documentation (3 items) | 2 Step 5; 4 Step 1 |
| Tests (3 files) | 1, 2, 3 |
| Known risk (error-enum derive) | 3 Step 5 fallback |
| Verification gate | 4 Step 3 |
| README | 4 Step 2 |

**Placeholder scan:** no `TBD` / `TODO` / "similar to Task N" / "add appropriate error handling". Every code step carries complete literal code; every command has an expected result.

**Type consistency:** `assert_round_trip::<T>` is defined once (Task 1 Step 4) and called with the same signature in Task 2. `schema_of::<T>()` is local to `openapi_schema.rs`. `ts()` and `sample_user_view()` are defined and used only in `wire_format.rs`. Field names in every `json!` literal match the spec's rename table (`userCode`, `passwordHash`, `tokenVersion`, `accessToken`, `refreshToken`, `domainName`, `createdAt`, `updatedAt`). The `#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]` line is byte-identical everywhere it appears.
