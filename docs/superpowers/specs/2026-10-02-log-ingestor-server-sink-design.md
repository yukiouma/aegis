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

The ingestor stays single-tenant; the server runs one ingestor per
process and writes to one daily-rotated file with the configured
prefix. The server's own `tracing` log file (`aegis-server.log`) and
the ingestor's client-log file (`aegis-desktop.log`) coexist under
the same `AEGIS_LOG_DIR` — `tracing_appender::rolling::daily` separates
them by file-name prefix.

## Server side — `aegis-server`

### Configuration — `Config`

Add one field:

```rust
/// File-name prefix for the server's daily-rotated client-log file.
/// The ingestor writes under `AEGIS_LOG_DIR` (the same dir as the
/// server's tracing logs) and uses this prefix to keep the two
/// streams separate on disk.
pub log_ingest_prefix: String,
```

Read one new env var in `Config::from_env`, defaulting when unset:

| Var | Default | Maps to |
|---|---|---|
| `AEGIS_LOG_INGEST_PREFIX` | `aegis-desktop.log` | `Config.log_ingest_prefix` |

**Reuse `AEGIS_LOG_DIR` directly.** Both the server's `init_tracing`
call (existing, `run.rs`) and the new `LogIngestor` read it from the
same env var — no parallel var. The per-source separation comes from
the file-name prefix (`aegis-server.log` vs `aegis-desktop.log`).

Other ingestor knobs (`cache_capacity`, `cache_ttl`, `channel_capacity`,
`shutdown_deadline`) use `LogIngestorConfig::builder` defaults. No new
env vars for them — YAGNI; expose them when a real knob is needed.

### State — `AppState`

Add `pub log_ingestor: Arc<logging_utils::LogIngestor>`. The `Arc` is
required because `LogIngestor` is `!Clone`. Tests construct a real
`LogIngestor::new(tempdir)` against a `tempfile::tempdir()`; the
test-support null pattern is **not** extended — `LogIngestor` is
straightforward to build per-test.

### Construction — `run.rs`

`AEGIS_LOG_DIR` is already read inline in `run.rs` for
`init_tracing`. Reuse the same `log_dir` for the ingestor so the two
log streams share one dir and rely on the per-source prefix for
separation:

```rust
let log_ingest = logging_utils::LogIngestorConfig::builder()
    .log_dir(std::path::PathBuf::from(&log_dir))
    .file_name_prefix(config.log_ingest_prefix.clone())
    .build();
let log_ingestor = Arc::new(
    logging_utils::LogIngestor::new(log_ingest)
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
- `Cargo.toml` — add `flate2 = { workspace = true }` (no new
  workspace dep — add the entry to `[workspace.dependencies]` if it
  isn't there yet).
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

### Content-Encoding: gzip

The endpoint accepts an optional `Content-Encoding: gzip` request
header. When present, the body is gzip-compressed JSON; the server
decompresses before parsing. Symmetric to the most common HTTP
client (`reqwest`'s `.send()` + `.gzip(true)`-equivalent). Uncompressed
JSON remains the default — clients that don't send the header work
unchanged.

**Wire flow:**
1. Read raw `Bytes` body (axum's default `Json` extractor does not
   read raw bytes; the handler takes `Bytes` directly).
2. If `Content-Encoding: gzip` is set, decompress via
   `flate2::read::GzDecoder` into a `Vec<u8>`. Anything else
   (unknown encoding, malformed gzip) → `400 validation_failed`.
3. Parse the decompressed bytes as `LogIngestRequest`. JSON parse
   errors → `400 validation_failed`.
4. Run the per-field validation (empty / over-cap) on the parsed
   struct.

**Validation cap is post-decompression.** The 10 000-entry cap is
applied to the decompressed `entries` vector, so a compressed batch
of 9 999 entries that decompresses to 10 001 → `400`, not a
decompression-time error. The wire-level body size is bounded by
axum's default 2 MB limit, well below zip-bomb territory for the
documented entry sizes.

**Failures:**
- `Content-Encoding: gzip` with malformed gzip body → `400
  validation_failed` (code `validation_failed`, message includes
  "gzip decode").
- `Content-Encoding: <anything other than identity>` (e.g.
  `deflate`, `br`) → `400 validation_failed` (only `gzip` is
  recognised).
- Missing / `identity` header → use bytes as-is (current behaviour).

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

## Out of scope (explicitly NOT in this PR)

- `aegis-desktop` client-side wiring — the desktop will call this
  endpoint from a separate work thread in a follow-up PR, not via a
  Tauri command. The endpoint is built and exercised by tests in this
  PR; no desktop integration yet.
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
cargo test --workspace
cargo check --workspace
```

## Acceptance criteria

1. `POST /api/log-ingest/submit` is mounted under `/api/log-ingest` and
   registered in `/api-docs/openapi.json` with the bearer lock icon.
2. Valid submission lands in the daily-rotated file under
   `AEGIS_LOG_DIR` with prefix `AEGIS_LOG_INGEST_PREFIX` (default
   `aegis-desktop.log`), one entry per line, no `BATCH` header (per
   the previous spec's writer format).
3. Duplicate `batch_id` returns `409 duplicate_batch_id` (no file
   write, since the cache short-circuits before the channel).
4. Empty / over-cap `entries` returns `400 validation_failed`.
5. Missing / invalid bearer returns `401`.
6. `Content-Encoding: gzip` requests decompress successfully and
   route through the same validation + ingestor path; malformed gzip
   and unrecognised encodings return `400 validation_failed`.
7. All tests + clippy + fmt + workspace check pass.
