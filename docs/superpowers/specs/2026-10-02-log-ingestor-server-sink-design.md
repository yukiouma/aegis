# `logging-utils::log_ingestor` Server Sink — Design

**Date:** 2026-10-02
**Follows from:** [`2026-10-02-log-ingestor-design.md`](2026-10-02-log-ingestor-design.md)
**Owner:** aegis-server, aegis-desktop

## Motivation

The `log_ingestor` crate (built in the previous spec) deduplicates and
writes batches to a daily-rotated local file. The original spec listed
the server integration as **out of scope**. This follow-up closes that
gap:

1. `aegis-server` exposes `POST /api/log-ingest/submit`, accepting
   `(batch_id, entries)` from clients and routing them through
   `logging_utils::LogIngestor` to a server-side daily-rotated file.
2. `aegis-desktop` calls that endpoint when flushing its client log
   buffer, so the endpoint is actually exercised.

The ingestor stays single-tenant; the server runs one ingestor per
process and writes to one daily-rotated file (default prefix
`aegis-desktop.log`, per request).

## Server side — `aegis-server`

### Configuration — `Config`

Add one field:

```rust
pub log_ingest: LogIngestorConfig,
```

Read two new env vars in `Config::from_env`, defaulting when unset:

| Var | Default | Maps to |
|---|---|---|
| `AEGIS_LOG_INGEST_DIR` | `./.data/logs/aegis-client` | `LogIngestorConfig::log_dir` |
| `AEGIS_LOG_INGEST_PREFIX` | `aegis-desktop.log` | `LogIngestorConfig::file_name_prefix` |

Other ingestor knobs (`cache_capacity`, `cache_ttl`, `channel_capacity`,
`shutdown_deadline`) use `LogIngestorConfig::builder` defaults. No new
env vars for them — YAGNI; expose them when a real knob is needed.

Do **not** reuse `AEGIS_LOG_DIR` — that var is the server's own
`tracing` log dir, with prefix `aegis-server.log`. Mixing them
collapses per-source separation that `tracing_appender::rolling::daily`
relies on.

### State — `AppState`

Add `pub log_ingestor: Arc<logging_utils::LogIngestor>`. The `Arc` is
required because `LogIngestor` is `!Clone`. Tests construct a real
`LogIngestor::new(tempdir)` against a `tempfile::tempdir()`; the
test-support null pattern is **not** extended — `LogIngestor` is
straightforward to build per-test.

### Construction — `run.rs`

After `init_tracing` and the service pool:

```rust
let log_ingestor = Arc::new(
    logging_utils::LogIngestor::new(config.log_ingest.clone())
        .map_err(|e| format!("log_ingestor init: {e}"))?,
);
```

Pass it onto `AppState`. Graceful shutdown rides on `Drop`: when
`AppState` is dropped at the end of `run`, `Arc::drop` runs
`LogIngestor::shutdown`, which drains the bounded channel and joins
the writer thread within `shutdown_deadline` (default 5s).

### HTTP endpoint — `POST /api/log-ingest/submit`

**Auth: required.** Handler takes `claims: AuthClaims`; the existing
`FromRequestParts` extractor enforces a valid bearer JWT. Desktop
reuses its access token (no new token type).

**Files (new):**
- `transport/http/log_ingest.rs` — module hub (`pub mod handlers; pub
  mod router; pub use router::router;`)
- `transport/http/log_ingest/handlers.rs` — `submit` handler +
  handler-level tests
- `transport/http/log_ingest/router.rs` — sub-router + router-level
  integration tests

**Files (edited):**
- `transport/http/http.rs` — add `pub mod log_ingest;`
- `transport/http/router.rs` — mount with
  `.nest("/log-ingest", log_ingest::router())`
- `transport/http/dto.rs` — add `LogIngestRequest`,
  `LogIngestResponse`
- `transport/http/error.rs` — add `ApiError::LogIngest(#[from]
  logging_utils::IngestorError)` + `log_ingest_status` +
  `log_ingest_code`
- `transport/http/openapi.rs` — register `LogIngestRequest` /
  `LogIngestResponse` in `components(schemas(...))`, add
  `tags("log-ingest")`, add the bearer security annotation

### Wire shape

Request (snake_case Rust → camelCase wire):

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestRequest {
    pub batch_id: String,
    pub entries: Vec<String>,
}
```

Response:

```rust
#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct LogIngestResponse {
    pub batch_id: String,
    pub accepted: usize,
}
```

### Validation

- `entries.is_empty()` → `400 Bad Request` with code
  `validation_failed` (silent drop at the writer is a client bug worth
  surfacing).
- `entries.len() > 10_000` → `400 Bad Request` with code
  `validation_failed` (matches `LogIngestorConfig`'s default
  `channel_capacity` order of magnitude; one runaway client cannot
  fill the bounded queue in a single shot).

Validation failures render through the existing `ApiError::Validation`
arm if present, otherwise a new arm.

### Error mapping

| `IngestorError` | HTTP | `code` |
|---|---|---|
| `DuplicateBatchId` | 409 Conflict | `duplicate_batch_id` |
| `ChannelFull` | 503 Service Unavailable | `channel_full` |
| `ShutDown` | 503 Service Unavailable | `ingestor_shut_down` |
| `WriterJoinTimeout` | 500 Internal Server Error | `writer_join_timeout` |
| `WriterPanic` | 500 Internal Server Error | `writer_panic` |
| `CreateDir` | 500 Internal Server Error | `log_dir_create_failed` |

### Tests

- **Handler unit tests** (`log_ingest/handlers.rs::tests`): drive
  `tower::ServiceExt::oneshot` against an `Arc<LogIngestor>` built
  with `tempfile::tempdir()`. Cases:
  - valid submit → `200`, file contains entries, response count
    matches
  - empty `entries` → `400 validation_failed`, no file write
  - > 10 000 entries → `400 validation_failed`, no file write
  - duplicate `batch_id` → `409 duplicate_batch_id`
  - missing bearer → `401` (the `AuthClaims` extractor handles this)
- **Router-level tests** (`log_ingest/router.rs::tests`): extend the
  existing openapi assertion to confirm
  `/api/log-ingest/submit` appears in `/api-docs/openapi.json`.
- **No live-DB test** — the endpoint does not touch Postgres.

Add `tempfile` as a dev-dependency on `aegis-server`.

## Client side — `aegis-desktop`

### Rust shim

A new Tauri command
`src-tauri/src/commands/submit_client_logs.rs` that:

1. Reads the request JSON (`{ batch_id, entries }`).
2. POSTs it to `${server_url}/api/log-ingest/submit` with the
   current access token in `Authorization: Bearer …`.
3. Maps non-2xx to a typed `ApiError` variant
   `LogIngestSubmitFailed { status, code }`.

Shimmed over `src-tauri/src/http/log_ingest.rs` (1:1 mirror of the
Rust HTTP client pattern used elsewhere in `src-tauri/src/http/`).

### Where it gets called

The desktop's client-log buffering path (the one in scope for the
ingestor) calls `submit_client_logs` on each flush. Generate
`batch_id` per submission (uuid v4 or monotonic counter — chosen at
implementation time). Existing client-side batching is the natural
extension point.

### OpenAPI / typed contract

Add `LogIngestRequest` / `LogIngestResponse` mirrors to
`src/shared/api/types.ts`, hand-duplicated per the repo's "wire DTOs
duplicated by hand" convention.

### Tests

- A Rust unit test on `submit_client_logs` that asserts the request
  shape (bearer header, JSON body) using a fake `reqwest::Client`
  mock.
- A TS unit test on the call site that asserts `batch_id` is generated
  per flush and the entries are the buffered ones.

## Out of scope (explicitly NOT in this PR)

- Per-batch idempotency-key header — `batchId` already covers it.
- Compression, archival, log shipping — plain daily-rotated files.
- Multi-tenant routing or per-`batch_id` filtering — single tenant by
  design.
- Auth beyond the existing bearer — no API-key / per-client
  credentials.
- New ingestor knobs as env vars — `cache_capacity`, `cache_ttl`,
  `channel_capacity`, `shutdown_deadline` stay at `LogIngestorConfig`
  defaults until a real reason to expose them appears.
- A separate "incoming" `LogIngestor` per source / per client — one
  ingestor per process; the on-disk file's prefix (`aegis-desktop.log`)
  is the only client identifier.
- Server-side retries of `503 channel_full` — clients retry on their
  own schedule; the server stays a dumb pipe.

## Verification gate

```bash
cargo test  -p aegis-server
cargo clippy -p aegis-server --all-targets --all-features -- -D warnings
cargo test  -p aegis-desktop
cargo clippy -p aegis-desktop --all-targets --all-features -- -D warnings
pnpm --filter aegis-desktop typecheck
pnpm --filter aegis-desktop test
cargo test --workspace
cargo check --workspace
```

## Acceptance criteria

1. `POST /api/log-ingest/submit` is mounted under `/api/log-ingest` and
   registered in `/api-docs/openapi.json` with the bearer lock icon.
2. Valid submission lands in the daily-rotated file under
   `AEGIS_LOG_INGEST_DIR` with the configured prefix, one entry per
   line, no `BATCH` header (per the previous spec's writer format).
3. Duplicate `batch_id` returns `409 duplicate_batch_id` (no file
   write, since the cache short-circuits before the channel).
4. Empty / over-cap `entries` returns `400 validation_failed`.
5. Missing / invalid bearer returns `401`.
6. `aegis-desktop` flush path calls the new endpoint with the
   access token in `Authorization`; non-2xx maps to a typed
   `ApiError::LogIngestSubmitFailed`.
7. All tests + clippy + fmt + workspace check pass.
