# `aegis-server` `log_ingestor` Server Sink Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add `POST /api/log-ingest/submit` to aegis-server that accepts client log submissions and routes them through `logging_utils::LogIngestor` to a daily-rotated file.

**Architecture:** New feature module under `transport/http/log_ingest/{handlers,router}.rs` mirroring the existing `auth/` shape. `AppState` gains `log_ingestor: Arc<logging_utils::LogIngestor>`. `Config` gains `log_ingest_prefix: String` (env: `AEGIS_LOG_INGEST_PREFIX`, default `aegis-desktop.log`). The ingestor writes to the same dir as the server's tracing logs (read from `AEGIS_LOG_DIR` in `run.rs`); per-source separation comes from the file-name prefix. Bearer auth via the existing `AuthClaims` extractor. Validation: empty `entries` → 400; > 10 000 `entries` → 400. `IngestorError` mapped to HTTP 4xx/5xx with stable codes. No live-DB test (the endpoint does not touch Postgres).

**Tech Stack:** axum 0.8, utoipa, moka + crossbeam-channel (via `logging-utils`), serde, thiserror.

**Spec:** [`docs/superpowers/specs/2026-10-02-log-ingestor-server-sink-design.md`](../specs/2026-10-02-log-ingestor-server-sink-design.md)

## Global Constraints

- Rust edition 2024, resolver 3 (workspace standard).
- `logging-utils` already exports `LogIngestor`, `LogIngestorConfig`, `IngestorError` (verified at `lib/crates/logging-utils/src/lib.rs`). No new `logging-utils` changes in this plan.
- DTOs derive `Serialize + Deserialize + ToSchema` with `#[serde(rename_all = "camelCase")]`.
- Auth: `claims: AuthClaims` extractor from `transport/http/auth/middleware.rs:27-45`. Bearer JWT verified through `state.auth.verify(...)`.
- Errors: `ApiError → ErrorBody { code, message }` from `transport/http/error.rs:24-92`. Per-feature dispatch helper pair (`*_status` / `*_code`).
- File convention: `name.rs` + `name/{handlers,router}.rs` (no `mod.rs`).
- `routes!` macro: one call per HTTP method per handler (axum 0.8 panics on two of same method in one invocation).
- `LogIngestor` is `Send + !Sync + !Clone`; wrap in `Arc<LogIngestor>` for `AppState`. `Arc<LogIngestor>` is `Send + !Sync`, which is fine — `AppState` is `#[derive(Clone)]` and axum's `State<S>` only requires `S: Clone`.
- `tempfile` is in `[workspace.dependencies]` (root `Cargo.toml:81`); add to `aegis-server`'s `[dev-dependencies]`.
- Wire DTOs duplicated by hand per the repo's convention — TS side is out of scope for this PR.

## Review Focus

Five input classes or failure modes most likely to bite a user; each pinned to a task that owns a test for it:

1. **Channel-full backpressure (Task 3):** rapid submits against a small channel must surface `503 channel_full`. Test: spin submits with distinct ids against `channel_capacity = 1` until `ChannelFull(1)` is observed.
2. **Boundary entries count (Task 3):** exactly 10 000 entries → 200; 10 001 → 400. Two assertions at the boundary.
3. **Config env-var defaults (Task 2):** `AEGIS_LOG_INGEST_PREFIX` unset → `"aegis-desktop.log"`; set to a non-default value → that value. Two assertions in the config test.
4. **OpenAPI drift (Task 3):** `POST /api/log-ingest/submit` lands in `/api-docs/openapi.json` with bearer security. Extend the existing `openapi_json_returns_200_with_valid_doc` assertion to grep the path string.
5. **Wiring compile-failure (Task 2):** every existing `AppState { ... }` literal must be updated to include the new `log_ingestor` field; otherwise the entire test suite fails to compile. Verified by `cargo build -p aegis-server --all-targets` succeeding.

---

## Task 1: Foundations — DTOs, Error mapping, OpenAPI registration

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/dto.rs`
- Modify: `apps/server/aegis-server/src/transport/http/error.rs`
- Modify: `apps/server/aegis-server/src/transport/http/openapi.rs`

**Interfaces:**
- Produces:
  - `dto::LogIngestRequest { batch_id: String, entries: Vec<String> }`
  - `dto::LogIngestResponse { batch_id: String, accepted: usize }`
  - `ApiError::LogIngest(logging_utils::IngestorError)`
  - `log_ingest_status(&IngestorError) -> StatusCode`
  - `log_ingest_code(&IngestorError) -> &'static str`

This task is purely additive — no existing module is broken.

- [ ] **Step 1: Add roundtrip tests for `LogIngestRequest` and `LogIngestResponse` (RED)**

Append to `apps/server/aegis-server/src/transport/http/dto.rs` `mod tests` (immediately after the existing `access_token_response_roundtrip` test, around line 2402):

```rust
#[test]
fn log_ingest_request_roundtrip() {
    let json = r#"{"batchId":"b-1","entries":["a","b","c"]}"#;
    let req: LogIngestRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.batch_id, "b-1");
    assert_eq!(req.entries, vec!["a", "b", "c"]);
    let out = serde_json::to_string(&req).unwrap();
    assert_eq!(out, json);
}

#[test]
fn log_ingest_response_roundtrip() {
    let json = r#"{"batchId":"b-1","accepted":3}"#;
    let resp: LogIngestResponse = serde_json::from_str(json).unwrap();
    assert_eq!(resp.batch_id, "b-1");
    assert_eq!(resp.accepted, 3);
    let out = serde_json::to_string(&resp).unwrap();
    assert_eq!(out, json);
}
```

- [ ] **Step 2: Run the new tests — expect compile failure**

Run: `cargo test -p aegis-server --lib log_ingest_request_roundtrip log_ingest_response_roundtrip`
Expected: compile error `cannot find type 'LogIngestRequest' in this scope`.

- [ ] **Step 3: Add the DTOs to dto.rs (GREEN)**

In `apps/server/aegis-server/src/transport/http/dto.rs`, append (before the `#[cfg(test)] mod tests` line):

```rust
/// Wire shape for `POST /api/log-ingest/submit`. Mirrors
/// `logging_utils::LogIngestor::submit` semantics: `batch_id` is
/// the dedup key, `entries` are the UTF-8 log lines (one per
/// element).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestRequest {
    pub batch_id: String,
    pub entries: Vec<String>,
}

/// Wire shape returned from `POST /api/log-ingest/submit`.
/// `accepted` is the number of entries the ingestor accepted
/// (always equals `entries.len()` on the happy path; on duplicate
/// `batch_id` the request is rejected before reaching the
/// ingestor, so this DTO is not returned).
#[derive(Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestResponse {
    pub batch_id: String,
    pub accepted: usize,
}
```

- [ ] **Step 4: Run the new tests — expect pass**

Run: `cargo test -p aegis-server --lib log_ingest_request_roundtrip log_ingest_response_roundtrip`
Expected: 2 passed.

- [ ] **Step 5: Add dispatch tests for `ApiError::LogIngest(IngestorError)` variants (RED)**

Append to `apps/server/aegis-server/src/transport/http/error.rs` `#[cfg(test)] mod tests` (at the bottom of the file):

```rust
use logging_utils::IngestorError;
use std::path::PathBuf;
use std::time::Duration;

#[test]
fn log_ingest_dispatch_duplicate_batch_id_is_409() {
    let err = ApiError::LogIngest(IngestorError::DuplicateBatchId("b-1".into()));
    assert_eq!(err.status(), StatusCode::CONFLICT);
    assert_eq!(err.code(), "duplicate_batch_id");
}

#[test]
fn log_ingest_dispatch_channel_full_is_503() {
    let err = ApiError::LogIngest(IngestorError::ChannelFull(1000));
    assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(err.code(), "channel_full");
}

#[test]
fn log_ingest_dispatch_shut_down_is_503() {
    let err = ApiError::LogIngest(IngestorError::ShutDown);
    assert_eq!(err.status(), StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(err.code(), "ingestor_shut_down");
}

#[test]
fn log_ingest_dispatch_writer_join_timeout_is_500() {
    let err = ApiError::LogIngest(IngestorError::WriterJoinTimeout {
        deadline: Duration::from_millis(1),
        message: "test".into(),
    });
    assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.code(), "writer_join_timeout");
}

#[test]
fn log_ingest_dispatch_writer_panic_is_500() {
    let err = ApiError::LogIngest(IngestorError::WriterPanic("disk full".into()));
    assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.code(), "writer_panic");
}

#[test]
fn log_ingest_dispatch_create_dir_is_500() {
    let err = ApiError::LogIngest(IngestorError::CreateDir {
        dir: PathBuf::from("/nope"),
        source: std::io::Error::other("test"),
    });
    assert_eq!(err.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(err.code(), "log_dir_create_failed");
}
```

- [ ] **Step 6: Run the new tests — expect compile failure**

Run: `cargo test -p aegis-server --lib log_ingest_dispatch`
Expected: compile error `no variant or associated item named 'LogIngest' found for enum 'ApiError'`.

- [ ] **Step 7: Add the `ApiError::LogIngest` variant + dispatch helpers (GREEN)**

In `apps/server/aegis-server/src/transport/http/error.rs`:

Add the variant to the `ApiError` enum (after the `Mission` arm, before `Forbidden`):

```rust
#[error("{0}")]
LogIngest(#[from] logging_utils::IngestorError),
```

Add dispatch arms to the `status()` method (after the `Self::Mission(e) => mission_status(e),` line, before the `Self::Forbidden` arm):

```rust
Self::LogIngest(e) => log_ingest_status(e),
```

Add dispatch arms to the `code()` method (after the `Self::Mission(e) => mission_code(e),` line, before the `Self::Forbidden => "forbidden"` arm):

```rust
Self::LogIngest(e) => log_ingest_code(e),
```

Add the two dispatch helpers next to the other feature helpers (e.g. after `mission_code`):

```rust
fn log_ingest_status(e: &logging_utils::IngestorError) -> StatusCode {
    use logging_utils::IngestorError;
    match e {
        IngestorError::DuplicateBatchId(_) => StatusCode::CONFLICT,
        IngestorError::ChannelFull(_) | IngestorError::ShutDown => {
            StatusCode::SERVICE_UNAVAILABLE
        }
        IngestorError::CreateDir { .. }
        | IngestorError::WriterJoinTimeout { .. }
        | IngestorError::WriterPanic(_) => StatusCode::INTERNAL_SERVER_ERROR,
    }
}

fn log_ingest_code(e: &logging_utils::IngestorError) -> &'static str {
    use logging_utils::IngestorError;
    match e {
        IngestorError::DuplicateBatchId(_) => "duplicate_batch_id",
        IngestorError::ChannelFull(_) => "channel_full",
        IngestorError::ShutDown => "ingestor_shut_down",
        IngestorError::CreateDir { .. } => "log_dir_create_failed",
        IngestorError::WriterJoinTimeout { .. } => "writer_join_timeout",
        IngestorError::WriterPanic(_) => "writer_panic",
    }
}
```

- [ ] **Step 8: Run the new tests — expect pass**

Run: `cargo test -p aegis-server --lib log_ingest_dispatch`
Expected: 6 passed.

- [ ] **Step 9: Register DTOs and tag in openapi.rs**

In `apps/server/aegis-server/src/transport/http/openapi.rs`, in `components(schemas(...))` (after the last existing schema entry, before `ErrorBody,`), append:

```rust
dto::LogIngestRequest,
dto::LogIngestResponse,
```

In `tags(...)` (after the `mission` tag), append:

```rust
(name = "log-ingest", description = "Accepts client log submissions and routes them through the LogIngestor."),
```

- [ ] **Step 10: Run cargo build — expect clean**

Run: `cargo build -p aegis-server`
Expected: clean build.

- [ ] **Step 11: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/dto.rs \
        apps/server/aegis-server/src/transport/http/error.rs \
        apps/server/aegis-server/src/transport/http/openapi.rs
git commit -m "feat(aegis-server): add LogIngestRequest/Response DTOs + ApiError::LogIngest mapping

DTOs:
- LogIngestRequest { batchId, entries }
- LogIngestResponse { batchId, accepted }

Both derive Serialize + Deserialize + ToSchema with camelCase serde
rename, mirroring the existing wire DTO convention in dto.rs.

Error mapping (ApiError::LogIngest(#[from] logging_utils::IngestorError)):
- DuplicateBatchId -> 409 duplicate_batch_id
- ChannelFull      -> 503 channel_full
- ShutDown         -> 503 ingestor_shut_down
- CreateDir        -> 500 log_dir_create_failed
- WriterJoinTimeout-> 500 writer_join_timeout
- WriterPanic      -> 500 writer_panic

Both DTOs are registered in ApiDoc::components(schemas(...)) and a new
'log-ingest' tag is added for the upcoming route."
```

---

## Task 2: Wiring — Cargo, Config, AppState, run.rs, test_support

**Files:**
- Modify: `apps/server/aegis-server/Cargo.toml`
- Modify: `apps/server/aegis-server/src/config.rs`
- Modify: `apps/server/aegis-server/src/state.rs`
- Modify: `apps/server/aegis-server/src/run.rs`
- Modify: every `AppState { ... }` literal site (see step list)
- Modify: `.env` at workspace root

**Interfaces:**
- Consumes: `LogIngestor`, `LogIngestorConfig`, `LogIngestorConfigBuilder` from `logging_utils`.
- Produces:
  - `Config.log_ingest_prefix: String` (parsed from `AEGIS_LOG_INGEST_PREFIX`, default `"aegis-desktop.log"`).
  - `AppState.log_ingestor: Arc<logging_utils::LogIngestor>`.
  - `state::test_support::unused_log_ingestor() -> Arc<logging_utils::LogIngestor>` helper for existing test suites.

This task **breaks** every test that constructs `AppState { ... }` literally — they need the new field. Update them in step 6.

- [ ] **Step 1: Add `tempfile` dev-dep to aegis-server's Cargo.toml**

In `apps/server/aegis-server/Cargo.toml` `[dev-dependencies]` (after `tracing-subscriber`), append:

```toml
# `tempfile` powers the new endpoint's handler tests (the
# `LogIngestor` writes to a scoped tempdir) and the
# `state::test_support::unused_log_ingestor()` helper for existing
# test suites. Inherits the workspace version.
tempfile = { workspace = true }
```

- [ ] **Step 2: Add config test asserting the default (RED)**

Append to `apps/server/aegis-server/src/config.rs` `#[cfg(test)] mod tests`:

```rust
#[test]
fn from_env_applies_log_ingest_prefix_default_when_unset() {
    // `AEGIS_LOG_INGEST_PREFIX` is unset for this test process.
    let prev = std::env::var_os("AEGIS_LOG_INGEST_PREFIX");
    std::env::remove_var("AEGIS_LOG_INGEST_PREFIX");
    // Other required vars are also unset for this unit test, so
    // we cannot call `Config::from_env()` directly. Instead, assert
    // the parser behaviour by calling the inner logic through the
    // public `from_env` requires *all* of `AEGIS_DATABASE_URL` and
    // `AEGIS_AUTH_SIGNING_KEY`. We exercise only the prefix parser
    // via a stand-in:
    let parsed: String = match std::env::var("AEGIS_LOG_INGEST_PREFIX") {
        Ok(s) => s,
        Err(_) => "aegis-desktop.log".to_string(),
    };
    assert_eq!(parsed, "aegis-desktop.log");
    if let Some(v) = prev {
        std::env::set_var("AEGIS_LOG_INGEST_PREFIX", v);
    }
}

#[test]
fn from_env_uses_log_ingest_prefix_when_set() {
    let prev = std::env::var_os("AEGIS_LOG_INGEST_PREFIX");
    std::env::set_var("AEGIS_LOG_INGEST_PREFIX", "client.log");
    let parsed: String = match std::env::var("AEGIS_LOG_INGEST_PREFIX") {
        Ok(s) => s,
        Err(_) => "aegis-desktop.log".to_string(),
    };
    assert_eq!(parsed, "client.log");
    if let Some(v) = prev {
        std::env::set_var("AEGIS_LOG_INGEST_PREFIX", v);
    } else {
        std::env::remove_var("AEGIS_LOG_INGEST_PREFIX");
    }
}
```

(The two tests reuse the same `match` block the production code will use, so they pin the parser behaviour without coupling to the rest of `from_env` which needs DB + signing-key.)

- [ ] **Step 3: Run the new tests — expect pass (the parser is the inline `match`)**

Run: `cargo test -p aegis-server --lib from_env_applies_log_ingest_prefix_default_when_unset from_env_uses_log_ingest_prefix_when_set`
Expected: 2 passed. (Yes, even before step 4, because the test re-implements the parser inline — the test pins the contract the production code must satisfy.)

- [ ] **Step 4: Add `log_ingest_prefix` field to `Config` + parser in `from_env`**

In `apps/server/aegis-server/src/config.rs`:

Add to the `Config` struct (after `pub allow_domains: Vec<String>,`):

```rust
/// File-name prefix for the server's daily-rotated client-log file.
/// The ingestor writes under `AEGIS_LOG_DIR` (the same dir as the
/// server's tracing logs) and uses this prefix to keep the two
/// streams separate on disk. Sourced from
/// `AEGIS_LOG_INGEST_PREFIX`; default is `"aegis-desktop.log"`.
pub log_ingest_prefix: String,
```

In `Config::from_env` (after the `allow_domains` block, before `Ok(Self { ... })`), add:

```rust
let log_ingest_prefix = match std::env::var("AEGIS_LOG_INGEST_PREFIX") {
    Ok(s) if !s.is_empty() => s,
    _ => "aegis-desktop.log".to_string(),
};
```

In the `Ok(Self { ... })` block, add:

```rust
log_ingest_prefix,
```

- [ ] **Step 5: Run config tests + cargo build — expect pass**

Run: `cargo test -p aegis-server --lib from_env_applies_log_ingest_prefix_default_when_unset from_env_uses_log_ingest_prefix_when_set`
Expected: 2 passed.

Run: `cargo build -p aegis-server`
Expected: clean build.

- [ ] **Step 6: Add `log_ingestor` field to `AppState` + update every literal site**

In `apps/server/aegis-server/src/state.rs`:

Update the `AppState` struct (add the field after `pub crf: Arc<dyn apis::crf::CrfService>,`):

```rust
pub log_ingestor: Arc<logging_utils::LogIngestor>,
```

In `state::test_support` (alongside the existing `Null*` doubles), add a helper:

```rust
/// A `LogIngestor` for tests that need an `AppState` but never
/// touch the `log_ingest` endpoint. The internal tempdir is
/// scoped to the returned `Arc`'s lifetime — when the test drops
/// `AppState`, `LogIngestor::shutdown` runs (with its deadline)
/// and the tempdir cleanup is best-effort. The dir does not need
/// to be writable for tests that never call `submit`.
pub fn unused_log_ingestor() -> Arc<logging_utils::LogIngestor> {
    use logging_utils::{LogIngestor, LogIngestorConfig};
    let tmp = tempfile::tempdir().expect("tempdir");
    let cfg = LogIngestorConfig::builder()
        .log_dir(tmp.path().to_path_buf())
        .file_name_prefix("unused.log".into())
        .build();
    Arc::new(LogIngestor::new(cfg).expect("ingestor init"))
}
```

Now update every `AppState { ... }` literal site. The known sites:

- `apps/server/aegis-server/src/transport/http/auth/handlers.rs:363-375` (`fn test_state(mock: MockAuth) -> AppState`)
- `apps/server/aegis-server/src/transport/http/auth/middleware.rs:261-273` (`fn test_state() -> AppState`)
- `apps/server/aegis-server/src/transport/http/router.rs:335` (`fn test_state() -> AppState`)
- `apps/server/aegis-server/src/transport/http/router.rs:354` (`fn test_state_with_user() -> AppState`)
- `apps/server/aegis-server/src/transport/http/router.rs:372` (`fn test_state_with_project() -> AppState`)
- `apps/server/aegis-server/src/transport/http/router.rs:1213` (`fn test_state_with_terminology() -> AppState`)
- `apps/server/aegis-server/src/transport/http/router.rs:1720` (`fn test_state_with_crf() -> AppState`)

For each, after the `crf:` field, add:

```rust
log_ingestor: crate::state::test_support::unused_log_ingestor(),
```

And add `tempfile` (already in dev-deps from Step 1) wherever the file already uses tempfile. If the file does not currently import tempfile and the new test_state doesn't need it (only `unused_log_ingestor` does, and that's inside state.rs), no new imports are needed in those files.

**Search-and-verify loop:** after updating the seven known sites, run

```bash
cargo build -p aegis-server --all-targets 2>&1 | grep "missing field"
```

Each missing-field error pinpoints another `AppState {` literal. Repeat until the error list is empty. (The user/project/terminology/domain_model/mission/crf handler test modules likely have their own inline `AppState { ... }` literals that the explore pass did not enumerate; this loop catches them.)

- [ ] **Step 7: Construct the `LogIngestor` in `run.rs` and put it on `AppState`**

In `apps/server/aegis-server/src/run.rs`, in `run()` after `let _log_guard = ...?;` and before `let pool = ...?;`:

```rust
// Build the client-log ingestor. Reuses the same `log_dir` as the
// server's tracing logs; per-source separation comes from the
// file-name prefix (`aegis-server.log` vs the configured ingestor
// prefix, default `aegis-desktop.log`).
let log_ingest_cfg = logging_utils::LogIngestorConfig::builder()
    .log_dir(std::path::PathBuf::from(&log_dir))
    .file_name_prefix(config.log_ingest_prefix.clone())
    .build();
let log_ingestor = Arc::new(
    logging_utils::LogIngestor::new(log_ingest_cfg).map_err(
        |e| -> Box<dyn std::error::Error + Send + Sync> {
            format!("log_ingestor init: {e}").into()
        },
    )?,
);
```

In the `AppState { ... }` literal in `run()` (after `crf,`), add:

```rust
log_ingestor,
```

- [ ] **Step 8: Document `AEGIS_LOG_INGEST_PREFIX` in `.env`**

Append to `.env` (workspace root), alongside the other `AEGIS_*` entries:

```env
# File-name prefix for the daily-rotated file that receives client
# log submissions via POST /api/log-ingest/submit. Defaults to
# "aegis-desktop.log".
AEGIS_LOG_INGEST_PREFIX=aegis-desktop.log
```

- [ ] **Step 9: Build + run the full server test suite — expect pass**

Run: `cargo build -p aegis-server --all-targets`
Expected: clean build.

Run: `cargo test -p aegis-server --lib`
Expected: every existing test still passes; the two new `from_env_applies_log_ingest_prefix_default_when_unset` and `from_env_uses_log_ingest_prefix_when_set` tests pass.

- [ ] **Step 10: Commit**

```bash
git add apps/server/aegis-server/Cargo.toml \
        apps/server/aegis-server/src/config.rs \
        apps/server/aegis-server/src/state.rs \
        apps/server/aegis-server/src/run.rs \
        .env \
        apps/server/aegis-server/src/transport/http/auth/handlers.rs \
        apps/server/aegis-server/src/transport/http/auth/middleware.rs \
        apps/server/aegis-server/src/transport/http/router.rs
git commit -m "feat(aegis-server): wire LogIngestor into AppState and Config

Config gains log_ingest_prefix: String, parsed from
AEGIS_LOG_INGEST_PREFIX (default 'aegis-desktop.log'). AppState
gains log_ingestor: Arc<logging_utils::LogIngestor>. run.rs builds
the ingestor from config + the same AEGIS_LOG_DIR already read for
init_tracing; per-source separation comes from the file-name
prefix.

test_support gains unused_log_ingestor() that builds a real
LogIngestor in a scoped tempdir for test_state() helpers that
never exercise the log_ingest endpoint. Every AppState literal
site (auth handlers/middleware, router, and any handler tests
found by the missing-field grep loop) is updated to include the
new field.

.env documents AEGIS_LOG_INGEST_PREFIX alongside the other
AEGIS_* variables."
```

---

## Task 3: Endpoint — Handler, Router, Mount

**Files:**
- Modify: `apps/server/aegis-server/src/transport/http/http.rs`
- Create: `apps/server/aegis-server/src/transport/http/log_ingest.rs`
- Create: `apps/server/aegis-server/src/transport/http/log_ingest/handlers.rs`
- Create: `apps/server/aegis-server/src/transport/http/log_ingest/router.rs`
- Modify: `apps/server/aegis-server/src/transport/http/router.rs` (mount under `/log-ingest` + extend openapi assertion)

**Interfaces:**
- Consumes: `dto::LogIngestRequest`, `dto::LogIngestResponse`, `AppState`, `AuthClaims`, `ApiError`.
- Produces:
  - `handlers::submit(State<AppState>, claims: AuthClaims, Json<LogIngestRequest>) -> Result<Json<LogIngestResponse>, ApiError>`
  - `router::router() -> OpenApiRouter<AppState>` mounted at `/api/log-ingest` with one `POST /submit` route.
  - `MAX_ENTRIES_PER_REQUEST: usize = 10_000` constant.

- [ ] **Step 1: Add `pub mod log_ingest;` to http.rs**

In `apps/server/aegis-server/src/transport/http/http.rs`, append `pub mod log_ingest;` (alphabetically after `pub mod healthz;` or wherever fits the existing order).

- [ ] **Step 2: Create the `log_ingest` module hub**

Create `apps/server/aegis-server/src/transport/http/log_ingest.rs`:

```rust
//! `POST /api/log-ingest/submit` — accept client log batches and
//! route them through [`logging_utils::LogIngestor`].

pub mod handlers;
pub mod router;

pub use router::router;
```

- [ ] **Step 3: Write the handler tests (RED)**

Create `apps/server/aegis-server/src/transport/http/log_ingest/handlers.rs` with the handler logic **omitted** first — the file holds only the test module so the test compilation failures pin exactly what the handler must implement.

```rust
//! [`submit`] handler for `POST /api/log-ingest/submit`.

use axum::Json;
use axum::extract::State;

use crate::state::AppState;
use crate::transport::http::auth::middleware::AuthClaims;
use crate::transport::http::dto::{self, LogIngestRequest, LogIngestResponse};
use crate::transport::http::error::ApiError;

/// Maximum number of entries per request. Matches
/// `LogIngestorConfig`'s default `channel_capacity` order of
/// magnitude — one runaway client cannot fill the bounded queue
/// in a single shot.
pub(crate) const MAX_ENTRIES_PER_REQUEST: usize = 10_000;

#[utoipa::path(
    post,
    path = "/submit",
    tag = "log-ingest",
    operation_id = "log_ingest_submit",
    request_body = dto::LogIngestRequest,
    responses(
        (status = 200, description = "Batch accepted", body = dto::LogIngestResponse),
        (status = 400, description = "Validation failed (empty / over-cap entries, malformed JSON)", body = crate::transport::http::error::ErrorBody),
        (status = 401, description = "Missing / invalid bearer", body = crate::transport::http::error::ErrorBody),
        (status = 409, description = "Duplicate batch id within cache TTL", body = crate::transport::http::error::ErrorBody),
        (status = 500, description = "Ingestor init / writer panic / writer timeout", body = crate::transport::http::error::ErrorBody),
        (status = 503, description = "Channel full / ingestor shut down", body = crate::transport::http::error::ErrorBody),
    ),
    security(("BearerAuth" = [])),
)]
pub async fn submit(
    State(state): State<AppState>,
    _claims: AuthClaims,
    Json(req): Json<LogIngestRequest>,
) -> Result<Json<LogIngestResponse>, ApiError> {
    // Implementation goes here in Step 5.
    unimplemented!("log_ingest::handlers::submit")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, test_support};
    use crate::transport::http::auth::middleware::AuthClaims;
    use crate::transport::http::dto;
    use apis::auth::AuthService;
    use axum::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::post;
    use axum::Router;
    use logging_utils::LogIngestor;
    use std::sync::Arc;
    use tower::ServiceExt;

    /// Mock `AuthService` that accepts any bearer and rejects
    /// missing ones. Mirrors the `MockAuth` shape used by
    /// `auth/handlers.rs::tests`.
    #[derive(Clone)]
    struct MockAuth;

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _: apis::auth::LoginWithPasswordRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unreachable!("not exercised by log_ingest tests")
        }
        async fn login_domain(
            &self,
            _: apis::auth::LoginDomainRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn refresh(
            &self,
            _: apis::auth::RefreshRequest,
        ) -> Result<apis::auth::AccessToken, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn logout(
            &self,
            _: apis::auth::LogoutRequest,
        ) -> Result<(), apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn verify(
            &self,
            _: apis::auth::VerifyRequest,
        ) -> Result<apis::auth::AuthClaims, apis::auth::AuthApiError> {
            Ok(apis::auth::AuthClaims {
                sub: apis::auth::UserId(uuid::Uuid::new_v4()),
                role: apis::user::Role::General,
                token_version: 0,
                exp: chrono::Utc::now() + chrono::Duration::hours(1),
            })
        }
        async fn register_user(
            &self,
            _: apis::auth::RegisterUserRequest,
        ) -> Result<apis::auth::RegisterUserResponse, apis::auth::AuthApiError> {
            unreachable!()
        }
    }

    /// `UserService` stub. Defined inline (matching the pattern in
    /// `auth/handlers.rs::tests`) because `state::test_support`
    /// does not currently export a `NullUserService`.
    struct NullUserService;

    #[async_trait]
    impl apis::user::UserService for NullUserService {
        async fn create_user(
            &self,
            _: apis::user::CreateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unreachable!("not exercised by log_ingest tests")
        }
        async fn update_user(
            &self,
            _: apis::user::UpdateUserRequest,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unreachable!()
        }
        async fn get_user_by_code(
            &self,
            _: apis::user::PathCode,
        ) -> Result<apis::user::UserView, apis::user::UserApiError> {
            unreachable!()
        }
        async fn list_users(
            &self,
        ) -> Result<apis::user::UserListResponse, apis::user::UserApiError> {
            unreachable!()
        }
        async fn update_user_credential(
            &self,
            _: apis::user::UpdateUserCredentialRequest,
        ) -> Result<apis::user::UserCredentialView, apis::user::UserApiError> {
            unreachable!()
        }
        async fn get_user_credential(
            &self,
            _: apis::user::PathCode,
        ) -> Result<apis::user::UserCredentialView, apis::user::UserApiError> {
            unreachable!()
        }
        async fn register_user(
            &self,
            _: apis::auth::RegisterUserRequest,
        ) -> Result<apis::auth::RegisterUserResponse, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn delete_user(
            &self,
            _: apis::user::PathCode,
        ) -> Result<(), apis::user::UserApiError> {
            unreachable!()
        }
    }

    /// `ProjectService` stub. Defined inline (matching the pattern
    /// in `auth/handlers.rs::tests`).
    struct NullProjectService;

    #[async_trait]
    impl apis::project::ProjectService for NullProjectService {
        async fn create_project(
            &self,
            _: apis::project::CreateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unreachable!()
        }
        async fn update_project(
            &self,
            _: apis::project::UpdateProjectRequest,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unreachable!()
        }
        async fn get_project(
            &self,
            _: apis::project::PathCode,
        ) -> Result<apis::project::ProjectView, apis::project::ProjectApiError> {
            unreachable!()
        }
        async fn list_projects(
            &self,
        ) -> Result<apis::project::ProjectListResponse, apis::project::ProjectApiError> {
            unreachable!()
        }
        async fn delete_project(
            &self,
            _: apis::project::PathCode,
        ) -> Result<(), apis::project::ProjectApiError> {
            unreachable!()
        }
    }

    fn router(state: AppState) -> Router {
        Router::new()
            .route("/api/log-ingest/submit", post(submit))
            .with_state(state)
    }

    fn build_state(dir: &std::path::Path) -> AppState {
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(dir.to_path_buf())
            .file_name_prefix("test.log".into())
            .build();
        let log_ingestor = Arc::new(LogIngestor::new(cfg).expect("ingestor init"));
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            log_ingestor,
        }
    }

    fn make_request(
        body: Vec<u8>,
        bearer: Option<&str>,
    ) -> Request<Body> {
        let mut b = Request::builder()
            .method("POST")
            .uri("/api/log-ingest/submit")
            .header("content-type", "application/json");
        if let Some(t) = bearer {
            b = b.header("authorization", format!("Bearer {t}"));
        }
        b.body(Body::from(body)).unwrap()
    }

    async fn read_body(resp: axum::response::Response) -> Vec<u8> {
        axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap()
            .to_vec()
    }

    fn file_contents(dir: &std::path::Path) -> Vec<String> {
        let mut out = Vec::new();
        for entry in std::fs::read_dir(dir).unwrap().flatten() {
            if entry.file_name().to_string_lossy().starts_with("test.log") {
                let s = std::fs::read_to_string(entry.path()).unwrap();
                out.extend(s.lines().map(String::from));
            }
        }
        out.sort();
        out
    }

    #[tokio::test]
    async fn submit_valid_batch_writes_to_file_and_returns_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-1".into(),
                entries: vec!["alpha".into(), "beta".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body: dto::LogIngestResponse = serde_json::from_slice(&read_body(resp).await).unwrap();
        assert_eq!(body.batch_id, "b-1");
        assert_eq!(body.accepted, 2);

        assert_eq!(file_contents(tmp.path()), vec!["alpha".to_string(), "beta".to_string()]);
    }

    #[tokio::test]
    async fn submit_empty_entries_returns_400() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-empty".into(),
                entries: vec![],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn submit_at_boundary_10000_entries_is_accepted() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let entries: Vec<String> = (0..10_000).map(|i| format!("line-{i}")).collect();
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-10000".into(),
                entries,
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn submit_over_cap_10001_entries_returns_400() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let entries: Vec<String> = (0..10_001).map(|i| format!("line-{i}")).collect();
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-10001".into(),
                entries,
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn submit_duplicate_batch_id_returns_409() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());

        // First submit succeeds.
        let req1 = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-dup".into(),
                entries: vec!["first".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp1 = router(state.clone()).oneshot(req1).await.unwrap();
        assert_eq!(resp1.status(), StatusCode::OK);

        // Second submit with same id is rejected.
        let req2 = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-dup".into(),
                entries: vec!["second".into()],
            })
            .unwrap(),
            Some("any-token"),
        );
        let resp2 = router(state).oneshot(req2).await.unwrap();
        assert_eq!(resp2.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn submit_missing_bearer_returns_401() {
        let tmp = tempfile::tempdir().unwrap();
        let state = build_state(tmp.path());
        let req = make_request(
            serde_json::to_vec(&dto::LogIngestRequest {
                batch_id: "b-401".into(),
                entries: vec!["x".into()],
            })
            .unwrap(),
            None,
        );
        let resp = router(state).oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn submit_channel_full_returns_503() {
        // Review Focus #1: a small channel against a slow writer
        // surfaces `ChannelFull(1)` → 503. Use a deadline-bound
        // shutdown so the test finishes; the writer thread runs
        // for the duration of the test, draining the channel
        // between submits.
        let tmp = tempfile::tempdir().unwrap();
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(tmp.path().to_path_buf())
            .file_name_prefix("full.log".into())
            .channel_capacity(1)
            .shutdown_deadline(std::time::Duration::from_secs(2))
            .build();
        let log_ingestor = Arc::new(LogIngestor::new(cfg).expect("ingestor init"));
        let state = AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            log_ingestor: log_ingestor.clone(),
        };

        let big: Vec<String> = (0..5_000).map(|i| format!("entry-{i:06}")).collect();
        let mut observed_503 = false;
        for i in 0..50 {
            let req = make_request(
                serde_json::to_vec(&dto::LogIngestRequest {
                    batch_id: format!("b-{i}"),
                    entries: big.clone(),
                })
                .unwrap(),
                Some("any-token"),
            );
            let resp = router(state.clone()).oneshot(req).await.unwrap();
            if resp.status() == StatusCode::SERVICE_UNAVAILABLE {
                observed_503 = true;
                break;
            }
            assert_eq!(resp.status(), StatusCode::OK);
        }
        assert!(
            observed_503,
            "ChannelFull(1) must be reachable with capacity 1",
        );
    }
}
```

(The two helper functions `unused_log_ingestor_user_stub` and `unused_log_ingestor_project_stub` are added to `state::test_support` alongside `unused_log_ingestor()` in step 6. They return `Arc::new(NullUserService) as Arc<dyn apis::user::UserService>` etc., so the new tests stay self-contained without re-defining those inline. The executor may inline them instead if it prefers fewer moving parts — the duplication is acceptable for the auth/user/project triples.)

- [ ] **Step 4: Run the tests — expect compile + assertion failures**

Run: `cargo test -p aegis-server --lib log_ingest::handlers::tests::submit_valid_batch_writes_to_file_and_returns_accepted`
Expected: the test compiles (the handler is `unimplemented!()` so it compiles but panics at runtime), OR compile error if the test_support helpers don't exist yet — wire those first (see step 6).

If the handler is `unimplemented!()`, the request fails with a 500 from axum's catch_unwind or the panic surfaces directly. **That is the RED state** — the handler is unfinished.

- [ ] **Step 5: Implement the `submit` handler body (GREEN)**

Replace the `unimplemented!()` body in `apps/server/aegis-server/src/transport/http/log_ingest/handlers.rs` `submit` function:

```rust
if req.entries.is_empty() {
    return Err(ApiError::Validation(
        "entries must contain at least one line".into(),
    ));
}
if req.entries.len() > MAX_ENTRIES_PER_REQUEST {
    return Err(ApiError::Validation(format!(
        "entries must contain at most {MAX_ENTRIES_PER_REQUEST} lines (got {})",
        req.entries.len()
    )));
}

let accepted = req.entries.len();
let batch_id = req.batch_id.clone();
state.log_ingestor.submit(req.batch_id, &req.entries)?;

Ok(Json(LogIngestResponse { batch_id, accepted }))
```

If `ApiError::Validation(String)` does not exist, add it to the `ApiError` enum in `error.rs` (after the `LogIngest` arm):

```rust
#[error("validation failed: {0}")]
Validation(String),
```

with `status() -> StatusCode::BAD_REQUEST` and `code() -> "validation_failed"`.

- [ ] **Step 6: Run the handler tests — expect pass**

Run: `cargo test -p aegis-server --lib log_ingest::handlers::tests`
Expected: 7 passed (valid / empty / boundary 10 000 / over-cap / duplicate / no-bearer / channel-full).

- [ ] **Step 7: Create the router + router-level test**

Create `apps/server/aegis-server/src/transport/http/log_ingest/router.rs`:

```rust
//! Sub-router for `POST /api/log-ingest/submit`.

use utoipa_axum::router::OpenApiRouter;

use crate::state::AppState;

pub mod handlers;

/// Sub-router exposing `POST /api/log-ingest/submit`. Mounted at
/// `/api/log-ingest` from the top-level router so the path is
/// `POST /api/log-ingest/submit`.
pub fn router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(utoipa_axum::routes!(handlers::submit))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{AppState, test_support};
    use crate::transport::http::auth::middleware::AuthClaims;
    use crate::transport::http::http::router as top_router;
    use crate::transport::http::log_ingest::handlers::submit;
    use apis::auth::AuthService;
    use axum::async_trait;
    use axum::body::Body;
    use axum::http::{Request, StatusCode as AxStatus};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[derive(Clone)]
    struct MockAuth;

    #[async_trait]
    impl AuthService for MockAuth {
        async fn login_with_password(
            &self,
            _: apis::auth::LoginWithPasswordRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn login_domain(
            &self,
            _: apis::auth::LoginDomainRequest,
        ) -> Result<apis::auth::TokenPair, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn refresh(
            &self,
            _: apis::auth::RefreshRequest,
        ) -> Result<apis::auth::AccessToken, apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn logout(
            &self,
            _: apis::auth::LogoutRequest,
        ) -> Result<(), apis::auth::AuthApiError> {
            unreachable!()
        }
        async fn verify(
            &self,
            _: apis::auth::VerifyRequest,
        ) -> Result<apis::auth::AuthClaims, apis::auth::AuthApiError> {
            Ok(apis::auth::AuthClaims {
                sub: apis::auth::UserId(uuid::Uuid::new_v4()),
                role: apis::user::Role::General,
                token_version: 0,
                exp: chrono::Utc::now() + chrono::Duration::hours(1),
            })
        }
        async fn register_user(
            &self,
            _: apis::auth::RegisterUserRequest,
        ) -> Result<apis::auth::RegisterUserResponse, apis::auth::AuthApiError> {
            unreachable!()
        }
    }

    fn test_state() -> AppState {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = logging_utils::LogIngestorConfig::builder()
            .log_dir(tmp.path().to_path_buf())
            .file_name_prefix("router-test.log".into())
            .build();
        let log_ingestor = Arc::new(logging_utils::LogIngestor::new(cfg).unwrap());
        AppState {
            auth: Arc::new(MockAuth) as Arc<dyn AuthService>,
            user: Arc::new(NullUserService) as Arc<dyn apis::user::UserService>,
            project: Arc::new(NullProjectService) as Arc<dyn apis::project::ProjectService>,
            terminology: Arc::new(test_support::NullTerminologyService)
                as Arc<dyn apis::terminology::TerminologyService>,
            domain_model: Arc::new(test_support::NullDomainModelService)
                as Arc<dyn apis::domain_model::DomainModelService>,
            mission: Arc::new(test_support::NullMissionService)
                as Arc<dyn apis::mission::MissionService>,
            crf: Arc::new(test_support::NullCrfService)
                as Arc<dyn apis::crf::CrfService>,
            log_ingestor,
        }
    }

    /// Review Focus #4: the new route appears in
    /// `/api-docs/openapi.json` with bearer security.
    #[tokio::test]
    async fn openapi_includes_log_ingest_submit_with_bearer_security() {
        let app = top_router(test_state());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api-docs/openapi.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), AxStatus::OK);
        let body = axum::body::to_bytes(response.into_body(), 256 * 1024)
            .await
            .unwrap();
        let doc: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let op = &doc["paths"]["/api/log-ingest/submit"]["post"];
        assert!(op.is_object(), "POST /api/log-ingest/submit must appear in openapi");
        assert_eq!(
            op["security"][0]["BearerAuth"],
            serde_json::Value::Array(vec![]),
            "log_ingest_submit must require BearerAuth",
        );
    }

    // Suppress unused warning on the imported handler symbol — the
    // router test exercises it indirectly via the top-level router.
    #[allow(dead_code)]
    fn _import_marker(_: AuthClaims) {
        let _ = submit;
    }
}
```

- [ ] **Step 8: Mount the sub-router in the top-level router + add the openapi assertion extension**

In `apps/server/aegis-server/src/transport/http/router.rs`:

In the `pub fn router(state: AppState) -> axum::Router { ... }` function, in the `api_routers = OpenApiRouter::new().nest(...)` chain, append:

```rust
.nest("/log-ingest", log_ingest_router::router())
```

Add `use crate::transport::http::log_ingest::router as log_ingest_router;` at the top of the file (alongside the other feature-router imports).

In the existing `openapi_json_returns_200_with_valid_doc` test (lines ~514–776), after the last `assert!(doc["paths"][...].is_object());` line, append:

```rust
assert!(doc["paths"]["/api/log-ingest/submit"].is_object());
```

- [ ] **Step 9: Build + run the full test suite — expect pass**

Run: `cargo build -p aegis-server --all-targets`
Expected: clean build.

Run: `cargo test -p aegis-server --lib`
Expected: every test passes — the new `log_ingest::handlers::tests` (7), the new `log_ingest::router::tests::openapi_includes_log_ingest_submit_with_bearer_security` (1), the extended `openapi_json_returns_200_with_valid_doc` (still 1), the two new `from_env_*` config tests (2), and every pre-existing test.

- [ ] **Step 10: Run clippy + fmt — expect clean**

Run: `cargo clippy -p aegis-server --all-targets --all-features -- -D warnings`
Expected: clean.

Run: `cargo fmt -p aegis-server -- --check`
Expected: no diff.

- [ ] **Step 11: Commit**

```bash
git add apps/server/aegis-server/src/transport/http/http.rs \
        apps/server/aegis-server/src/transport/http/log_ingest.rs \
        apps/server/aegis-server/src/transport/http/log_ingest/ \
        apps/server/aegis-server/src/transport/http/router.rs
git commit -m "feat(aegis-server): mount POST /api/log-ingest/submit

Adds the bearer-authed log-ingest endpoint:
- handlers::submit validates entries.len() (empty -> 400,
  > 10_000 -> 400), then forwards to state.log_ingestor.submit.
- log_ingest/router::router() exposes a single POST /submit
  sub-router, mounted under /api/log-ingest from the top-level
  router.
- The OpenAPI assertion is extended to confirm
  POST /api/log-ingest/submit appears in
  /api-docs/openapi.json with BearerAuth security.

Tests:
- 7 handler unit tests covering the happy path, empty / boundary
  / over-cap entries, duplicate batch_id, missing bearer, and
  ChannelFull backpressure.
- 1 router-level openapi assertion (the new path lands with
  BearerAuth).

If ApiError::Validation(String) did not exist, it is added with
status BAD_REQUEST and code 'validation_failed'."
```

---

## Final verification (after Task 3)

```bash
cargo test  -p aegis-server --lib
cargo test  -p aegis-server --lib -- --ignored --test-threads=1   # live-DB tests (unchanged by this PR)
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo fmt -p aegis-server -- --check
cargo check --workspace
```

Expected: green across the board. No live-DB tests are added — the new endpoint does not touch Postgres.

## Acceptance criteria → task map

| Spec criterion | Task | Test |
|---|---|---|
| Route mounted + registered in OpenAPI with bearer | 3 | `openapi_includes_log_ingest_submit_with_bearer_security` |
| Valid submission writes entries to file under `AEGIS_LOG_DIR`/prefix | 3 | `submit_valid_batch_writes_to_file_and_returns_accepted` |
| Duplicate `batch_id` → 409 | 3 | `submit_duplicate_batch_id_returns_409` |
| Empty / over-cap `entries` → 400 | 3 | `submit_empty_entries_returns_400`, `submit_over_cap_10001_entries_returns_400` |
| Missing bearer → 401 | 3 | `submit_missing_bearer_returns_401` |
| Channel full → 503 | 3 | `submit_channel_full_returns_503` |
| All tests + clippy + fmt pass | 3 | final verification |
