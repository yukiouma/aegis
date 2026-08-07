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