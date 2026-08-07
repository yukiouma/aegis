# aegis-server

The HTTP entry point of Project Aegis. Wires the [`auth`][auth] crate's
`AuthServiceImpl` against a Postgres pool and an in-memory
token-version cache, mounts the auth-flow endpoints under
`/api/auth/*` with `axum`, and exposes the OpenAPI document at
`/api-docs/openapi.json` plus swagger-ui at `/swagger-ui`.

The binary is intentionally thin (parse env, run tracing, call
`aegis_server::run(config)`); all real work lives in the library
so it can be exercised by integration tests in this crate.

## Layout

```
src/
  config.rs        env-var wiring (Config::from_env)
  run.rs           bootstrap: pool, repos, AuthUsecase, listener, shutdown
  main.rs          binary entry point (calls aegis_server::run)
  state.rs         AppState (Arc<dyn AuthService>, Arc<dyn UserService>)
  transport/
    http/
      router.rs    top-level Router / route table
      healthz.rs   /healthz
      openapi.rs   utoipa ApiDoc + openapi() builder
      auth/
        handlers.rs    POST /api/auth/{login,login-domain,refresh,logout}
        middleware.rs  AuthClaims extractor (Bearer -> claims)
      dto.rs       wire-level DTOs (serde + utoipa)
      error.rs     ErrorBody + ApiError + IntoResponse
tests/
  integration_auth.rs  live-DB end-to-end test (#[ignore]-d)
```

## Routes

| Method | Path                          | Description                          |
| ------ | ----------------------------- | ------------------------------------ |
| POST   | `/api/auth/login`             | exchange `(code, password)` for tokens |
| POST   | `/api/auth/login-domain`      | exchange domain identity for tokens  |
| POST   | `/api/auth/refresh`           | exchange refresh token for access    |
| POST   | `/api/auth/logout`            | invalidate a session                 |
| GET    | `/healthz`                    | liveness probe                       |
| GET    | `/swagger-ui/`                | swagger-ui HTML                      |
| GET    | `/api-docs/openapi.json`      | OpenAPI v3 document                  |

All `/api/*` routes emit a tracing span via `tower-http`'s
`TraceLayer`. Auth is required for any future protected route:
handlers take an `AuthClaims` extractor that pulls
`Authorization: Bearer <token>` and verifies it before the handler
body runs.

## Configuration

Every setting is read from the environment at startup. `main.rs`
calls `dotenvy::dotenv()` first so a `.env` file in the working
directory is honoured.

| Variable                  | Required | Default          | Notes                          |
| ------------------------- | -------- | ---------------- | ------------------------------ |
| `AEGIS_DATABASE_URL`      | yes      | -                | Postgres connection string     |
| `AEGIS_AUTH_SIGNING_KEY`  | yes      | -                | hex-encoded HS256 key, ≥ 32 B   |
| `AEGIS_HTTP_BIND`         | no       | `0.0.0.0:8080`   | listen address                 |
| `AEGIS_ACCESS_TTL_SECS`   | no       | `900`            | access-token lifetime          |
| `AEGIS_REFRESH_TTL_SECS`  | no       | `604800`         | refresh-token lifetime (7d)    |
| `RUST_LOG`                | no       | `aegis_server=info,axum=info,sqlx=warn,tower_http=info` | tracing filter |

`AEGIS_AUTH_SIGNING_KEY` must be 32 bytes (or more) of hex. The
server does not log it. Rotate by deploying the new value and
restarting — every outstanding JWT becomes invalid at the moment
the new key is loaded.

## Running

```bash
# from the workspace root
export AEGIS_DATABASE_URL=postgres://user:pass@localhost/aegis
export AEGIS_AUTH_SIGNING_KEY=$(openssl rand -hex 32)
cargo run -p aegis-server
```

The migrations for both the `users` and `auth_user_credentials`
tables must already be applied; the server does not auto-migrate.

## Verification gate

Run the full test suite before opening a PR:

```bash
# 1. Compile + unit tests + doc tests (runs by default).
cargo test -p aegis-server

# 2. Static analysis (clippy, no warnings allowed).
cargo clippy -p aegis-server --all-targets -- -D warnings

# 3. Live-DB integration test (requires a real Postgres).
export AEGIS_DATABASE_URL=postgres://user:pass@localhost/aegis_test
cargo test -p aegis-server --test integration_auth -- --ignored
```

The live-DB test creates a per-run user row, runs the full
login → refresh → logout → wrong-password flow, and cleans up. If
`AEGIS_DATABASE_URL` is unset, the test skips itself.

## Errors

Every error is rendered as a typed `ErrorBody`:

```json
{ "code": "invalid_credentials", "message": "invalid credentials" }
```

| `code`                       | HTTP | Cause                                |
| ---------------------------- | ---- | ------------------------------------ |
| `validation_failed`          | 400  | request body failed validation       |
| `not_found`                  | 404  | user / domain identity not found     |
| `user_inactive`              | 403  | user exists but `active = false`     |
| `invalid_credentials`        | 401  | wrong password                       |
| `token_verification_failed`  | 401  | bad / expired / stale-version token  |
| `duplicate_code`             | 409  | credential row already exists        |
| `signing_failed`             | 500  | JWT mint failure (call site bug)     |
| `repository_error`           | 500  | wrapping any other backend error     |

[auth]: ../../../lib/crates/auth
