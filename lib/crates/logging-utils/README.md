# `logging-utils`

Unified tracing bootstrap + trace-id generator for `aegis-server` and
`aegis-desktop`, plus the `LogIngestor` for `aegis-desktop`'s
client log submissions. Centralises the JSON daily-rotating file
appender, the `AEGIS_LOG_LEVEL` env filter, and the `C-` / `S-` trace
id format so neither app has to re-implement it.

> **Note on duplication:** `lib/crates/logging-utils::trace_id` and
> `lib/crates/trace-id` carry the same `TraceIdGenerator` / `Side`
> logic independently. The two crates do NOT depend on each other —
> both run their own copy of the logic. A future PR may consolidate.

## Modules

- `logging_utils::tracing_init` — `LoggingConfig`, `init_tracing`,
  `LogGuard`, `LoggingInitError`, `build_filter`.
- `logging_utils::trace_id` — `Side`, `TraceIdGenerator`
  (independent copy of the same logic that lives in
  `lib/crates/trace-id`).
- `logging_utils::log_ingestor` — `LogIngestor`,
  `LogIngestorConfig` (and builder), `IngestorError`. **For
  `aegis-desktop` only** — `aegis-server` has its own structured
  log pipeline.

## Usage

```rust
use logging_utils::{LoggingConfig, init_tracing};

// Server:
let dir = std::env::var("AEGIS_LOG_DIR").unwrap_or_else(|_| "./logs".into());
let _guard = init_tracing(&LoggingConfig {
    log_dir: dir.into(),
    file_name_prefix: "aegis-server.log".into(),
})?;

// Desktop:
let _guard = init_tracing(&LoggingConfig {
    log_dir: log_dir.into(),
    file_name_prefix: "aegis-desktop.log".into(),
})?;
```

`LogGuard` MUST be held for the lifetime of the program (drop it and
buffered writes are lost). `init_tracing` is idempotent — a second
call is a no-op.

## Trace ids

```rust
use logging_utils::{Side, TraceIdGenerator};

let generator = TraceIdGenerator::new(Some("desktop".to_string()));
let client_id = generator.client_side();   // e.g. "C-desktop-01H…"
let server_id = generator.server_side();   // e.g. "S-desktop-01H…"
```

Construct the generator once per process and reuse it; the generator
is stateless apart from `device_prefix`.

## Log ingestor (aegis-desktop only)

```rust
use logging_utils::{LogIngestor, LogIngestorConfig};
use std::time::Duration;

let ingestor = LogIngestor::new(
    LogIngestorConfig::builder()
        .log_dir("./logs".into())
        .file_name_prefix("desktop-events".into())
        .cache_capacity(10_000)              // default
        .cache_ttl(Duration::from_secs(600))  // default
        .channel_capacity(1_000)              // default
        .build(),
)?;

ingestor.submit("batch-123", &[
    "INFO  request_id=abc status=200".to_string(),
    "WARN  request_id=def retry=2".to_string(),
])?;

ingestor.shutdown()?;  // drains pending + joins writer; also runs in Drop
```

The ingestor is **for `aegis-desktop` only** — `aegis-server`'s log
pipeline is intentionally excluded. Each `batch_id` is accepted at
most once within the cache TTL (default 600 s); repeated IDs return
`IngestorError::DuplicateBatchId`. When the bounded channel (default
1 000 envelopes) is full, `submit` returns `IngestorError::ChannelFull`
and the just-inserted cache entry is invalidated so the same
`batch_id` can be retried once the writer drains. See
`docs/superpowers/specs/2026-10-02-log-ingestor-design.md` for the
full design.

## Verification

```bash
cargo test  -p logging-utils
cargo clippy -p logging-utils --all-targets --all-features -- -D warnings
cargo doc   -p logging-utils --no-deps
cargo check --workspace
```
