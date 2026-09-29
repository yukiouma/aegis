# aegis-desktop Logging & Trace-Id Wiring — Design

**Date:** 2026-09-29
**Scope:** Add `tracing` to the Tauri backend of `aegis-desktop`, mint a per-install device prefix, and thread the trace id through every command shim into the outbound HTTP request as `X-Trace-ID`.

## Motivation

The desktop backend currently emits no observability at all — no `tracing` calls, no log files. When the user reports a failure (network blip, refresh loop, parse error) there is nothing to point at. The `trace-id` crate already exists (PR #86) but has no consumer; the desktop needs to be that consumer so every command → request carries an id that can be matched server-side.

The server already has the other half: `tracing-appender` writes JSON to `$AEGIS_LOG_DIR/aegis-server.log.YYYY-MM-DD`, every request is wrapped in a `TraceLayer` span. Sending `X-Trace-ID` from the desktop lets the server log lines and the desktop log lines join on a single id.

## Approach

- Initialise `tracing` once in `.setup` with a JSON daily-rotating file sink under `<app_data_dir>/logs` (overridable via `AEGIS_LOG_DIR`).
- Construct a single `TraceIdGenerator` from a persisted per-install nanoid; store it as managed state.
- Every `#[tauri::command]` mints a fresh trace id, opens a `tracing::info_span!` carrying the id, logs `enter` / `success` / `failed` events around the existing http fn call, and scopes the call into a `tokio::task_local!` so the outbound `reqwest` request picks the id up as the `X-Trace-ID` header.
- The `http/*` modules stay byte-identical. The only edit inside `http/` is `client.rs`'s `send` method: it reads the task-local and attaches the header.

## Architecture

```
src-tauri/src/
  lib.rs                    # + tracing init in .setup; + app.manage(TraceIdGenerator)
  tracing_init.rs           # new: init_tracing(&AppHandle) -> WorkerGuard
  trace_id_setup.rs         # new: load_or_create_device_prefix(&AppHandle) -> TraceIdGenerator
  commands.rs               # unchanged (barrel)
  commands/
    auth.rs                 # mint + span + scope + enter/success/failed logs
    healthz.rs              # same pattern
    identity.rs             # same pattern
    mission.rs              # same pattern
    project.rs              # same pattern
    user.rs                 # same pattern
    user_credential.rs      # same pattern
    crf/{form,item,option,unit,annotation,domain_annotation,version}.rs
                            # same pattern (one shim per file)
    domain_model/{domain,variable,version}.rs
                            # same pattern
    terminology/{code_item,code_list,import,version}.rs
                            # same pattern
  http.rs                   # unchanged (barrel)
  http/
    client.rs               # + tokio::task_local! TRACE_ID; + X-Trace-ID on send();
                             #   + #[tracing::instrument] on send()
    ...all 11 modules...    # unchanged — http fns stay pure
  system.rs                 # unchanged
```

## Components

### 1. `src-tauri/src/tracing_init.rs` (new)

Owns the tracing bootstrap. Mirrors the server's `init_tracing` in shape but lives in the Tauri setup phase because it needs an `AppHandle` to resolve `app_data_dir`.

```rust
pub struct LogGuard(pub tracing_appender::non_blocking::WorkerGuard);

pub fn init_tracing(app: &tauri::AppHandle) -> Result<LogGuard, Box<dyn std::error::Error>> {
    let dir = match std::env::var("AEGIS_LOG_DIR").ok() {
        Some(d) => d,
        None => {
            let base = app.path().app_data_dir()
                .map_err(|e| format!("app_data_dir: {e}"))?;
            base.join("logs").to_string_lossy().into_owned()
        }
    };
    std::fs::create_dir_all(&dir)?;
    let filter = build_filter();
    let file_appender = tracing_appender::rolling::daily(&dir, "aegis-desktop.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .json()
        .with_current_span(true)
        .with_span_list(false)
        .with_writer(non_blocking)
        .try_init();
    Ok(LogGuard(guard))
}

fn build_filter() -> EnvFilter {
    let level = std::env::var("AEGIS_LOG_LEVEL").unwrap_or_else(|_| "info".to_string());
    EnvFilter::new(level)
}
```

`LogGuard` is a newtype that owns the `WorkerGuard` so we can `app.manage(LogGuard(…))` and the guard lives for the process lifetime.

### 2. `src-tauri/src/trace_id_setup.rs` (new)

Owns the per-install device prefix file. Uses `nanoid` 21-char default alphabet.

```rust
const FILE_NAME: &str = "aegis-desktop-device-prefix";

pub fn load_or_create(app: &tauri::AppHandle) -> TraceIdGenerator {
    let dir = match app.path().app_data_dir() {
        Ok(d) => d,
        Err(_) => return TraceIdGenerator::new(None),  // fall back: still launches
    };
    let path = dir.join(FILE_NAME);
    let prefix = match std::fs::read_to_string(&path) {
        Ok(s) => s.trim().to_string(),
        Err(_) => {
            let id = nanoid::nanoid!();
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(&path, &id);  // best-effort
            id
        }
    };
    TraceIdGenerator::new(Some(prefix))
}
```

Failure mode: any I/O error (no app data dir, permission denied, write failure) silently falls back to `device_prefix = None`. A warning is logged at startup so the operator can see it, but the app launches. The trace id is observability, not a hard requirement.

### 3. `src-tauri/src/lib.rs` changes

Two edits only:
- Add `mod tracing_init;` and `mod trace_id_setup;`
- In `.setup`, after opening the store:
  ```rust
  let log_guard = tracing_init::init_tracing(app.handle())?;
  app.manage(log_guard);
  let generator = trace_id_setup::load_or_create(app.handle());
  app.manage(generator);
  ```
- No other changes — the command shims still pull `HttpClient` from managed state; the new `TraceIdGenerator` is an additional `State<…>` they pull.

### 4. `src-tauri/src/http/client.rs` changes (only)

- Module-top: `tokio::task_local! { pub static TRACE_ID: String; }`
- On `HttpClient::send`, before the `.send()`:
  ```rust
  let mut rb = self.http.request(method, &url);
  if let Some(t) = TRACE_ID.try_with(|id| id.clone()).ok() {
      rb = rb.header("X-Trace-ID", t);
  }
  ```
- Wrap the body of `send` in `#[tracing::instrument(skip_all, fields(method = %method, path = %path))]` so request lines appear at debug level. The `trace_id` field from the command span is inherited automatically.
- The rest of `client.rs` (token loading, refresh lock, parse_error) is untouched.

### 5. Every `src-tauri/src/commands/*.rs` (shim layer)

Add the new `State<'_, TraceIdGenerator>` parameter to every command, and wrap the existing body with the logging pattern. Skeleton (login is the template — all ~70 commands follow this shape):

```rust
#[tauri::command]
pub async fn login(
    client: State<'_, HttpClient>,
    generator: State<'_, TraceIdGenerator>,
    code: String,
    password: String,
) -> Result<(), ApiError> {
    let trace_id = generator.client_side();
    let span = tracing::info_span!("command", trace_id = %trace_id, command = "login");
    let _enter = span.enter();
    tracing::info!("enter");

    let result = TRACE_ID
        .scope(trace_id, async {
            auth::login(&client, LoginRequest { code, password }).await
        })
        .await;

    match &result {
        Ok(_) => tracing::info!("success"),
        Err(e) => tracing::error!(error = %e, "failed"),
    }
    result
}
```

Three rules for the wrap:
- `_enter` must be created **before** the `.await` enters the body so the span is entered on the right task; the `_enter` guard is dropped at the end of the function, after all logging.
- `TRACE_ID.scope(trace_id, async { … }).await` is the only `await`; the `match &result` block must use `&result` so we log without moving the value out.
- The `command = "…"` field uses the Rust function name (no `snake_case → camelCase`) — it's a tracing field, not a wire field.

### 6. Test additions

In `src-tauri/src/http/client.rs` (existing test module):
- `#[tokio::test] async fn trace_id_header_attached_when_task_local_set()` — mints a fake id, scopes `request_bytes` in `TRACE_ID.scope(…)`, asserts `wiremock::matchers::header("X-Trace-ID", id)` matches.
- `#[tokio::test] async fn trace_id_header_absent_when_no_task_local()` — calls without scoping, asserts a `NoHeader("X-Trace-ID")` matcher matches.

In `src-tauri/src/commands/auth.rs`:
- Existing `login_persists_tokens` and `login_propagates_401` stay green because the new args are pulled from `State` (test setup builds the state).
- One new `#[tokio::test] async fn login_logs_enter_and_success()` that captures log output (a custom `tracing_subscriber::fmt::TestWriter`-style guard, or just verify via the command layer via `State` plumbing).

## Data model & wire shape

No DDL, no DTO changes. The only new wire element is the outbound request header `X-Trace-ID` — present on every command that calls `HttpClient::request`/`request_bytes`; absent on calls that bypass it (the `refresh_with_lock` path goes through `self.http.post(...)` directly, **not** through `send` — leave that one alone for now; the task_local is set inside the command shim, so any path the command goes through picks it up automatically). The header name follows the existing convention from the server's tower-http `TraceLayer` (`x-request-id` is the default there, so we explicitly pick `X-Trace-ID` to disambiguate — no protocol collision).

## Key Decisions Locked In

- **Log sink:** JSON, daily rotation, file `aegis-desktop.log.YYYY-MM-DD` under `AEGIS_LOG_DIR` or `<app_data_dir>/logs` by default. Mirrors the server's `init_tracing`.
- **Filter:** `EnvFilter` from `AEGIS_LOG_LEVEL` (default `info`).
- **device_prefix:** persisted per-install `nanoid!()` (21 chars, default URL-safe alphabet) at `<app_data_dir>/aegis-desktop-device-prefix`; lazy-create-on-startup; `load_or_create` is best-effort and falls back to `None` on I/O errors.
- **Plumbing:** `tokio::task_local! { TRACE_ID: String }`. The command shim scopes the existing http fn call once; the client reads it inside `send()`.

## Trade-offs / things we accept

- **Every command gets two managed-state params.** Slightly more boilerplate at every shim (~70 sites), but each shim stays narrow — `HttpClient` does HTTP, `TraceIdGenerator` mints ids.
- **`refresh_with_lock` doesn't go through `send()`.** That's a one-off path inside `HttpClient::request_bytes` for the 401 retry; it uses `self.http.post(&url).json(…)`. We will thread the same task-local into it via a small private helper (`send_with_token`) so the retry also carries the header. Documented in code.
- **No structured `enter` / `success` arguments beyond `error = %e` in failure logs.** The commands' success path carries no domain-specific fields today; we can add per-command fields later without breaking the pattern.
- **`_enter` span guard is `!Send`** so we cannot `.await` while holding it across a non-`Send` boundary. The body is carefully written so the only `.await` happens inside the `TRACE_ID.scope(...)` block (which captures the span because it runs on the same task).
- **App data dir resolution requires `tauri::Manager`.** Already imported in `lib.rs`; no change.

## Verification gate, before any PR

```bash
# Backend
cargo fmt --all -- --check
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
cargo test -p aegis-desktop

# Manual smoke (after build)
pnpm --filter aegis-desktop tauri build
# Launch the built .exe, log in, observe a line per command + one request
# line per HTTP call in %APPDATA%/com.yukichen.aegis-desktop/logs/
# Open the persisted device-prefix file and confirm it matches the trace
# ids' middle segment.
```

## File changes summary

- New: `src-tauri/src/tracing_init.rs` (~40 lines + tests)
- New: `src-tauri/src/trace_id_setup.rs` (~30 lines + tests)
- Modified: `src-tauri/src/lib.rs` (4 lines in `.setup` + 2 new `mod` lines)
- Modified: `src-tauri/src/http/client.rs` (task_local decl, `send()` header attachment + instrument, `refresh_with_lock` thread, 2 new tests)
- Modified: every `src-tauri/src/commands/*.rs` shim (~90 commands across 7 leaf modules + 3 sub-modules — uniform wrap, one mechanical edit per `#[tauri::command]`)
- Modified: `src-tauri/Cargo.toml` (`nanoid`, `tracing`, `tracing-subscriber`, `tracing-appender` — pinned via `{ workspace = true }`)
- Modified: `Cargo.toml` (workspace root) — add `nanoid` to `[workspace.dependencies]` (the three tracing crates are already there)